//! Local, uncached integrity admission before any executable is trusted.
use super::{AdapterComponent, NotationAdapterFault, SupportReason};
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub(super) const MANIFEST: &[u8] = include_bytes!("../../../../tools/notation/package.json");
pub(super) const LOCK: &[u8] = include_bytes!("../../../../tools/notation/package-lock.json");
pub(super) const DIGESTS: &[u8] = include_bytes!("../../../../tools/notation/support-digests.json");
pub(super) const NODE_PIN: &str = include_str!("../../../../tools/notation/.node-version");
const METADATA_BYTES: usize = 1_048_576;
const METADATA_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportIoOperation {
    Canonicalize,
    Inspect,
    ListDirectory,
    Read,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportCopy {
    Manifest,
    Lockfile,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportEntryKind {
    Symlink,
    Other,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledPackageChange {
    pub package: String,
    pub expected: String,
    pub actual: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportMetadataReason {
    InvalidUtf8 {
        offset: usize,
        error_len: Option<usize>,
    },
    Bom,
    Syntax {
        message: String,
        line: usize,
        column: usize,
    },
    DuplicateKey {
        key: String,
    },
    NonInteger {
        token: String,
    },
    UnknownField {
        name: String,
    },
    MissingField {
        name: String,
    },
    WrongType {
        expected: String,
        actual: String,
    },
    InvalidValue {
        expected: String,
        actual: String,
    },
    DuplicateEntry {
        key: String,
    },
    OutOfOrder {
        previous: String,
        current: String,
    },
    SizeLimit {
        limit: usize,
        actual: usize,
    },
    DepthLimit {
        limit: usize,
        actual: usize,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportIntegrityFault {
    SupportIo {
        file: PathBuf,
        operation: SupportIoOperation,
        kind: io::ErrorKind,
    },
    CopyMismatch {
        copy: SupportCopy,
        external: PathBuf,
        committed: String,
        first_difference: usize,
        expected_length: usize,
        actual_length: usize,
    },
    Metadata {
        file: PathBuf,
        path: String,
        reason: SupportMetadataReason,
    },
    InstalledTreeMismatch {
        file: PathBuf,
        missing: Vec<String>,
        unexpected: Vec<String>,
        changed: Vec<InstalledPackageChange>,
    },
    FileInventoryMismatch {
        directory: PathBuf,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    UnexpectedSupportEntry {
        file: PathBuf,
        kind: SupportEntryKind,
    },
    DigestMismatch {
        file: PathBuf,
        expected: String,
        actual: String,
    },
    UnsupportedPlatform {
        os: String,
        architecture: String,
    },
}
impl fmt::Display for SupportIntegrityFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
type Result<T> = std::result::Result<T, SupportIntegrityFault>;
fn io_fault(
    file: &Path,
    operation: SupportIoOperation,
    error: &io::Error,
) -> SupportIntegrityFault {
    SupportIntegrityFault::SupportIo {
        file: file.into(),
        operation,
        kind: error.kind(),
    }
}
fn metadata(file: &Path, path: &str, reason: SupportMetadataReason) -> SupportIntegrityFault {
    SupportIntegrityFault::Metadata {
        file: file.into(),
        path: path.into(),
        reason,
    }
}
fn syntax(file: &Path, path: &str, e: &serde_json::Error) -> SupportIntegrityFault {
    metadata(
        file,
        path,
        SupportMetadataReason::Syntax {
            message: e.to_string(),
            line: e.line(),
            column: e.column(),
        },
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Value {
    Object(BTreeMap<String, Self>),
    Array(Vec<Self>),
    String(String),
    Integer(String),
    Bool(bool),
    Null,
}
impl Value {
    fn kind(&self) -> &str {
        match self {
            Self::Object(_) => "object",
            Self::Array(_) => "array",
            Self::String(_) => "string",
            Self::Integer(_) => "integer",
            Self::Bool(_) => "boolean",
            Self::Null => "null",
        }
    }
    fn canonical(&self) -> String {
        match self {
            Self::Object(fields) => format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("string serialization"),
                        value.canonical()
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Self::canonical)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::String(value) => serde_json::to_string(value).expect("string serialization"),
            Self::Integer(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Null => "null".into(),
        }
    }
}
struct Container<'a> {
    file: &'a Path,
    path: &'a str,
    fault: &'a mut Option<SupportIntegrityFault>,
}
impl<'de> Visitor<'de> for Container<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON container")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<Value, M::Error> {
        let mut fields = BTreeMap::new();
        while let Some(key) = map.next_key::<String>()? {
            let raw = map.next_value::<&RawValue>()?;
            let result = if fields.contains_key(&key) {
                Err(metadata(
                    self.file,
                    self.path,
                    SupportMetadataReason::DuplicateKey { key: key.clone() },
                ))
            } else {
                walk(self.file, &format!("{}.{key}", self.path), raw)
            };
            match result {
                Ok(value) => {
                    fields.insert(key, value);
                }
                Err(e) => {
                    *self.fault = Some(e);
                    return Err(serde::de::Error::custom("strict metadata refusal"));
                }
            }
        }
        Ok(Value::Object(fields))
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> std::result::Result<Value, S::Error> {
        let mut values = vec![];
        while let Some(raw) = seq.next_element::<&RawValue>()? {
            match walk(self.file, &format!("{}[{}]", self.path, values.len()), raw) {
                Ok(value) => values.push(value),
                Err(e) => {
                    *self.fault = Some(e);
                    return Err(serde::de::Error::custom("strict metadata refusal"));
                }
            }
        }
        Ok(Value::Array(values))
    }
}
fn walk(file: &Path, path: &str, raw: &RawValue) -> Result<Value> {
    let token = raw.get();
    match token.as_bytes()[0] {
        b'{' | b'[' => {
            let mut fault = None;
            let visitor = Container {
                file,
                path,
                fault: &mut fault,
            };
            let mut de = serde_json::Deserializer::from_str(token);
            let value = if token.starts_with('{') {
                de.deserialize_map(visitor)
            } else {
                de.deserialize_seq(visitor)
            };
            if let Some(fault) = fault {
                return Err(fault);
            }
            value.map_err(|e| syntax(file, path, &e))
        }
        b'"' => serde_json::from_str(token)
            .map(Value::String)
            .map_err(|e| syntax(file, path, &e)),
        b't' => Ok(Value::Bool(true)),
        b'f' => Ok(Value::Bool(false)),
        b'n' => Ok(Value::Null),
        _ => {
            if token.contains(['.', 'e', 'E']) {
                Err(metadata(
                    file,
                    path,
                    SupportMetadataReason::NonInteger {
                        token: token.into(),
                    },
                ))
            } else {
                Ok(Value::Integer(token.into()))
            }
        }
    }
}
pub(super) fn parse(file: &Path, bytes: &[u8]) -> Result<Value> {
    if bytes.len() > METADATA_BYTES {
        return Err(metadata(
            file,
            "$",
            SupportMetadataReason::SizeLimit {
                limit: METADATA_BYTES,
                actual: METADATA_BYTES + 1,
            },
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| {
        metadata(
            file,
            "$",
            SupportMetadataReason::InvalidUtf8 {
                offset: e.valid_up_to(),
                error_len: e.error_len(),
            },
        )
    })?;
    if text.starts_with('\u{feff}') {
        return Err(metadata(file, "$", SupportMetadataReason::Bom));
    }
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for byte in text.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > METADATA_DEPTH {
                        return Err(metadata(
                            file,
                            "$",
                            SupportMetadataReason::DepthLimit {
                                limit: METADATA_DEPTH,
                                actual: METADATA_DEPTH + 1,
                            },
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let mut de = serde_json::Deserializer::from_str(text);
    let raw = <&RawValue>::deserialize(&mut de).map_err(|e| syntax(file, "$", &e))?;
    let value = walk(file, "$", raw)?;
    de.end().map_err(|e| syntax(file, "$", &e))?;
    Ok(value)
}

fn wrong(file: &Path, path: &str, expected: &str, actual: &Value) -> SupportIntegrityFault {
    metadata(
        file,
        path,
        SupportMetadataReason::WrongType {
            expected: expected.into(),
            actual: actual.kind().into(),
        },
    )
}
fn object<'a>(
    file: &Path,
    path: &str,
    value: &'a Value,
    fields: &[&str],
) -> Result<&'a BTreeMap<String, Value>> {
    let Value::Object(map) = value else {
        return Err(wrong(file, path, "object", value));
    };
    for key in map.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(metadata(
                file,
                path,
                SupportMetadataReason::UnknownField { name: key.clone() },
            ));
        }
    }
    for key in fields {
        if !map.contains_key(*key) {
            return Err(metadata(
                file,
                path,
                SupportMetadataReason::MissingField {
                    name: (*key).into(),
                },
            ));
        }
    }
    Ok(map)
}
fn string<'a>(file: &Path, path: &str, value: &'a Value) -> Result<&'a str> {
    match value {
        Value::String(s) => Ok(s),
        _ => Err(wrong(file, path, "string", value)),
    }
}
fn array<'a>(file: &Path, path: &str, value: &'a Value) -> Result<&'a [Value]> {
    match value {
        Value::Array(a) => Ok(a),
        _ => Err(wrong(file, path, "array", value)),
    }
}
fn equal(file: &Path, path: &str, actual: &Value, expected: &Value) -> Result<()> {
    if actual.kind() != expected.kind() {
        return Err(wrong(file, path, expected.kind(), actual));
    }
    if actual != expected {
        return Err(metadata(
            file,
            path,
            SupportMetadataReason::InvalidValue {
                expected: expected.canonical(),
                actual: actual.canonical(),
            },
        ));
    }
    Ok(())
}
fn digest(file: &Path, path: &str, value: &Value) -> Result<String> {
    let s = string(file, path, value)?;
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(metadata(
            file,
            path,
            SupportMetadataReason::InvalidValue {
                expected: "64 lowercase hexadecimal characters".into(),
                actual: s.into(),
            },
        ));
    }
    Ok(s.into())
}
#[derive(Debug)]
pub(super) struct Inventory {
    pub packages: BTreeMap<String, String>,
    pub runtimes: BTreeMap<String, String>,
}
pub(super) fn inventory(bytes: &[u8]) -> Result<Inventory> {
    let file = Path::new("tools/notation/support-digests.json");
    let value = parse(file, bytes)?;
    let map = object(
        file,
        "$",
        &value,
        &[
            "schema",
            "algorithm",
            "node-version",
            "packages",
            "runtimes",
        ],
    )?;
    equal(
        file,
        "$.schema",
        &map["schema"],
        &Value::Integer("1".into()),
    )?;
    equal(
        file,
        "$.algorithm",
        &map["algorithm"],
        &Value::String("sha256".into()),
    )?;
    equal(
        file,
        "$.node-version",
        &map["node-version"],
        &Value::String(NODE_PIN.trim_end().into()),
    )?;
    let mut packages = BTreeMap::new();
    let mut previous: Option<&str> = None;
    for (i, value) in array(file, "$.packages", &map["packages"])?
        .iter()
        .enumerate()
    {
        let path = format!("$.packages[{i}]");
        let record = object(file, &path, value, &["path", "sha256"])?;
        let name = string(file, &format!("{path}.path"), &record["path"])?;
        if !["node_modules/katex/", "node_modules/commander/"]
            .iter()
            .any(|p| name.starts_with(p))
            || name.contains('\\')
            || name.split('/').any(|s| matches!(s, "" | "." | ".."))
        {
            return Err(metadata(
                file,
                &format!("{path}.path"),
                SupportMetadataReason::InvalidValue {
                    expected: "relative locked-package file path".into(),
                    actual: name.into(),
                },
            ));
        }
        let sha = digest(file, &format!("{path}.sha256"), &record["sha256"])?;
        if packages.insert(name.into(), sha).is_some() {
            return Err(metadata(
                file,
                &path,
                SupportMetadataReason::DuplicateEntry { key: name.into() },
            ));
        }
        if let Some(prior) = previous
            && prior.as_bytes() > name.as_bytes()
        {
            return Err(metadata(
                file,
                &path,
                SupportMetadataReason::OutOfOrder {
                    previous: prior.into(),
                    current: name.into(),
                },
            ));
        }
        previous = Some(name);
    }
    let runtimes = runtime_inventory(file, &map["runtimes"])?;
    Ok(Inventory { packages, runtimes })
}
fn runtime_inventory(file: &Path, value: &Value) -> Result<BTreeMap<String, String>> {
    let mut runtimes = BTreeMap::new();
    for (i, value) in array(file, "$.runtimes", value)?.iter().enumerate() {
        let path = format!("$.runtimes[{i}]");
        let r = object(
            file,
            &path,
            value,
            &[
                "platform",
                "release-artifact",
                "release-sha256",
                "executable-sha256",
            ],
        )?;
        equal(
            file,
            &format!("{path}.platform"),
            &r["platform"],
            &Value::String("linux-x64".into()),
        )?;
        equal(
            file,
            &format!("{path}.release-artifact"),
            &r["release-artifact"],
            &Value::String("node-v22.16.0-linux-x64.tar.xz".into()),
        )?;
        digest(
            file,
            &format!("{path}.release-sha256"),
            &r["release-sha256"],
        )?;
        let sha = digest(
            file,
            &format!("{path}.executable-sha256"),
            &r["executable-sha256"],
        )?;
        if runtimes.insert("linux-x64".into(), sha).is_some() {
            return Err(metadata(
                file,
                &path,
                SupportMetadataReason::DuplicateEntry {
                    key: "linux-x64".into(),
                },
            ));
        }
    }
    Ok(runtimes)
}

fn read(file: &Path) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    fs::File::open(file)
        .and_then(|f| f.take((METADATA_BYTES + 1) as u64).read_to_end(&mut bytes))
        .map_err(|e| io_fault(file, SupportIoOperation::Read, &e))?;
    Ok(bytes)
}
fn copy(file: &Path, kind: SupportCopy, expected: &[u8], committed: &str) -> Result<()> {
    let actual = read(file)?;
    if actual != expected {
        let length = fs::metadata(file)
            .map_err(|e| io_fault(file, SupportIoOperation::Inspect, &e))?
            .len();
        return Err(SupportIntegrityFault::CopyMismatch {
            copy: kind,
            external: file.into(),
            committed: committed.into(),
            first_difference: expected
                .iter()
                .zip(&actual)
                .position(|(a, b)| a != b)
                .unwrap_or(expected.len().min(actual.len())),
            expected_length: expected.len(),
            actual_length: usize::try_from(length).unwrap_or(usize::MAX),
        });
    }
    Ok(())
}
pub(super) fn installed(file: &Path, bytes: &[u8]) -> Result<()> {
    let actual = parse(file, bytes)?;
    let expected = parse(Path::new("tools/notation/package-lock.json"), LOCK)?;
    let fields = ["name", "version", "lockfileVersion", "requires", "packages"];
    let a = object(file, "$", &actual, &fields)?;
    let e = object(
        Path::new("tools/notation/package-lock.json"),
        "$",
        &expected,
        &fields,
    )?;
    for field in &fields[..4] {
        equal(file, &format!("$.{field}"), &a[*field], &e[*field])?;
    }
    let Value::Object(actual) = &a["packages"] else {
        return Err(wrong(file, "$.packages", "object", &a["packages"]));
    };
    let Value::Object(expected) = &e["packages"] else {
        unreachable!("admitted committed lock");
    };
    let mut expected = expected.clone();
    expected.remove("");
    let missing: Vec<_> = expected
        .keys()
        .filter(|key| !actual.contains_key(*key))
        .cloned()
        .collect();
    let unexpected: Vec<_> = actual
        .keys()
        .filter(|key| !expected.contains_key(*key))
        .cloned()
        .collect();
    let changed: Vec<_> = expected
        .iter()
        .filter_map(|(key, e)| {
            actual
                .get(key)
                .filter(|a| *a != e)
                .map(|a| InstalledPackageChange {
                    package: key.clone(),
                    expected: e.canonical(),
                    actual: a.canonical(),
                })
        })
        .collect();
    if !missing.is_empty() || !unexpected.is_empty() || !changed.is_empty() {
        return Err(SupportIntegrityFault::InstalledTreeMismatch {
            file: file.into(),
            missing,
            unexpected,
            changed,
        });
    }
    Ok(())
}
fn files(directory: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut pending = vec![directory.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(path) = pending.pop() {
        let kind = fs::symlink_metadata(&path)
            .map_err(|e| io_fault(&path, SupportIoOperation::Inspect, &e))?
            .file_type();
        if kind.is_symlink() {
            return Err(SupportIntegrityFault::UnexpectedSupportEntry {
                file: path,
                kind: SupportEntryKind::Symlink,
            });
        }
        if kind.is_dir() {
            let mut entries = fs::read_dir(&path)
                .map_err(|e| io_fault(&path, SupportIoOperation::ListDirectory, &e))?
                .map(|e| {
                    e.map(|e| e.path())
                        .map_err(|e| io_fault(&path, SupportIoOperation::ListDirectory, &e))
                })
                .collect::<Result<Vec<_>>>()?;
            entries.sort();
            pending.extend(entries.into_iter().rev());
        } else if kind.is_file() {
            files.insert(path);
        } else {
            return Err(SupportIntegrityFault::UnexpectedSupportEntry {
                file: path,
                kind: SupportEntryKind::Other,
            });
        }
    }
    Ok(files)
}
pub(super) fn hash(file: &Path) -> Result<String> {
    let mut f = fs::File::open(file).map_err(|e| io_fault(file, SupportIoOperation::Read, &e))?;
    let mut sha = Sha256::new();
    let mut buffer = vec![0u8; 65_536];
    loop {
        let count = f
            .read(&mut buffer)
            .map_err(|e| io_fault(file, SupportIoOperation::Read, &e))?;
        if count == 0 {
            break;
        }
        sha.update(&buffer[..count]);
    }
    Ok(format!("{:x}", sha.finalize()))
}
fn check_hash(file: &Path, expected: &str) -> Result<()> {
    let actual = hash(file)?;
    if actual != expected {
        return Err(SupportIntegrityFault::DigestMismatch {
            file: file.into(),
            expected: expected.into(),
            actual,
        });
    }
    Ok(())
}

#[derive(Debug)]
pub(super) struct Support {
    pub root: PathBuf,
    pub runtime: PathBuf,
    pub entry: PathBuf,
}
pub(super) fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("compiled repository root")
}
pub(super) fn resolve(value: Option<&Path>) -> std::result::Result<Support, NotationAdapterFault> {
    let config = |reason| NotationAdapterFault::SupportConfiguration {
        value: value.map(Path::to_path_buf),
        reason,
    };
    let original = value.ok_or_else(|| config(SupportReason::Missing))?;
    if !original.is_absolute() {
        return Err(config(SupportReason::Relative));
    }
    let root = original.canonicalize().map_err(|e| {
        NotationAdapterFault::Integrity(io_fault(original, SupportIoOperation::Canonicalize, &e))
    })?;
    let repo = repository();
    if root.starts_with(&repo) || repo.starts_with(&root) {
        return Err(config(SupportReason::RepositoryOverlap));
    }
    let runtime = root.join("runtime/bin/node");
    let entry = root.join("packages/node_modules/katex/dist/katex.js");
    for (path, component) in [
        (&runtime, AdapterComponent::Runtime),
        (&entry, AdapterComponent::KaTeX),
        (&root.join("packages/package.json"), AdapterComponent::KaTeX),
        (
            &root.join("packages/package-lock.json"),
            AdapterComponent::KaTeX,
        ),
    ] {
        // Missing files retain the integrity reader's precise operation later.
        match path.canonicalize() {
            Ok(resolved) if !resolved.starts_with(&root) => {
                return Err(config(SupportReason::EscapingEntry {
                    component,
                    resolved,
                }));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(NotationAdapterFault::Integrity(io_fault(
                    path,
                    SupportIoOperation::Canonicalize,
                    &e,
                )));
            }
        }
    }
    Ok(Support {
        root,
        runtime,
        entry,
    })
}
impl Support {
    pub fn check(&self) -> std::result::Result<(), NotationAdapterFault> {
        // Re-resolve every invocation: a prior successful path lookup is no trust cache.
        let current = resolve(Some(&self.root))?;
        current
            .check_inventory(DIGESTS, std::env::consts::OS, std::env::consts::ARCH)
            .map_err(NotationAdapterFault::Integrity)
    }
    pub(super) fn check_inventory(
        &self,
        digests: &[u8],
        os: &str,
        architecture: &str,
    ) -> Result<()> {
        let packages = self.root.join("packages");
        copy(
            &packages.join("package.json"),
            SupportCopy::Manifest,
            MANIFEST,
            "tools/notation/package.json",
        )?;
        copy(
            &packages.join("package-lock.json"),
            SupportCopy::Lockfile,
            LOCK,
            "tools/notation/package-lock.json",
        )?;
        let inventory = inventory(digests)?;
        let hidden = packages.join("node_modules/.package-lock.json");
        let hidden_kind = fs::symlink_metadata(&hidden)
            .map_err(|e| io_fault(&hidden, SupportIoOperation::Inspect, &e))?
            .file_type();
        if !hidden_kind.is_file() {
            return Err(SupportIntegrityFault::UnexpectedSupportEntry {
                file: hidden,
                kind: if hidden_kind.is_symlink() {
                    SupportEntryKind::Symlink
                } else {
                    SupportEntryKind::Other
                },
            });
        }
        installed(&hidden, &read(&hidden)?)?;
        let mut actual = BTreeSet::new();
        for name in ["commander", "katex"] {
            actual.extend(
                files(&packages.join("node_modules").join(name))?
                    .into_iter()
                    .map(|p| {
                        p.strip_prefix(&packages)
                            .expect("descendant")
                            .to_string_lossy()
                            .into_owned()
                    }),
            );
        }
        let expected: BTreeSet<_> = inventory.packages.keys().cloned().collect();
        if actual != expected {
            return Err(SupportIntegrityFault::FileInventoryMismatch {
                directory: packages.clone(),
                missing: expected.difference(&actual).cloned().collect(),
                unexpected: actual.difference(&expected).cloned().collect(),
            });
        }
        for (path, expected) in &inventory.packages {
            check_hash(&packages.join(path), expected)?;
        }
        if os != "linux"
            || architecture != "x86_64"
            || !inventory.runtimes.contains_key("linux-x64")
        {
            return Err(SupportIntegrityFault::UnsupportedPlatform {
                os: os.into(),
                architecture: architecture.into(),
            });
        }
        check_hash(&self.runtime, &inventory.runtimes["linux-x64"])
    }
}
