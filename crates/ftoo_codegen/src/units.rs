//! Strict authored TOML input, separate from the projection contract.
mod files;
#[cfg(test)]
mod tests;
mod toml;
use ftoo_manifest::{LanguageCheckedUnits, UnitDiagnostic, UnitErrors, admit_unit_drafts};
use std::{
    collections::BTreeMap,
    fmt, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitSource {
    pub file: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitIoOperation {
    ReadDeclarations,
    ListStatements,
    InspectEntry,
    ReadStatement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    Directory,
    Symlink,
    NonTomlFile,
    Other,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitInputFault {
    InvalidSourceName { value: String },
    DuplicateSource { file: String, positions: Vec<usize> },
    EmptyCorpus { directory: String },
    UnexpectedEntry { file: PathBuf, kind: EntryKind },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitLoadError {
    Input(UnitInputFault),
    Io {
        operation: UnitIoOperation,
        file: PathBuf,
        kind: io::ErrorKind,
    },
    Admission(UnitErrors),
}
impl fmt::Display for UnitInputFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl fmt::Display for UnitLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(errors) => write!(f, "{errors}"),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for UnitLoadError {}
fn valid_source_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value.split('/').all(|p| !matches!(p, "" | "." | ".."))
}
fn phase<T>(
    values: impl Iterator<Item = Result<T, Box<UnitDiagnostic>>>,
) -> Result<Vec<T>, UnitLoadError> {
    let mut accepted = Vec::new();
    let mut errors = Vec::new();
    for value in values {
        match value {
            Ok(value) => accepted.push(value),
            Err(error) => errors.push(*error),
        }
    }
    if let Some(errors) = UnitErrors::from_diagnostics(errors) {
        Err(UnitLoadError::Admission(errors))
    } else {
        Ok(accepted)
    }
}
/// Read one declaration and a duplicate-preserving statement vector.
/// Source-name positions count the declaration as zero.
/// # Errors
/// Refuses collection errors, then the earliest failing global byte or semantic phase.
/// # Panics
/// Panics only if the private shape pass loses the unique declaration source,
/// which caller-supplied bytes cannot arrange.
pub fn load_unit_sources(
    declaration: UnitSource,
    statements: Vec<UnitSource>,
) -> Result<LanguageCheckedUnits, UnitLoadError> {
    if statements.is_empty() {
        return Err(UnitLoadError::Input(UnitInputFault::EmptyCorpus {
            directory: "units/statements".to_owned(),
        }));
    }
    let declaration_name = declaration.file.clone();
    let mut sources = vec![declaration];
    sources.extend(statements);
    let mut names: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (position, source) in sources.iter().enumerate() {
        if !valid_source_name(&source.file) {
            return Err(UnitLoadError::Input(UnitInputFault::InvalidSourceName {
                value: source.file.clone(),
            }));
        }
        names.entry(&source.file).or_default().push(position);
    }
    for (file, positions) in names {
        if positions.len() > 1 {
            return Err(UnitLoadError::Input(UnitInputFault::DuplicateSource {
                file: file.to_owned(),
                positions,
            }));
        }
    }
    sources.sort_by(|a, b| a.file.cmp(&b.file));
    let texts = phase(sources.iter().map(toml::encoding))?;
    let trees = phase(
        sources
            .iter()
            .zip(&texts)
            .map(|(source, text)| toml::parse(&source.file, text)),
    )?;
    phase(
        trees
            .iter()
            .zip(&texts)
            .map(|(tree, text)| toml::numeric(tree, text)),
    )?;
    let drafts = phase(
        trees
            .iter()
            .zip(&texts)
            .map(|(tree, text)| toml::shape(tree, text, tree.site.file == declaration_name)),
    )?;
    let mut declarations = None;
    let mut units = Vec::new();
    for draft in drafts {
        match draft {
            toml::Draft::Documents(draft) => declarations = Some(draft),
            toml::Draft::Unit(draft) => units.push(*draft),
        }
    }
    // Exactly one source had the unique declaration name.
    let Some(declarations) = declarations else {
        unreachable!("unique declaration source was shaped")
    };
    admit_unit_drafts(declarations, units).map_err(UnitLoadError::Admission)
}
/// Read the declaration and every immediate statement entry without writing files.
/// # Errors
/// Refuses I/O and non-flat inventories before applying the same byte admission.
pub fn load_unit_files(root: &Path) -> Result<LanguageCheckedUnits, UnitLoadError> {
    let (declaration, statements) = files::collect(root)?;
    load_unit_sources(declaration, statements)
}

/// Owned counts from a successful actual-file check; no detached resolved identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitCheckSummary {
    pub documents: usize,
    pub units: usize,
    pub texts: usize,
    pub tag_sites: usize,
}
/// Reread actual sources and resolve every tag through the supplied book.
/// No file is written and no projection authority is selected implicitly.
/// # Errors
/// Preserves loading failures and the complete located resolution fault collection.
pub fn check_unit_files(
    root: &Path,
    book: &ftoo_manifest::AddressBook<'_>,
) -> Result<UnitCheckSummary, UnitLoadError> {
    let corpus = load_unit_files(root)?;
    let resolved =
        ftoo_manifest::resolve_unit_tags(&corpus, book).map_err(UnitLoadError::Admission)?;
    Ok(UnitCheckSummary {
        documents: corpus.documents().len(),
        units: corpus.units().len(),
        texts: corpus.units().iter().map(|unit| unit.texts().len()).sum(),
        tag_sites: resolved.tags().len(),
    })
}
