use super::{LabelEntryFault, LabelFileError, LabelIoOperation, MarkdownEntryKind};
use ftoo_manifest::MarkdownSource;
#[cfg(test)]
use std::path::PathBuf;
use std::{fs, io, path::Path};
fn io_error(operation: LabelIoOperation, path: &Path, error: &io::Error) -> LabelFileError {
    LabelFileError::Io {
        operation,
        file: path.to_owned(),
        kind: error.kind(),
    }
}
fn entry_error(path: &Path, kind: MarkdownEntryKind) -> LabelFileError {
    LabelFileError::Input(LabelEntryFault::UnexpectedEntry {
        file: path.to_owned(),
        kind,
    })
}
pub(super) fn collect(root: &Path) -> Result<Vec<MarkdownSource>, LabelFileError> {
    #[cfg(test)]
    let _guard = checkout_fixture_read_guard(root)?;
    collect_with(root, |_, _| Ok(()))
}
#[cfg(test)]
pub(crate) fn checkout_fixture_read_guard(root: &Path) -> Result<Option<fs::File>, LabelFileError> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if root.canonicalize().ok() != repository.canonicalize().ok() {
        return Ok(None);
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| repository.join("target"), PathBuf::from);
    fs::create_dir_all(&target)
        .map_err(|error| io_error(LabelIoOperation::ListDirectory, &target, &error))?;
    let target = target
        .canonicalize()
        .map_err(|error| io_error(LabelIoOperation::ListDirectory, &target, &error))?;
    let fixture = target.join("ftoo-native-fixtures");
    fs::create_dir_all(&fixture)
        .map_err(|error| io_error(LabelIoOperation::ListDirectory, &fixture, &error))?;
    let lock_path = fixture.join("lock");
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| io_error(LabelIoOperation::InspectEntry, &lock_path, &error))?;
    lock.lock_shared()
        .map_err(|error| io_error(LabelIoOperation::InspectEntry, &lock_path, &error))?;
    Ok(Some(lock))
}
pub(super) fn collect_with(
    root: &Path,
    mut before: impl FnMut(LabelIoOperation, &Path) -> io::Result<()>,
) -> Result<Vec<MarkdownSource>, LabelFileError> {
    let mut sources = Vec::new();
    visit(root, "", &mut sources, &mut before)?;
    // Path components give the same order as Python's pathlib traversal.
    sources.sort_by(|a, b| a.file.split('/').cmp(b.file.split('/')));
    Ok(sources)
}
fn visit(
    directory: &Path,
    relative: &str,
    sources: &mut Vec<MarkdownSource>,
    before: &mut impl FnMut(LabelIoOperation, &Path) -> io::Result<()>,
) -> Result<(), LabelFileError> {
    before(LabelIoOperation::ListDirectory, directory)
        .map_err(|e| io_error(LabelIoOperation::ListDirectory, directory, &e))?;
    let entries = fs::read_dir(directory)
        .map_err(|e| io_error(LabelIoOperation::ListDirectory, directory, &e))?;
    let mut paths = Vec::new();
    for entry in entries {
        paths.push(
            entry
                .map_err(|e| io_error(LabelIoOperation::InspectEntry, directory, &e))?
                .path(),
        );
    }
    paths.sort();
    for path in paths {
        before(LabelIoOperation::InspectEntry, &path)
            .map_err(|e| io_error(LabelIoOperation::InspectEntry, &path, &e))?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| io_error(LabelIoOperation::InspectEntry, &path, &e))?;
        let matching = path
            .file_name()
            .is_some_and(|n| n.as_encoded_bytes().ends_with(b".md"));
        if !matching && !metadata.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| entry_error(&path, MarkdownEntryKind::NonUtf8Name))?;
        if matching {
            let kind = if metadata.file_type().is_symlink() {
                Some(MarkdownEntryKind::Symlink)
            } else if metadata.is_dir() {
                Some(MarkdownEntryKind::Directory)
            } else if !metadata.is_file() {
                Some(MarkdownEntryKind::Other)
            } else {
                None
            };
            if let Some(kind) = kind {
                return Err(entry_error(&path, kind));
            }
        }
        let file = if relative.is_empty() {
            name.to_owned()
        } else {
            format!("{relative}/{name}")
        };
        if metadata.is_dir() {
            visit(&path, &file, sources, before)?;
        } else if matching {
            before(LabelIoOperation::ReadMarkdown, &path)
                .map_err(|e| io_error(LabelIoOperation::ReadMarkdown, &path, &e))?;
            sources.push(MarkdownSource {
                file,
                bytes: fs::read(&path)
                    .map_err(|e| io_error(LabelIoOperation::ReadMarkdown, &path, &e))?,
            });
        }
    }
    Ok(())
}
