//! Independent Markdown admission and permanent cross-surface mint ownership.
mod diagnostics;
#[cfg(test)]
mod tests;
use crate::{HistoryChange, NodeId, PathPart, ResolvedUnits, RuleId, SourceAnchor, UnitSite};
pub use diagnostics::*;
use std::collections::BTreeMap;

/// Untrusted bytes with a relative source name; not proof of complete discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownSource {
    pub file: String,
    pub bytes: Vec<u8>,
}
/// One lexical occurrence, retaining duplicates and original byte positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelOccurrence {
    pub label: String,
    pub site: LabelSite,
}
/// A source snapshot admitted through independent local mint and citation checks.
/// ```compile_fail,E0451
/// let _ = ftoo_manifest::CheckedMarkdown { sources: vec![], mints: vec![], citations: vec![] };
/// ```
/// ```compile_fail,E0616
/// fn mutate(v: &mut ftoo_manifest::CheckedMarkdown) { v.mints.clear(); }
/// ```
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::CheckedMarkdown::default();
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::CheckedMarkdown>();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedMarkdown {
    sources: Vec<MarkdownSource>,
    mints: Vec<LabelOccurrence>,
    citations: Vec<LabelOccurrence>,
}
impl CheckedMarkdown {
    #[must_use]
    pub fn sources(&self) -> &[MarkdownSource] {
        &self.sources
    }
    #[must_use]
    pub fn mints(&self) -> &[LabelOccurrence] {
        &self.mints
    }
    #[must_use]
    pub fn citations(&self) -> &[LabelOccurrence] {
        &self.citations
    }
}
fn diagnostic(
    rule: RuleId,
    stage: LabelStage,
    site: LabelSite,
    fault: LabelFault,
) -> LabelDiagnostic {
    LabelDiagnostic {
        rule,
        stage,
        unit: None,
        language: None,
        site,
        fault,
    }
}
fn markdown_site(file: &str, bytes: std::ops::Range<usize>) -> LabelSite {
    LabelSite::Markdown {
        file: file.to_owned(),
        bytes,
    }
}
fn label_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start;
    for component in 0..3 {
        let begin = at;
        while bytes
            .get(at)
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        {
            at += 1;
        }
        if at == begin {
            return None;
        }
        if component < 2 {
            if bytes.get(at) != Some(&b':') {
                return None;
            }
            at += 1;
        }
    }
    (bytes.get(at) == Some(&b'`')).then_some(at + 1)
}
fn spans(text: &str, file: &str, offset: usize, prefix: &str) -> Vec<LabelOccurrence> {
    let mut occurrences = Vec::new();
    let mut at = 0;
    while let Some(relative) = text[at..].find(prefix) {
        let start = at + relative;
        let body = start + prefix.len();
        if let Some(end) = label_end(text.as_bytes(), body) {
            occurrences.push(LabelOccurrence {
                label: text[body..end - 1].to_owned(),
                site: markdown_site(file, offset + start..offset + end),
            });
            at = end;
        } else {
            at = body;
        }
    }
    occurrences
}
fn harvest(text: &str, file: &str) -> (Vec<LabelOccurrence>, Vec<LabelOccurrence>) {
    let mints = spans(text, file, 0, "· `");
    let mut citations = Vec::new();
    let mut open = None;
    for (at, byte) in text.bytes().enumerate() {
        match byte {
            b'(' => open = Some(at + 1),
            b')' => {
                if let Some(start) = open.take() {
                    citations.extend(spans(&text[start..at], file, start, "`"));
                }
            }
            _ => {}
        }
    }
    (mints, citations)
}
type Mints = BTreeMap<String, Vec<MintOrigin>>;
fn markdown_mints(occurrences: &[LabelOccurrence]) -> Mints {
    let mut mints: Mints = BTreeMap::new();
    for occurrence in occurrences {
        mints
            .entry(occurrence.label.clone())
            .or_default()
            .push(MintOrigin {
                kind: MintKind::Markdown,
                site: occurrence.site.clone(),
            });
    }
    mints
}
fn duplicates(mints: &Mints, stage: LabelStage) -> Vec<LabelDiagnostic> {
    mints
        .iter()
        .filter(|(_, origins)| origins.len() > 1)
        .map(|(label, origins)| {
            diagnostic(
                RuleId::M043,
                stage,
                origins[0].site.clone(),
                LabelFault::DuplicateMint {
                    label: label.clone(),
                    expected: 1,
                    actual: origins.len(),
                    origins: origins.clone(),
                },
            )
        })
        .collect()
}
fn source_names(sources: &[MarkdownSource]) -> Result<(), LabelSourceError> {
    let mut names: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (position, source) in sources.iter().enumerate() {
        if source.file.is_empty()
            || source.file.contains('\\')
            || source
                .file
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(LabelSourceError::Input(
                LabelSourceFault::InvalidSourceName {
                    value: source.file.clone(),
                },
            ));
        }
        names.entry(&source.file).or_default().push(position);
    }
    for (file, positions) in names {
        if positions.len() > 1 {
            return Err(LabelSourceError::Input(LabelSourceFault::DuplicateSource {
                file: file.to_owned(),
                positions,
            }));
        }
    }
    Ok(())
}
/// Admit the complete supplied Markdown inventory with independent lexical checks.
/// # Errors
/// Returns source-name, encoding, duplicate-mint or Markdown-local citation failures.
pub fn check_markdown_sources(
    mut sources: Vec<MarkdownSource>,
) -> Result<CheckedMarkdown, LabelSourceError> {
    source_names(&sources)?;
    sources.sort_by(|a, b| a.file.cmp(&b.file));
    let mut errors = Vec::new();
    let mut mints = Vec::new();
    let mut citations = Vec::new();
    for source in &sources {
        match std::str::from_utf8(&source.bytes) {
            Err(error) => {
                let offset = error.valid_up_to();
                errors.push(diagnostic(
                    RuleId::M001,
                    LabelStage::Encoding,
                    markdown_site(
                        &source.file,
                        offset
                            ..error
                                .error_len()
                                .map_or(source.bytes.len(), |len| offset + len),
                    ),
                    LabelFault::InvalidUtf8 {
                        offset,
                        error_len: error.error_len(),
                    },
                ));
            }
            Ok(text) if text.starts_with('\u{feff}') => errors.push(diagnostic(
                RuleId::M001,
                LabelStage::Encoding,
                markdown_site(&source.file, 0..3),
                LabelFault::Bom,
            )),
            Ok(text) => {
                let (mint, cite) = harvest(text, &source.file);
                mints.extend(mint);
                citations.extend(cite);
            }
        }
    }
    if !errors.is_empty() {
        return Err(LabelSourceError::Admission(LabelErrors(errors)));
    }
    let index = markdown_mints(&mints);
    errors = duplicates(&index, LabelStage::Markdown);
    if !errors.is_empty() {
        return Err(LabelSourceError::Admission(LabelErrors(errors)));
    }
    for cite in &citations {
        if !index.contains_key(&cite.label) {
            errors.push(diagnostic(
                RuleId::M043,
                LabelStage::Markdown,
                cite.site.clone(),
                LabelFault::UnresolvedMarkdownCitation {
                    label: cite.label.clone(),
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::MarkdownOnly,
                },
            ));
        }
    }
    if !errors.is_empty() {
        return Err(LabelSourceError::Admission(LabelErrors(errors)));
    }
    Ok(CheckedMarkdown {
        sources,
        mints,
        citations,
    })
}

/// Unique permanent mints and resolved documentary citations, bound to exact inputs.
/// No language-set equality or authored label-span admission is claimed.
/// ```compile_fail,E0451
/// fn forge<'r,'m,'u,'b,'g>(resolved: &'r ftoo_manifest::ResolvedUnits<'u,'b,'g>, markdown: &'m ftoo_manifest::CheckedMarkdown) {
///     let _ = ftoo_manifest::LabelIntegratedUnits { resolved, markdown, mints: std::collections::BTreeMap::new(), rust_citations: vec![] };
/// }
/// ```
/// ```compile_fail,E0596
/// fn mutate(v: &ftoo_manifest::LabelIntegratedUnits<'_, '_, '_, '_, '_>) { v.mints().clear(); }
/// ```
/// ```compile_fail,E0515
/// fn escape<'r,'u,'b,'g>(resolved: &'r ftoo_manifest::ResolvedUnits<'u,'b,'g>) -> ftoo_manifest::LabelIntegratedUnits<'r,'static,'u,'b,'g> {
///     let markdown = ftoo_manifest::check_markdown_sources(vec![]).unwrap();
///     ftoo_manifest::integrate_labels(resolved, &markdown).unwrap()
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape<'m,'u,'b,'g>(corpus: &'u ftoo_manifest::LanguageCheckedUnits, book: &'b ftoo_manifest::AddressBook<'g>, markdown: &'m ftoo_manifest::CheckedMarkdown) -> ftoo_manifest::LabelIntegratedUnits<'static,'m,'u,'b,'g> {
///     let resolved = ftoo_manifest::resolve_unit_tags(corpus,book).unwrap();
///     ftoo_manifest::integrate_labels(&resolved, markdown).unwrap()
/// }
/// ```
/// ```compile_fail,E0616
/// fn mutate(v: &mut ftoo_manifest::LabelIntegratedUnits<'_, '_, '_, '_, '_>) { v.mints.clear(); }
/// ```
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::LabelIntegratedUnits::default();
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::LabelIntegratedUnits<'static,'static,'static,'static,'static>>();
/// ```
#[derive(Debug)]
pub struct LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g> {
    resolved: &'r ResolvedUnits<'u, 'b, 'g>,
    markdown: &'m CheckedMarkdown,
    mints: BTreeMap<String, MintOrigin>,
    rust_citations: Vec<LabelOccurrence>,
}
impl<'r, 'm, 'u, 'b, 'g> LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g> {
    #[must_use]
    pub fn resolved(&self) -> &'r ResolvedUnits<'u, 'b, 'g> {
        self.resolved
    }
    #[must_use]
    pub fn markdown(&self) -> &'m CheckedMarkdown {
        self.markdown
    }
    #[must_use]
    pub fn mints(&self) -> &BTreeMap<String, MintOrigin> {
        &self.mints
    }
    #[must_use]
    pub fn rust_citations(&self) -> &[LabelOccurrence] {
        &self.rust_citations
    }
}
fn joint_mints(
    resolved: &ResolvedUnits<'_, '_, '_>,
    markdown: &CheckedMarkdown,
    identities: &[NodeId],
) -> Mints {
    let mut mints: Mints = BTreeMap::new();
    let mut identities = identities.to_vec();
    identities.sort();
    for identity in identities {
        mints
            .entry(identity.semantic_name().to_owned())
            .or_default()
            .push(MintOrigin {
                kind: MintKind::RustIdentity,
                site: LabelSite::RustIdentity {
                    file: "crates/ftoo_manifest/src/identity.rs".into(),
                    identity,
                    field: "semantic_name".into(),
                },
            });
    }
    for occurrence in markdown.mints() {
        mints
            .entry(occurrence.label.clone())
            .or_default()
            .push(MintOrigin {
                kind: MintKind::Markdown,
                site: occurrence.site.clone(),
            });
    }
    for unit in resolved.corpus().units() {
        if let Some(text) = unit.texts().values().next() {
            mints
                .entry(unit.label().as_str().to_owned())
                .or_default()
                .push(MintOrigin {
                    kind: MintKind::UnitMetadata,
                    site: LabelSite::Unit {
                        unit: unit.label().as_str().to_owned(),
                        site: UnitSite {
                            file: text.site().file.clone(),
                            path: vec![PathPart::Key("unit".into()), PathPart::Key("id".into())],
                            anchor: SourceAnchor::Unavailable,
                            decoded: None,
                            occurrence: None,
                        },
                    },
                });
        }
    }
    mints
}
/// Join all permanent Rust identities, Markdown declarations and unit metadata.
/// # Errors
/// Refuses joint duplicates before resolving every history authority record in Markdown.
pub fn integrate_labels<'r, 'm, 'u, 'b, 'g>(
    resolved: &'r ResolvedUnits<'u, 'b, 'g>,
    markdown: &'m CheckedMarkdown,
) -> Result<LabelIntegratedUnits<'r, 'm, 'u, 'b, 'g>, LabelErrors> {
    let mints = joint_mints(resolved, markdown, &NodeId::ALL);
    let mut errors = duplicates(&mints, LabelStage::Namespace);
    if !errors.is_empty() {
        return Err(LabelErrors(errors));
    }
    let mut rust_citations = Vec::new();
    let mut history: Vec<_> = resolved.book().history().collect();
    history.sort_by_key(|event| event.ordinal());
    for event in history {
        match event.change() {
            HistoryChange::AdmitIdentity { .. }
            | HistoryChange::AdmitTag { .. }
            | HistoryChange::RetireIdentity { .. }
            | HistoryChange::RetireTag { .. } => {
                rust_citations.push(LabelOccurrence {
                    label: event.authority_record().to_owned(),
                    site: LabelSite::RustHistory {
                        file: "crates/ftoo_manifest/src/tag_history.rs".into(),
                        ordinal: event.ordinal(),
                        field: "authority_record".into(),
                    },
                });
            }
        }
    }
    for cite in &rust_citations {
        if !markdown.mints().iter().any(|mint| mint.label == cite.label) {
            errors.push(diagnostic(
                RuleId::M044,
                LabelStage::RustCitation,
                cite.site.clone(),
                LabelFault::UnresolvedRustCitation {
                    label: cite.label.clone(),
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::MarkdownOnly,
                },
            ));
        }
    }
    if !errors.is_empty() {
        return Err(LabelErrors(errors));
    }
    Ok(LabelIntegratedUnits {
        resolved,
        markdown,
        mints: mints
            .into_iter()
            .filter_map(|(label, origins)| origins.into_iter().next().map(|origin| (label, origin)))
            .collect(),
        rust_citations,
    })
}
