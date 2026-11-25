//! Complete statement-document preparation against one retained source snapshot.
#![allow(clippy::result_large_err)]
pub(crate) mod artifacts;
mod bibliography;
mod destinations;
mod diagnostics;
mod display;
mod html;
mod html_recovery;
mod inherited_support;
pub(crate) mod input;
mod latex;
mod lock_scan;
mod markdown;
mod native;
mod native_format;
mod native_support;
mod pending;
mod plan;
pub(crate) mod registry;
mod retained;
mod runtime;
#[cfg(test)]
mod tests;
pub use artifacts::check_documents;
pub use diagnostics::*;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, DocumentOutput, DocumentTarget, RuleId, SourceAnchor,
    SourceOrigin, UnitSite,
};
pub use native_support::NativeSupport;
pub use registry::{PaperLookupError, PaperLookupFault};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentArtifact {
    path: String,
    bytes: Vec<u8>,
    origins: Vec<SourceOrigin>,
    emissions: Vec<EmissionRecord>,
    html_gap_sites: Vec<UnitSite>,
}
impl DocumentArtifact {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn origins(&self) -> &[SourceOrigin] {
        &self.origins
    }
    #[must_use]
    pub fn emissions(&self) -> &[EmissionRecord] {
        &self.emissions
    }
}
/// An owned checked snapshot, carrying no audit verdict or publication permission.
/// ```compile_fail,E0451
/// let value = ftoo_codegen::PreparedDocuments {
///     artifacts: Vec::new(), inputs: Default::default(), observed_outputs: Default::default(),
///     unavailable: Vec::new(),
///     html_gap_rows: Vec::new(),
///     screen_gap_rows: Vec::new(),
///     snapshot: None,
/// };
/// ```
/// ```compile_fail,E0277
/// let value: ftoo_codegen::PreparedDocuments = Vec::<ftoo_codegen::DocumentArtifact>::new().into();
/// ```
/// ```compile_fail,E0596
/// fn mutate(value: &mut ftoo_codegen::PreparedDocuments) { value.projection().fill(0); }
/// ```
/// ```compile_fail,E0061
/// fn certify(root: &std::path::Path, support: &ftoo_codegen::NativeSupport) {
///     ftoo_codegen::prepare_documents(root, support, true);
/// }
/// ```
/// ```
/// fn inspect(value: &ftoo_codegen::PreparedDocuments) {
///     let _: &[ftoo_codegen::DocumentArtifact] = value.artifacts();
///     let _: &[u8] = value.projection();
///     for artifact in value.artifacts() {
///         let _: &str = artifact.path();
///         let _: &[u8] = artifact.bytes();
///         let _: &[ftoo_manifest::SourceOrigin] = artifact.origins();
///         let _: &[ftoo_codegen::EmissionRecord] = artifact.emissions();
///     }
/// }
/// ```
/// ```compile_fail,E0599
/// let value = ftoo_codegen::PreparedDocuments::default();
/// ```
/// ```compile_fail,E0277
/// let value: ftoo_codegen::PreparedDocuments = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0616
/// fn mutate(value: &mut ftoo_codegen::PreparedDocuments) { value.artifacts.clear(); }
/// ```
/// ```compile_fail,E0596
/// fn mutate(value: &mut ftoo_codegen::PreparedDocuments) { value.artifacts()[0].bytes().fill(0); }
/// ```
/// ```compile_fail,E0277
/// fn promote(value: ftoo_codegen::PreparedDocuments) -> ftoo_audit::AuditedStatus {
///     value.into()
/// }
/// ```
#[derive(Debug)]
pub struct PreparedDocuments {
    snapshot: Option<crate::retention::DocumentSnapshot>,
    artifacts: Vec<DocumentArtifact>,
    html_gap_rows: Vec<HtmlGapRow>,
    inputs: BTreeMap<String, Vec<u8>>,
    observed_outputs: BTreeMap<String, Vec<u8>>,
    unavailable: Vec<destinations::IndexEntry>,
    screen_gap_rows: Vec<ScreenGapRow>,
}
impl PreparedDocuments {
    #[must_use]
    pub(crate) fn html_gap_rows(&self) -> &[HtmlGapRow] {
        &self.html_gap_rows
    }
    #[must_use]
    pub fn artifacts(&self) -> &[DocumentArtifact] {
        &self.artifacts
    }
    #[must_use]
    pub fn inputs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.inputs
    }
    #[must_use]
    pub fn observed_outputs(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.observed_outputs
    }
    #[must_use]
    pub fn projection(&self) -> &[u8] {
        &self.inputs["generated/manifest.json"]
    }
    pub(crate) fn check_release_destinations(&self) -> Result<(), String> {
        destinations::release(&self.unavailable, !self.screen_gap_rows.is_empty()).map_err(
            |error| {
                let entries = self
                    .unavailable
                    .iter()
                    .map(|entry| {
                        format!(
                            "{:?} / {} #{}",
                            entry.document, entry.language, entry.fragment
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let gaps = self
                    .screen_gap_rows
                    .iter()
                    .map(|row| {
                        format!(
                            "{} / {} position {} between {:?} and {:?}",
                            row.document,
                            row.language,
                            row.position,
                            row.before_unit,
                            row.after_unit
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{error:?}; unavailable: {entries}; content gaps: {gaps}")
            },
        )
    }
    pub(crate) fn screen_gap_inventory(&self) -> serde_json::Value {
        use serde_json::json;
        let entries = self
            .unavailable
            .iter()
            .map(|entry| {
                let document = match entry.document {
                    destinations::Reading::Paper => "paper-pilot",
                    destinations::Reading::Explainer => "explainer-pilot",
                };
                json!({
                    "document": document,
                    "language": entry.language,
                    "fragment": entry.fragment,
                    "locator": entry.locator,
                    "message": entry.message,
                    "title": entry.title,
                })
            })
            .collect::<Vec<_>>();
        let gaps = self
            .screen_gap_rows
            .iter()
            .map(|row| {
                json!({
                    "document": row.document,
                    "language": row.language,
                    "position": row.position,
                    "before-unit": row.before_unit,
                    "after-unit": row.after_unit,
                    "message": row.message,
                })
            })
            .collect::<Vec<_>>();
        json!({"schema": 1, "entries": entries, "content-gaps": gaps})
    }
    #[cfg(test)]
    pub(crate) fn clear_screen_gaps_for_test(&mut self) {
        self.unavailable.clear();
        self.screen_gap_rows.clear();
    }
}

#[derive(Debug)]
struct ScreenGapRow {
    document: String,
    language: String,
    position: usize,
    before_unit: Option<String>,
    after_unit: Option<String>,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HtmlGapRow {
    pub document: String,
    pub unit: String,
    pub block: usize,
    pub inline_kind: String,
    pub site: UnitSite,
}

fn html_gap_rows(
    plans: &[plan::DocumentPlan],
    selected: &BTreeMap<String, Vec<DocumentOutput>>,
) -> Vec<HtmlGapRow> {
    let mut rows = Vec::new();
    for plan in plans {
        if !selected
            .get(&plan.document)
            .is_some_and(|targets| targets.contains(&DocumentOutput::Html))
        {
            continue;
        }
        if let Some(screen) = &plan.screen {
            for &block in screen.html_gaps.keys() {
                let passage = &plan.blocks[block];
                for inline_kind in html::unrealizable_kinds(&passage.runs, passage.paragraph) {
                    rows.push(HtmlGapRow {
                        document: plan.document.clone(),
                        unit: passage.unit.clone(),
                        block,
                        inline_kind: inline_kind.into(),
                        site: passage.site.clone(),
                    });
                }
            }
        }
    }
    rows
}

pub(crate) fn refuse_release_html_gaps(
    artifacts: &[DocumentArtifact],
) -> Result<(), DocumentError> {
    for artifact in artifacts {
        if let Some(site) = artifact.html_gap_sites.first() {
            return Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Source(site.clone()),
                AssemblyFault::UnknownToken {
                    value: artifact.path.clone(),
                    expected: vec!["complete HTML passage".into()],
                },
            ));
        }
    }
    Ok(())
}
/// Regenerate excerpt skeleton projections before generating document artifacts.
/// # Errors
/// Reports invalid source scopes, projection writes, or document generation failures.
pub fn generate_documents(
    root: &Path,
    support: &NativeSupport,
) -> Result<PreparedDocuments, DocumentError> {
    input::generate_projections(root)?;
    artifacts::generate_documents(root, support)
}
fn failure(
    rule: RuleId,
    stage: AssemblyStage,
    site: AssemblySite,
    fault: AssemblyFault,
) -> DocumentError {
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule,
            stage,
            document: None,
            unit: None,
            language: None,
            target: None,
            site,
            fault,
        }])
        .expect("one diagnostic"),
    )
}
fn neutral(
    markdown: &ftoo_manifest::CheckedMarkdown,
    generated_only: bool,
    stage: AssemblyStage,
) -> Result<(), DocumentError> {
    for source in markdown.sources() {
        if generated_only && !source.file.starts_with("generated/documents/") {
            continue;
        }
        let collect =
            |occurrences: &[ftoo_manifest::LabelOccurrence]| -> Vec<GeneratedLabelOccurrence> {
                occurrences
                    .iter()
                    .filter_map(|o| match &o.site {
                        ftoo_manifest::LabelSite::Markdown { file, bytes }
                            if file == &source.file =>
                        {
                            Some(GeneratedLabelOccurrence {
                                label: o.label.clone(),
                                bytes: bytes.clone(),
                            })
                        }
                        _ => None,
                    })
                    .collect()
            };
        let mints = collect(markdown.mints());
        let citations = collect(markdown.citations());
        if !mints.is_empty() || !citations.is_empty() {
            return Err(failure(
                RuleId::M055,
                stage,
                AssemblySite::Inventory {
                    path: source.file.clone(),
                },
                AssemblyFault::GeneratedAddressSyntax {
                    path: source.file.clone(),
                    mints,
                    citations,
                },
            ));
        }
    }
    Ok(())
}
/// Prepare both targets, shared support and actual native consumers before writing.
/// # Errors
/// Preserves prerequisite failures, exact assembly refusals and dependency evidence.
/// # Panics
/// Panics only if private shape or ownership invariants disagree, which input cannot arrange.
pub fn prepare_documents(
    root: &Path,
    support: &NativeSupport,
) -> Result<PreparedDocuments, DocumentError> {
    prepare_context(root, support, None)
}
// The direct snapshot control cannot detect a caller that bypasses this merge;
// every checker and Rust read of the same path must pass through it.
fn merge_checked_input(
    inputs: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), DocumentError> {
    if inputs.get(&path).is_some_and(|held| held != &bytes) {
        return Err(DocumentError::PaperLookup(registry::PaperLookupError {
            rule: RuleId::M063,
            site: path.clone(),
            fault: registry::PaperLookupFault::InputChanged { path },
        }));
    }
    inputs.insert(path, bytes);
    Ok(())
}
#[allow(clippy::too_many_lines)] // Admission and the final source snapshot form one preparation boundary.
pub(crate) fn prepare_context(
    root: &Path,
    support: &NativeSupport,
    accepted: Option<&ftoo_audit::retention::AcceptedRetention>,
) -> Result<PreparedDocuments, DocumentError> {
    let _preparation = crate::telemetry::Phase::new("documents.preparation");
    let admission = crate::telemetry::Phase::new("documents.source_admission");
    let support = NativeSupport::resolve(root, Some(support.directory()))?;
    let projection = input::read(root, "generated/manifest.json")?;
    crate::check_projection(root).map_err(DocumentError::Projection)?;
    let mut inputs = BTreeMap::from([("generated/manifest.json".into(), projection.clone())]);
    for file in [
        "documents/paper-build/fontconfig.conf",
        inherited_support::COMPARED_PATHS[0],
        inherited_support::COMPARED_PATHS[1],
        inherited_support::COMPARED_PATHS[2],
        inherited_support::TYPED_PATH,
        inherited_support::MACROS_PATH,
        "documents/paper/references.bib",
        "tools/assembly/native-lock.toml",
        "tools/assembly/prepare-native.py",
        "tools/notation/package-lock.json",
        "tools/notation/package.json",
        "tools/notation/support-digests.json",
        "tools/notation/.node-version",
    ] {
        inputs.insert(file.into(), input::read(root, file)?);
    }
    input::reread(root, &inputs)?;
    let loaded =
        ftoo_manifest::load_projection(&projection).map_err(DocumentError::ProjectionLoad)?;
    let book = loaded.addressing().map_err(|e| {
        DocumentError::ProjectionLoad(ftoo_manifest::ProjectionError::Addressing(
            e.diagnostics().to_vec(),
        ))
    })?;
    let declaration = crate::UnitSource {
        file: "units/documents.toml".into(),
        bytes: input::read(root, "units/documents.toml")?,
    };
    let statement_files = input::discover(root, "units/statements", false)?;
    let statements = input::sources(root, &statement_files)?;
    let corpus = crate::load_unit_sources(declaration.clone(), statements.clone())
        .map_err(DocumentError::Units)?;
    let selected = crate::units::toml::selected_targets(&declaration)
        .map_err(|errors| DocumentError::Units(crate::UnitLoadError::Admission(errors)))?;
    let paper_contexts = crate::units::toml::paper_contexts(&declaration)
        .map_err(|errors| DocumentError::Units(crate::UnitLoadError::Admission(errors)))?;
    let paper_lookup = if paper_contexts.is_empty() {
        None
    } else {
        Some(registry::CheckedPaperLookup::check(root).map_err(DocumentError::PaperLookup)?)
    };
    if let Some(lookup) = &paper_lookup {
        lookup
            .validate_econ(&corpus)
            .map_err(DocumentError::PaperLookup)?;
    }
    if let Some(lookup) = &paper_lookup {
        for (path, bytes) in lookup.inputs() {
            merge_checked_input(&mut inputs, path.clone(), bytes.clone())?;
        }
    }
    let markdown = crate::load_markdown_files(root).map_err(DocumentError::Labels)?;
    neutral(&markdown, true, AssemblyStage::Source)?;
    let joint = ftoo_manifest::collect_joint_mints(&corpus, &markdown)
        .map_err(|e| DocumentError::Labels(crate::LabelFileError::Labels(e)))?;
    let mut registry_joint = joint.clone();
    if root.join(registry::PAPER_MINTS).try_exists().map_err(|e| {
        input::io_error(
            DocumentIoOperation::Inspect,
            &root.join(registry::PAPER_MINTS),
            &e,
        )
    })? {
        let data = input::read(root, registry::PAPER_MINTS)?;
        registry_joint = registry_joint
            .with_paper_mints(registry::paper_mint_rows(&data)?)
            .map_err(|e| DocumentError::Labels(crate::LabelFileError::Labels(e)))?;
        merge_checked_input(&mut inputs, registry::PAPER_MINTS.into(), data)?;
    }
    for unit in corpus.units() {
        for text in unit.texts().values() {
            for reference in text.reference_occurrences() {
                if let Some(ftoo_manifest::MintOrigin {
                    kind: ftoo_manifest::MintKind::PaperTex,
                    site: ftoo_manifest::LabelSite::PaperMintFile { file },
                }) = registry_joint.mints().get(reference.label())
                {
                    merge_checked_input(&mut inputs, file.clone(), input::read(root, file)?)?;
                }
            }
        }
    }
    let source = registry::read(root)?;
    let events = registry::load(&source)?;
    merge_checked_input(&mut inputs, source.file, source.bytes)?;
    let book = book
        .with_documentary(&registry_joint, events)
        .map_err(DocumentError::DocumentaryRegistry)?;
    let paper_documents = paper_contexts.into_keys().collect();
    let resolved = ftoo_manifest::resolve_unit_tags_with(&corpus, &book, &paper_documents, |key| {
        paper_lookup.as_ref().and_then(|lookup| lookup.resolve(key))
    })
    .map_err(|e| DocumentError::Units(crate::UnitLoadError::Admission(e)))?;
    let integrated =
        ftoo_manifest::integrate_joint_labels(&resolved, &joint.clone().strict_units())
            .map_err(|e| DocumentError::Labels(crate::LabelFileError::Labels(e)))?;
    let welded = ftoo_manifest::weld_units(&integrated).map_err(DocumentError::Weld)?;
    let skeleton_files = input::discover(root, "units/skeletons", false)?;
    let scope_files = input::discover(root, "units/scopes", false)?;
    let display_files = input::discover(root, "units/displays", true)?;
    let sources = input::sources(root, &skeleton_files)?;
    let scope_sources = input::sources(root, &scope_files)?;
    let display_sources = input::sources(root, &display_files)?;
    let (drafts, display_drafts) = input::assembly_sources(&sources, &display_sources)?;
    let inclusions = input::composition(root, &declaration, &scope_sources, &drafts)?;
    let master_sources = input::read(root, "documents/paper-build/sources.tsv")?;
    let master_sources = std::str::from_utf8(&master_sources).map_err(|_| {
        DocumentError::Input(DocumentInputFault::InvalidSourceName {
            value: "documents/paper-build/sources.tsv".into(),
        })
    })?;
    merge_checked_input(
        &mut inputs,
        "documents/paper-build/sources.tsv".into(),
        master_sources.as_bytes().to_vec(),
    )?;
    for path in master_sources.lines().filter(|path| {
        *path == "documents/paper/main.tex" || path.starts_with("documents/paper/parts/")
    }) {
        merge_checked_input(&mut inputs, path.into(), input::read(root, path)?)?;
    }
    for inclusion in &inclusions {
        merge_checked_input(
            &mut inputs,
            inclusion.source.clone(),
            input::read(root, &inclusion.source)?,
        )?;
    }
    let (metadata, owner_sites) = metadata(&statements);
    let document_sites = document_sites(&corpus);
    let skeletons =
        ftoo_manifest::check_skeletons(&corpus, &drafts, &metadata, &owner_sites, &document_sites)
            .map_err(|e| {
                DocumentError::Assembly(
                    AssemblyErrors::from_diagnostics(e.into_iter().map(Into::into).collect())
                        .expect("nonempty errors"),
                )
            })?;
    let displays =
        ftoo_manifest::check_displays(&display_drafts, &skeletons, &welded).map_err(|e| {
            DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(e.into_iter().map(Into::into).collect())
                    .expect("nonempty errors"),
            )
        })?;
    let catalogue = ftoo_manifest::notation(&book)
        .map_err(|e| DocumentError::Notation(crate::NotationRenderError::Admission(e)))?;
    let effective = accepted
        .map(|accepted| ftoo_audit::retention::RetainedNotation::new(&catalogue, accepted))
        .transpose()
        .map_err(|error| DocumentError::Retention(Box::new(error.into())))?;
    if let Some(accepted) = accepted {
        inputs.insert(
            "generated/audit-report.json".into(),
            accepted.bytes().to_vec(),
        );
    }
    let sources: Vec<_> = std::iter::once(declaration)
        .chain(statements)
        .chain(sources)
        .chain(scope_sources)
        .chain(display_sources)
        .collect();
    let snapshot = accepted
        .map(|_| {
            crate::retention::DocumentSnapshot::capture(
                root,
                inputs
                    .iter()
                    .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
                    .chain(
                        markdown
                            .sources()
                            .iter()
                            .map(|source| (source.file.as_str(), source.bytes.as_slice())),
                    )
                    .chain(
                        sources
                            .iter()
                            .map(|source| (source.file.as_str(), source.bytes.as_slice())),
                    ),
            )
        })
        .transpose()
        .map_err(|error| DocumentError::Retention(Box::new(error)))?;
    drop(admission);
    crate::notation::with_preparation(effective.as_ref(), &catalogue, |notation| {
        let bibliography =
            std::str::from_utf8(&inputs["documents/paper/references.bib"]).map_err(|_| {
                failure(
                    RuleId::M055,
                    AssemblyStage::Source,
                    AssemblySite::Inventory {
                        path: "documents/paper/references.bib".into(),
                    },
                    AssemblyFault::UnknownToken {
                        value: "references.bib".into(),
                        expected: vec!["UTF-8 bibliography".into()],
                    },
                )
            })?;
        let mut plans = plan::build(
            &skeletons,
            &welded,
            notation,
            &metadata,
            &displays,
            bibliography,
            paper_lookup.as_ref(),
            &inputs,
            &paper_documents,
        )?;
        plan::plan_screens(
            &mut plans,
            &selected,
            &book,
            &skeletons,
            &document_sites,
            &inclusions,
            &inputs,
            registry_joint.mints(),
            paper_lookup.as_ref(),
        )?;
        let gap_rows = html_gap_rows(&plans, &selected);
        let known_labels: Vec<_> = registry_joint.mints().keys().map(String::as_str).collect();
        let mut artifacts = render_artifacts(
            root,
            &support,
            &book,
            &selected,
            &plans,
            notation,
            &inputs,
            &known_labels,
        )?;
        let mut inclusion_text = String::new();
        {
            use std::fmt::Write as _;
            for inclusion in &inclusions {
                writeln!(
                    inclusion_text,
                    "{}\t{}\t{}\t{}\t{}",
                    inclusion.source,
                    inclusion.unit,
                    inclusion.fragment,
                    inclusion.role,
                    inclusion.line
                )
                .expect("string write");
            }
        }
        artifacts.push(DocumentArtifact {
            path: "generated/documents/inclusions.tsv".into(),
            bytes: inclusion_text.into_bytes(),
            origins: Vec::new(),
            emissions: Vec::new(),
            html_gap_sites: Vec::new(),
        });
        let mut prepared = finish_snapshot(
            root,
            inputs,
            &markdown,
            sources.into_iter(),
            [
                ("units/statements", statement_files, false),
                ("units/skeletons", skeleton_files, false),
                ("units/scopes", scope_files, false),
                ("units/displays", display_files, true),
            ],
            artifacts,
        )?;
        prepared.unavailable = plans
            .iter()
            .flat_map(|plan| {
                plan.screen
                    .iter()
                    .flat_map(|screen| screen.index.iter().cloned())
            })
            .collect();
        prepared.screen_gap_rows = plans
            .iter()
            .flat_map(|plan| {
                plan.screen.iter().flat_map(|screen| {
                    screen
                        .gaps
                        .iter()
                        .enumerate()
                        .filter(|(_, gap)| !gap.message.is_empty())
                        .map(|(position, gap)| ScreenGapRow {
                            document: plan.document.clone(),
                            language: plan.language.clone(),
                            position,
                            before_unit: (position > 0).then(|| plan.blocks[position].unit.clone()),
                            after_unit: plan
                                .blocks
                                .get(position + 1)
                                .map(|block| block.unit.clone()),
                            message: gap.message.clone(),
                        })
                })
            })
            .collect();
        prepared.html_gap_rows = gap_rows;
        if let Some(snapshot) = snapshot {
            snapshot
                .recheck(root)
                .map_err(|error| DocumentError::Retention(Box::new(error)))?;
            prepared.snapshot = Some(snapshot.without_outputs());
        }
        Ok(prepared)
    })
}
fn finish_snapshot(
    root: &Path,
    mut inputs: BTreeMap<String, Vec<u8>>,
    markdown: &ftoo_manifest::CheckedMarkdown,
    sources: impl Iterator<Item = crate::UnitSource>,
    inventories: [(&str, Vec<String>, bool); 4],
    artifacts: Vec<DocumentArtifact>,
) -> Result<PreparedDocuments, DocumentError> {
    let _phase = crate::telemetry::Phase::new("documents.snapshot_io");
    let mut observed_outputs = BTreeMap::new();
    for source in markdown.sources() {
        if source.file.starts_with("generated/documents/") {
            observed_outputs.insert(source.file.clone(), source.bytes.clone());
        } else {
            inputs.insert(source.file.clone(), source.bytes.clone());
        }
    }
    for source in sources {
        inputs.insert(source.file, source.bytes);
    }
    input::reread(root, &inputs)?;
    let final_markdown = crate::load_markdown_files(root).map_err(DocumentError::Labels)?;
    let expected = markdown
        .sources()
        .iter()
        .map(|s| s.file.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let actual = final_markdown
        .sources()
        .iter()
        .map(|s| s.file.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        return Err(DocumentError::Input(
            DocumentInputFault::SourceInventoryChanged {
                missing: expected.difference(&actual).cloned().collect(),
                unexpected: actual.difference(&expected).cloned().collect(),
            },
        ));
    }
    input::reread(root, &observed_outputs)?;
    for (directory, expected, absent) in inventories {
        let actual = input::discover(root, directory, absent)?;
        if actual != expected {
            return Err(DocumentError::Input(
                DocumentInputFault::SourceInventoryChanged {
                    missing: expected
                        .iter()
                        .filter(|p| !actual.contains(p))
                        .cloned()
                        .collect(),
                    unexpected: actual
                        .iter()
                        .filter(|p| !expected.contains(p))
                        .cloned()
                        .collect(),
                },
            ));
        }
    }
    Ok(PreparedDocuments {
        snapshot: None,
        artifacts,
        html_gap_rows: Vec::new(),
        inputs,
        observed_outputs,
        unavailable: Vec::new(),
        screen_gap_rows: Vec::new(),
    })
}

fn metadata(
    statements: &[crate::UnitSource],
) -> (BTreeMap<String, UnitSite>, BTreeMap<String, UnitSite>) {
    let mut metadata = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for statement in statements {
        let text = std::str::from_utf8(&statement.bytes).expect("admitted UTF-8");
        let tree = crate::units::toml::parse(&statement.file, text).expect("admitted syntax");
        let fields =
            crate::units::toml::fixed(&tree, &["schema", "unit"]).expect("admitted fields");
        let mut allowed = vec!["id", "document", "math", "texts"];
        if fields[1]
            .fields
            .iter()
            .any(|(key, _, _)| key == "documentary-targets")
        {
            allowed.push("documentary-targets");
        }
        if fields[1].fields.iter().any(|(key, _, _)| key == "econ") {
            allowed.push("econ");
        }
        let unit = crate::units::toml::fixed(fields[1], &allowed).expect("admitted fields");
        let id = crate::units::toml::string(unit[0]).expect("admitted id");
        owners.insert(id.value.clone(), unit[1].site.clone());
        metadata.insert(id.value, id.site);
    }
    (metadata, owners)
}

#[allow(clippy::too_many_lines)] // Selected outputs and shared support remain in one preparation scope.
#[allow(clippy::too_many_arguments)] // Source snapshots are separate from document selection and rendered support.
fn render_artifacts(
    root: &Path,
    support: &NativeSupport,
    book: &ftoo_manifest::AddressBook<'_>,
    selected: &BTreeMap<String, Vec<DocumentOutput>>,
    plans: &[plan::DocumentPlan],
    notation: &crate::RenderableNotation<'_, '_, '_>,
    inputs: &BTreeMap<String, Vec<u8>>,
    known_labels: &[&str],
) -> Result<Vec<DocumentArtifact>, DocumentError> {
    let addressing = runtime::render(book, plans)?;
    check_rendered_bytes("generated/addressing.tex", addressing.as_bytes(), &[])?;
    check_rendered_bytes(
        "generated/notation.tex",
        notation.declarations().as_bytes(),
        &[],
    )?;
    let notation_status = notation.status_artifact();
    check_rendered_bytes("generated/notation-status.json", &notation_status, &[])?;
    let source_text = |path: &str| std::str::from_utf8(&inputs[path]).unwrap_or("");
    let inherited = inherited_support::generate(
        notation.catalogue(),
        inherited_support::Sources {
            symbols: source_text(inherited_support::COMPARED_PATHS[0]),
            historical: source_text(inherited_support::COMPARED_PATHS[1]),
            registers: source_text(inherited_support::COMPARED_PATHS[2]),
            typed_tools: source_text(inherited_support::TYPED_PATH),
            macros_overlap: source_text(inherited_support::MACROS_PATH),
        },
    )
    .map_err(|refusal| {
        let (rule, path) = match &refusal {
            inherited_support::Refusal::ChangedSource(short) => {
                let path = inherited_support::COMPARED_PATHS
                    .into_iter()
                    .find(|path| path.ends_with(short))
                    .unwrap_or_else(|| {
                        if *short == "ftoo_typed_tools.tex" {
                            inherited_support::TYPED_PATH
                        } else {
                            inherited_support::MACROS_PATH
                        }
                    });
                let rule = if inherited_support::COMPARED_PATHS.contains(&path) {
                    RuleId::M078
                } else {
                    RuleId::M056
                };
                (rule, path)
            }
            inherited_support::Refusal::MissingPrecondition(_) => {
                (RuleId::M056, inherited_support::MACROS_PATH)
            }
        };
        failure(
            rule,
            AssemblyStage::Source,
            AssemblySite::Inventory { path: path.into() },
            AssemblyFault::InheritedSupport {
                reason: format!("{refusal:?}"),
            },
        )
    })?;
    check_rendered_bytes(inherited_support::PATH, inherited.as_bytes(), &[])?;
    support.set_inherited(inherited.as_bytes(), &inputs[inherited_support::TYPED_PATH]);
    let lock = native_support::lock(root)?;
    support.scope(&lock, || {
        let mut artifacts = Vec::new();
        for plan in plans {
            let targets = &selected[&plan.document];
            if targets.contains(&DocumentOutput::Markdown) {
                markdown::check_math_admission(plan)?;
            }
            refuse_quoted_screen_targets(plan, targets)?;
            for target in targets {
                match target {
                    DocumentOutput::Markdown => {
                        let (annotated, reader) = markdown::emit(plan, known_labels)?;
                        plan::check_emission(plan, &annotated, DocumentTarget::Markdown)?;
                        let path = format!(
                            "generated/documents/{}/{}.labels.md",
                            plan.document, plan.language
                        );
                        let reader_path =
                            format!("generated/documents/{}/{}.md", plan.document, plan.language);
                        let checked = ftoo_manifest::check_markdown_sources(vec![
                            ftoo_manifest::MarkdownSource {
                                file: path.clone(),
                                bytes: annotated.as_bytes().to_vec(),
                            },
                            ftoo_manifest::MarkdownSource {
                                file: reader_path.clone(),
                                bytes: reader.as_bytes().to_vec(),
                            },
                        ])
                        .map_err(|e| DocumentError::Labels(crate::LabelFileError::Source(e)))?;
                        neutral(&checked, false, AssemblyStage::Emission)?;
                        artifacts.push(DocumentArtifact {
                            path,
                            bytes: annotated.into_bytes(),
                            origins: plan.origins(DocumentTarget::Markdown),
                            emissions: plan.records(DocumentTarget::Markdown),
                            html_gap_sites: Vec::new(),
                        });
                        artifacts.push(DocumentArtifact {
                            path: reader_path,
                            bytes: reader.into_bytes(),
                            origins: plan.origins(DocumentTarget::Markdown),
                            emissions: plan.records(DocumentTarget::Markdown),
                            html_gap_sites: Vec::new(),
                        });
                    }
                    DocumentOutput::Latex => {
                        let path = format!(
                            "generated/documents/{}/{}.tex",
                            plan.document, plan.language
                        );
                        let tex = latex::render(plan);
                        let origins = plan.origins(DocumentTarget::Latex);
                        check_rendered_bytes(&path, tex.as_bytes(), &origins)?;
                        plan::check_emission(plan, &tex, DocumentTarget::Latex)?;
                        native::consume_plan(
                            support,
                            &lock,
                            &tex,
                            notation.declarations(),
                            &addressing,
                            plan,
                        )?;
                        artifacts.push(DocumentArtifact {
                            path,
                            bytes: tex.into_bytes(),
                            origins,
                            emissions: plan.records(DocumentTarget::Latex),
                            html_gap_sites: Vec::new(),
                        });
                    }
                    DocumentOutput::Fragment => {
                        for block in plan.blocks.iter().filter(|block| block.depth.is_none()) {
                            let source = fragment_line(block)?;
                            let path = input::fragment_path(&plan.document, &block.unit);
                            check_rendered_bytes(&path, source.as_bytes(), &[])?;
                            artifacts.push(DocumentArtifact {
                                path,
                                bytes: source.into_bytes(),
                                origins: Vec::new(),
                                emissions: Vec::new(),
                                html_gap_sites: Vec::new(),
                            });
                        }
                    }
                    DocumentOutput::Html => {
                        html::check_destinations(plan, plans, selected)?;
                        let path = html::destination(html::Destination::Artifact(plan));
                        let source = html::render(plan, support, known_labels, Some(notation))?;
                        check_rendered_bytes(&path, source.as_bytes(), &[])?;
                        artifacts.push(DocumentArtifact {
                            path,
                            bytes: source.into_bytes(),
                            origins: Vec::new(),
                            emissions: Vec::new(),
                            html_gap_sites: plan
                                .screen
                                .as_ref()
                                .into_iter()
                                .flat_map(|screen| screen.html_gaps.values())
                                .map(|gap| gap.site.clone())
                                .collect(),
                        });
                    }
                }
            }
        }
        consume_sweep(support, &lock, notation, &addressing)?;
        artifacts.push(DocumentArtifact {
            path: "generated/notation.tex".into(),
            bytes: notation.declarations().as_bytes().to_vec(),
            origins: Vec::new(),
            emissions: Vec::new(),
            html_gap_sites: Vec::new(),
        });
        artifacts.push(DocumentArtifact {
            path: "generated/notation-status.json".into(),
            bytes: notation_status,
            origins: Vec::new(),
            emissions: Vec::new(),
            html_gap_sites: Vec::new(),
        });
        artifacts.push(DocumentArtifact {
            path: inherited_support::PATH.into(),
            bytes: inherited.into_bytes(),
            origins: Vec::new(),
            emissions: Vec::new(),
            html_gap_sites: Vec::new(),
        });
        artifacts.push(DocumentArtifact {
            path: "generated/addressing.tex".into(),
            bytes: addressing.into_bytes(),
            origins: Vec::new(),
            emissions: Vec::new(),
            html_gap_sites: Vec::new(),
        });
        unique_artifact_paths(&artifacts)?;
        Ok(artifacts)
    })
}

fn check_rendered_bytes(
    path: &str,
    bytes: &[u8],
    origins: &[SourceOrigin],
) -> Result<(), DocumentError> {
    if bytes.len() > 16_777_216 {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Generated {
                path: path.into(),
                bytes: 16_777_216..16_777_217,
                origins: origins.to_vec(),
            },
            AssemblyFault::Limit {
                resource: ftoo_manifest::AssemblyResource::DocumentBytes,
                limit: 16_777_216,
                actual: 16_777_217,
            },
        ));
    }
    Ok(())
}

fn refuse_quoted_screen_targets(
    plan: &plan::DocumentPlan,
    targets: &[DocumentOutput],
) -> Result<(), DocumentError> {
    if let Some(block) = plan.blocks.iter().find(|block| block.quotation)
        && targets.iter().any(|target| {
            *target == DocumentOutput::Latex
                || (*target == DocumentOutput::Markdown && plan.screen.is_none())
        })
    {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(block.site.clone()),
            AssemblyFault::UnknownToken {
                value: block.unit.clone(),
                expected: vec!["quotation emitter".into()],
            },
        ));
    }
    Ok(())
}

fn unique_artifact_paths(artifacts: &[DocumentArtifact]) -> Result<(), DocumentError> {
    let mut paths = BTreeSet::new();
    for artifact in artifacts {
        if !paths.insert(&artifact.path) {
            return Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: artifact.path.clone(),
                    bytes: 0..0,
                    origins: artifact.origins.clone(),
                },
                AssemblyFault::UnknownToken {
                    value: artifact.path.clone(),
                    expected: vec!["unique document artifact path".into()],
                },
            ));
        }
    }
    Ok(())
}

fn fragment_macro(block: &plan::Block) -> &'static str {
    if block.quotation {
        "paperrefrain"
    } else {
        "refrain"
    }
}

fn fragment_line(block: &plan::Block) -> Result<String, DocumentError> {
    if let Some(econ) = &block.econ {
        return econ_fragment_line(block, econ);
    }
    if block.equation {
        let (_, source) = block.equation_pair().ok_or_else(|| {
            failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Source(block.site.clone()),
                AssemblyFault::UnsupportedParagraphBody {
                    reason: "equation body must contain one display insertion and a terminal stop"
                        .into(),
                },
            )
        })?;
        return Ok(format!(
            "\\PaperNativeEquation{{{}}}{{\\[{source}.\\]}}\n",
            block.unit
        ));
    }
    if block.paragraph {
        return paragraph_fragment_line(block);
    }
    let text = fragment_text(block)?;
    let macro_name = fragment_macro(block);
    Ok(format!(
        "\\{macro_name}{{{}}}{{{}}}\n",
        block.unit,
        latex::prose_text(text)
    ))
}

fn econ_fragment_line(block: &plan::Block, econ: &plan::EconPlan) -> Result<String, DocumentError> {
    use std::fmt::Write;
    let refuse = |site: &ftoo_manifest::UnitSite, reason: &str| {
        failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(site.clone()),
            AssemblyFault::UnsupportedParagraphBody {
                reason: reason.into(),
            },
        )
    };
    let mut body = String::new();
    for run in &block.runs {
        match run {
            plan::Inline::Text(value) => {
                if adjacent_dash_marks(value) {
                    return Err(refuse(&block.site, "adjacent dash marks in econ body"));
                }
                let emitted = latex::prose_text(value).replace('—', "---");
                if emitted
                    .chars()
                    .any(|c| matches!(c, '{' | '}' | '\\' | '\n' | '\r'))
                {
                    return Err(refuse(
                        &block.site,
                        "econ body requires unsupported TeX escape",
                    ));
                }
                body.push_str(&emitted);
            }
            plan::Inline::BibliographyCitation {
                kind: ftoo_manifest::BibliographyCitationKind::Parenthetical,
                entries,
                site,
                ..
            } => {
                if entries.len() != 1 {
                    return Err(refuse(site, "econ citation arity"));
                }
                write!(body, "\\parencite{{{}}}", entries[0].key).expect("string write");
            }
            _ => return Err(refuse(&block.site, "unsupported econ body run")),
        }
    }
    let options = [
        &econ.target,
        &econ.title,
        &econ.reference,
        &econ.departure,
        &econ.stops_at,
    ];
    if options.iter().any(|value| {
        value
            .chars()
            .any(|c| matches!(c, '{' | '}' | '[' | ']' | '\\' | '%' | '#' | '\n' | '\r'))
    }) {
        return Err(refuse(
            &block.site,
            "econ option requires unsupported TeX syntax",
        ));
    }
    Ok(format!(
        "\\begin{{econ}}[label={{{}}},title={{{}}},ref={{{}}},cites={{{}}},departs={{{}}},stopsat={{{}}}]{}\\end{{econ}}\n",
        econ.target,
        econ.title,
        econ.reference,
        econ.citations.join(","),
        econ.departure,
        econ.stops_at,
        body
    ))
}

#[allow(clippy::too_many_lines)] // Fragment forms share one marker and one refusal site.
fn paragraph_fragment_line(block: &plan::Block) -> Result<String, DocumentError> {
    use std::fmt::Write;
    let refuse = |reason: &str| {
        failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(block.site.clone()),
            AssemblyFault::UnsupportedParagraphBody {
                reason: reason.into(),
            },
        )
    };
    if !block.displays.is_empty() {
        return Err(refuse("a paragraph fragment has a display"));
    }
    let mut body = String::new();
    for run in &block.runs {
        match run {
            plan::Inline::Text(value) => {
                if adjacent_dash_marks(value) {
                    return Err(refuse("adjacent hyphen or em-dash marks"));
                }
                let emitted = latex::prose_text(value).replace('—', "---");
                if emitted
                    .chars()
                    .any(|c| matches!(c, '{' | '}' | '\\' | '\n' | '\r'))
                {
                    return Err(refuse(
                        "text requires braced, backslash or multiline output",
                    ));
                }
                body.push_str(&emitted);
            }
            plan::Inline::PaperTag { value, mint, .. } => {
                write!(
                    body,
                    "\\{}{{{}}}",
                    if mint.is_some() { "handle" } else { "ftag" },
                    value.key
                )
                .expect("string write");
            }
            plan::Inline::See { label, .. } => {
                write!(body, "\\see{{{label}}}").expect("string write");
            }
            plan::Inline::Virtue {
                word, companion, ..
            } => {
                let command = if *companion { "papervirtue" } else { "virtue" };
                write!(body, "\\{command}{{{word}}}").expect("string write");
            }
            plan::Inline::Math { latex, site, .. } => {
                if !plan::native_paragraph_command(latex) {
                    return Err(failure(
                        RuleId::M055,
                        AssemblyStage::Emission,
                        AssemblySite::Source(site.clone()),
                        AssemblyFault::UnsupportedParagraphBody {
                            reason: "mathematics is not a compared inherited command".into(),
                        },
                    ));
                }
                write!(body, "\\({latex}\\)").expect("string write");
            }
            plan::Inline::BibliographyCitation { kind, entries, .. } => {
                let command = match kind {
                    ftoo_manifest::BibliographyCitationKind::Parenthetical => "parencite",
                    ftoo_manifest::BibliographyCitationKind::Textual => "textcite",
                };
                write!(
                    body,
                    "\\{command}{{{}}}",
                    entries
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .expect("string write");
            }
            plan::Inline::Styled {
                kind: plan::StyleKind::Emphasis,
                children,
                ..
            } if matches!(children.as_slice(), [plan::Inline::Text(_)]) => {
                let plan::Inline::Text(value) = &children[0] else {
                    unreachable!()
                };
                if adjacent_dash_marks(value) {
                    return Err(refuse("adjacent hyphen or em-dash marks"));
                }
                let emitted = latex::prose_text(value).replace('—', "---");
                if emitted
                    .chars()
                    .any(|c| matches!(c, '{' | '}' | '\\' | '\n' | '\r'))
                {
                    return Err(refuse("emphasis text needs multiline or escaped TeX"));
                }
                write!(body, "\\emph{{{emitted}}}").expect("string write");
            }
            _ => return Err(refuse("paragraph contains a non-text run")),
        }
    }
    let mut output = String::new();
    writeln!(output, "\\PaperNativeParagraph{{{}}}{{{body}}}", block.unit).expect("string write");
    Ok(output)
}

fn adjacent_dash_marks(value: &str) -> bool {
    let mut marks = 0;
    for scalar in value.chars() {
        if matches!(scalar, '-' | '—') {
            marks += 1;
            if marks > 1 {
                return true;
            }
        } else {
            marks = 0;
        }
    }
    false
}

fn fragment_text(block: &plan::Block) -> Result<&str, DocumentError> {
    let ([plan::Inline::Text(text)], true) = (block.runs.as_slice(), block.displays.is_empty())
    else {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(block.site.clone()),
            AssemblyFault::UnknownToken {
                value: block.unit.clone(),
                expected: vec!["plain refrain text".into()],
            },
        ));
    };
    let emitted = latex::prose_text(text);
    if emitted.contains('{') || emitted.contains('}') {
        return Err(failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Source(block.site.clone()),
            AssemblyFault::UnknownToken {
                value: block.unit.clone(),
                expected: vec!["brace-free plain refrain text".into()],
            },
        ));
    }
    Ok(text)
}

fn document_sites(corpus: &ftoo_manifest::LanguageCheckedUnits) -> BTreeMap<String, UnitSite> {
    corpus
        .documents()
        .keys()
        .map(|d| {
            (
                d.clone(),
                UnitSite {
                    file: "units/documents.toml".into(),
                    path: vec![
                        ftoo_manifest::PathPart::Key("documents".into()),
                        ftoo_manifest::PathPart::Key(d.clone()),
                    ],
                    anchor: SourceAnchor::Unavailable,
                    decoded: None,
                    occurrence: None,
                },
            )
        })
        .collect()
}

fn consume_sweep(
    support: &NativeSupport,
    lock: &native_support::Lock,
    notation: &crate::RenderableNotation<'_, '_, '_>,
    addressing: &str,
) -> Result<(), DocumentError> {
    let mut sweep = latex::start("assembly-sweep", "en");
    let mut rows = Vec::new();
    // The native sweep typesets generated notation and the inherited commands
    // supplied by compared native support; readout awaits its math alphabet.
    for row in notation
        .sweep_rows()
        .iter()
        .filter(|row| native::sweep_declared(notation.catalogue(), row.owner()))
    {
        let mut origin = native::sweep_row(&mut sweep, row.owner(), row.case(), row.macro_source());
        let explanation = pending::sweep(row.pending_entries());
        if !plan::check_sweep_pending(&explanation, row.pending_entries()) {
            let record = |value| EmissionRecord {
                kind: EmissionKind::Paragraph,
                unit: String::new(),
                language: "en".into(),
                target: DocumentTarget::Latex,
                occurrence: None,
                value,
                site: origin.origin.site.clone(),
            };
            let expected = row
                .pending_entries()
                .iter()
                .map(|entry| {
                    record(format!(
                        "{}: {}",
                        entry.body_macro(),
                        entry
                            .reasons()
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("; ")
                    ))
                })
                .collect();
            return Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: "assembly-sweep.tex".into(),
                    bytes: sweep.len()..sweep.len() + explanation.len(),
                    origins: vec![origin.origin.clone()],
                },
                AssemblyFault::EmissionMismatch {
                    missing: expected,
                    unexpected: vec![record(explanation)],
                    changed: vec![],
                },
            ));
        }
        sweep.push_str(&explanation);
        origin.bytes.end = sweep.len();
        origin.origin.site.anchor = SourceAnchor::Bytes(origin.bytes.start, origin.bytes.end);
        rows.push(origin);
    }
    let cases = sweep
        .lines()
        .filter_map(|line| line.strip_prefix("% sweep "))
        .filter_map(|record| record.split_once(' '))
        .map(|(owner, case)| NativeCase::Sweep {
            owner: notation
                .sweep_rows()
                .iter()
                .find(|row| format!("{:?}", row.owner()) == owner)
                .map_or(
                    ftoo_manifest::RenderOwner::Expression,
                    crate::SweepRow::owner,
                ),
            case: case.into(),
        })
        .collect::<Vec<_>>();
    native::sweep_inventory(notation.catalogue(), &cases)?;
    sweep.push_str("\\end{document}\n");
    native::consume(
        support,
        lock,
        &sweep,
        notation.declarations(),
        addressing,
        NativeCase::Sweep {
            owner: ftoo_manifest::RenderOwner::Expression,
            case: "complete".into(),
        },
        rows.iter().map(|row| row.origin.clone()).collect(),
    )
    .map_err(|error| native::sweep_origin(error, &sweep, &rows))?;
    Ok(())
}
