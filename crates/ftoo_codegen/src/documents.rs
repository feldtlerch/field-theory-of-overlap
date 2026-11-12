//! Complete statement-document preparation against one retained source snapshot.
#![allow(clippy::result_large_err)]
mod artifacts;
mod diagnostics;
mod display;
mod input;
mod latex;
mod lock_scan;
mod markdown;
mod native;
mod native_support;
mod plan;
mod runtime;
#[cfg(test)]
mod tests;
pub use artifacts::{check_documents, generate_documents};
pub use diagnostics::*;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, DocumentTarget, RuleId, SourceAnchor, SourceOrigin, UnitSite,
};
pub use native_support::NativeSupport;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentArtifact {
    path: String,
    bytes: Vec<u8>,
    origins: Vec<SourceOrigin>,
    emissions: Vec<EmissionRecord>,
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
    artifacts: Vec<DocumentArtifact>,
    inputs: BTreeMap<String, Vec<u8>>,
    observed_outputs: BTreeMap<String, Vec<u8>>,
}
impl PreparedDocuments {
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
    let _preparation = crate::telemetry::Phase::new("documents.preparation");
    let admission = crate::telemetry::Phase::new("documents.source_admission");
    let support = NativeSupport::resolve(root, Some(support.directory()))?;
    let projection = input::read(root, "generated/manifest.json")?;
    crate::check_projection(root).map_err(DocumentError::Projection)?;
    let mut inputs = BTreeMap::from([("generated/manifest.json".into(), projection.clone())]);
    for file in [
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
    let resolved = ftoo_manifest::resolve_unit_tags(&corpus, &book)
        .map_err(|e| DocumentError::Units(crate::UnitLoadError::Admission(e)))?;
    let markdown = crate::load_markdown_files(root).map_err(DocumentError::Labels)?;
    neutral(&markdown, true, AssemblyStage::Source)?;
    let integrated = ftoo_manifest::integrate_labels(&resolved, &markdown)
        .map_err(|e| DocumentError::Labels(crate::LabelFileError::Labels(e)))?;
    let welded = ftoo_manifest::weld_units(&integrated).map_err(DocumentError::Weld)?;
    let skeleton_files = input::discover(root, "units/skeletons", false)?;
    let display_files = input::discover(root, "units/displays", true)?;
    let sources = input::sources(root, &skeleton_files)?;
    let display_sources = input::sources(root, &display_files)?;
    let (drafts, display_drafts) = input::assembly_sources(&sources, &display_sources)?;
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
    drop(admission);
    crate::notation::with_preparation(&catalogue, |notation| {
        let plans = plan::build(&skeletons, &welded, notation, &metadata, &displays)?;
        let artifacts = render_artifacts(root, &support, &book, &plans, notation)?;
        finish_snapshot(
            root,
            inputs,
            &markdown,
            std::iter::once(declaration)
                .chain(statements)
                .chain(sources)
                .chain(display_sources),
            [
                ("units/statements", statement_files, false),
                ("units/skeletons", skeleton_files, false),
                ("units/displays", display_files, true),
            ],
            artifacts,
        )
    })
}
fn finish_snapshot(
    root: &Path,
    mut inputs: BTreeMap<String, Vec<u8>>,
    markdown: &ftoo_manifest::CheckedMarkdown,
    sources: impl Iterator<Item = crate::UnitSource>,
    inventories: [(&str, Vec<String>, bool); 3],
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
        artifacts,
        inputs,
        observed_outputs,
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
        let unit = crate::units::toml::fixed(fields[1], &["id", "document", "math", "texts"])
            .expect("admitted fields");
        let id = crate::units::toml::string(unit[0]).expect("admitted id");
        owners.insert(id.value.clone(), unit[1].site.clone());
        metadata.insert(id.value, id.site);
    }
    (metadata, owners)
}

fn render_artifacts(
    root: &Path,
    support: &NativeSupport,
    book: &ftoo_manifest::AddressBook<'_>,
    plans: &[plan::DocumentPlan],
    notation: &crate::RenderableNotation<'_, '_, '_>,
) -> Result<Vec<DocumentArtifact>, DocumentError> {
    let addressing = runtime::render(book, plans)?;
    let lock = native_support::lock(root)?;
    support.scope(&lock, || {
        let mut artifacts = Vec::new();
        for plan in plans {
            document_size(plan)?;
            let md = markdown::render(plan);
            let tex = latex::render(plan);
            plan::check_emission(plan, &md, DocumentTarget::Markdown)?;
            plan::check_emission(plan, &tex, DocumentTarget::Latex)?;
            let md_path = format!("generated/documents/{}/{}.md", plan.document, plan.language);
            let checked =
                ftoo_manifest::check_markdown_sources(vec![ftoo_manifest::MarkdownSource {
                    file: md_path.clone(),
                    bytes: md.as_bytes().to_vec(),
                }])
                .map_err(|e| DocumentError::Labels(crate::LabelFileError::Source(e)))?;
            neutral(&checked, false, AssemblyStage::Emission)?;
            native::consume_plan(
                support,
                &lock,
                &tex,
                notation.declarations(),
                &addressing,
                plan,
            )?;
            artifacts.push(DocumentArtifact {
                path: md_path,
                bytes: md.into_bytes(),
                origins: plan.origins(DocumentTarget::Markdown),
                emissions: plan.records(DocumentTarget::Markdown),
            });
            artifacts.push(DocumentArtifact {
                path: format!(
                    "generated/documents/{}/{}.tex",
                    plan.document, plan.language
                ),
                bytes: tex.into_bytes(),
                origins: plan.origins(DocumentTarget::Latex),
                emissions: plan.records(DocumentTarget::Latex),
            });
        }
        consume_sweep(support, &lock, notation, &addressing)?;
        artifacts.push(DocumentArtifact {
            path: "generated/notation.tex".into(),
            bytes: notation.declarations().as_bytes().to_vec(),
            origins: Vec::new(),
            emissions: Vec::new(),
        });
        artifacts.push(DocumentArtifact {
            path: "generated/addressing.tex".into(),
            bytes: addressing.into_bytes(),
            origins: Vec::new(),
            emissions: Vec::new(),
        });
        for artifact in &artifacts {
            if artifact.bytes.len() > 16_777_216 {
                return Err(failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    AssemblySite::Generated {
                        path: artifact.path.clone(),
                        bytes: 16_777_216..16_777_217,
                        origins: artifact.origins.clone(),
                    },
                    AssemblyFault::Limit {
                        resource: ftoo_manifest::AssemblyResource::DocumentBytes,
                        limit: 16_777_216,
                        actual: 16_777_217,
                    },
                ));
            }
        }
        Ok(artifacts)
    })
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
    for row in notation.sweep_rows() {
        rows.push(native::sweep_row(
            &mut sweep,
            row.owner(),
            row.case(),
            row.macro_source(),
        ));
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

fn document_size(plan: &plan::DocumentPlan) -> Result<(), DocumentError> {
    for (target, size) in [
        (DocumentTarget::Markdown, markdown::size(plan)),
        (DocumentTarget::Latex, latex::size(plan)),
    ] {
        if size > 16_777_216 {
            return Err(failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: format!(
                        "generated/documents/{}/{}.{}",
                        plan.document,
                        plan.language,
                        if target == DocumentTarget::Markdown {
                            "md"
                        } else {
                            "tex"
                        }
                    ),
                    bytes: 16_777_216..16_777_217,
                    origins: plan.origins(target),
                },
                AssemblyFault::Limit {
                    resource: ftoo_manifest::AssemblyResource::DocumentBytes,
                    limit: 16_777_216,
                    actual: 16_777_217,
                },
            ));
        }
    }
    Ok(())
}
