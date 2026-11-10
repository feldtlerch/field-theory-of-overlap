use super::{LabelEntryFault, LabelFileError, LabelIoOperation, MarkdownEntryKind};
use ftoo_manifest::MarkdownSource;
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
    collect_with(root, |_, _| Ok(()))
}
pub(super) fn collect_with(
    root: &Path,
    mut before: impl FnMut(LabelIoOperation, &Path) -> io::Result<()>,
) -> Result<Vec<MarkdownSource>, LabelFileError> {
    let mut sources = Vec::new();
    visit(root, "", &mut sources, &mut before)?;
    // pathlib orders paths by components, not by the slash-joined spelling.
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
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| entry_error(&path, MarkdownEntryKind::NonUtf8Name))?;
        let matching = name.as_bytes().ends_with(b".md");
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
