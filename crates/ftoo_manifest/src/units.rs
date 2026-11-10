//! Statement metadata, language completeness and opaque shared mathematics.
//! Draft admission proves no byte provenance, theory identity, weld or rendering.
mod content;
mod diagnostics;
#[cfg(test)]
mod tests;
use crate::RuleId;
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
/// Nonblank admitted prose with only math insertions and literal at-sign escapes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedText {
    source: LocatedString,
    occurrences: Vec<MathOccurrence>,
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
}
/// Unique editorial metadata, complete language slots and used shared mathematics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedUnit {
    label: UnitLabel,
    document: String,
    math: BTreeMap<String, LocatedString>,
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
fn label_name(value: &str) -> bool {
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
/// Validate metadata, language contracts and math-only insertion content.
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
            Ok((prose, occurrences)) => {
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
                    },
                );
            }
            Err(failure) => {
                let (site, fault) = *failure;
                scanner_failed = true;
                errors.push(diagnostic(
                    RuleId::M041,
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
