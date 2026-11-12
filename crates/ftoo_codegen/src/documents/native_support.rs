use super::diagnostics::{
    DocumentEntryKind, NativeAdapterFault, NativeComponent, NativeConfigurationReason,
    NativeMetadataReason, NativeOperation,
};
use crate::{
    UnitSource,
    units::toml::{self, Node},
};
use ftoo_manifest::{ShapeKind, UnitDiagnostic, UnitSite};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, NativeAdapterFault>;
/// Explicit support location; execution never consults PATH or a default bundle.
#[derive(Clone, Debug)]
pub struct NativeSupport {
    pub(super) root: PathBuf,
}
impl NativeSupport {
    /// # Errors
    /// Missing, relative, overlapping and linked support locations are refused.
    pub fn from_environment(repository: &Path) -> Result<Self> {
        let value = std::env::var_os("FTOO_ASSEMBLY_SUPPORT").map(PathBuf::from);
        Self::resolve(repository, value.as_deref())
    }
    /// # Errors
    /// Requires an absolute canonical directory disjoint from the repository.
    pub fn resolve(repository: &Path, value: Option<&Path>) -> Result<Self> {
        let Some(value) = value else {
            return Err(configuration(
                None,
                None,
                NativeConfigurationReason::Missing,
            ));
        };
        if !value.is_absolute() {
            return Err(configuration(
                None,
                Some(value),
                NativeConfigurationReason::RelativeSupport {
                    value: value.into(),
                },
            ));
        }
        let repository = fs::canonicalize(repository)
            .map_err(|e| io_error(NativeOperation::Inspect, repository, &e))?;
        let support = fs::canonicalize(value).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                configuration(None, Some(value), NativeConfigurationReason::Missing)
            } else {
                io_error(NativeOperation::Inspect, value, &e)
            }
        })?;
        if repository.starts_with(&support) || support.starts_with(&repository) {
            return Err(configuration(
                None,
                Some(value),
                NativeConfigurationReason::OverlappingSupport {
                    repository,
                    support,
                },
            ));
        }
        if support != value {
            return Err(configuration(
                None,
                Some(value),
                NativeConfigurationReason::EscapingEntry { resolved: support },
            ));
        }
        inspect(value, None, true)?;
        Ok(Self { root: support })
    }
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.root
    }
}
pub(super) fn configuration(
    component: Option<NativeComponent>,
    path: Option<&Path>,
    reason: NativeConfigurationReason,
) -> NativeAdapterFault {
    NativeAdapterFault::Configuration {
        component,
        path: path.map(Path::to_owned),
        reason,
    }
}
pub(super) fn io_error(
    operation: NativeOperation,
    path: &Path,
    e: &io::Error,
) -> NativeAdapterFault {
    NativeAdapterFault::Io {
        operation,
        path: path.into(),
        kind: e.kind(),
    }
}
fn inspect(path: &Path, component: Option<NativeComponent>, directory: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            configuration(component, Some(path), NativeConfigurationReason::Missing)
        } else {
            io_error(NativeOperation::Inspect, path, &e)
        }
    })?;
    let kind = if metadata.file_type().is_symlink() {
        Some(DocumentEntryKind::Symlink)
    } else if metadata.is_dir() && !directory {
        Some(DocumentEntryKind::Directory)
    } else if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        Some(DocumentEntryKind::Other)
    } else {
        None
    };
    if let Some(kind) = kind {
        return Err(configuration(
            component,
            Some(path),
            NativeConfigurationReason::NonRegularEntry { kind },
        ));
    }
    Ok(())
}
fn boundary(d: Box<UnitDiagnostic>) -> NativeAdapterFault {
    NativeAdapterFault::Metadata {
        site: d.site,
        reason: NativeMetadataReason::Boundary(d.fault),
    }
}
fn invalid(site: &UnitSite, expected: &str, actual: &str) -> NativeAdapterFault {
    NativeAdapterFault::Metadata {
        site: site.clone(),
        reason: NativeMetadataReason::InvalidValue {
            expected: expected.into(),
            actual: actual.into(),
        },
    }
}
fn string(n: &Node) -> Result<String> {
    Ok(toml::string(n).map_err(boundary)?.value)
}
fn digest(n: &Node) -> Result<String> {
    let value = string(n)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(&n.site, "lowercase 64-hex SHA-256", &value));
    }
    Ok(value)
}
fn member(n: &Node) -> Result<String> {
    let value = string(n)?;
    if value.is_empty()
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || value.split('/').any(|s| matches!(s, "" | "." | ".."))
    {
        return Err(invalid(
            &n.site,
            "relative member with nonempty safe components",
            &value,
        ));
    }
    Ok(value)
}
fn url(n: &Node) -> Result<String> {
    let value = string(n)?;
    if !value.starts_with("https://")
        || value.len() <= 8
        || value.starts_with("https://relay.fullyjustified.net/")
    {
        return Err(configuration(
            None,
            None,
            NativeConfigurationReason::UnpinnedArtifact { url: value },
        ));
    }
    Ok(value)
}
#[derive(Debug)]
pub(super) struct Lock {
    pub engine: String,
    pub files: BTreeMap<String, String>,
    pub bundle_digest: String,
}
pub(super) fn lock(root: &Path) -> Result<Lock> {
    let path = root.join("tools/assembly/native-lock.toml");
    inspect(&path, None, false)?;
    let metadata =
        fs::metadata(&path).map_err(|e| io_error(NativeOperation::Inspect, &path, &e))?;
    if metadata.len() > 16_777_216 {
        return Err(NativeAdapterFault::Metadata {
            site: super::input::site("tools/assembly/native-lock.toml"),
            reason: NativeMetadataReason::Limit {
                resource: "lock-bytes".into(),
                maximum: 16_777_216,
                actual: 16_777_217,
            },
        });
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .and_then(|file| file.take(16_777_217).read_to_end(&mut bytes))
        .map_err(|e| io_error(NativeOperation::Read, &path, &e))?;
    if bytes.len() > 16_777_216 {
        return Err(NativeAdapterFault::Metadata {
            site: super::input::site("tools/assembly/native-lock.toml"),
            reason: NativeMetadataReason::Limit {
                resource: "lock-bytes".into(),
                maximum: 16_777_216,
                actual: 16_777_217,
            },
        });
    }
    let source = UnitSource {
        file: "tools/assembly/native-lock.toml".into(),
        bytes,
    };
    let text = toml::encoding(&source).map_err(boundary)?;
    super::lock_scan::records(text, 262_144).map_err(|()| NativeAdapterFault::Metadata {
        site: super::input::site("tools/assembly/native-lock.toml"),
        reason: NativeMetadataReason::Limit {
            resource: "file-records".into(),
            maximum: 262_144,
            actual: 262_145,
        },
    })?;
    let tree = toml::parse(&source.file, text).map_err(boundary)?;
    toml::numeric(&tree, text).map_err(boundary)?;
    let f = toml::fixed(
        &tree,
        &[
            "schema",
            "engine-version",
            "format-serial",
            "executable",
            "bundle",
        ],
    )
    .map_err(boundary)?;
    toml::typed(f[0], ShapeKind::Integer).map_err(boundary)?;
    if toml::raw(f[0], text) != "1" {
        return Err(invalid(&f[0].site, "1", toml::raw(f[0], text)));
    }
    toml::typed(f[2], ShapeKind::Integer).map_err(boundary)?;
    if toml::raw(f[2], text)
        .parse::<toml_edit::Value>()
        .ok()
        .and_then(|value| value.as_integer())
        != Some(33)
    {
        return Err(invalid(&f[2].site, "33", toml::raw(f[2], text)));
    }
    if string(f[1])? != "0.15.0" {
        return Err(invalid(&f[1].site, "0.15.0", &string(f[1])?));
    }
    let e = toml::fixed(
        f[3],
        &["platform", "url", "archive-sha256", "member", "sha256"],
    )
    .map_err(boundary)?;
    if string(e[0])? != "x86_64-unknown-linux-musl" {
        return Err(invalid(
            &e[0].site,
            "x86_64-unknown-linux-musl",
            &string(e[0])?,
        ));
    }
    url(e[1])?;
    digest(e[2])?;
    member(e[3])?;
    let engine = digest(e[4])?;
    let b = toml::fixed(f[4], &["url", "archive-sha256", "bundle-digest", "files"])
        .map_err(boundary)?;
    url(b[0])?;
    digest(b[1])?;
    let bundle_digest = digest(b[2])?;
    let files = file_rows(b[3])?;
    Ok(Lock {
        engine,
        files,
        bundle_digest,
    })
}
fn file_rows(node: &Node) -> Result<BTreeMap<String, String>> {
    let rows = toml::typed(node, ShapeKind::Array).map_err(boundary)?;
    if rows.elements.len() > 262_144 {
        return Err(NativeAdapterFault::Metadata {
            site: rows.site.clone(),
            reason: NativeMetadataReason::Limit {
                resource: "file-records".into(),
                maximum: 262_144,
                actual: 262_145,
            },
        });
    }
    if rows.elements.is_empty() {
        return Err(invalid(&rows.site, "nonempty file inventory", "empty"));
    }
    let mut files = BTreeMap::new();
    let mut sites: BTreeMap<String, UnitSite> = BTreeMap::new();
    let mut previous = String::new();
    for row in &rows.elements {
        let fields = toml::fixed(row, &["member", "sha256"]).map_err(boundary)?;
        let name = member(fields[0])?;
        let hash = digest(fields[1])?;
        if let Some(site) = sites.get(&name) {
            return Err(NativeAdapterFault::Metadata {
                site: fields[0].site.clone(),
                reason: NativeMetadataReason::DuplicateMember {
                    member: name,
                    sites: vec![site.clone(), fields[0].site.clone()],
                },
            });
        }
        if name < previous {
            return Err(NativeAdapterFault::Metadata {
                site: fields[0].site.clone(),
                reason: NativeMetadataReason::OutOfOrder {
                    previous,
                    current: name,
                },
            });
        }
        previous.clone_from(&name);
        sites.insert(name.clone(), fields[0].site.clone());
        files.insert(name, hash);
    }
    Ok(files)
}
fn hash(path: &Path) -> Result<String> {
    let mut bytes = 0;
    let mut file = fs::File::open(path).map_err(|e| io_error(NativeOperation::Read, path, &e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 65_536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| io_error(NativeOperation::Read, path, &e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    crate::telemetry::hashed(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
fn list(root: &Path, relative: &str, output: &mut Vec<String>) -> Result<()> {
    let directory = root.join(relative);
    inspect(&directory, Some(NativeComponent::Bundle), true)?;
    let mut paths = fs::read_dir(&directory)
        .map_err(|e| io_error(NativeOperation::Inspect, &directory, &e))?
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|e| io_error(NativeOperation::Inspect, &directory, &e))
        })
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| io_error(NativeOperation::Inspect, &path, &e))?;
        let name = path
            .strip_prefix(root)
            .ok()
            .and_then(Path::to_str)
            .ok_or_else(|| {
                configuration(
                    Some(NativeComponent::Bundle),
                    Some(&path),
                    NativeConfigurationReason::NonRegularEntry {
                        kind: DocumentEntryKind::NonUtf8Name,
                    },
                )
            })?;
        if metadata.is_dir() {
            list(root, name, output)?;
        } else {
            inspect(&path, Some(NativeComponent::Bundle), false)?;
            output.push(name.into());
        }
    }
    Ok(())
}
pub(super) fn verify(support: &NativeSupport, lock: &Lock) -> Result<()> {
    let phase = crate::telemetry::Phase::new("native.full_verification");
    let result = verify_inner(support, lock);
    phase.finish(result.is_ok());
    result
}
fn verify_inner(support: &NativeSupport, lock: &Lock) -> Result<()> {
    inspect(&support.root, None, true)?;
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return Err(configuration(
            None,
            None,
            NativeConfigurationReason::UnsupportedPlatform {
                os: std::env::consts::OS.into(),
                architecture: std::env::consts::ARCH.into(),
            },
        ));
    }
    for relative in ["runtime", "runtime/bin", "bundle", "downloads", "tmp"] {
        inspect(&support.root.join(relative), None, true)?;
    }
    let executable = support.root.join("runtime/bin/tectonic");
    inspect(&executable, Some(NativeComponent::Executable), false)?;
    let actual = hash(&executable)?;
    if actual != lock.engine {
        return Err(NativeAdapterFault::Integrity {
            component: NativeComponent::Executable,
            member: "runtime/bin/tectonic".into(),
            expected: lock.engine.clone(),
            actual,
        });
    }
    let root = support.root.join("bundle");
    let mut files = Vec::new();
    list(&root, "", &mut files)?;
    files.sort();
    let missing: Vec<_> = lock
        .files
        .keys()
        .filter(|name| files.binary_search(name).is_err())
        .cloned()
        .collect();
    let unexpected: Vec<_> = files
        .iter()
        .filter(|name| !lock.files.contains_key(*name))
        .cloned()
        .collect();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(NativeAdapterFault::Inventory {
            component: NativeComponent::Bundle,
            missing,
            unexpected,
        });
    }
    for (member, expected) in &lock.files {
        let actual = hash(&root.join(member))?;
        if &actual != expected {
            return Err(NativeAdapterFault::Integrity {
                component: NativeComponent::Bundle,
                member: member.clone(),
                expected: expected.clone(),
                actual,
            });
        }
    }
    let path = root.join("SHA256SUM");
    let actual =
        fs::read_to_string(&path).map_err(|e| io_error(NativeOperation::Read, &path, &e))?;
    if actual.trim() != lock.bundle_digest {
        return Err(NativeAdapterFault::Integrity {
            component: NativeComponent::Bundle,
            member: "SHA256SUM".into(),
            expected: lock.bundle_digest.clone(),
            actual,
        });
    }
    Ok(())
}
