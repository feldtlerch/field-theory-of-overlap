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
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, NativeAdapterFault>;
/// Explicit support location; execution never consults PATH or a default bundle.
#[derive(Debug)]
pub struct NativeSupport {
    pub(super) root: PathBuf,
    active: RefCell<Option<Active>>,
    #[cfg(test)]
    pub(super) cold_format: std::cell::Cell<bool>,
    #[cfg(test)]
    pub(super) evidence: RefCell<Option<Vec<super::native::Evidence>>>,
    #[cfg(test)]
    pub(super) bootstrap_evidence: RefCell<Option<super::native::Evidence>>,
    #[cfg(test)]
    closing_hash: RefCell<Option<Result<()>>>,
    #[cfg(test)]
    pub(super) identity_observations: std::cell::Cell<usize>,
}
#[derive(Debug)]
struct Active {
    snapshot: Option<crate::support_bracket::Snapshot>,
    poison: Option<NativeAdapterFault>,
    formats: Vec<super::native_format::Lease>,
    observation_available: Option<crate::support_bracket::Observation>,
}
struct Scope<'a>(&'a NativeSupport);
impl Drop for Scope<'_> {
    fn drop(&mut self) {
        if self.0.active.borrow_mut().take().is_some() {
            crate::telemetry::counter("native.bracket_aborted", 1);
        }
    }
}
impl Clone for NativeSupport {
    fn clone(&self) -> Self {
        Self::at(self.root.clone())
    }
}
impl NativeSupport {
    pub(super) fn invalidate_observation(&self) {
        self.active
            .borrow_mut()
            .as_mut()
            .expect("open bracket")
            .observation_available = None;
    }
    pub(super) fn before_consumer(&self, lock: &Lock) -> Result<()> {
        self.ensure_open(lock)?;
        let available = self
            .active
            .borrow()
            .as_ref()
            .expect("open bracket")
            .observation_available;
        if available.is_none() || available != crate::support_bracket::idle_observation(&self.root)
        {
            self.observe(lock)?;
        }
        self.invalidate_observation();
        Ok(())
    }
    pub(super) fn after_consumer(&self, lock: &Lock) -> Result<()> {
        self.after_consumer_observed(lock, || {})
    }
    pub(super) fn after_consumer_observed(
        &self,
        lock: &Lock,
        observed: impl FnOnce(),
    ) -> Result<()> {
        let before = crate::support_bracket::idle_observation(&self.root);
        self.observe(lock)?;
        observed();
        let after = crate::support_bracket::idle_observation(&self.root);
        // A consumer starting during the observation also prevents reuse.
        self.active
            .borrow_mut()
            .as_mut()
            .expect("open bracket")
            .observation_available = (before == after).then_some(after).flatten();
        Ok(())
    }
    pub(super) fn format_seed(
        &self,
        lock: &Lock,
        bootstrap: &str,
    ) -> std::result::Result<std::sync::Arc<super::native_format::Seed>, super::DocumentError> {
        self.ensure_open(lock)?;
        let _lookup = crate::telemetry::Phase::new("native.format_lookup");
        if let Some(seed) = self
            .active
            .borrow()
            .as_ref()
            .expect("open bracket")
            .formats
            .iter()
            .find(|lease| lease.matches(bootstrap))
            .and_then(|lease| lease.seed.clone())
        {
            return Ok(seed);
        }
        let key = super::native_format::Key::read(self, lock, bootstrap)?;
        let mut lease = super::native_format::Lease::acquire(key);
        let seed = if let Some(seed) = &lease.seed {
            seed.clone()
        } else {
            let seed = std::sync::Arc::new(super::native::bootstrap(self, lock, bootstrap)?);
            lease.seed = Some(seed.clone());
            seed
        };
        self.active
            .borrow_mut()
            .as_mut()
            .expect("open bracket")
            .formats
            .push(lease);
        Ok(seed)
    }
    pub(super) fn poison(&self, fault: NativeAdapterFault) -> NativeAdapterFault {
        let mut active = self.active.borrow_mut();
        let active = active.as_mut().expect("open bracket");
        active.poison.get_or_insert(fault).clone()
    }

    pub(super) fn spawn_error(&self, path: &Path, error: &io::Error) -> NativeAdapterFault {
        if let Some(fault) = error
            .get_ref()
            .and_then(|error| error.downcast_ref::<crate::support_bracket::confinement::Fault>())
        {
            self.poison(NativeAdapterFault::Confinement {
                operation: fault.operation,
                path: fault.path.clone(),
                kind: fault.kind,
            })
        } else {
            io_error(NativeOperation::Spawn, path, error)
        }
    }
    pub(super) fn at(root: PathBuf) -> Self {
        #[cfg(test)]
        crate::support_bracket::mutation::register(&root);
        Self {
            root,
            active: RefCell::new(None),
            #[cfg(test)]
            cold_format: std::cell::Cell::new(false),
            #[cfg(test)]
            evidence: RefCell::new(None),
            #[cfg(test)]
            bootstrap_evidence: RefCell::new(None),
            #[cfg(test)]
            closing_hash: RefCell::new(None),
            #[cfg(test)]
            identity_observations: std::cell::Cell::new(0),
        }
    }
    #[cfg(test)]
    pub(super) fn closing_hash(&self) -> Option<Result<()>> {
        self.closing_hash.borrow().clone()
    }
    fn observation_fault(
        &self,
        lock: &Lock,
        fault: crate::support_bracket::Fault,
    ) -> NativeAdapterFault {
        use crate::support_bracket::Fault;
        match fault {
            Fault::Changed {
                path,
                expected,
                actual,
            } => NativeAdapterFault::SupportChanged {
                member: path.to_string_lossy().into_owned(),
                expected,
                actual,
            },
            Fault::Io { path, kind } => {
                verify(self, lock).err().unwrap_or(NativeAdapterFault::Io {
                    operation: NativeOperation::Inspect,
                    path,
                    kind,
                })
            }
            Fault::Inventory {
                missing,
                unexpected,
            } => verify(self, lock)
                .err()
                .unwrap_or_else(|| NativeAdapterFault::Inventory {
                    component: NativeComponent::Bundle,
                    missing: missing
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect(),
                    unexpected: unexpected
                        .iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect(),
                }),
        }
    }
    pub(super) fn observe(&self, lock: &Lock) -> Result<()> {
        #[cfg(test)]
        self.identity_observations
            .set(self.identity_observations.get() + 1);
        self.ensure_open(lock)?;
        let (phase, result) = {
            let active = self.active.borrow();
            let active = active.as_ref().expect("private bracket is open");
            if let Some(fault) = &active.poison {
                return Err(fault.clone());
            }
            let phase = crate::telemetry::Phase::new("native.identity_pass");
            (
                phase,
                active.snapshot.as_ref().expect("verified opening").check(),
            )
        };
        let result = result
            .map(|members| crate::telemetry::counter("native.identity_members", members))
            .map_err(|fault| self.observation_fault(lock, fault));
        if let Err(fault) = &result {
            self.active
                .borrow_mut()
                .as_mut()
                .expect("open bracket")
                .poison = Some(fault.clone());
        }
        phase.finish(result.is_ok());
        result
    }
    pub(super) fn scope<T, E: From<NativeAdapterFault>>(
        &self,
        lock: &Lock,
        run: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        if self.active.borrow().is_none() {
            return self.owned_scope(lock, run);
        }
        if let Some(fault) = self.active.borrow().as_ref().and_then(|a| a.poison.clone()) {
            return Err(fault.into());
        }
        run()
    }
    pub(super) fn ensure_open(&self, lock: &Lock) -> Result<()> {
        {
            let active = self.active.borrow();
            let active = active.as_ref().expect("private scope owns opening");
            if let Some(fault) = &active.poison {
                return Err(fault.clone());
            }
            if active.snapshot.is_some() {
                return Ok(());
            }
        }
        let open_phase = crate::telemetry::Phase::new("native.bracket_open");
        let opening = (|| {
            let mut roots = vec![
                self.root.join("runtime/bin/tectonic"),
                self.root.join("bundle"),
            ];
            if let Some((path, _)) = &lock.source {
                roots.push(path.clone());
            }
            let snapshot = crate::support_bracket::Snapshot::capture(&roots)
                .map_err(|e| self.observation_fault(lock, e))?;
            crate::telemetry::counter("native.opening_members", snapshot.member_count());
            verify(self, lock)?;
            let stabilization = crate::telemetry::Phase::new("native.opening_stabilization");
            let stabilized = snapshot
                .check()
                .map_err(|e| self.observation_fault(lock, e));
            stabilization.finish(stabilized.is_ok());
            crate::telemetry::counter("native.stabilized_members", stabilized?);
            Ok::<_, NativeAdapterFault>(snapshot)
        })();
        open_phase.finish(opening.is_ok());
        let snapshot = match opening {
            Ok(snapshot) => snapshot,
            Err(fault) => {
                self.active
                    .borrow_mut()
                    .as_mut()
                    .expect("owning scope")
                    .poison = Some(fault.clone());
                return Err(fault);
            }
        };
        self.active
            .borrow_mut()
            .as_mut()
            .expect("owning scope")
            .snapshot = Some(snapshot);
        crate::telemetry::counter("native.bracket_opened", 1);
        Ok(())
    }
    fn owned_scope<T, E: From<NativeAdapterFault>>(
        &self,
        lock: &Lock,
        run: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<T, E> {
        #[cfg(test)]
        {
            *self.closing_hash.borrow_mut() = None;
        }
        *self.active.borrow_mut() = Some(Active {
            snapshot: None,
            poison: None,
            formats: Vec::new(),
            observation_available: None,
        });
        let guard = Scope(self);
        let result = run();
        if self
            .active
            .borrow()
            .as_ref()
            .expect("owning scope")
            .snapshot
            .is_none()
        {
            let active = self.active.borrow_mut().take().expect("owning scope");
            drop(guard);
            if let Some(fault) = active.poison {
                crate::telemetry::counter("native.bracket_aborted", 1);
                return Err(fault.into());
            }
            return result;
        }
        let close_phase = crate::telemetry::Phase::new("native.bracket_close");
        let full = verify(self, lock);
        #[cfg(test)]
        {
            *self.closing_hash.borrow_mut() = Some(full.clone());
        }
        let identity = self
            .active
            .borrow()
            .as_ref()
            .expect("open bracket")
            .snapshot
            .as_ref()
            .expect("verified opening")
            .check()
            .map_err(|e| self.observation_fault(lock, e));
        let mut active = self.active.borrow_mut().take().expect("open bracket");
        drop(guard);
        let closing = active.poison.map_or(full.and(identity.map(|_| ())), Err);
        close_phase.finish(closing.is_ok());
        if closing.is_err() {
            crate::telemetry::counter("native.bracket_aborted", 1);
        }
        closing?;
        crate::telemetry::counter("native.bracket_closed", 1);
        if result.is_ok() {
            for lease in &mut active.formats {
                lease.publish();
            }
        }
        result
    }
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
        Ok(Self::at(support))
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
    pub source: Option<(PathBuf, String)>,
    pub engine: String,
    pub files: BTreeMap<String, String>,
    pub bundle_digest: String,
}
fn lock_bytes(path: &Path) -> Result<Vec<u8>> {
    inspect(path, None, false)?;
    let metadata = fs::metadata(path).map_err(|e| io_error(NativeOperation::Inspect, path, &e))?;
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
    crate::support_bracket::open_file(path)
        .and_then(|file| file.take(16_777_217).read_to_end(&mut bytes))
        .map_err(|e| io_error(NativeOperation::Read, path, &e))?;
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
    Ok(bytes)
}
pub(super) fn lock(root: &Path) -> Result<Lock> {
    let root = fs::canonicalize(root).map_err(|e| io_error(NativeOperation::Inspect, root, &e))?;
    let path = root.join("tools/assembly/native-lock.toml");
    let bytes = lock_bytes(&path)?;
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
        source: Some((
            fs::canonicalize(&path).map_err(|e| io_error(NativeOperation::Inspect, &path, &e))?,
            format!("{:x}", Sha256::digest(&source.bytes)),
        )),
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
    let file = crate::support_bracket::open_file(path)
        .map_err(|e| io_error(NativeOperation::Read, path, &e))?;
    hash_file(path, file)
}
fn hash_file(path: &Path, mut file: fs::File) -> Result<String> {
    let mut bytes = 0;
    let before = file
        .metadata()
        .map_err(|e| io_error(NativeOperation::Inspect, path, &e))?;
    crate::telemetry::hash_identity();
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
    let after = file
        .metadata()
        .map_err(|e| io_error(NativeOperation::Inspect, path, &e))?;
    crate::telemetry::hash_identity();
    if crate::SupportIdentity::from(&before) != crate::SupportIdentity::from(&after) {
        return Err(NativeAdapterFault::SupportChanged {
            member: path.to_string_lossy().into_owned(),
            expected: (&before).into(),
            actual: Some((&after).into()),
        });
    }
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
    if let Some((path, expected)) = &lock.source {
        let actual = hash(path)?;
        if actual != *expected {
            return Err(invalid(
                &super::input::site("tools/assembly/native-lock.toml"),
                expected,
                &actual,
            ));
        }
    }
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
    let directory = crate::support_bracket::open_directory(&root)
        .map_err(|e| io_error(NativeOperation::Inspect, &root, &e))?;
    for (member, expected) in &lock.files {
        let path = root.join(member);
        let file = crate::support_bracket::file_beneath(&directory, Path::new(member))
            .map_err(|e| io_error(NativeOperation::Read, &path, &e))?;
        let actual = hash_file(&path, file)?;
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
