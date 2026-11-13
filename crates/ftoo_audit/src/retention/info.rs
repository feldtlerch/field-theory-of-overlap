//! Informational build context is never a required-equal input or acceptance key.
use super::{syntax::Value, wire};
use std::collections::{BTreeSet, VecDeque};

const MAX_INFO: usize = 2 * 1024 * 1024;
const LOCK: &str = include_str!("../../../../Cargo.lock");
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Run {
    Retained,
    Fresh1,
    Fresh2,
}
impl Run {
    pub(super) const fn token(self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::Fresh1 => "fresh-1",
            Self::Fresh2 => "fresh-2",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unavailable {
    NotCaptured,
    ReadFailed,
    ParseFailed,
    Ambiguous,
    TooLarge,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Compiler {
    Available {
        release: String,
        commit_hash: String,
    },
    Unavailable(Unavailable),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Closure {
    Available(Vec<Package>),
    Unavailable(Unavailable),
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identity {
    pub name: String,
    pub version: String,
    pub source: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    pub identity: Identity,
    pub checksum: String,
    pub dependencies: Vec<Identity>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InfoRecord {
    pub run: Run,
    pub compiler: Compiler,
    pub closure: Closure,
}
fn s(value: &str) -> Value {
    Value::String(value.into())
}
fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
impl Unavailable {
    const fn token(self) -> &'static str {
        match self {
            Self::NotCaptured => "not-captured",
            Self::ReadFailed => "read-failed",
            Self::ParseFailed => "parse-failed",
            Self::Ambiguous => "ambiguous",
            Self::TooLarge => "too-large",
        }
    }
    fn value(self) -> Value {
        object([("state", s("unavailable")), ("reason", s(self.token()))])
    }
}
impl Identity {
    fn value(&self) -> Value {
        object([
            ("name", s(&self.name)),
            ("version", s(&self.version)),
            ("source", s(&self.source)),
        ])
    }
}
impl InfoRecord {
    pub(super) fn collect(run: Run) -> Self {
        if run == Run::Retained {
            return Self {
                run,
                compiler: Compiler::Unavailable(Unavailable::NotCaptured),
                closure: Closure::Unavailable(Unavailable::NotCaptured),
            };
        }
        let compiler = match (
            option_env!("FTOO_BUILD_RUSTC_RELEASE"),
            option_env!("FTOO_BUILD_RUSTC_COMMIT"),
        ) {
            (Some(release), Some(commit_hash)) if release.len() + commit_hash.len() <= MAX_INFO => {
                Compiler::Available {
                    release: release.into(),
                    commit_hash: commit_hash.into(),
                }
            }
            (Some(_), Some(_)) => Compiler::Unavailable(Unavailable::TooLarge),
            _ => Compiler::Unavailable(Unavailable::NotCaptured),
        };
        let closure = match closure(LOCK) {
            Ok(packages) => Closure::Available(packages),
            Err(reason) => Closure::Unavailable(reason),
        };
        let mut result = Self {
            run,
            compiler,
            closure,
        };
        if result.json().len() > MAX_INFO {
            result.closure = Closure::Unavailable(Unavailable::TooLarge);
            if result.json().len() > MAX_INFO {
                result.compiler = Compiler::Unavailable(Unavailable::TooLarge);
            }
        }
        result
    }
    pub(super) fn json(&self) -> String {
        let run = match self.run {
            Run::Retained => "retained",
            Run::Fresh1 => "fresh-1",
            Run::Fresh2 => "fresh-2",
        };
        let compiler = match &self.compiler {
            Compiler::Available {
                release,
                commit_hash,
            } => object([
                ("state", s("available")),
                ("release", s(release)),
                ("commit-hash", s(commit_hash)),
            ]),
            Compiler::Unavailable(reason) => reason.value(),
        };
        let closure = match &self.closure {
            Closure::Unavailable(reason) => reason.value(),
            Closure::Available(packages) => object([
                ("state", s("available")),
                (
                    "packages",
                    Value::Array(
                        packages
                            .iter()
                            .map(|package| {
                                object([
                                    ("name", s(&package.identity.name)),
                                    ("version", s(&package.identity.version)),
                                    ("source", s(&package.identity.source)),
                                    ("checksum", s(&package.checksum)),
                                    (
                                        "dependencies",
                                        Value::Array(
                                            package
                                                .dependencies
                                                .iter()
                                                .map(Identity::value)
                                                .collect(),
                                        ),
                                    ),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
        };
        let mut output = String::new();
        wire::render(
            &object([
                ("run", s(run)),
                ("compiler", compiler),
                ("closure", closure),
            ]),
            &mut output,
        );
        output
    }
}
impl std::fmt::Display for InfoRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.json())
    }
}
fn closure(text: &str) -> Result<Vec<Package>, Unavailable> {
    if text.len() > MAX_INFO {
        return Err(Unavailable::TooLarge);
    }
    let document = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| Unavailable::ParseFailed)?;
    let tables = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or(Unavailable::ParseFailed)?;
    let mut packages = Vec::new();
    let mut edges = Vec::new();
    for table in tables {
        let field = |key| {
            table
                .get(key)
                .and_then(toml_edit::Item::as_str)
                .map(str::to_owned)
                .ok_or(Unavailable::ParseFailed)
        };
        let identity = Identity {
            name: field("name")?,
            version: field("version")?,
            source: table
                .get("source")
                .and_then(toml_edit::Item::as_str)
                .unwrap_or("workspace")
                .into(),
        };
        let checksum = if identity.source.starts_with("registry+") {
            table
                .get("checksum")
                .and_then(toml_edit::Item::as_str)
                .unwrap_or("")
                .into()
        } else {
            String::new()
        };
        let dependencies = if let Some(value) = table.get("dependencies") {
            value
                .as_array()
                .ok_or(Unavailable::ParseFailed)?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or(Unavailable::ParseFailed)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        packages.push(Package {
            identity,
            checksum,
            dependencies: Vec::new(),
        });
        edges.push(dependencies);
    }
    selected(&packages, &edges)
}
fn selected(packages: &[Package], edges: &[Vec<String>]) -> Result<Vec<Package>, Unavailable> {
    let resolve = |text: &str| -> Result<usize, Unavailable> {
        let (head, source) = match text.split_once(" (") {
            Some((head, tail)) => (
                head,
                Some(tail.strip_suffix(')').ok_or(Unavailable::ParseFailed)?),
            ),
            None => (text, None),
        };
        let words: Vec<_> = head.split_whitespace().collect();
        let (name, version) = match words.as_slice() {
            [name] => (*name, None),
            [name, version] => (*name, Some(*version)),
            _ => return Err(Unavailable::ParseFailed),
        };
        let found: Vec<_> = packages
            .iter()
            .enumerate()
            .filter(|(_, package)| {
                package.identity.name == name
                    && version.is_none_or(|value| value == package.identity.version)
                    && source.is_none_or(|value| value == package.identity.source)
            })
            .map(|(i, _)| i)
            .collect();
        match found.as_slice() {
            [index] => Ok(*index),
            [] => Err(Unavailable::ParseFailed),
            _ => Err(Unavailable::Ambiguous),
        }
    };
    let mut queue = VecDeque::from([resolve("ftoo_audit")?, resolve("ftoo_manifest")?]);
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(index) = queue.pop_front() {
        if !seen.insert(index) {
            continue;
        }
        let mut package = packages[index].clone();
        for edge in &edges[index] {
            let index = resolve(edge)?;
            queue.push_back(index);
            package.dependencies.push(packages[index].identity.clone());
        }
        package.dependencies.sort();
        result.push(package);
    }
    result.sort_by(|a, b| a.identity.cmp(&b.identity));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    const LOCK: &str = "[[package]]\nname='ftoo_audit'\nversion='0'\ndependencies=['dep 2 (registry+example)', 'ftoo_manifest']\n[[package]]\nname='ftoo_manifest'\nversion='0'\n[[package]]\nname='dep'\nversion='1'\nsource='registry+example'\nchecksum='old'\n[[package]]\nname='dep'\nversion='2'\nsource='registry+example'\nchecksum='new'\n";
    fn identity(name: &str, version: &str, source: &str) -> Identity {
        Identity {
            name: name.into(),
            version: version.into(),
            source: source.into(),
        }
    }
    #[test]
    fn resolved_closure_is_sorted_and_ambiguous_edges_are_unavailable() {
        let dependency = identity("dep", "2", "registry+example");
        let manifest = identity("ftoo_manifest", "0", "workspace");
        let expected = vec![
            Package {
                identity: dependency.clone(),
                checksum: "new".into(),
                dependencies: vec![],
            },
            Package {
                identity: identity("ftoo_audit", "0", "workspace"),
                checksum: String::new(),
                dependencies: vec![dependency, manifest.clone()],
            },
            Package {
                identity: manifest,
                checksum: String::new(),
                dependencies: vec![],
            },
        ];
        assert_eq!(closure(LOCK), Ok(expected.clone()));
        assert_eq!(
            closure(&LOCK.replace("dep 2 (registry+example)", "dep")),
            Err(Unavailable::Ambiguous)
        );
        assert_eq!(closure(LOCK), Ok(expected.clone()));
        assert_eq!(closure("["), Err(Unavailable::ParseFailed));
        assert_eq!(closure(LOCK), Ok(expected.clone()));
        assert_eq!(
            closure(&" ".repeat(MAX_INFO + 1)),
            Err(Unavailable::TooLarge)
        );
        assert_eq!(closure(LOCK), Ok(expected));
        let record = InfoRecord {
            run: Run::Fresh1,
            compiler: Compiler::Unavailable(Unavailable::NotCaptured),
            closure: Closure::Unavailable(Unavailable::Ambiguous),
        };
        assert_eq!(
            record.to_string(),
            "{\"run\":\"fresh-1\",\"compiler\":{\"state\":\"unavailable\",\"reason\":\"not-captured\"},\"closure\":{\"state\":\"unavailable\",\"reason\":\"ambiguous\"}}"
        );
    }
}
