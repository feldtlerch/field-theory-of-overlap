//! Recursive Markdown and actual-file joint label integration.
mod files;
#[cfg(test)]
mod tests;
use crate::UnitLoadError;
#[cfg(test)]
pub(crate) use files::checkout_fixture_read_guard;
use ftoo_manifest::{
    AddressBook, CheckedMarkdown, LabelErrors, LabelSourceError, MintKind, check_markdown_sources,
    integrate_labels,
};
use std::{
    fmt, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelIoOperation {
    ListDirectory,
    InspectEntry,
    ReadMarkdown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkdownEntryKind {
    Symlink,
    Directory,
    NonUtf8Name,
    Other,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelEntryFault {
    UnexpectedEntry {
        file: PathBuf,
        kind: MarkdownEntryKind,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelFileError {
    Units(UnitLoadError),
    Source(LabelSourceError),
    Input(LabelEntryFault),
    Io {
        operation: LabelIoOperation,
        file: PathBuf,
        kind: io::ErrorKind,
    },
    Labels(LabelErrors),
}
impl fmt::Display for LabelFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Units(error) => write!(f, "{error}"),
            Self::Source(error) => write!(f, "{error}"),
            Self::Labels(error) => write!(f, "{error}"),
            Self::Input(_) | Self::Io { .. } => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for LabelFileError {}
/// Discover and independently admit every recursively matching Markdown source.
/// # Errors
/// Refuses collection, encoding and Markdown-local semantic faults.
pub fn load_markdown_files(root: &Path) -> Result<CheckedMarkdown, LabelFileError> {
    check_markdown_sources(files::collect(root)?).map_err(LabelFileError::Source)
}
/// Owned counts of a complete actual-file label check, without a weld claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LabelCheckSummary {
    pub units: usize,
    pub texts: usize,
    pub tag_sites: usize,
    pub rust_mints: usize,
    pub markdown_mints: usize,
    pub unit_mints: usize,
    pub markdown_citations: usize,
    pub rust_citations: usize,
}
/// Reread units and Markdown, and join labels against the exact supplied book.
/// # Errors
/// Preserves unit prerequisites, source failures and complete label diagnostics.
pub fn check_label_files(
    root: &Path,
    book: &AddressBook<'_>,
) -> Result<LabelCheckSummary, LabelFileError> {
    let (corpus, declaration) =
        crate::units::load_unit_files_with_declaration(root).map_err(LabelFileError::Units)?;
    let resolved = crate::units::checked_resolution(root, &declaration, &corpus, book)
        .map_err(LabelFileError::Units)?;
    let markdown = load_markdown_files(root)?;
    let integrated = integrate_labels(&resolved, &markdown).map_err(LabelFileError::Labels)?;
    Ok(LabelCheckSummary {
        units: corpus.units().len(),
        texts: corpus.units().iter().map(|u| u.texts().len()).sum(),
        tag_sites: resolved.tags().len(),
        rust_mints: integrated
            .mints()
            .values()
            .filter(|o| o.kind == MintKind::RustIdentity)
            .count(),
        markdown_mints: markdown.mints().len(),
        unit_mints: corpus.units().len(),
        markdown_citations: markdown.citations().len(),
        rust_citations: integrated.rust_citations().len(),
    })
}
