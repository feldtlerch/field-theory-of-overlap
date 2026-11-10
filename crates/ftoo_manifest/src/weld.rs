//! Complete per-unit language weld over admitted labels and live tag occurrences.
mod diagnostics;
#[cfg(test)]
mod tests;
use crate::{
    CheckedText, LabelIntegratedUnits, LabelSite, MintOrigin, ReferenceDomain, RuleId, UnitSite,
};
pub use diagnostics::*;
use std::collections::{BTreeMap, BTreeSet};

/// One resolved bare citation, retaining its unique declaration and authored site.
#[derive(Debug)]
pub struct ResolvedUnitCitation {
    label: String,
    origin: MintOrigin,
    site: LabelUseSite,
}
impl ResolvedUnitCitation {
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub fn origin(&self) -> &MintOrigin {
        &self.origin
    }
    #[must_use]
    pub fn site(&self) -> &LabelUseSite {
        &self.site
    }
}
/// The separate addressed sets and all label-span occurrences in one text.
#[derive(Debug)]
pub struct WeldedLanguage {
    tags: TagSide,
    labels: LabelSide,
    citations: Vec<ResolvedUnitCitation>,
    mentions: Vec<LabelUseSite>,
}
impl WeldedLanguage {
    #[must_use]
    pub fn language(&self) -> &str {
        &self.tags.language
    }
    #[must_use]
    pub fn tags(&self) -> &TagSide {
        &self.tags
    }
    #[must_use]
    pub fn labels(&self) -> &LabelSide {
        &self.labels
    }
    #[must_use]
    pub fn citations(&self) -> &[ResolvedUnitCitation] {
        &self.citations
    }
    #[must_use]
    pub fn mentions(&self) -> &[LabelUseSite] {
        &self.mentions
    }
}
#[derive(Debug)]
pub struct WeldedUnit {
    unit: String,
    languages: Vec<WeldedLanguage>,
}
impl WeldedUnit {
    #[must_use]
    pub fn unit(&self) -> &str {
        &self.unit
    }
    #[must_use]
    pub fn languages(&self) -> &[WeldedLanguage] {
        &self.languages
    }
}
/// Resolved citations and separate equal tag/label sets for every unit's languages.
/// This does not certify translation fidelity, mathematical truth or rendering.
/// ```compile_fail,E0451
/// fn forge<'w,'r,'m,'u,'b,'g>(integrated: &'w ftoo_manifest::LabelIntegratedUnits<'r,'m,'u,'b,'g>) {
///     let _ = ftoo_manifest::WeldedUnits { integrated, units: vec![] };
/// }
/// ```
/// ```compile_fail,E0616
/// fn mutate(v: &mut ftoo_manifest::WeldedUnits<'_, '_, '_, '_, '_, '_>) { v.units.clear(); }
/// ```
/// ```compile_fail,E0596
/// fn mutate(v: &ftoo_manifest::WeldedUnits<'_, '_, '_, '_, '_, '_>) { v.units()[0].languages()[0].tags().members.clear(); }
/// ```
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::WeldedUnits::default();
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::WeldedUnits<'static,'static,'static,'static,'static,'static>>();
/// ```
/// ```compile_fail,E0515
/// fn escape<'r,'m,'u,'b,'g>(resolved: &'r ftoo_manifest::ResolvedUnits<'u,'b,'g>, markdown: &'m ftoo_manifest::CheckedMarkdown) -> ftoo_manifest::WeldedUnits<'static,'r,'m,'u,'b,'g> {
///     let integrated = ftoo_manifest::integrate_labels(resolved, markdown).unwrap();
///     ftoo_manifest::weld_units(&integrated).unwrap()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape(resolved: &'static ftoo_manifest::ResolvedUnits<'static,'static,'static>, markdown: ftoo_manifest::CheckedMarkdown) -> ftoo_manifest::WeldedUnits<'static,'static,'static,'static,'static,'static> {
///     let integrated = ftoo_manifest::integrate_labels(resolved, &markdown).unwrap();
///     ftoo_manifest::weld_units(&integrated).unwrap()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape(corpus: ftoo_manifest::LanguageCheckedUnits, book: &'static ftoo_manifest::AddressBook<'static>, markdown: &'static ftoo_manifest::CheckedMarkdown) -> ftoo_manifest::WeldedUnits<'static,'static,'static,'static,'static,'static> {
///     let resolved = ftoo_manifest::resolve_unit_tags(&corpus, book).unwrap();
///     let integrated = ftoo_manifest::integrate_labels(&resolved, markdown).unwrap();
///     ftoo_manifest::weld_units(&integrated).unwrap()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape(corpus: &'static ftoo_manifest::LanguageCheckedUnits, book: ftoo_manifest::AddressBook<'static>, markdown: &'static ftoo_manifest::CheckedMarkdown) -> ftoo_manifest::WeldedUnits<'static,'static,'static,'static,'static,'static> {
///     let resolved = ftoo_manifest::resolve_unit_tags(corpus, &book).unwrap();
///     let integrated = ftoo_manifest::integrate_labels(&resolved, markdown).unwrap();
///     ftoo_manifest::weld_units(&integrated).unwrap()
/// }
/// ```
#[derive(Debug)]
pub struct WeldedUnits<'w, 'r, 'm, 'u, 'b, 'g> {
    integrated: &'w LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g>,
    units: Vec<WeldedUnit>,
}
impl<'w, 'r, 'm, 'u, 'b, 'g> WeldedUnits<'w, 'r, 'm, 'u, 'b, 'g> {
    #[must_use]
    pub fn integrated(&self) -> &'w LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g> {
        self.integrated
    }
    #[must_use]
    pub fn units(&self) -> &[WeldedUnit] {
        &self.units
    }
}
fn diagnostic(unit: &str, language: &str, site: &UnitSite, fault: WeldFault) -> WeldDiagnostic {
    let citation = matches!(fault, WeldFault::UnresolvedUnitCitation { .. });
    WeldDiagnostic {
        rule: if citation { RuleId::M045 } else { RuleId::M046 },
        stage: if citation {
            WeldStage::UnitCitation
        } else {
            WeldStage::Equality
        },
        unit: Some(unit.to_owned()),
        language: Some(language.to_owned()),
        site: LabelSite::Unit {
            unit: unit.to_owned(),
            site: site.clone(),
        },
        fault,
    }
}
fn harvest(text: &CheckedText) -> Vec<(String, LabelUseSite, bool)> {
    let skip: BTreeMap<_, _> = text
        .occurrences()
        .iter()
        .map(crate::MathOccurrence::site)
        .chain(
            text.tag_occurrences()
                .iter()
                .map(crate::TagOccurrence::site),
        )
        .filter_map(|s| s.decoded.as_ref().map(|r| (r.start, r.end)))
        .collect();
    let mut chars = text.text().char_indices().peekable();
    let mut spans = Vec::new();
    while let Some((start, c)) = chars.next() {
        if let Some(end) = skip.get(&start) {
            while chars.peek().is_some_and(|(at, _)| at < end) {
                chars.next();
            }
        } else if c == '`' {
            let span = crate::units::spans::parse(text.text(), text.site(), start, spans.len())
                .expect("admitted text shares the span scanner");
            while chars.peek().is_some_and(|(at, _)| *at < span.bytes.end) {
                chars.next();
            }
            let mut site = text.site().clone();
            site.decoded = Some(span.bytes);
            site.occurrence = None;
            spans.push((
                span.body.to_owned(),
                LabelUseSite {
                    span_index: spans.len(),
                    site,
                },
                span.citation,
            ));
        }
    }
    spans
}
fn language(
    integrated: &LabelIntegratedUnits<'_, '_, '_, '_, '_>,
    unit: &str,
    language: &str,
    text: &CheckedText,
    errors: &mut Vec<WeldDiagnostic>,
) -> WeldedLanguage {
    let mut tags: BTreeMap<String, TagMember> = BTreeMap::new();
    for tag in integrated
        .resolved()
        .tags()
        .iter()
        .filter(|t| t.unit().as_str() == unit && t.language() == language)
    {
        tags.entry(tag.occurrence().input().to_owned())
            .or_insert_with(|| TagMember {
                input: tag.occurrence().input().to_owned(),
                target: tag.target(),
                sites: Vec::new(),
            })
            .sites
            .push(tag.occurrence().site().clone());
    }
    let mut labels: BTreeMap<String, LabelMember> = BTreeMap::new();
    let mut citations = Vec::new();
    let mut mentions = Vec::new();
    for (label, site, citation) in harvest(text) {
        if !citation {
            mentions.push(site);
            continue;
        }
        if let Some(origin) = integrated.mints().get(&label) {
            labels
                .entry(label.clone())
                .or_insert_with(|| LabelMember {
                    label: label.clone(),
                    sites: Vec::new(),
                })
                .sites
                .push(site.clone());
            citations.push(ResolvedUnitCitation {
                label,
                origin: origin.clone(),
                site,
            });
        } else {
            errors.push(diagnostic(
                unit,
                language,
                &site.site,
                WeldFault::UnresolvedUnitCitation {
                    label,
                    span_index: site.span_index,
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::Joint,
                },
            ));
        }
    }
    WeldedLanguage {
        tags: TagSide {
            language: language.to_owned(),
            text_site: text.site().clone(),
            members: tags.into_values().collect(),
        },
        labels: LabelSide {
            language: language.to_owned(),
            text_site: text.site().clone(),
            members: labels.into_values().collect(),
        },
        citations,
        mentions,
    }
}
fn difference<'a>(
    reference: impl Iterator<Item = &'a str>,
    actual: impl Iterator<Item = &'a str>,
) -> (Vec<String>, Vec<String>) {
    let reference: BTreeSet<_> = reference.collect();
    let actual: BTreeSet<_> = actual.collect();
    (
        reference
            .difference(&actual)
            .map(|s| (*s).to_owned())
            .collect(),
        actual
            .difference(&reference)
            .map(|s| (*s).to_owned())
            .collect(),
    )
}
fn compare(unit: &WeldedUnit, errors: &mut Vec<WeldDiagnostic>) {
    let Some((reference, others)) = unit.languages.split_first() else {
        return;
    };
    for actual in others {
        let (missing, unexpected) = difference(
            reference.tags.members.iter().map(|m| m.input.as_str()),
            actual.tags.members.iter().map(|m| m.input.as_str()),
        );
        if !missing.is_empty() || !unexpected.is_empty() {
            errors.push(diagnostic(
                &unit.unit,
                actual.language(),
                &actual.tags.text_site,
                WeldFault::TagSetMismatch {
                    reference: reference.tags.clone(),
                    actual: actual.tags.clone(),
                    missing,
                    unexpected,
                },
            ));
        }
        let (missing, unexpected) = difference(
            reference.labels.members.iter().map(|m| m.label.as_str()),
            actual.labels.members.iter().map(|m| m.label.as_str()),
        );
        if !missing.is_empty() || !unexpected.is_empty() {
            errors.push(diagnostic(
                &unit.unit,
                actual.language(),
                &actual.labels.text_site,
                WeldFault::LabelSetMismatch {
                    reference: reference.labels.clone(),
                    actual: actual.labels.clone(),
                    missing,
                    unexpected,
                },
            ));
        }
    }
}
/// Resolve every unit citation, then compare the separate addressed sets per unit.
/// # Errors
/// Returns all unresolved citations, or all inequalities after citation resolution succeeds.
/// # Panics
/// Panics only if the private admission scanner and its shared span helper disagree,
/// or an internal diagnostic loses its unit site, which callers cannot arrange.
pub fn weld_units<'w, 'r, 'm, 'u, 'b, 'g>(
    integrated: &'w LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g>,
) -> Result<WeldedUnits<'w, 'r, 'm, 'u, 'b, 'g>, WeldErrors> {
    let mut errors = Vec::new();
    let mut units = Vec::new();
    for unit in integrated.resolved().corpus().units() {
        let languages = unit
            .texts()
            .iter()
            .map(|(name, text)| {
                language(integrated, unit.label().as_str(), name, text, &mut errors)
            })
            .collect();
        units.push(WeldedUnit {
            unit: unit.label().as_str().to_owned(),
            languages,
        });
    }
    if let Some(errors) = WeldErrors::collect(errors) {
        return Err(errors);
    }
    let mut errors = Vec::new();
    for unit in &units {
        compare(unit, &mut errors);
    }
    if let Some(errors) = WeldErrors::collect(errors) {
        return Err(errors);
    }
    Ok(WeldedUnits { integrated, units })
}
