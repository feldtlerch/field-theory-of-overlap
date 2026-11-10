//! Actual-file composition of unit admission, joint labels and the complete weld.
#[cfg(test)]
mod tests;
use crate::{LabelFileError, UnitLoadError, load_markdown_files, load_unit_files};
use ftoo_manifest::{
    AddressBook, MintKind, WeldErrors, integrate_labels, resolve_unit_tags, weld_units,
};
use std::{fmt, path::Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeldFileError {
    Labels(LabelFileError),
    Weld(WeldErrors),
}
impl fmt::Display for WeldFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Labels(error) => write!(f, "{error}"),
            Self::Weld(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for WeldFileError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeldCheckSummary {
    pub units: usize,
    pub texts: usize,
    pub tag_sites: usize,
    pub rust_mints: usize,
    pub markdown_mints: usize,
    pub unit_mints: usize,
    pub markdown_citations: usize,
    pub rust_citations: usize,
    pub unit_citation_sites: usize,
    pub mention_sites: usize,
    pub language_comparisons: usize,
}
/// Reread all unit and Markdown sources and apply the complete per-unit weld.
/// # Errors
/// Preserves every earlier unit and label refusal before reporting weld failures.
pub fn check_weld_files(
    root: &Path,
    book: &AddressBook<'_>,
) -> Result<WeldCheckSummary, WeldFileError> {
    let corpus =
        load_unit_files(root).map_err(|e| WeldFileError::Labels(LabelFileError::Units(e)))?;
    let resolved = resolve_unit_tags(&corpus, book)
        .map_err(|e| WeldFileError::Labels(LabelFileError::Units(UnitLoadError::Admission(e))))?;
    let markdown = load_markdown_files(root).map_err(WeldFileError::Labels)?;
    let integrated = integrate_labels(&resolved, &markdown)
        .map_err(|e| WeldFileError::Labels(LabelFileError::Labels(e)))?;
    let welded = weld_units(&integrated).map_err(WeldFileError::Weld)?;
    Ok(WeldCheckSummary {
        units: corpus.units().len(),
        texts: corpus.units().iter().map(|u| u.texts().len()).sum(),
        tag_sites: resolved.tags().len(),
        rust_mints: integrated
            .mints()
            .values()
            .filter(|m| m.kind == MintKind::RustIdentity)
            .count(),
        markdown_mints: markdown.mints().len(),
        unit_mints: corpus.units().len(),
        markdown_citations: markdown.citations().len(),
        rust_citations: integrated.rust_citations().len(),
        unit_citation_sites: welded
            .units()
            .iter()
            .flat_map(ftoo_manifest::WeldedUnit::languages)
            .map(|l| l.citations().len())
            .sum(),
        mention_sites: welded
            .units()
            .iter()
            .flat_map(ftoo_manifest::WeldedUnit::languages)
            .map(|l| l.mentions().len())
            .sum(),
        language_comparisons: welded
            .units()
            .iter()
            .map(|u| u.languages().len().saturating_sub(1))
            .sum(),
    })
}
