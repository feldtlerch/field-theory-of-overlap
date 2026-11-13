//! Pure document ownership and bounded shared-math interpretation.
// Keep complete typed provenance inline, matching the document error contract.
#![allow(clippy::result_large_err)]
mod diagnostics;
mod display;
#[cfg(test)]
mod display_tests;
mod math;
mod skeleton;
#[cfg(test)]
mod tests;
pub use diagnostics::*;
pub use display::*;
pub use math::{MathCommandUse, MathSourceRange, ParsedAssemblyMath, parse_assembly_math};
pub use skeleton::{
    CheckedSkeletons, SkeletonChild, SkeletonDraft, SkeletonPosition, SkeletonSection,
    check_skeletons,
};

/// Locate complete semantic-label tokens in decoded reader prose.
#[must_use]
pub fn reader_label_literals(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (at, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if c.is_alphanumeric() || matches!(c, ':' | '-' | '_') {
            start.get_or_insert(at);
        } else if let Some(begin) = start.take()
            && crate::units::label_name(&text[begin..at])
        {
            ranges.push(begin..at);
        }
    }
    ranges
}

/// Read admitted label spans through the same scanner used by unit admission.
/// # Panics
/// Panics only if the shared scanner disagrees with its own admission, which callers cannot arrange.
#[must_use]
pub fn assembly_text_spans(text: &crate::CheckedText) -> Vec<(String, crate::UnitSite, bool)> {
    assembly_spans(
        text.text(),
        text.site(),
        text.occurrences(),
        text.tag_occurrences(),
    )
}
fn assembly_spans(
    text: &str,
    site: &crate::UnitSite,
    math: &[crate::MathOccurrence],
    tags: &[crate::TagOccurrence],
) -> Vec<(String, crate::UnitSite, bool)> {
    let mut spans = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if let Some(end) = math
            .iter()
            .map(crate::MathOccurrence::site)
            .chain(tags.iter().map(crate::TagOccurrence::site))
            .filter_map(|s| s.decoded.as_ref())
            .find(|r| r.start == at)
            .map(|r| r.end)
        {
            at = end;
            continue;
        }
        if text.as_bytes()[at] == b'`' {
            let span = crate::units::spans::parse(text, site, at, spans.len())
                .expect("same admitted scanner");
            let mut site = site.clone();
            site.decoded = Some(span.bytes.clone());
            site.occurrence = None;
            at = span.bytes.end;
            spans.push((span.body.into(), site, span.citation));
        } else {
            at += text[at..]
                .chars()
                .next()
                .expect("nonempty suffix")
                .len_utf8();
        }
    }
    spans
}
