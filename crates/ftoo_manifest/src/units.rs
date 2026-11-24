//! Statement metadata, language completeness and opaque shared mathematics.
//! Draft admission proves no byte provenance, theory identity, weld or rendering.
mod content;
mod diagnostics;
pub(crate) mod spans;
#[cfg(test)]
mod tests;
use crate::{AddressBook, NodeId, RuleId};
pub use diagnostics::*;
use std::collections::{BTreeMap, BTreeSet};

/// An untrusted decoded string and an owned source location supplied by its caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedString {
    pub value: String,
    pub site: UnitSite,
}
/// A duplicate-free map entry after byte parsing, or an untrusted direct draft.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedEntryDraft {
    pub key_site: UnitSite,
    pub value: LocatedString,
}
/// Untrusted document language requirements, retaining duplicate array positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentEntryDraft {
    pub key_site: UnitSite,
    pub table_site: UnitSite,
    pub languages_site: UnitSite,
    pub required_languages: Vec<LocatedString>,
}
/// Structurally typed declarations; construction does not certify source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentDraft {
    pub documents_site: UnitSite,
    pub documents: BTreeMap<String, DocumentEntryDraft>,
}
/// Structurally typed statement data; every field still requires admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitDraft {
    pub id: LocatedString,
    pub document: LocatedString,
    pub table_site: UnitSite,
    pub math_site: UnitSite,
    pub texts_site: UnitSite,
    pub documentary_targets: BTreeMap<String, LocatedEntryDraft>,
    pub math: BTreeMap<String, LocatedEntryDraft>,
    pub texts: BTreeMap<String, LocatedEntryDraft>,
}
/// An editorial label, not a theory identity.
/// ```compile_fail,E0277
/// fn convert(label: ftoo_manifest::UnitLabel) -> ftoo_manifest::NodeId { label.into() }
/// ```
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnitLabel(String);
impl UnitLabel {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A retained math reference with its exact text occurrence and raw literal site.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathOccurrence {
    name: String,
    site: UnitSite,
}
impl MathOccurrence {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.site
    }
}
/// An opaque authored handle, retained without claiming live resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagOccurrence {
    input: String,
    site: UnitSite,
}
/// A visible local placement of an auxiliary paper mint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleOccurrence {
    selector: String,
    site: UnitSite,
}
impl HandleOccurrence {
    #[must_use]
    pub fn selector(&self) -> &str {
        &self.selector
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.site
    }
}
/// One located authored see reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceOccurrence {
    label: String,
    site: UnitSite,
}
impl ReferenceOccurrence {
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.site
    }
}
/// A source-ordered bibliography citation with its exact insertion site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BibliographyCitationKind {
    Parenthetical,
    Textual,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BibliographyCitationOccurrence {
    kind: BibliographyCitationKind,
    keys: Vec<String>,
    site: UnitSite,
}
impl BibliographyCitationOccurrence {
    #[must_use]
    pub fn kind(&self) -> BibliographyCitationKind {
        self.kind
    }
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.site
    }
}
impl TagOccurrence {
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.site
    }
}

/// Nonblank admitted prose with math and tag insertions and literal at-sign escapes.
/// Tag spellings here have syntax admission only; use `resolve_unit_tags` for liveness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedText {
    source: LocatedString,
    occurrences: Vec<MathOccurrence>,
    tags: Vec<TagOccurrence>,
    handles: Vec<HandleOccurrence>,
    references: Vec<ReferenceOccurrence>,
    bibliography_citations: Vec<BibliographyCitationOccurrence>,
    styles: Vec<content::StyleOccurrence>,
}
impl CheckedText {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.source.value
    }
    #[must_use]
    pub fn site(&self) -> &UnitSite {
        &self.source.site
    }
    #[must_use]
    pub fn occurrences(&self) -> &[MathOccurrence] {
        &self.occurrences
    }
    #[must_use]
    pub fn tag_occurrences(&self) -> &[TagOccurrence] {
        &self.tags
    }
    #[must_use]
    pub fn handle_occurrences(&self) -> &[HandleOccurrence] {
        &self.handles
    }
    #[must_use]
    pub fn reference_occurrences(&self) -> &[ReferenceOccurrence] {
        &self.references
    }
    #[must_use]
    pub fn bibliography_citations(&self) -> &[BibliographyCitationOccurrence] {
        &self.bibliography_citations
    }
}
/// Unique editorial metadata, complete language slots and used shared mathematics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedUnit {
    label: UnitLabel,
    document: String,
    math: BTreeMap<String, LocatedString>,
    documentary_targets: BTreeMap<String, LocatedString>,
    texts: BTreeMap<String, CheckedText>,
}
impl CheckedUnit {
    #[must_use]
    pub fn label(&self) -> &UnitLabel {
        &self.label
    }
    #[must_use]
    pub fn document(&self) -> &str {
        &self.document
    }
    #[must_use]
    pub fn math(&self) -> &BTreeMap<String, LocatedString> {
        &self.math
    }
    #[must_use]
    pub fn documentary_targets(&self) -> &BTreeMap<String, LocatedString> {
        &self.documentary_targets
    }
    #[must_use]
    pub fn texts(&self) -> &BTreeMap<String, CheckedText> {
        &self.texts
    }
}
/// Immutable admission of structurally typed drafts through metadata and content.
/// This is neither a byte-validation certificate nor a rendered document.
/// ```compile_fail,E0451
/// use std::collections::BTreeMap;
/// let units = ftoo_manifest::LanguageCheckedUnits { documents: BTreeMap::new(), units: vec![] };
/// ```
/// ```compile_fail,E0599
/// let units = ftoo_manifest::LanguageCheckedUnits::default();
/// ```
/// ```compile_fail,E0596
/// fn mutate(units: &ftoo_manifest::LanguageCheckedUnits) { units.units().clear(); }
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::LanguageCheckedUnits>();
/// ```
/// ```compile_fail,E0616
/// fn mutate(units: &mut ftoo_manifest::LanguageCheckedUnits) { units.units.clear(); }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCheckedUnits {
    documents: BTreeMap<String, Vec<String>>,
    units: Vec<CheckedUnit>,
}
impl LanguageCheckedUnits {
    #[must_use]
    pub fn documents(&self) -> &BTreeMap<String, Vec<String>> {
        &self.documents
    }
    #[must_use]
    pub fn units(&self) -> &Vec<CheckedUnit> {
        &self.units
    }
}

fn local_name(value: &str) -> bool {
    let mut parts = value.split('-');
    parts.next().is_some_and(|p| {
        p.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    }) && parts.all(|p| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    })
}
pub(crate) fn label_name(value: &str) -> bool {
    value.split(':').count() == 3
        && value.split(':').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn language_name(value: &str) -> bool {
    let mut parts = value.split('-');
    parts
        .next()
        .is_some_and(|p| (2..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_lowercase()))
        && parts.all(|p| {
            (2..=8).contains(&p.len())
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
fn diagnostic(
    rule: RuleId,
    stage: UnitStage,
    unit: Option<&str>,
    language: Option<&str>,
    site: &UnitSite,
    fault: UnitFault,
) -> UnitDiagnostic {
    UnitDiagnostic {
        rule,
        stage,
        unit: unit.map(str::to_owned),
        language: language.map(str::to_owned),
        site: site.clone(),
        fault,
    }
}
fn metadata(documents: &DocumentDraft, units: &[UnitDraft]) -> Vec<UnitDiagnostic> {
    let mut errors = Vec::new();
    if !documents.documents.contains_key("main") {
        errors.push(diagnostic(
            RuleId::M040,
            UnitStage::Metadata,
            None,
            None,
            &documents.documents_site.parent(),
            UnitFault::MissingMain,
        ));
    }
    for (name, document) in &documents.documents {
        document_metadata(name, document, &mut errors);
    }
    unit_metadata(documents, units, &mut errors);
    errors
}
fn document_metadata(name: &str, document: &DocumentEntryDraft, errors: &mut Vec<UnitDiagnostic>) {
    if !local_name(name) {
        errors.push(diagnostic(
            RuleId::M039,
            UnitStage::Metadata,
            None,
            None,
            &document.key_site,
            UnitFault::InvalidName {
                domain: NameDomain::DocumentName,
                value: name.to_owned(),
            },
        ));
    }
    let mut languages: BTreeMap<&str, Vec<UnitSite>> = BTreeMap::new();
    let mut valid = true;
    for language in &document.required_languages {
        if !language_name(&language.value) {
            valid = false;
            errors.push(diagnostic(
                RuleId::M040,
                UnitStage::Metadata,
                None,
                Some(&language.value),
                &language.site,
                UnitFault::InvalidLanguage {
                    value: language.value.clone(),
                },
            ));
        }
        languages
            .entry(&language.value)
            .or_default()
            .push(language.site.clone());
    }
    for (language, sites) in &languages {
        if sites.len() > 1 {
            valid = false;
            for site in sites {
                errors.push(diagnostic(
                    RuleId::M040,
                    UnitStage::Metadata,
                    None,
                    Some(language),
                    site,
                    UnitFault::DuplicateRequiredLanguage {
                        document: name.to_owned(),
                        language: (*language).to_owned(),
                        sites: sites.clone(),
                    },
                ));
            }
        }
    }
    if languages.is_empty() {
        valid = false;
        errors.push(diagnostic(
            RuleId::M040,
            UnitStage::Metadata,
            None,
            None,
            &document.languages_site,
            UnitFault::EmptyRequiredLanguages {
                document: name.to_owned(),
            },
        ));
    }
    if valid && name == "main" && languages.keys().copied().collect::<Vec<_>>() != ["en"] {
        errors.push(diagnostic(
            RuleId::M040,
            UnitStage::Metadata,
            None,
            None,
            &document.languages_site,
            UnitFault::MainLanguages {
                actual: document
                    .required_languages
                    .iter()
                    .map(|l| l.value.clone())
                    .collect(),
            },
        ));
    }
}
fn unit_metadata(documents: &DocumentDraft, units: &[UnitDraft], errors: &mut Vec<UnitDiagnostic>) {
    let mut ids: BTreeMap<&str, Vec<UnitSite>> = BTreeMap::new();
    for unit in units {
        ids.entry(&unit.id.value)
            .or_default()
            .push(unit.id.site.clone());
        for (value, domain, valid) in [
            (&unit.id, NameDomain::UnitLabel, label_name(&unit.id.value)),
            (
                &unit.document,
                NameDomain::DocumentName,
                local_name(&unit.document.value),
            ),
        ] {
            if !valid {
                errors.push(diagnostic(
                    RuleId::M039,
                    UnitStage::Metadata,
                    Some(&unit.id.value),
                    None,
                    &value.site,
                    UnitFault::InvalidName {
                        domain,
                        value: value.value.clone(),
                    },
                ));
            }
        }
        if local_name(&unit.document.value)
            && !documents.documents.contains_key(&unit.document.value)
        {
            errors.push(diagnostic(
                RuleId::M039,
                UnitStage::Metadata,
                Some(&unit.id.value),
                None,
                &unit.document.site,
                UnitFault::UnknownDocument {
                    name: unit.document.value.clone(),
                    available: documents.documents.keys().cloned().collect(),
                },
            ));
        }
        for (language, entry) in &unit.texts {
            if !language_name(language) {
                errors.push(diagnostic(
                    RuleId::M040,
                    UnitStage::Metadata,
                    Some(&unit.id.value),
                    Some(language),
                    &entry.key_site,
                    UnitFault::InvalidLanguage {
                        value: language.clone(),
                    },
                ));
            }
        }
    }
    for (id, mut sites) in ids {
        if sites.len() > 1 {
            sites.sort_by_key(|site| {
                (
                    site.file.clone(),
                    match site.anchor {
                        SourceAnchor::Bytes(a, _) => a,
                        _ => 0,
                    },
                )
            });
            for site in &sites {
                errors.push(diagnostic(
                    RuleId::M039,
                    UnitStage::Metadata,
                    Some(id),
                    None,
                    site,
                    UnitFault::DuplicateUnit {
                        id: id.to_owned(),
                        sites: sites.clone(),
                    },
                ));
            }
        }
    }
}
/// Validate metadata, language contracts and insertion content before tag resolution.
/// # Errors
/// Returns the complete ordered failures in the earliest failing semantic phase.
pub fn admit_unit_drafts(
    documents: DocumentDraft,
    mut units: Vec<UnitDraft>,
) -> Result<LanguageCheckedUnits, UnitErrors> {
    if let Some(errors) = UnitErrors::semantic(metadata(&documents, &units)) {
        return Err(errors);
    }
    units.sort_by(|a, b| a.id.value.cmp(&b.id.value));
    let mut errors = Vec::new();
    let mut checked = Vec::new();
    for unit in units {
        let required = &documents.documents[&unit.document.value].required_languages;
        checked.push(check_unit(unit, required, &mut errors));
    }
    if let Some(errors) = UnitErrors::semantic(errors) {
        return Err(errors);
    }
    Ok(LanguageCheckedUnits {
        documents: documents
            .documents
            .into_iter()
            .map(|(k, v)| {
                let mut languages: Vec<_> =
                    v.required_languages.into_iter().map(|l| l.value).collect();
                languages.sort();
                (k, languages)
            })
            .collect(),
        units: checked,
    })
}

#[allow(clippy::too_many_lines)] // Admission reports metadata and text faults in their authored order.
fn check_unit(
    unit: UnitDraft,
    required: &[LocatedString],
    errors: &mut Vec<UnitDiagnostic>,
) -> CheckedUnit {
    let required_names: BTreeSet<_> = required.iter().map(|l| l.value.clone()).collect();
    for language in required {
        if !unit.texts.contains_key(&language.value) {
            errors.push(diagnostic(
                RuleId::M040,
                UnitStage::Language,
                Some(&unit.id.value),
                Some(&language.value),
                &unit.texts_site.parent().child(&language.value),
                UnitFault::MissingLanguage {
                    document: unit.document.value.clone(),
                    required_at: language.site.clone(),
                },
            ));
        }
    }
    let (texts, used, scanner_failed) = check_texts(&unit, &required_names, errors);
    let mut placed = BTreeMap::<String, usize>::new();
    for (selector, entry) in &unit.documentary_targets {
        if !local_name(selector)
            || entry.value.value.is_empty()
            || label_name(&entry.value.value)
            || !entry.value.value.contains(':')
            || !entry
                .value
                .value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
        {
            errors.push(diagnostic(
                RuleId::M039,
                UnitStage::Metadata,
                Some(&unit.id.value),
                None,
                &entry.key_site,
                UnitFault::InvalidDocumentaryTarget {
                    selector: selector.clone(),
                    key: entry.value.value.clone(),
                },
            ));
        }
    }
    for (language, text) in &texts {
        for handle in text.handle_occurrences() {
            let selector = handle.selector();
            if !unit.documentary_targets.contains_key(selector) {
                errors.push(diagnostic(
                    RuleId::M041,
                    UnitStage::Content,
                    Some(&unit.id.value),
                    Some(language),
                    handle.site(),
                    UnitFault::UnknownHandle {
                        selector: selector.to_owned(),
                    },
                ));
            }
            if language != "en" {
                errors.push(diagnostic(
                    RuleId::M041,
                    UnitStage::Content,
                    Some(&unit.id.value),
                    Some(language),
                    handle.site(),
                    UnitFault::HandleOutsidePaperEnglish {
                        selector: selector.to_owned(),
                    },
                ));
            }
            let count = placed.entry(selector.to_owned()).or_default();
            *count += 1;
            if *count > 1 {
                errors.push(diagnostic(
                    RuleId::M041,
                    UnitStage::Content,
                    Some(&unit.id.value),
                    Some(language),
                    handle.site(),
                    UnitFault::RepeatedHandle {
                        selector: selector.to_owned(),
                    },
                ));
            }
        }
    }
    for (selector, entry) in &unit.documentary_targets {
        if !placed.contains_key(selector) {
            errors.push(diagnostic(
                RuleId::M041,
                UnitStage::Content,
                Some(&unit.id.value),
                Some("en"),
                &entry.key_site,
                UnitFault::UnusedHandle {
                    selector: selector.clone(),
                },
            ));
        }
    }
    for (name, entry) in &unit.math {
        if !local_name(name) {
            errors.push(diagnostic(
                RuleId::M041,
                UnitStage::Content,
                Some(&unit.id.value),
                None,
                &entry.key_site,
                UnitFault::InvalidMathName {
                    value: name.clone(),
                },
            ));
        }
        if let Some((site, fault)) = content::math(name, &entry.value) {
            errors.push(diagnostic(
                RuleId::M041,
                UnitStage::Content,
                Some(&unit.id.value),
                None,
                &site,
                fault,
            ));
        }
        if !scanner_failed && !used.contains(name) {
            errors.push(diagnostic(
                RuleId::M041,
                UnitStage::Content,
                Some(&unit.id.value),
                None,
                &entry.key_site,
                UnitFault::UnusedMath { name: name.clone() },
            ));
        }
    }
    CheckedUnit {
        label: UnitLabel(unit.id.value),
        document: unit.document.value,
        math: unit.math.into_iter().map(|(k, v)| (k, v.value)).collect(),
        documentary_targets: unit
            .documentary_targets
            .into_iter()
            .map(|(k, v)| (k, v.value))
            .collect(),
        texts,
    }
}
type TextChecks = (BTreeMap<String, CheckedText>, BTreeSet<String>, bool);
fn check_texts(
    unit: &UnitDraft,
    required_names: &BTreeSet<String>,
    errors: &mut Vec<UnitDiagnostic>,
) -> TextChecks {
    let mut texts = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut scanner_failed = false;
    for (language, entry) in &unit.texts {
        if !required_names.contains(language) {
            errors.push(diagnostic(
                RuleId::M040,
                UnitStage::Language,
                Some(&unit.id.value),
                Some(language),
                &entry.key_site,
                UnitFault::UndeclaredLanguage {
                    document: unit.document.value.clone(),
                    required: required_names.iter().cloned().collect(),
                },
            ));
        }
        match content::scan(&entry.value) {
            Ok(content::ScannedText {
                prose,
                math: occurrences,
                tags,
                handles,
                references,
                bibliography_citations,
                styles,
            }) => {
                if !prose {
                    errors.push(diagnostic(
                        RuleId::M040,
                        UnitStage::Language,
                        Some(&unit.id.value),
                        Some(language),
                        &entry.value.site,
                        UnitFault::BlankText,
                    ));
                }
                for occurrence in &occurrences {
                    if unit.math.contains_key(&occurrence.name) {
                        used.insert(occurrence.name.clone());
                    } else {
                        errors.push(diagnostic(
                            RuleId::M041,
                            UnitStage::Content,
                            Some(&unit.id.value),
                            Some(language),
                            &occurrence.site,
                            UnitFault::UnknownMath {
                                name: occurrence.name.clone(),
                                available: unit.math.keys().cloned().collect(),
                            },
                        ));
                    }
                }
                texts.insert(
                    language.clone(),
                    CheckedText {
                        source: entry.value.clone(),
                        occurrences,
                        tags,
                        handles,
                        references,
                        bibliography_citations,
                        styles,
                    },
                );
            }
            Err(failure) => {
                let (site, fault) = *failure;
                scanner_failed = true;
                errors.push(diagnostic(
                    if matches!(fault, UnitFault::TextMintForbidden { .. }) {
                        RuleId::M045
                    } else {
                        RuleId::M041
                    },
                    UnitStage::Content,
                    Some(&unit.id.value),
                    Some(language),
                    &site,
                    fault,
                ));
            }
        }
    }
    (texts, used, scanner_failed)
}

/// A live identity at one authored occurrence, accessible through its resolved corpus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaperEvidenceSite {
    pub file: String,
    pub line: usize,
}
/// A checked paper binding remains outside the permanent theory identity domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaperTagValue {
    pub key: String,
    pub spelling: String,
    pub target: String,
    pub target_kind: String,
    pub lookup_site: PaperEvidenceSite,
    pub alias_site: Option<PaperEvidenceSite>,
    pub declaration_sites: Vec<PaperEvidenceSite>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaperResolution {
    Available(PaperTagValue),
    Unavailable { status: String },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedTagTarget {
    Theory(NodeId),
    Paper(PaperTagValue),
}
#[derive(Debug)]
pub struct ResolvedUnitTag<'u> {
    unit: &'u UnitLabel,
    language: &'u str,
    occurrence: &'u TagOccurrence,
    target: ResolvedTagTarget,
}
impl ResolvedUnitTag<'_> {
    #[must_use]
    pub fn unit(&self) -> &UnitLabel {
        self.unit
    }
    #[must_use]
    pub fn language(&self) -> &str {
        self.language
    }
    #[must_use]
    pub fn occurrence(&self) -> &TagOccurrence {
        self.occurrence
    }
    #[must_use]
    /// # Panics
    /// Panics for a paper-scoped binding, which has no theory node.
    pub fn target(&self) -> NodeId {
        match &self.target {
            ResolvedTagTarget::Theory(target) => *target,
            ResolvedTagTarget::Paper(_) => panic!("paper bindings have no theory node"),
        }
    }
    #[must_use]
    pub fn resolved_target(&self) -> &ResolvedTagTarget {
        &self.target
    }
}

/// Every tag occurrence is live in this exact borrowed book and corpus.
/// This establishes no weld, translation equivalence or rendering readiness.
/// ```compile_fail,E0451
/// fn forge<'u,'b,'g>(corpus: &'u ftoo_manifest::LanguageCheckedUnits, book: &'b ftoo_manifest::AddressBook<'g>) {
///     let _ = ftoo_manifest::ResolvedUnits { corpus, book, tags: vec![] };
/// }
/// ```
/// ```compile_fail,E0616
/// fn mutate(value: &mut ftoo_manifest::ResolvedUnits<'_, '_, '_>) { value.tags.clear(); }
/// ```
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::ResolvedUnits::default();
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::ResolvedUnits<'static, 'static, 'static>>();
/// ```
/// ```compile_fail,E0515
/// fn escape<'b,'g>(corpus: &ftoo_manifest::LanguageCheckedUnits, book: &'b ftoo_manifest::AddressBook<'g>)
///     -> ftoo_manifest::ResolvedUnits<'static,'b,'g> {
///     let local = corpus.clone();
///     ftoo_manifest::resolve_unit_tags(&local, book).unwrap()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape<'u,'g>(corpus: &'u ftoo_manifest::LanguageCheckedUnits, graph: &'g ftoo_manifest::ValidatedManifest)
///     -> ftoo_manifest::ResolvedUnits<'u,'static,'g> {
///     let book = ftoo_manifest::addressing(graph).unwrap();
///     ftoo_manifest::resolve_unit_tags(corpus, &book).unwrap()
/// }
/// ```
#[derive(Debug)]
pub struct ResolvedUnits<'u, 'b, 'g> {
    corpus: &'u LanguageCheckedUnits,
    book: &'b AddressBook<'g>,
    tags: Vec<ResolvedUnitTag<'u>>,
}
impl<'u, 'b, 'g> ResolvedUnits<'u, 'b, 'g> {
    #[must_use]
    pub fn corpus(&self) -> &'u LanguageCheckedUnits {
        self.corpus
    }
    #[must_use]
    pub fn book(&self) -> &'b AddressBook<'g> {
        self.book
    }
    #[must_use]
    pub fn tags(&self) -> &[ResolvedUnitTag<'u>] {
        &self.tags
    }
}
/// Resolve every occurrence through the supplied book's existing live resolver.
/// # Errors
/// Returns all located resolution refusals in semantic order, with no partial result.
pub fn resolve_unit_tags<'u, 'b, 'g>(
    corpus: &'u LanguageCheckedUnits,
    book: &'b AddressBook<'g>,
) -> Result<ResolvedUnits<'u, 'b, 'g>, UnitErrors> {
    resolve_unit_tags_with(corpus, book, &BTreeSet::new(), |_| None)
}
/// Resolve permanent tags first, then unknown spellings in explicitly selected paper documents.
/// # Errors
/// Returns located refusals for every unavailable or mismatched occurrence.
pub fn resolve_unit_tags_with<'u, 'b, 'g>(
    corpus: &'u LanguageCheckedUnits,
    book: &'b AddressBook<'g>,
    paper_documents: &BTreeSet<String>,
    paper: impl Fn(&str) -> Option<PaperResolution>,
) -> Result<ResolvedUnits<'u, 'b, 'g>, UnitErrors> {
    let mut errors = Vec::new();
    let mut tags = Vec::new();
    for unit in &corpus.units {
        for (language, text) in &unit.texts {
            for occurrence in &text.tags {
                match book.resolve(&occurrence.input) {
                    Ok(crate::AddressTarget::Theory(target)) => tags.push(ResolvedUnitTag {
                        unit: &unit.label,
                        language,
                        occurrence,
                        target: ResolvedTagTarget::Theory(target),
                    }),
                    Ok(crate::AddressTarget::Documentary(label)) => errors.push(diagnostic(
                        RuleId::M042,
                        UnitStage::Resolution,
                        Some(unit.label.as_str()),
                        Some(language),
                        &occurrence.site,
                        UnitFault::DocumentaryTagTarget {
                            input: occurrence.input.clone(),
                            label,
                        },
                    )),
                    Err(crate::TagResolutionFault::UnknownTag { .. })
                        if paper_documents.contains(&unit.document) =>
                    {
                        match paper(&occurrence.input) {
                            Some(PaperResolution::Available(target)) => {
                                tags.push(ResolvedUnitTag {
                                    unit: &unit.label,
                                    language,
                                    occurrence,
                                    target: ResolvedTagTarget::Paper(target),
                                });
                            }
                            Some(PaperResolution::Unavailable { status }) => {
                                errors.push(diagnostic(
                                    RuleId::M042,
                                    UnitStage::Resolution,
                                    Some(unit.label.as_str()),
                                    Some(language),
                                    &occurrence.site,
                                    UnitFault::PaperTagUnavailable {
                                        input: occurrence.input.clone(),
                                        status,
                                    },
                                ));
                            }
                            None => errors.push(diagnostic(
                                RuleId::M042,
                                UnitStage::Resolution,
                                Some(unit.label.as_str()),
                                Some(language),
                                &occurrence.site,
                                UnitFault::ResolutionFault {
                                    cause: crate::TagResolutionFault::UnknownTag {
                                        input: occurrence.input.clone(),
                                    },
                                },
                            )),
                        }
                    }
                    Err(cause) => errors.push(diagnostic(
                        RuleId::M042,
                        UnitStage::Resolution,
                        Some(unit.label.as_str()),
                        Some(language),
                        &occurrence.site,
                        UnitFault::ResolutionFault { cause },
                    )),
                }
            }
        }
    }
    if let Some(errors) = UnitErrors::semantic(errors) {
        return Err(errors);
    }
    Ok(ResolvedUnits { corpus, book, tags })
}

pub(crate) use content::AssemblyScanFault;
// Assembly reuses lexical admission without constructing a statement CheckedText.
pub(crate) fn assembly_inline(
    source: &LocatedString,
    maximum: usize,
) -> Result<(Vec<MathOccurrence>, Vec<TagOccurrence>), AssemblyScanFault> {
    let scanned = content::scan_limited(source, Some(maximum), false)?;
    Ok((scanned.math, scanned.tags))
}
