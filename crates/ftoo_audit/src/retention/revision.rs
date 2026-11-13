//! Resolved Git identity, independent of lock formatting and unrelated packages.
use std::fmt;
use toml_edit::DocumentMut;

const BUILD_LOCK: &str = include_str!("../../../../Cargo.lock");
const PACKAGE: &str = "nrz-cas";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Source {
    BuildLock,
    CurrentLock,
    CargoManifest,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reason {
    Missing,
    Unreadable {
        kind: std::io::ErrorKind,
        raw_os_error: Option<i32>,
    },
    Malformed {
        message: String,
        span: Option<std::ops::Range<usize>>,
    },
    Ambiguous {
        entries: Vec<(String, String)>,
    },
    Unsupported {
        value: String,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unavailable {
    pub source: Source,
    pub path: &'static str,
    pub package: &'static str,
    pub reason: Reason,
}
fn failure(source: Source, reason: Reason) -> Unavailable {
    Unavailable {
        source,
        path: if source == Source::CargoManifest {
            "crates/ftoo_audit/Cargo.toml"
        } else {
            "Cargo.lock"
        },
        package: PACKAGE,
        reason,
    }
}
fn parse(text: &str, source: Source) -> Result<DocumentMut, Unavailable> {
    text.parse::<DocumentMut>().map_err(|error| {
        failure(
            source,
            Reason::Malformed {
                message: error.to_string(),
                span: error.span(),
            },
        )
    })
}
fn valid(revision: &str) -> bool {
    revision.len() == 40
        && revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
pub(super) fn resolved(text: &str, source: Source) -> Result<String, Unavailable> {
    let document = parse(text, source)?;
    let packages = document
        .get("package")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| failure(source, Reason::Missing))?;
    let entries: Vec<_> = packages
        .iter()
        .filter(|package| package.get("name").and_then(toml_edit::Item::as_str) == Some(PACKAGE))
        .collect();
    let entry = match entries.as_slice() {
        [] => return Err(failure(source, Reason::Missing)),
        [entry] => *entry,
        _ => {
            return Err(failure(
                source,
                Reason::Ambiguous {
                    entries: entries
                        .iter()
                        .map(|entry| {
                            (
                                entry
                                    .get("version")
                                    .and_then(toml_edit::Item::as_str)
                                    .unwrap_or("")
                                    .into(),
                                entry
                                    .get("source")
                                    .and_then(toml_edit::Item::as_str)
                                    .unwrap_or("")
                                    .into(),
                            )
                        })
                        .collect(),
                },
            ));
        }
    };
    let item = entry
        .get("source")
        .ok_or_else(|| failure(source, Reason::Missing))?;
    let value = item.as_str().ok_or_else(|| {
        failure(
            source,
            Reason::Unsupported {
                value: item
                    .span()
                    .and_then(|span| text.get(span))
                    .unwrap_or(text)
                    .into(),
            },
        )
    })?;
    if let Some((prefix, revision)) = value.rsplit_once('#')
        && prefix.starts_with("git+")
        && !prefix.contains('#')
        && valid(revision)
    {
        return Ok(revision.into());
    }
    Err(failure(
        source,
        Reason::Unsupported {
            value: value.into(),
        },
    ))
}
pub(super) fn manifest(text: &str) -> Result<String, Unavailable> {
    let source = Source::CargoManifest;
    let document = parse(text, source)?;
    let entry = document
        .get("dependencies")
        .and_then(|item| item.get(PACKAGE))
        .ok_or_else(|| failure(source, Reason::Missing))?;
    let revision = entry.get("rev").and_then(toml_edit::Item::as_str);
    let git = entry.get("git").and_then(toml_edit::Item::as_str);
    if let (Some(revision), Some(_)) = (revision, git)
        && valid(revision)
        && entry.get("path").is_none()
    {
        return Ok(revision.into());
    }
    Err(failure(
        source,
        Reason::Unsupported {
            value: entry
                .span()
                .and_then(|span| text.get(span))
                .unwrap_or(text)
                .into(),
        },
    ))
}
pub(super) fn linked() -> Result<String, Unavailable> {
    resolved(BUILD_LOCK, Source::BuildLock)
}
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization is infallible")
}
impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = match self.source {
            Source::BuildLock => "build-lock",
            Source::CurrentLock => "current-lock",
            Source::CargoManifest => "cargo-manifest",
        };
        write!(
            f,
            "retention provenance M061 at file {}: engine-revision-unavailable source={source} package={} reason=",
            quoted(self.path),
            quoted(self.package)
        )?;
        match &self.reason {
            Reason::Missing => f.write_str("missing"),
            Reason::Unreadable { kind, raw_os_error } => {
                write!(
                    f,
                    "unreadable kind={} raw-os-error=",
                    quoted(&kind.to_string())
                )?;
                match raw_os_error {
                    Some(code) => write!(f, "{code}"),
                    None => f.write_str("none"),
                }
            }
            Reason::Malformed { message, span } => {
                write!(f, "malformed message={} span=", quoted(message))?;
                match span {
                    None => f.write_str("none"),
                    Some(span) => write!(f, "{}..{}", span.start, span.end),
                }
            }
            Reason::Unsupported { value } => write!(f, "unsupported value={}", quoted(value)),
            Reason::Ambiguous { entries } => {
                f.write_str("ambiguous entries=[")?;
                for (i, (version, source)) in entries.iter().enumerate() {
                    if i != 0 {
                        f.write_str(",")?;
                    }
                    write!(
                        f,
                        "{{\"version\":{},\"source\":{}}}",
                        quoted(version),
                        quoted(source)
                    )?;
                }
                f.write_str("]")
            }
        }
    }
}
