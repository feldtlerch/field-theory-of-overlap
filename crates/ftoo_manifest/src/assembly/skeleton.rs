use super::{
    AssemblyInputDiagnostic as Diagnostic, AssemblyInputFault as Fault, AssemblyResource,
    AssemblySite, AssemblyStage,
};
use crate::{LanguageCheckedUnits, LocatedString, RuleId, UnitSite};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkeletonDraft {
    pub document: LocatedString,
    pub sections_site: UnitSite,
    pub sections: Vec<SkeletonSection>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkeletonSection {
    pub heading: LocatedString,
    pub children: Vec<SkeletonChild>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkeletonChild {
    Unit(LocatedString),
    Paragraph(LocatedString),
    CorpusSlot(LocatedString),
    Econ(LocatedString),
    Equation(LocatedString),
    Quotation(LocatedString),
    Section(SkeletonSection),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProseRole {
    Paragraph,
    Econ,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkeletonPosition {
    unit: String,
    depth: Option<usize>,
    quotation: bool,
    prose: Option<ProseRole>,
    equation: bool,
    corpus_slot: bool,
    site: UnitSite,
}
impl SkeletonPosition {
    #[must_use]
    pub fn unit(&self) -> &str {
        &self.unit
    }
    #[must_use]
    pub const fn depth(&self) -> Option<usize> {
        self.depth
    }
    #[must_use]
    pub const fn quotation(&self) -> bool {
        self.quotation
    }
    #[must_use]
    pub const fn paragraph(&self) -> bool {
        self.prose.is_some()
    }
    #[must_use]
    pub const fn equation(&self) -> bool {
        self.equation
    }
    #[must_use]
    pub const fn corpus_slot(&self) -> bool {
        self.corpus_slot
    }
    #[must_use]
    pub const fn econ(&self) -> bool {
        matches!(self.prose, Some(ProseRole::Econ))
    }
    #[must_use]
    pub const fn site(&self) -> &UnitSite {
        &self.site
    }
}
/// Ownership is complete over the retained corpus, including every heading.
/// ```compile_fail,E0599
/// let value = ftoo_manifest::CheckedSkeletons::default();
/// ```
/// ```compile_fail,E0277
/// let value: ftoo_manifest::CheckedSkeletons<'_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0616
/// fn mutate(value: &mut ftoo_manifest::CheckedSkeletons<'_>) { value.documents.clear(); }
/// ```
/// ```compile_fail,E0596
/// fn mutate(value: &mut ftoo_manifest::CheckedSkeletons<'_>) { value.documents().clear(); }
/// ```
/// ```compile_fail,E0277
/// fn bypass(corpus: &ftoo_manifest::LanguageCheckedUnits) -> ftoo_manifest::CheckedSkeletons<'_> {
///     corpus.into()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape(corpus: ftoo_manifest::LanguageCheckedUnits) -> ftoo_manifest::CheckedSkeletons<'static> {
///     ftoo_manifest::check_skeletons(&corpus, &[], &Default::default(), &Default::default(), &Default::default()).unwrap()
/// }
/// ```
/// ```
/// fn inspect<'a>(value: &ftoo_manifest::CheckedSkeletons<'a>) -> &'a ftoo_manifest::LanguageCheckedUnits {
///     for positions in value.documents().values() {
///         for position in positions {
///             let _: &str = position.unit();
///             let _: Option<usize> = position.depth();
///             let _: &ftoo_manifest::UnitSite = position.site();
///         }
///     }
///     value.corpus()
/// }
/// ```
#[derive(Debug)]
pub struct CheckedSkeletons<'a> {
    corpus: &'a LanguageCheckedUnits,
    documents: BTreeMap<String, Vec<SkeletonPosition>>,
}
impl<'a> CheckedSkeletons<'a> {
    #[must_use]
    pub const fn corpus(&self) -> &'a LanguageCheckedUnits {
        self.corpus
    }
    #[must_use]
    pub fn documents(&self) -> &BTreeMap<String, Vec<SkeletonPosition>> {
        &self.documents
    }
}
fn diagnostic(document: &str, unit: Option<&str>, site: &UnitSite, fault: Fault) -> Diagnostic {
    Diagnostic {
        rule: RuleId::M054,
        stage: AssemblyStage::Ownership,
        document: Some(document.into()),
        unit: unit.map(str::to_owned),
        language: None,
        target: None,
        site: AssemblySite::Source(site.clone()),
        fault,
    }
}
/// Check the complete ordered structure against admitted units and factual sites.
/// # Errors
/// Returns every independent ownership fault, or the first resource excess.
#[allow(clippy::too_many_lines)] // Source-order ownership checks share one traversal.
pub fn check_skeletons<'a>(
    corpus: &'a LanguageCheckedUnits,
    drafts: &[SkeletonDraft],
    unit_sites: &BTreeMap<String, UnitSite>,
    owner_sites: &BTreeMap<String, UnitSite>,
    document_sites: &BTreeMap<String, UnitSite>,
) -> Result<CheckedSkeletons<'a>, Vec<Diagnostic>> {
    check_skeletons_with_slots(
        corpus,
        drafts,
        unit_sites,
        owner_sites,
        document_sites,
        &BTreeMap::new(),
    )
}

/// Check the ordered structure, including each checked coarse slot and its native child uses.
/// # Errors
/// Returns every independent ownership fault, or the first resource excess.
#[allow(clippy::too_many_lines)] // Source-order ownership checks share one traversal.
pub fn check_skeletons_with_slots<'a>(
    corpus: &'a LanguageCheckedUnits,
    drafts: &[SkeletonDraft],
    unit_sites: &BTreeMap<String, UnitSite>,
    owner_sites: &BTreeMap<String, UnitSite>,
    document_sites: &BTreeMap<String, UnitSite>,
    slots: &BTreeMap<String, crate::corpus::CheckedSlot>,
) -> Result<CheckedSkeletons<'a>, Vec<Diagnostic>> {
    let mut documents = BTreeMap::new();
    let mut errors = Vec::new();
    let mut owners: BTreeMap<String, Vec<UnitSite>> = BTreeMap::new();
    let mut references: BTreeMap<String, Vec<UnitSite>> = BTreeMap::new();
    let mut slot_references: BTreeMap<String, Vec<UnitSite>> = BTreeMap::new();
    let mut positions = 0;
    for draft in drafts {
        let document = &draft.document.value;
        owners
            .entry(document.clone())
            .or_default()
            .push(draft.document.site.clone());
        document_shape(corpus, draft, &mut errors);
        let mut blocks = Vec::new();
        let mut stack: Vec<_> = draft
            .sections
            .iter()
            .rev()
            .map(|s| {
                (
                    Some(s),
                    &s.heading,
                    Some(1),
                    false,
                    false,
                    false,
                    false,
                    false,
                )
            })
            .collect();
        while let Some((section, location, depth, quotation, paragraph, equation, econ, slot)) =
            stack.pop()
        {
            if depth.is_some_and(|d| d > 6) {
                return Err(vec![diagnostic(
                    document,
                    Some(&location.value),
                    &location.site,
                    Fault::Limit {
                        resource: AssemblyResource::SkeletonDepth,
                        limit: 6,
                        actual: 7,
                    },
                )]);
            }
            if positions == 4096 {
                return Err(vec![diagnostic(
                    document,
                    Some(&location.value),
                    &location.site,
                    Fault::Limit {
                        resource: AssemblyResource::UnitPositions,
                        limit: 4096,
                        actual: 4097,
                    },
                )]);
            }
            positions += 1;
            if let Some(unit) = corpus
                .units()
                .iter()
                .find(|unit| unit.label().as_str() == location.value)
                && unit.econ().is_some() != econ
            {
                errors.push(diagnostic(
                    document,
                    Some(&location.value),
                    &location.site,
                    Fault::WrongOwner {
                        label: location.value.clone(),
                        expected: "econ metadata and role agree".into(),
                        actual: if econ {
                            "econ role without metadata"
                        } else {
                            "econ metadata without role"
                        }
                        .into(),
                        declared_at: location.site.clone(),
                    },
                ));
            }
            if slot {
                let Some(checked) = slots.get(&location.value) else {
                    errors.push(diagnostic(
                        document,
                        Some(&location.value),
                        &location.site,
                        Fault::UnknownUnit {
                            label: location.value.clone(),
                            available: slots.keys().cloned().collect(),
                        },
                    ));
                    continue;
                };
                if checked.document() != document {
                    errors.push(diagnostic(
                        document,
                        Some(&location.value),
                        &location.site,
                        Fault::WrongOwner {
                            label: location.value.clone(),
                            expected: checked.document().into(),
                            actual: document.clone(),
                            declared_at: location.site.clone(),
                        },
                    ));
                }
                slot_references
                    .entry(location.value.clone())
                    .or_default()
                    .push(location.site.clone());
                for child in checked.blocks() {
                    if let crate::corpus::SlotBlock::Use { unit, .. } = child {
                        references
                            .entry(unit.clone())
                            .or_default()
                            .push(location.site.clone());
                        reference(
                            corpus,
                            document,
                            &LocatedString {
                                value: unit.clone(),
                                site: location.site.clone(),
                            },
                            owner_sites,
                            &mut errors,
                        );
                    }
                }
            } else if quotation {
                quotation_reference(corpus, document, location, &mut errors);
            } else {
                references
                    .entry(location.value.clone())
                    .or_default()
                    .push(location.site.clone());
                reference(corpus, document, location, owner_sites, &mut errors);
            }
            blocks.push(SkeletonPosition {
                unit: location.value.clone(),
                depth,
                quotation,
                prose: if econ {
                    Some(ProseRole::Econ)
                } else if paragraph {
                    Some(ProseRole::Paragraph)
                } else {
                    None
                },
                equation,
                corpus_slot: slot,
                site: location.site.clone(),
            });
            if let Some(section) = section {
                for child in section.children.iter().rev() {
                    match child {
                        SkeletonChild::Unit(unit) => {
                            stack.push((None, unit, None, false, false, false, false, false));
                        }
                        SkeletonChild::Paragraph(unit) => {
                            stack.push((None, unit, None, false, true, false, false, false));
                        }
                        SkeletonChild::CorpusSlot(unit) => {
                            stack.push((None, unit, None, false, false, false, false, true));
                        }
                        SkeletonChild::Econ(unit) => {
                            stack.push((None, unit, None, false, false, false, true, false));
                        }
                        SkeletonChild::Equation(unit) => {
                            stack.push((None, unit, None, false, false, true, false, false));
                        }
                        SkeletonChild::Quotation(unit) => {
                            stack.push((None, unit, None, true, false, false, false, false));
                        }
                        SkeletonChild::Section(section) => stack.push((
                            Some(section),
                            &section.heading,
                            Some(depth.unwrap_or(0) + 1),
                            false,
                            false,
                            false,
                            false,
                            false,
                        )),
                    }
                }
            }
        }
        documents.insert(document.clone(), blocks);
    }
    multiplicity(
        corpus,
        &owners,
        &references,
        unit_sites,
        document_sites,
        &mut errors,
    );
    for (label, checked) in slots {
        let sites = slot_references.get(label).cloned().unwrap_or_default();
        if sites.len() != 1 {
            let site = sites.first().cloned().unwrap_or_else(|| UnitSite {
                file: checked.source().into(),
                path: Vec::new(),
                anchor: crate::SourceAnchor::Unavailable,
                decoded: None,
                occurrence: None,
            });
            errors.push(diagnostic(
                checked.document(),
                Some(label),
                &site,
                Fault::UnitMultiplicity {
                    label: label.clone(),
                    expected: 1,
                    actual: sites.len(),
                    sites,
                },
            ));
        }
    }
    if errors.is_empty() {
        Ok(CheckedSkeletons { corpus, documents })
    } else {
        super::diagnostics::sort(&mut errors);
        Err(errors)
    }
}
fn quotation_reference(
    corpus: &LanguageCheckedUnits,
    document: &str,
    location: &LocatedString,
    errors: &mut Vec<Diagnostic>,
) {
    let Some(unit) = corpus
        .units()
        .iter()
        .find(|u| u.label().as_str() == location.value)
    else {
        errors.push(diagnostic(
            document,
            Some(&location.value),
            &location.site,
            Fault::UnknownUnit {
                label: location.value.clone(),
                available: corpus
                    .units()
                    .iter()
                    .map(|u| u.label().as_str().into())
                    .collect(),
            },
        ));
        return;
    };
    if unit.document() == document {
        errors.push(diagnostic(
            document,
            Some(&location.value),
            &location.site,
            Fault::WrongQuotationOwner {
                label: location.value.clone(),
                document: document.into(),
            },
        ));
    }
    if let Some(languages) = corpus.documents().get(document) {
        for language in languages {
            if !unit.texts().contains_key(language) {
                errors.push(diagnostic(
                    document,
                    Some(&location.value),
                    &location.site,
                    Fault::MissingQuotationLanguage {
                        label: location.value.clone(),
                        language: language.clone(),
                    },
                ));
            }
        }
    }
}
fn reference(
    corpus: &LanguageCheckedUnits,
    document: &str,
    location: &LocatedString,
    owner_sites: &BTreeMap<String, UnitSite>,
    errors: &mut Vec<Diagnostic>,
) {
    if let Some(unit) = corpus
        .units()
        .iter()
        .find(|u| u.label().as_str() == location.value)
    {
        if unit.document() != document {
            let declared_at = owner_sites
                .get(&location.value)
                .cloned()
                .unwrap_or_else(|| location.site.clone());
            errors.push(diagnostic(
                document,
                Some(&location.value),
                &location.site,
                Fault::WrongOwner {
                    label: location.value.clone(),
                    expected: unit.document().into(),
                    actual: document.into(),
                    declared_at,
                },
            ));
        }
    } else {
        errors.push(diagnostic(
            document,
            Some(&location.value),
            &location.site,
            Fault::UnknownUnit {
                label: location.value.clone(),
                available: corpus
                    .units()
                    .iter()
                    .map(|u| u.label().as_str().into())
                    .collect(),
            },
        ));
    }
}
fn multiplicity(
    corpus: &LanguageCheckedUnits,
    owners: &BTreeMap<String, Vec<UnitSite>>,
    references: &BTreeMap<String, Vec<UnitSite>>,
    unit_sites: &BTreeMap<String, UnitSite>,
    document_sites: &BTreeMap<String, UnitSite>,
    errors: &mut Vec<Diagnostic>,
) {
    for document in corpus.documents().keys() {
        let sites = owners.get(document).cloned().unwrap_or_default();
        if sites.len() != 1 {
            let site = sites.first().unwrap_or(&document_sites[document]);
            errors.push(diagnostic(
                document,
                None,
                site,
                Fault::SkeletonMultiplicity {
                    document: document.clone(),
                    expected: 1,
                    actual: sites.len(),
                    sites: sites.clone(),
                },
            ));
        }
    }
    for unit in corpus.units() {
        let label = unit.label().as_str();
        let sites = references.get(label).cloned().unwrap_or_default();
        if sites.len() != 1 {
            let mut site = sites.first().unwrap_or(&unit_sites[label]).clone();
            if sites.is_empty() {
                site.anchor = crate::SourceAnchor::Unavailable;
            }
            errors.push(diagnostic(
                unit.document(),
                Some(label),
                &site,
                Fault::UnitMultiplicity {
                    label: label.into(),
                    expected: 1,
                    actual: sites.len(),
                    sites: sites.clone(),
                },
            ));
        }
    }
}

fn document_shape(
    corpus: &LanguageCheckedUnits,
    draft: &SkeletonDraft,
    errors: &mut Vec<Diagnostic>,
) {
    let document = &draft.document.value;
    if !corpus.documents().contains_key(document) {
        errors.push(diagnostic(
            document,
            None,
            &draft.document.site,
            Fault::UnknownSkeletonDocument {
                name: document.clone(),
                available: corpus.documents().keys().cloned().collect(),
            },
        ));
    }
    if draft.sections.is_empty() {
        errors.push(diagnostic(
            document,
            None,
            &draft.sections_site,
            Fault::EmptySkeleton {
                document: document.clone(),
            },
        ));
    }
}
