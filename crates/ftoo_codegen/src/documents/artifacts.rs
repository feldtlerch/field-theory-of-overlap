use super::{
    NativeSupport, PreparedDocuments,
    diagnostics::{
        AssemblyFault, DocumentEntryKind, DocumentError, DocumentInputFault, DocumentIoOperation,
        IoFailure,
    },
    failure, input,
};
use ftoo_manifest::{AssemblySite, AssemblyStage, RuleId};
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn inventory(root: &Path) -> Result<Vec<String>, DocumentError> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<String>) -> Result<(), DocumentError> {
        let entries =
            fs::read_dir(path).map_err(|e| input::io_error(DocumentIoOperation::List, path, &e))?;
        for entry in entries {
            let path = entry
                .map_err(|e| input::io_error(DocumentIoOperation::Inspect, path, &e))?
                .path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|e| input::io_error(DocumentIoOperation::Inspect, &path, &e))?;
            if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
                return Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
                    path,
                    kind: if metadata.file_type().is_symlink() {
                        DocumentEntryKind::Symlink
                    } else {
                        DocumentEntryKind::Other
                    },
                }));
            }
            if metadata.is_dir() {
                visit(root, &path, files)?;
            } else {
                files.push(
                    path.strip_prefix(root)
                        .ok()
                        .and_then(Path::to_str)
                        .ok_or_else(|| {
                            DocumentError::Input(DocumentInputFault::UnexpectedEntry {
                                path: path.clone(),
                                kind: DocumentEntryKind::NonUtf8Name,
                            })
                        })?
                        .into(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    let directory = root.join("generated/documents");
    match fs::symlink_metadata(&directory) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
                    path: directory,
                    kind: if metadata.file_type().is_symlink() {
                        DocumentEntryKind::Symlink
                    } else {
                        DocumentEntryKind::Other
                    },
                }));
            }
            visit(root, &directory, &mut files)?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(input::io_error(
                DocumentIoOperation::Inspect,
                &directory,
                &e,
            ));
        }
    }
    for name in ["generated/notation.tex", "generated/addressing.tex"] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(_) => {
                input::read(root, name)?;
                files.push(name.into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(input::io_error(
                    DocumentIoOperation::Inspect,
                    &root.join(name),
                    &e,
                ));
            }
        }
    }
    files.sort();
    Ok(files)
}
fn window(bytes: &[u8], offset: usize) -> String {
    use std::fmt::Write as _;
    let start = offset.saturating_sub(64).min(bytes.len());
    let end = offset.saturating_add(257).min(bytes.len());
    let mut hex = String::new();
    for byte in &bytes[start..end] {
        write!(hex, "{byte:02x}").expect("string write");
    }
    format!(
        "length={} window@{start}={hex}{}",
        bytes.len(),
        if end == bytes.len() { " EOF" } else { "" }
    )
}
pub(crate) fn verify(root: &Path, prepared: &PreparedDocuments) -> Result<(), DocumentError> {
    let actual = inventory(root)?;
    let mut expected: Vec<_> = prepared.artifacts.iter().map(|a| a.path.clone()).collect();
    expected.sort();
    let missing = expected
        .iter()
        .filter(|p| !actual.contains(p))
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = actual
        .iter()
        .filter(|p| !expected.contains(p))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(failure(
            RuleId::M057,
            AssemblyStage::Bytes,
            AssemblySite::Inventory {
                path: "generated/documents".into(),
            },
            AssemblyFault::ArtifactInventory {
                missing,
                unexpected,
            },
        ));
    }
    for artifact in &prepared.artifacts {
        let actual = input::read(root, &artifact.path)?;
        if actual != artifact.bytes {
            let offset = actual
                .iter()
                .zip(&artifact.bytes)
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(artifact.bytes.len()));
            return Err(failure(
                RuleId::M057,
                AssemblyStage::Bytes,
                AssemblySite::Generated {
                    path: artifact.path.clone(),
                    bytes: offset..offset + usize::from(offset < actual.len()),
                    origins: artifact.origins.clone(),
                },
                AssemblyFault::NonCanonicalDocument {
                    path: artifact.path.clone(),
                    offset,
                    diff: format!(
                        "byte {offset}: expected {}; actual {}\n",
                        window(&artifact.bytes, offset),
                        window(&actual, offset)
                    ),
                },
            ));
        }
    }
    Ok(())
}
/// Check actual files without repairs after the complete preparation consumer.
/// # Errors
/// Preserves source/native failures, then reports inventory or exact byte mismatch.
pub fn check_documents(
    root: &Path,
    support: &NativeSupport,
) -> Result<PreparedDocuments, DocumentError> {
    let prepared = crate::retention::prepare_retained_documents(root, support)
        .map_err(|error| DocumentError::Retention(Box::new(error)))?;
    verify(root, &prepared)?;
    recheck(root, &prepared)?;
    Ok(prepared)
}
/// Replace each artifact only after all candidates pass preparation.
/// # Errors
/// Reports I/O with the exact completed replacement/removal inventory.
pub fn generate_documents(
    root: &Path,
    support: &NativeSupport,
) -> Result<PreparedDocuments, DocumentError> {
    let prepared = crate::retention::prepare_retained_documents(root, support)
        .map_err(|error| DocumentError::Retention(Box::new(error)))?;
    recheck(root, &prepared)?;
    publish(root, &prepared, |_, _| Ok(()))?;
    recheck(root, &prepared).map_err(|error| {
        DocumentError::Retention(Box::new(
            crate::retention::RetentionCheckError::AfterReplacement {
                error: Box::new(crate::retention::RetentionCheckError::Documents(Box::new(
                    error,
                ))),
                replaced: prepared
                    .artifacts
                    .iter()
                    .map(|artifact| artifact.path.clone())
                    .collect(),
            },
        ))
    })?;
    Ok(prepared)
}
pub(crate) fn recheck(root: &Path, prepared: &PreparedDocuments) -> Result<(), DocumentError> {
    input::reread(root, prepared.inputs())?;
    if let Some(snapshot) = &prepared.snapshot {
        snapshot
            .recheck(root)
            .map_err(|error| DocumentError::Retention(Box::new(error)))?;
    }
    for directory in ["units/statements", "units/skeletons", "units/displays"] {
        let prefix = format!("{directory}/");
        compare_inventory(
            &prepared
                .inputs
                .keys()
                .filter(|path| path.starts_with(&prefix))
                .cloned()
                .collect::<Vec<_>>(),
            &input::discover(root, directory, true)?,
        )?;
    }
    let markdown = crate::load_markdown_files(root).map_err(DocumentError::Labels)?;
    compare_inventory(
        &prepared
            .inputs
            .keys()
            .filter(|path| path.as_bytes().ends_with(b".md"))
            .cloned()
            .collect::<Vec<_>>(),
        &markdown
            .sources()
            .iter()
            .filter(|source| !source.file.starts_with("generated/documents/"))
            .map(|source| source.file.clone())
            .collect::<Vec<_>>(),
    )?;
    Ok(())
}
fn compare_inventory(expected: &[String], actual: &[String]) -> Result<(), DocumentError> {
    let missing = expected
        .iter()
        .filter(|path| !actual.contains(path))
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = actual
        .iter()
        .filter(|path| !expected.contains(path))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    Err(DocumentError::Input(
        DocumentInputFault::SourceInventoryChanged {
            missing,
            unexpected,
        },
    ))
}
pub(crate) fn publish(
    root: &Path,
    prepared: &PreparedDocuments,
    mut before: impl FnMut(DocumentIoOperation, &Path) -> std::io::Result<()>,
) -> Result<(), DocumentError> {
    let old = inventory(root)?;
    let unexpected = old
        .iter()
        .filter(|file| {
            !prepared.artifacts.iter().any(|a| &a.path == *file)
                && !(file.starts_with("generated/documents/")
                    && (file.as_bytes().ends_with(b".md") || file.as_bytes().ends_with(b".tex")))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !unexpected.is_empty() {
        return Err(failure(
            RuleId::M057,
            AssemblyStage::Bytes,
            AssemblySite::Inventory {
                path: "generated/documents".into(),
            },
            AssemblyFault::ArtifactInventory {
                missing: Vec::new(),
                unexpected,
            },
        ));
    }
    let mut replaced = Vec::new();
    let mut removed = Vec::new();
    for artifact in &prepared.artifacts {
        replace_artifact(root, artifact, &replaced, &removed, &mut before)?;
        replaced.push(artifact.path.clone());
    }
    for file in old {
        if !prepared.artifacts.iter().any(|a| a.path == file)
            && file.starts_with("generated/documents/")
            && (file.as_bytes().ends_with(b".md") || file.as_bytes().ends_with(b".tex"))
        {
            let path = root.join(&file);
            before(DocumentIoOperation::Remove, &path)
                .and_then(|()| fs::remove_file(&path))
                .map_err(|e| DocumentError::Io {
                    operation: DocumentIoOperation::Remove,
                    path,
                    kind: e.kind(),
                    replaced: replaced.clone(),
                    removed: removed.clone(),
                    cleanup: Vec::new(),
                })?;
            removed.push(file);
            let parent = root.join(&removed[removed.len() - 1]);
            let parent = parent.parent().unwrap_or(root);
            before(DocumentIoOperation::SyncDirectory, parent)
                .and_then(|()| fs::File::open(parent)?.sync_all())
                .map_err(|error| DocumentError::Io {
                    operation: DocumentIoOperation::SyncDirectory,
                    path: parent.into(),
                    kind: error.kind(),
                    replaced: replaced.clone(),
                    removed: removed.clone(),
                    cleanup: Vec::new(),
                })?;
        }
    }
    for artifact in &prepared.artifacts {
        let path = root.join(&artifact.path);
        before(DocumentIoOperation::Read, &path).map_err(|e| DocumentError::Io {
            operation: DocumentIoOperation::Read,
            path,
            kind: e.kind(),
            replaced: replaced.clone(),
            removed: removed.clone(),
            cleanup: Vec::new(),
        })?;
    }
    verify(root, prepared).map_err(|error| match error {
        DocumentError::Io {
            operation,
            path,
            kind,
            cleanup,
            ..
        } => DocumentError::Io {
            operation,
            path,
            kind,
            replaced,
            removed,
            cleanup,
        },
        other => other,
    })
}

fn replace_artifact(
    root: &Path,
    artifact: &super::DocumentArtifact,
    replaced: &[String],
    removed: &[String],
    before: &mut impl FnMut(DocumentIoOperation, &Path) -> std::io::Result<()>,
) -> Result<(), DocumentError> {
    let path = root.join(&artifact.path);
    let parent = path.parent().unwrap_or(root);
    before(DocumentIoOperation::CreateDirectory, parent)
        .and_then(|()| fs::create_dir_all(parent))
        .map_err(|e| DocumentError::Io {
            operation: DocumentIoOperation::CreateDirectory,
            path: parent.into(),
            kind: e.kind(),
            replaced: replaced.to_vec(),
            removed: removed.to_vec(),
            cleanup: Vec::new(),
        })?;
    let temporary = parent.join(format!(
        ".document-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        before(DocumentIoOperation::CreateTemporary, &temporary)
            .map_err(|e| (DocumentIoOperation::CreateTemporary, temporary.clone(), e))?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| (DocumentIoOperation::CreateTemporary, temporary.clone(), e))?;
        before(DocumentIoOperation::Write, &temporary)
            .and_then(|()| file.write_all(&artifact.bytes))
            .map_err(|e| (DocumentIoOperation::Write, temporary.clone(), e))?;
        before(DocumentIoOperation::Flush, &temporary)
            .and_then(|()| file.flush())
            .map_err(|e| (DocumentIoOperation::Flush, temporary.clone(), e))?;
        before(DocumentIoOperation::SyncFile, &temporary)
            .and_then(|()| file.sync_all())
            .map_err(|e| (DocumentIoOperation::SyncFile, temporary.clone(), e))?;
        drop(file);
        before(DocumentIoOperation::Rename, &path)
            .and_then(|()| fs::rename(&temporary, &path))
            .map_err(|e| (DocumentIoOperation::Rename, path.clone(), e))
    })();
    if let Err((operation, path, error)) = result {
        let cleanup = match before(DocumentIoOperation::Remove, &temporary)
            .and_then(|()| fs::remove_file(&temporary))
        {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => vec![IoFailure {
                operation: DocumentIoOperation::Remove,
                path: temporary,
                kind: e.kind(),
            }],
            _ => Vec::new(),
        };
        return Err(DocumentError::Io {
            operation,
            path,
            kind: error.kind(),
            replaced: replaced.to_vec(),
            removed: removed.to_vec(),
            cleanup,
        });
    }
    // Rename has completed: a directory-sync refusal must report this replacement too.
    before(DocumentIoOperation::SyncDirectory, parent)
        .and_then(|()| fs::File::open(parent)?.sync_all())
        .map_err(|error| DocumentError::Io {
            operation: DocumentIoOperation::SyncDirectory,
            path: parent.into(),
            kind: error.kind(),
            replaced: replaced
                .iter()
                .cloned()
                .chain([artifact.path.clone()])
                .collect(),
            removed: removed.to_vec(),
            cleanup: Vec::new(),
        })?;
    Ok(())
}
