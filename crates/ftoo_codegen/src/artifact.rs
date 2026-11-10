use crate::projection::{Prepared, compare, prepare_projection, registry};
use ftoo_manifest::{Diagnostic, ProjectionError, RuleId, ValidationErrors, load_projection};
use std::{
    error::Error,
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IoOperation {
    CreateDirectory,
    CreateTemporary,
    Write,
    Rename,
    Read,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ByteGateFault {
    NonCanonicalBytes {
        path: PathBuf,
        offset: usize,
        diff: String,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionCheckError {
    SourceGraph(ValidationErrors),
    Projection(ProjectionError),
    Registry(Vec<Diagnostic>),
    MissingArtifact {
        path: PathBuf,
    },
    Io {
        operation: IoOperation,
        path: PathBuf,
        kind: io::ErrorKind,
        replaced: bool,
    },
    ByteGate {
        rule: RuleId,
        fault: ByteGateFault,
    },
    AfterReplacement(Box<Self>),
}
impl fmt::Display for ProjectionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ByteGate {
                rule,
                fault: ByteGateFault::NonCanonicalBytes { path, offset, diff },
            } => write!(
                f,
                "{rule:?}: {} differs at byte {offset}\n{diff}",
                path.display()
            ),
            Self::AfterReplacement(e) => write!(f, "artifact was replaced: {e}"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl Error for ProjectionCheckError {}
fn io_error(
    operation: IoOperation,
    path: &Path,
    error: &io::Error,
    replaced: bool,
) -> ProjectionCheckError {
    ProjectionCheckError::Io {
        operation,
        path: path.into(),
        kind: error.kind(),
        replaced,
    }
}
fn destination(root: &Path) -> PathBuf {
    root.join("generated/manifest.json")
}
type IoHook<'a> = dyn FnMut(IoOperation, &Path, bool) -> io::Result<()> + 'a;
fn read(
    path: &Path,
    replaced: bool,
    hook: &mut IoHook<'_>,
) -> Result<Vec<u8>, ProjectionCheckError> {
    hook(IoOperation::Read, path, replaced)
        .and_then(|()| fs::read(path))
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound && !replaced {
                ProjectionCheckError::MissingArtifact { path: path.into() }
            } else {
                io_error(IoOperation::Read, path, &e, replaced)
            }
        })
}
fn excerpt(bytes: &[u8], start: usize, end: usize) -> String {
    let mut output = String::new();
    for byte in &bytes[start.min(bytes.len())..end.min(bytes.len())] {
        for escaped in std::ascii::escape_default(*byte) {
            output.push(char::from(escaped));
        }
    }
    if end >= bytes.len() {
        output.push_str("<EOF>");
    }
    output
}
fn verify(path: &Path, bytes: &[u8], prepared: &Prepared) -> Result<(), ProjectionCheckError> {
    let loaded = load_projection(bytes).map_err(ProjectionCheckError::Projection)?;
    compare(&prepared.graph, &loaded)?;
    registry(loaded.manifest())?;
    if bytes == prepared.bytes {
        return Ok(());
    }
    let offset = bytes
        .iter()
        .zip(&prepared.bytes)
        .position(|(a, e)| a != e)
        .unwrap_or(bytes.len().min(prepared.bytes.len()));
    let start = offset.saturating_sub(24);
    let end = offset + 25;
    let diff = format!(
        "--- expected/{}\n+++ actual/{}\n-{}\n+{}\n@@ byte {offset} @@\n-{}\n+{}\n",
        path.display(),
        path.display(),
        excerpt(&prepared.bytes, 0, prepared.bytes.len()),
        excerpt(bytes, 0, bytes.len()),
        excerpt(&prepared.bytes, start, end),
        excerpt(bytes, start, end)
    );
    Err(ProjectionCheckError::ByteGate {
        rule: RuleId::M038,
        fault: ByteGateFault::NonCanonicalBytes {
            path: path.into(),
            offset,
            diff,
        },
    })
}
/// Check the actual artifact without modifying it or changing the working directory.
///
/// # Errors
/// Reports missing/I/O, strict admission, semantic, registry or byte-gate faults.
pub fn check_projection(root: impl AsRef<Path>) -> Result<(), ProjectionCheckError> {
    check_with(root.as_ref(), &mut |_, _, _| Ok(()))
}
pub(crate) fn check_with(root: &Path, hook: &mut IoHook<'_>) -> Result<(), ProjectionCheckError> {
    let prepared = prepare_projection()?;
    let path = destination(root);
    verify(&path, &read(&path, false, hook)?, &prepared)
}
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
/// Atomically replace the artifact after preparation, then reopen and verify it.
///
/// # Errors
/// Pre-replacement errors preserve the destination. Post-replacement errors
/// explicitly report replacement rather than claiming rollback.
pub fn generate_projection(root: impl AsRef<Path>) -> Result<(), ProjectionCheckError> {
    generate_with(root.as_ref(), &mut |_, _, _| Ok(()))
}
pub(crate) fn generate_with(
    root: &Path,
    hook: &mut IoHook<'_>,
) -> Result<(), ProjectionCheckError> {
    let prepared = prepare_projection()?;
    let path = destination(root);
    let directory = root.join("generated");
    hook(IoOperation::CreateDirectory, &directory, false)
        .and_then(|()| fs::create_dir_all(&directory))
        .map_err(|e| io_error(IoOperation::CreateDirectory, &directory, &e, false))?;
    let temporary = directory.join(format!(
        ".manifest-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = hook(IoOperation::CreateTemporary, &temporary, false)
        .and_then(|()| {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
        })
        .map_err(|e| io_error(IoOperation::CreateTemporary, &temporary, &e, false))?;
    let result = hook(IoOperation::Write, &temporary, false)
        .and_then(|()| file.write_all(&prepared.bytes))
        .map_err(|e| io_error(IoOperation::Write, &temporary, &e, false));
    drop(file);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) =
        hook(IoOperation::Rename, &path, false).and_then(|()| fs::rename(&temporary, &path))
    {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(IoOperation::Rename, &path, &error, false));
    }
    let bytes = read(&path, true, hook)?;
    verify(&path, &bytes, &prepared)
        .map_err(|e| ProjectionCheckError::AfterReplacement(Box::new(e)))
}
