use super::{EntryKind, UnitInputFault, UnitIoOperation, UnitLoadError, UnitSource};
use std::{fs, io, path::Path};
fn io_error(operation: UnitIoOperation, path: &Path, error: &io::Error) -> UnitLoadError {
    UnitLoadError::Io {
        operation,
        file: path.to_owned(),
        kind: error.kind(),
    }
}
pub(super) fn collect(root: &Path) -> Result<(UnitSource, Vec<UnitSource>), UnitLoadError> {
    collect_with(root, |_, _| Ok(()))
}
pub(super) fn collect_with(
    root: &Path,
    mut before: impl FnMut(UnitIoOperation, &Path) -> io::Result<()>,
) -> Result<(UnitSource, Vec<UnitSource>), UnitLoadError> {
    let path = root.join("units/documents.toml");
    before(UnitIoOperation::ReadDeclarations, &path)
        .map_err(|e| io_error(UnitIoOperation::ReadDeclarations, &path, &e))?;
    let declaration = UnitSource {
        file: "units/documents.toml".to_owned(),
        bytes: fs::read(&path)
            .map_err(|e| io_error(UnitIoOperation::ReadDeclarations, &path, &e))?,
    };
    let directory = root.join("units/statements");
    before(UnitIoOperation::ListStatements, &directory)
        .map_err(|e| io_error(UnitIoOperation::ListStatements, &directory, &e))?;
    let entries = fs::read_dir(&directory)
        .map_err(|e| io_error(UnitIoOperation::ListStatements, &directory, &e))?;
    let mut paths = Vec::new();
    for entry in entries {
        paths.push(
            entry
                .map_err(|e| io_error(UnitIoOperation::InspectEntry, &directory, &e))?
                .path(),
        );
    }
    paths.sort();
    let mut statements = Vec::new();
    for path in paths {
        before(UnitIoOperation::InspectEntry, &path)
            .map_err(|e| io_error(UnitIoOperation::InspectEntry, &path, &e))?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| io_error(UnitIoOperation::InspectEntry, &path, &e))?;
        let name = path.file_name().and_then(|s| s.to_str());
        let kind = if metadata.file_type().is_symlink() {
            Some(EntryKind::Symlink)
        } else if metadata.is_dir() {
            Some(EntryKind::Directory)
        } else if !metadata.is_file() {
            Some(EntryKind::Other)
        } else if !name
            .is_some_and(|n| n.as_bytes().ends_with(b".toml") && super::valid_source_name(n))
        {
            Some(EntryKind::NonTomlFile)
        } else {
            None
        };
        if let Some(kind) = kind {
            return Err(UnitLoadError::Input(UnitInputFault::UnexpectedEntry {
                file: path,
                kind,
            }));
        }
        let Some(name) = name else {
            unreachable!("validated UTF-8 entry")
        };
        before(UnitIoOperation::ReadStatement, &path)
            .map_err(|e| io_error(UnitIoOperation::ReadStatement, &path, &e))?;
        statements.push(UnitSource {
            file: format!("units/statements/{name}"),
            bytes: fs::read(&path)
                .map_err(|e| io_error(UnitIoOperation::ReadStatement, &path, &e))?,
        });
    }
    Ok((declaration, statements))
}
