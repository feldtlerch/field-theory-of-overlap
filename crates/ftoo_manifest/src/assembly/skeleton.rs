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
    Section(SkeletonSection),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkeletonPosition {
    unit: String,
    depth: Option<usize>,
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
pub fn check_skeletons<'a>(
    corpus: &'a LanguageCheckedUnits,
    drafts: &[SkeletonDraft],
    unit_sites: &BTreeMap<String, UnitSite>,
    owner_sites: &BTreeMap<String, UnitSite>,
    document_sites: &BTreeMap<String, UnitSite>,
) -> Result<CheckedSkeletons<'a>, Vec<Diagnostic>> {
    let mut documents = BTreeMap::new();
    let mut errors = Vec::new();
    let mut owners: BTreeMap<String, Vec<UnitSite>> = BTreeMap::new();
    let mut references: BTreeMap<String, Vec<UnitSite>> = BTreeMap::new();
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
            .map(|s| (Some(s), &s.heading, Some(1)))
            .collect();
        while let Some((section, location, depth)) = stack.pop() {
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
            references
                .entry(location.value.clone())
                .or_default()
                .push(location.site.clone());
            reference(corpus, document, location, owner_sites, &mut errors);
            blocks.push(SkeletonPosition {
                unit: location.value.clone(),
                depth,
                site: location.site.clone(),
            });
            if let Some(section) = section {
                for child in section.children.iter().rev() {
                    match child {
                        SkeletonChild::Unit(unit) => stack.push((None, unit, None)),
                        SkeletonChild::Section(section) => stack.push((
                            Some(section),
                            &section.heading,
                            Some(depth.unwrap_or(0) + 1),
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
    if errors.is_empty() {
        Ok(CheckedSkeletons { corpus, documents })
    } else {
        super::diagnostics::sort(&mut errors);
        Err(errors)
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
