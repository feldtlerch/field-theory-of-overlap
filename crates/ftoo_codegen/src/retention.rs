//! Stable repository binding and atomic lifecycle for the one retained audit report.
mod display;
mod lifecycle;
#[cfg(test)]
mod tests;
use ftoo_audit::{
    AuditReport, ReleaseAssessment,
    retention::{self, AcceptedRetention, RetentionError},
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

const REPORT: &str = "generated/audit-report.json";
const PROJECTION: &str = "generated/manifest.json";
const MAX_REPORT: u64 = 32 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Read,
    Metadata,
    Create,
    Write,
    Flush,
    Sync,
    Rename,
    DirectorySync,
    Reread,
    Remove,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoFailure {
    pub path: String,
    pub operation: Operation,
    pub kind: io::ErrorKind,
    pub raw_os_error: Option<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    pub length: u64,
    pub modified: Option<std::time::SystemTime>,
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
    pub changed: (i64, i64),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionCheckError {
    Projection(crate::ProjectionCheckError),
    Audit(Box<RetentionError>),
    Documents(Box<crate::DocumentError>),
    Io {
        primary: IoFailure,
        cleanup: Vec<IoFailure>,
        replaced: Vec<String>,
    },
    NotRegular {
        path: String,
        identity: FileIdentity,
    },
    InputChanged {
        path: String,
        expected: Box<FileIdentity>,
        actual: Option<Box<FileIdentity>>,
    },
    Limit {
        path: String,
        limit: u64,
        actual: u64,
    },
    ReleaseRejected(ReleaseAssessment),
    AfterReplacement {
        error: Box<Self>,
        replaced: Vec<String>,
    },
    Cleanup {
        error: Box<Self>,
        failures: Vec<IoFailure>,
    },
}
#[derive(Clone, Debug)]
struct Snapshot {
    identity: FileIdentity,
    bytes: Vec<u8>,
}
// The document consumer reuses the report's regular-file identity and byte recheck.
#[derive(Debug)]
pub(crate) struct DocumentSnapshot(BTreeMap<String, Snapshot>);
impl DocumentSnapshot {
    pub(crate) fn capture<'a>(
        root: &Path,
        files: impl Iterator<Item = (&'a str, &'a [u8])>,
    ) -> Result<Self, RetentionCheckError> {
        let mut snapshots = BTreeMap::new();
        for (path, bytes) in files {
            let mut snapshot = read(root, path, MAX_REPORT, true)?;
            snapshot.bytes = bytes.to_vec();
            reread(root, path, &snapshot)?;
            snapshots.insert(path.into(), snapshot);
        }
        Ok(Self(snapshots))
    }
    pub(crate) fn recheck(&self, root: &Path) -> Result<(), RetentionCheckError> {
        for (path, snapshot) in &self.0 {
            reread(root, path, snapshot)?;
        }
        engine(&engine_sources(root)?)?;
        Ok(())
    }
    pub(crate) fn without_outputs(mut self) -> Self {
        self.0
            .retain(|path, _| !path.starts_with("generated/documents/"));
        self
    }
}
/// A checked report bound to one stable repository source snapshot.
/// ```compile_fail
/// use ftoo_codegen::retention::PreparedRetention;
/// let forged = PreparedRetention { accepted: todo!(), sources: todo!(), report_identity: todo!() };
/// ```
#[derive(Debug)]
pub struct PreparedRetention {
    accepted: AcceptedRetention,
    sources: BTreeMap<String, Snapshot>,
    report_identity: FileIdentity,
}
impl PreparedRetention {
    #[must_use]
    pub fn information(&self) -> &retention::InfoRecord {
        self.accepted.information()
    }
    #[must_use]
    pub fn report(&self) -> &AuditReport {
        self.accepted.report()
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.accepted.bytes()
    }
}
fn identity(metadata: &fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    FileIdentity {
        length: metadata.len(),
        modified: metadata.modified().ok(),
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        changed: (metadata.ctime(), metadata.ctime_nsec()),
    }
}
impl From<RetentionError> for RetentionCheckError {
    fn from(error: RetentionError) -> Self {
        Self::Audit(Box::new(error))
    }
}

fn failure(path: &str, operation: Operation, error: &io::Error) -> IoFailure {
    IoFailure {
        path: path.into(),
        operation,
        kind: error.kind(),
        raw_os_error: error.raw_os_error(),
    }
}
fn io(path: &str, operation: Operation, error: &io::Error) -> RetentionCheckError {
    RetentionCheckError::Io {
        primary: failure(path, operation, error),
        cleanup: Vec::new(),
        replaced: Vec::new(),
    }
}
fn read(
    root: &Path,
    path: &str,
    maximum: u64,
    stable: bool,
) -> Result<Snapshot, RetentionCheckError> {
    check_parents(root, path)?;
    let full = root.join(path);
    let metadata =
        fs::symlink_metadata(&full).map_err(|error| io(path, Operation::Metadata, &error))?;
    let before = identity(&metadata);
    if !metadata.is_file() {
        return Err(RetentionCheckError::NotRegular {
            path: path.into(),
            identity: before,
        });
    }
    let mut file = open_regular(root, path)?;
    let metadata = file
        .metadata()
        .map_err(|error| io(path, Operation::Metadata, &error))?;
    let opened = identity(&metadata);
    if !metadata.is_file() {
        return Err(RetentionCheckError::NotRegular {
            path: path.into(),
            identity: opened,
        });
    }
    if stable && before != opened {
        return Err(RetentionCheckError::InputChanged {
            path: path.into(),
            expected: Box::new(before),
            actual: Some(Box::new(opened)),
        });
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io(path, Operation::Read, &error))?;
    if bytes.len() as u64 > maximum {
        return Err(RetentionCheckError::Limit {
            path: path.into(),
            limit: maximum,
            actual: maximum + 1,
        });
    }
    let after = identity(
        &fs::symlink_metadata(&full).map_err(|error| io(path, Operation::Metadata, &error))?,
    );
    if stable && opened != after {
        return Err(RetentionCheckError::InputChanged {
            path: path.into(),
            expected: Box::new(opened),
            actual: Some(Box::new(after)),
        });
    }
    Ok(Snapshot {
        identity: after,
        bytes,
    })
}
fn check_parents(root: &Path, path: &str) -> Result<(), RetentionCheckError> {
    let mut parent = String::new();
    let parts: Vec<_> = path.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if !parent.is_empty() {
            parent.push('/');
        }
        parent.push_str(part);
        let metadata = fs::symlink_metadata(root.join(&parent))
            .map_err(|error| io(&parent, Operation::Metadata, &error))?;
        if !metadata.is_dir() {
            return Err(RetentionCheckError::NotRegular {
                path: parent,
                identity: identity(&metadata),
            });
        }
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn open_regular(root: &Path, path: &str) -> Result<File, RetentionCheckError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let mut directory = File::open(root).map_err(|error| io(".", Operation::Read, &error))?;
    let parts: Vec<_> = path.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        let name = std::ffi::CString::new(*part).map_err(|_| {
            io(
                path,
                Operation::Read,
                &io::Error::from(io::ErrorKind::InvalidInput),
            )
        })?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if index + 1 == parts.len() {
                0
            } else {
                libc::O_DIRECTORY
            };
        // The borrowed directory descriptor and NUL-terminated name remain live
        // for this call. Each successful descriptor receives exactly one owner.
        let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if descriptor < 0 {
            return Err(io(path, Operation::Read, &io::Error::last_os_error()));
        }
        directory = unsafe { File::from_raw_fd(descriptor) };
    }
    Ok(directory)
}
const ENGINE_FILES: [(&str, retention::EngineRevisionSource); 2] = [
    ("Cargo.lock", retention::EngineRevisionSource::CurrentLock),
    (
        "crates/ftoo_audit/Cargo.toml",
        retention::EngineRevisionSource::CargoManifest,
    ),
];
fn unavailable(
    path: &'static str,
    source: retention::EngineRevisionSource,
    reason: retention::EngineRevisionReason,
) -> RetentionCheckError {
    RetentionError::EngineRevisionUnavailable(retention::EngineRevisionUnavailable {
        source,
        path,
        package: "nrz-cas",
        reason,
    })
    .into()
}
fn engine(sources: &BTreeMap<String, Snapshot>) -> Result<(), RetentionCheckError> {
    let text = |(path, source): (&'static str, retention::EngineRevisionSource)| {
        std::str::from_utf8(&sources[path].bytes).map_err(|error| {
            unavailable(
                path,
                source,
                retention::EngineRevisionReason::Malformed {
                    message: error.to_string(),
                    span: Some(
                        error.valid_up_to()..error.valid_up_to() + error.error_len().unwrap_or(0),
                    ),
                },
            )
        })
    };
    retention::check_current_engine(text(ENGINE_FILES[0])?, text(ENGINE_FILES[1])?)
        .map_err(Into::into)
}
fn sources(root: &Path) -> Result<BTreeMap<String, Snapshot>, RetentionCheckError> {
    let mut sources = BTreeMap::from([(
        PROJECTION.into(),
        read(root, PROJECTION, 4 * 1024 * 1024, true)?,
    )]);
    sources.extend(engine_sources(root)?);
    Ok(sources)
}
fn engine_sources(root: &Path) -> Result<BTreeMap<String, Snapshot>, RetentionCheckError> {
    let mut sources = BTreeMap::new();
    for (path, source) in ENGINE_FILES {
        let snapshot = read(root, path, MAX_REPORT, false).map_err(|error| match error {
            RetentionCheckError::Io { primary, .. } => unavailable(
                path,
                source,
                if primary.kind == io::ErrorKind::NotFound {
                    retention::EngineRevisionReason::Missing
                } else {
                    retention::EngineRevisionReason::Unreadable {
                        kind: primary.kind,
                        raw_os_error: primary.raw_os_error,
                    }
                },
            ),
            other => other,
        })?;
        sources.insert(path.into(), snapshot);
    }
    Ok(sources)
}
fn reread(root: &Path, path: &str, expected: &Snapshot) -> Result<Snapshot, RetentionCheckError> {
    let actual =
        read(root, path, MAX_REPORT, true).map_err(|_| RetentionCheckError::InputChanged {
            path: path.into(),
            expected: Box::new(expected.identity.clone()),
            actual: fs::symlink_metadata(root.join(path))
                .ok()
                .map(|metadata| Box::new(identity(&metadata))),
        })?;
    if expected.identity != actual.identity || expected.bytes != actual.bytes {
        return Err(RetentionCheckError::InputChanged {
            path: path.into(),
            expected: Box::new(expected.identity.clone()),
            actual: Some(Box::new(actual.identity)),
        });
    }
    Ok(actual)
}
fn recheck(root: &Path, before: &BTreeMap<String, Snapshot>) -> Result<(), RetentionCheckError> {
    if let Some(projection) = before.get(PROJECTION) {
        reread(root, PROJECTION, projection)?;
        engine(&engine_sources(root)?)?;
    }
    Ok(())
}
/// Accept the actual retained report without rewriting it.
/// # Errors
/// Returns complete projection, audit, file or source-stability refusals.
pub fn check_retention(root: impl AsRef<Path>) -> Result<PreparedRetention, RetentionCheckError> {
    check_with(root.as_ref(), &mut || {})
}
fn check_with(
    root: &Path,
    after_derivation: &mut impl FnMut(),
) -> Result<PreparedRetention, RetentionCheckError> {
    check_against(root, None, after_derivation)
}
fn check_against(
    root: &Path,
    production: Option<&std::sync::Arc<retention::Produced>>,
    after_derivation: &mut impl FnMut(),
) -> Result<PreparedRetention, RetentionCheckError> {
    let report = read(root, REPORT, MAX_REPORT, true)?;
    let mut captured = BTreeMap::new();
    let accepted = retention::accept_after(
        report.bytes.clone(),
        || {
            captured = sources(root)?;
            engine(&captured)?;
            crate::check_projection(root).map_err(RetentionCheckError::Projection)?;
            Ok::<_, RetentionCheckError>(captured[PROJECTION].bytes.clone())
        },
        production,
    );
    after_derivation();
    recheck(root, &captured)?;
    reread(root, REPORT, &report)?;

    Ok(PreparedRetention {
        accepted: accepted?,
        sources: captured,
        report_identity: report.identity,
    })
}
/// Derive once, atomically replace the report, then strictly reread that same result.
/// # Errors
/// Preserves primary I/O failures, cleanup failures and replacement paths.
pub fn regenerate_retention(
    root: impl AsRef<Path>,
) -> Result<PreparedRetention, RetentionCheckError> {
    regenerate_with(root.as_ref(), &mut |_, _| Ok(()))
}
fn regenerate_with(
    root: &Path,
    before: &mut impl FnMut(Operation, &str) -> io::Result<()>,
) -> Result<PreparedRetention, RetentionCheckError> {
    let sources = sources(root)?;
    engine(&sources)?;
    crate::check_projection(root).map_err(RetentionCheckError::Projection)?;
    let production = retention::produce(sources[PROJECTION].bytes.clone())
        .map_err(|error| RetentionCheckError::Audit(Box::new(error)))?;
    recheck(root, &sources)?;
    publish(root, &sources, &production, before)
}
fn publish(
    root: &Path,
    sources: &BTreeMap<String, Snapshot>,
    production: &std::sync::Arc<retention::Produced>,
    before: &mut impl FnMut(Operation, &str) -> io::Result<()>,
) -> Result<PreparedRetention, RetentionCheckError> {
    let temporary = format!(
        "generated/.audit-report-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let directory = open_regular(root, "generated")?;
    before(Operation::Create, &temporary)
        .map_err(|error| io(&temporary, Operation::Create, &error))?;
    let mut file = lifecycle::create(&directory, root, &temporary)
        .map_err(|error| io(&temporary, Operation::Create, &error))?;
    let result = (|| {
        before(Operation::Write, &temporary)
            .map_err(|error| io(&temporary, Operation::Write, &error))?;
        file.write_all(production.bytes())
            .map_err(|error| io(&temporary, Operation::Write, &error))?;
        before(Operation::Flush, &temporary)
            .map_err(|error| io(&temporary, Operation::Flush, &error))?;
        file.flush()
            .map_err(|error| io(&temporary, Operation::Flush, &error))?;
        before(Operation::Sync, &temporary)
            .map_err(|error| io(&temporary, Operation::Sync, &error))?;
        file.sync_all()
            .map_err(|error| io(&temporary, Operation::Sync, &error))?;
        before(Operation::Rename, REPORT).map_err(|error| io(REPORT, Operation::Rename, &error))?;
        recheck(root, sources)?;
        lifecycle::replace(&directory, root, &temporary)
            .map_err(|error| io(REPORT, Operation::Rename, &error))
    })();
    if let Err(mut error) = result {
        drop(file);
        if let Err(cleanup) = before(Operation::Remove, &temporary)
            .and_then(|()| lifecycle::remove(&directory, root, &temporary))
        {
            let cleanup = failure(&temporary, Operation::Remove, &cleanup);
            error = match error {
                RetentionCheckError::Io {
                    primary,
                    cleanup: mut failures,
                    replaced,
                } => {
                    failures.push(cleanup);
                    RetentionCheckError::Io {
                        primary,
                        cleanup: failures,
                        replaced,
                    }
                }
                error => RetentionCheckError::Cleanup {
                    error: Box::new(error),
                    failures: vec![cleanup],
                },
            };
        }
        return Err(error);
    }
    let published = identity(
        &file
            .metadata()
            .map_err(|error| after_replacement(io(REPORT, Operation::Metadata, &error)))?,
    );
    drop(file);
    before(Operation::DirectorySync, "generated")
        .map_err(|error| after_replacement(io("generated", Operation::DirectorySync, &error)))?;
    directory
        .sync_all()
        .map_err(|error| after_replacement(io("generated", Operation::DirectorySync, &error)))?;
    before(Operation::Reread, REPORT)
        .map_err(|error| after_replacement(io(REPORT, Operation::Reread, &error)))?;
    let checked = check_against(root, Some(production), &mut || {});
    recheck(root, sources).map_err(after_replacement)?;
    let checked = checked.map_err(after_replacement)?;
    if checked.report_identity != published {
        return Err(after_replacement(RetentionCheckError::InputChanged {
            path: REPORT.into(),
            expected: Box::new(published),
            actual: Some(Box::new(checked.report_identity)),
        }));
    }
    Ok(checked)
}
fn after_replacement(error: RetentionCheckError) -> RetentionCheckError {
    if let RetentionCheckError::Io {
        primary,
        cleanup,
        mut replaced,
    } = error
    {
        replaced.push(REPORT.into());
        return RetentionCheckError::Io {
            primary,
            cleanup,
            replaced,
        };
    }
    RetentionCheckError::AfterReplacement {
        error: Box::new(error),
        replaced: vec![REPORT.into()],
    }
}
/// Check faithful retention, passing release assessment and existing base documents.
/// # Errors
/// Returns technical refusals; this does not certify the separate publication-policy audit.
pub fn check_release_retention(
    root: &Path,
    support: &crate::NativeSupport,
) -> Result<PreparedRetention, RetentionCheckError> {
    check_release_with_documents(root, support).map(|(retained, _)| retained)
}
pub(crate) fn check_release_with_documents(
    root: &Path,
    support: &crate::NativeSupport,
) -> Result<(PreparedRetention, crate::PreparedDocuments), RetentionCheckError> {
    let prepared = check_retention(root)?;
    release_assessment(prepared.report().assessment())?;
    let documents = prepare_bound_documents(root, support, &prepared)?;
    crate::documents::artifacts::verify(root, &documents)
        .map_err(|error| RetentionCheckError::Documents(Box::new(error)))?;
    crate::documents::artifacts::recheck(root, &documents)
        .map_err(|error| RetentionCheckError::Documents(Box::new(error)))?;
    recheck(root, &prepared.sources)?;
    reread(
        root,
        REPORT,
        &Snapshot {
            identity: prepared.report_identity.clone(),
            bytes: prepared.bytes().into(),
        },
    )?;
    Ok((prepared, documents))
}
fn release_assessment(assessment: &ReleaseAssessment) -> Result<(), RetentionCheckError> {
    if assessment.disposition() == ftoo_manifest::Disposition::Pass {
        Ok(())
    } else {
        Err(RetentionCheckError::ReleaseRejected(assessment.clone()))
    }
}

/// Prepare reader artifacts from an accepted, stable retained report.
/// # Errors
/// Refuses retained inputs, source changes, rendering or typesetting failures.
pub fn prepare_retained_documents(
    root: &Path,
    support: &crate::NativeSupport,
) -> Result<crate::PreparedDocuments, RetentionCheckError> {
    let retained = check_retention(root)?;
    prepare_bound_documents(root, support, &retained)
}
fn prepare_bound_documents(
    root: &Path,
    support: &crate::NativeSupport,
    retained: &PreparedRetention,
) -> Result<crate::PreparedDocuments, RetentionCheckError> {
    recheck(root, &retained.sources)?;
    let snapshot = Snapshot {
        identity: retained.report_identity.clone(),
        bytes: retained.bytes().to_vec(),
    };
    reread(root, REPORT, &snapshot)?;
    let result = crate::documents::prepare_context(root, support, Some(&retained.accepted));
    recheck(root, &retained.sources)?;
    reread(root, REPORT, &snapshot)?;
    result.map_err(|error| RetentionCheckError::Documents(Box::new(error)))
}
