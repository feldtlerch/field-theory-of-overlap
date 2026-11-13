use crate::{
    Awaiting, ClaimAudit, ENGINE_PIN, Edge, ManifestDraft, NodeId, NodeKind, NodePayload,
    RelationKind, ValidatedManifest, adapter_implemented,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

/// Permanent numbers in the manifest rule register.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuleId {
    M001,
    M002,
    M003,
    M004,
    M005,
    M006,
    M007,
    M008,
    M009,
    M010,
    M011,
    M012,
    M013,
    M014,
    M015,
    M016,
    M017,
    M018,
    M019,
    M020,
    M021,
    M022,
    M023,
    M024,
    M025,
    M026,
    M027,
    M028,
    M029,
    M030,
    M031,
    M032,
    M033,
    M034,
    M035,
    M036,
    M037,
    M038,
    M039,
    M040,
    M041,
    M042,
    M043,
    M044,
    M045,
    M046,
    M047,
    M048,
    M049,
    M050,
    M051,
    M052,
    M053,
    M054,
    M055,
    M056,
    M057,
    M058,
    M059,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleState {
    Structural,
    Enforced,
    DeferredM2,
    DeferredExclusiveRoutes,
}

/// Scope and deferral are part of the rule, not inferred from an empty domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleSpec {
    pub number: RuleId,
    pub statement: &'static str,
    pub scope: &'static str,
    pub state: RuleState,
    pub deferred_reason: Option<&'static str>,
}

pub const RULES: [RuleSpec; 59] = [
    RuleSpec {
        number: RuleId::M001,
        statement: "Input is valid UTF-8 without a BOM.",
        scope: "Every byte load.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M002,
        statement: "Decoded object keys are unique and input contains exactly one document.",
        scope: "Every object at every depth.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M003,
        statement: "Numeric tokens are integers and the wire schema is exactly integer 1.",
        scope: "Every numeric token at the byte boundary.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M004,
        statement: "Wire fields have exact required shapes and exclusive variant fields.",
        scope: "Every wire object and scalar.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M005,
        statement: "Identifiers, kinds, relations, guards, status, binding, capabilities, budgets, verdict classifications and Unconditional routes are closed; payload determines kind.",
        scope: "Every admitted Rust value.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M006,
        statement: "Node IDs are unique, payload kinds match identity, and descriptions are nonblank.",
        scope: "Every draft node.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M007,
        statement: "Every edge endpoint and coverage member resolves to a present node.",
        scope: "All references regardless of closure flags.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M008,
        statement: "Endpoint kinds satisfy the relation's own permitted pair table.",
        scope: "Edges with resolved, valid nodes.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M009,
        statement: "Edge tuples are unique and self edges obey the relation policy.",
        scope: "Every directed (relation, source, target, guard) record.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M010,
        statement: "Each acyclic relation has no cycle in its entire subgraph.",
        scope: "Edges passing endpoint and self-edge checks, including nodes outside the slice; multiplicity does not affect connectivity.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M011,
        statement: "Only maturity is authored; base status derives from identity/payload and AuditedStatus only through private checked-report construction.",
        scope: "Base and audited status APIs.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M012,
        statement: "Assertions equal their surface's derivation: unaudited projection or replay-checked audited report.",
        scope: "Every loaded status assertion.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M013,
        statement: "Every declared slice member is unique and is a claim; M007 resolves membership before this shape check.",
        scope: "Resolved members; shape precedes eligibility.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M014,
        statement: "GraphAdmission requires each slice claim Bound; RegistryAdmission requires transitive claim premises selected; ReportConsumption requires reached IDs equal the declared slice.",
        scope: "GraphAdmission, RegistryAdmission and ReportConsumption.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M015,
        statement: "Audit closure true requires every claim Bound to its implementation; ExplicitPartial and Complete satisfy the stated report predicates, including nonvacuity.",
        scope: "Every manifest, including an empty graph.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M016,
        statement: "Shared weighting compares paths of the same kind only.",
        scope: "Holding and covector payloads carrying weighting paths.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit typed weighting paths and their consumer."),
    },
    RuleSpec {
        number: RuleId::M017,
        statement: "Reads-through is required only for field-anchored readings.",
        scope: "Field readings, not latent readings.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit anchored readings and route checking."),
    },
    RuleSpec {
        number: RuleId::M018,
        statement: "Converters satisfy their declared input, conversion and output.",
        scope: "Converter declarations, not inferred derivation contracts.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit converters and their checking consumer."),
    },
    RuleSpec {
        number: RuleId::M019,
        statement: "Indexed-space families and operations satisfy family-level typing at their node index.",
        scope: "Every admitted indexed family and operation.",
        state: RuleState::DeferredM2,
        deferred_reason: Some("M2 must admit indexed spaces and operation typing."),
    },
    RuleSpec {
        number: RuleId::M020,
        statement: "Every Bound claim has exactly its designated implementation and every implementation exactly one present Bound claim; admitted contracts pass their numbered consumers.",
        scope: "Bound claims and the executing registry.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M021,
        statement: "Every symmetric edge has its reverse with the same relation and guard.",
        scope: "Edges passing M007, M008 and self-edge policy; compatibility only.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M022,
        statement: "Fixed producers have exactly their declared dependencies; Certified premises unlock execution and target witnesses contain full coefficient dependencies.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M023,
        statement: "Every Bound claim has its designated reviewed budget; all records are used, pin/profile agree and claim work shares one budget.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M024,
        statement: "Supported means adapter-implemented at this pin solely from the table; unsupported requirements never execute and Awaiting supported contracts is refused.",
        scope: "Registry and execution consumers.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M025,
        statement: "Exactly one outcome exists per claim, none for quantities; claim, producer, revision, pin, budget and semantic snapshot match.",
        scope: "Report domain and semantic provenance.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M026,
        statement: "Certified/Refuted consume matching replay-checked evidence; only typed limits yield exhaustion and invalid construction never refutes.",
        scope: "Private receipt construction and verdict interpretation.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M027,
        statement: "Each execution has a fresh store and one budget; gated evidence is independent of measurements and previous caches.",
        scope: "Fresh execution and complete gated report reproducibility.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M028,
        statement: "Every unselected or blocked claim has its exact derived deferral; uncertified prerequisites cannot promote dependents.",
        scope: "Total report blockers and Certified-premise scheduling.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M029,
        statement: "Alternative routes execute alone and combine by a reviewed rule.",
        scope: "Registry and execution consumers.",
        state: RuleState::DeferredExclusiveRoutes,
        deferred_reason: Some("Exclusive routes await their first claim and isolation consumer."),
    },
    RuleSpec {
        number: RuleId::M030,
        statement: "Tag targets and spellings are fixed; no lifecycle operation can rebind or rename a tag or reactivate a retired tag.",
        scope: "Every public addressing value and operation.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M031,
        statement: "Tag spellings are nonempty, contain no whitespace or control character, and are unique by exact UTF-8 bytes.",
        scope: "CatalogAdmission over all descriptors including retired handles.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M032,
        statement: "History admits every permanent identity and tag exactly once, in a contiguous event order, with identity admission before tag admission.",
        scope: "HistoryAdmission over the whole ledger, not just present nodes.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M033,
        statement: "History records carry nonblank reasons, valid authority-record labels and nonblank original identity formulations.",
        scope: "HistoryAdmission for every event's metadata.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M034,
        statement: "Retirement is terminal, follows admission, and a retired identity cannot remain in the addressed graph.",
        scope: "HistoryAdmission and StoreConstruction.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M035,
        statement: "The live, retired and out-of-slice stores partition every admitted tag according to retirement and declared-slice dependency support.",
        scope: "StoreConstruction over the full catalog.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M036,
        statement: "Every permanent identity has a tag, and resolution returns its bound NodeId only for a live exact handle, otherwise its exact named refusal.",
        scope: "Completed catalog coverage and the resolution consumer.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M037,
        statement: "A projection preserves every graph record and fixed authority assertion, including complete budgets, bindings, descriptors, history and store event references.",
        scope: "Projection loading of fixed assertions and generation/check semantic comparison against Rust authority.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M038,
        statement: "Generation and check share one canonical writer, and a generated projection must equal its regenerated bytes.",
        scope: "The generated manifest artifact and its shared generation/check path.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M039,
        statement: "Unit metadata carries a well-formed label unique within the admitted unit corpus and references a declared document language contract; document names obey the admitted grammar.",
        scope: "Statement-unit metadata and document declaration names in one corpus admission.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M040,
        statement: "Documents declare nonempty distinct language sets, a main document is declared and requires English alone, and each unit has exactly one nonblank authored text per required language and no text for an undeclared language; language tokens obey the admitted grammar.",
        scope: "Document language declarations and all associated statement-unit texts.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M041,
        statement: "Statement texts use the admitted insertion syntax and reference only their unit's shared nonblank TeX math entries, each of which is used; ordinary prose and math entries use only the admitted characters and fragment syntax, and math names obey the local-name grammar.",
        scope: "Every authored statement text, shared math entry and local math insertion.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M042,
        statement: "Every tag-facing statement-text occurrence resolves through the supplied AddressBook's live resolver or refuses admission with its unit, language and site.",
        scope: "All tag insertions in every admitted language text.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M043,
        statement: "Every label has one mint across the permanent Rust identity catalog, Markdown mint occurrences and admitted unit metadata; Markdown citations resolve within the Markdown mint domain.",
        scope: "The complete supplied Markdown corpus, every permanent NodeId and every admitted unit metadata mint.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M044,
        statement: "Every Rust-held authority-record citation resolves to exactly one Markdown mint.",
        scope: "Every history event in the supplied resolved corpus's AddressBook, including admission and retirement records.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M045,
        statement: "Unit texts never mint labels; bare label spans cite uniquely minted labels and doubled spans mention without harvesting or resolution.",
        scope: "Every admitted statement text in every required language, with citations resolved in the joint label domain.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M046,
        statement: "Within each statement unit, every language has equal addressed tag sets and equal cited label sets, compared separately from the shared metadata mint and independently of position or multiplicity.",
        scope: "All language texts of each resolved unit after joint label and citation admission.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M047,
        statement: "Notation expressions use only the closed registry, exact signatures, bounded literal grammar and scoped constructor parameters, and every formal is used, within declared notation limits.",
        scope: "Every primitive descriptor, definition body and submitted notation expression.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M048,
        statement: "Definitions have their kind's signature shape and any backing equals their object; stems obey the reserved alphabetic grammar and generated names are unique; every permanent object has exactly one canonical symbol; aliases preserve that object and terminate at its canonical symbol; all definition dependencies are acyclic.",
        scope: "The complete notation definition catalogue, including absent objects and unused definitions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M049,
        statement: "Symbol tiers are derived from the supplied graph and transitive symbol uses; an alias takes the lesser of its own binding tier and its target tier, retaining every pending reason.",
        scope: "Checked notation catalogues and all exposed symbol-status values.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M050,
        statement: "Generated notation declarations refuse collisions in the consuming typesetting context.",
        scope: "Every emitted notation declaration and its checked KaTeX context.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M051,
        statement: "Both notation forms derive from the same checked AST with explicit token boundaries and within the output limit; full Unicode expansion contains no project commands and every returned expression passes the pinned KaTeX grammar.",
        scope: "Generated declaration bodies, macro invocations and fully expanded notation expressions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M052,
        statement: "A generated sweep typesets every emitted notation command with its declared arguments and checks its fully expanded counterpart.",
        scope: "The complete emitted command inventory, including aliases, constructors and pending symbols.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M053,
        statement: "Every shared math source parses completely into the closed notation AST within the assembly math bounds, resolves every command with its declared syntax, and passes both rendering consumers with all authored use sites retained.",
        scope: "All math entries of every welded unit consumed by assembly.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M054,
        statement: "Every declared document has one ordered skeleton, every skeleton names a declared document, and every admitted unit occurs exactly once in its associated document's skeleton within the assembly bounds.",
        scope: "The complete skeleton and admitted unit corpora, including heading positions.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M055,
        statement: "Document emission preserves the checked block and occurrence inventory, uses the matching notation and addressing context, refuses every unavailable runtime tag at its site, and generated documents contribute no Markdown mint or citation. Generated coverage and pending object names require a unique live tag.",
        scope: "Both document targets and their generated notation and addressing support.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M056,
        statement: "Every generated LaTeX document and the complete derived notation sweep pass the pinned native document context with checked dependency integrity.",
        scope: "Prepared LaTeX documents, their actual preamble and support files, and every native sweep case.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M057,
        statement: "Document generation and check share one preparation and writer, and the complete generated document and support inventory equals its regenerated bytes.",
        scope: "Generated Markdown and LaTeX documents, notation.tex and addressing.tex.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M058,
        statement: "Every display has one existing unit owner, exactly one admitted body per target and required language, every body occurrence resolves under that unit's checked context, and a display unit occurs only as a skeleton leaf.",
        scope: "All display companion records and every cell or figure-label occurrence.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M059,
        statement: "Within each display, addressed tag sets and cited label sets are separately equal across all target and language bodies, retaining every occurrence independently of order and multiplicity.",
        scope: "All display bodies after complete body reference resolution.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
];

/// Original draft positions, retained separately from semantic sorting.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DraftLocation {
    Node(usize),
    Edge(usize),
    Slice(usize),
    Coverage,
    Binding(usize),
    Budget(usize),
    Capability(usize),
    Verdict(usize),
    Provenance,
    Cycle {
        relation: RelationKind,
        edges: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub stage: AdmissionStage,
    pub rule: RuleId,
    /// Edge-scoped IDs retain source/target order; other IDs use semantic order,
    /// except cycles which retain their concrete traversal path order.
    pub ids: Vec<NodeId>,
    pub edge: Option<Edge>,
    pub location: DraftLocation,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors {
    diagnostics: Vec<Diagnostic>,
}
impl ValidationErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(
                f,
                "{:?} at {:?}: {}",
                diagnostic.rule, diagnostic.location, diagnostic.explanation
            )?;
        }
        Ok(())
    }
}
impl Error for ValidationErrors {}

fn report(
    diagnostics: &mut Vec<Diagnostic>,
    rule: RuleId,
    ids: Vec<NodeId>,
    edge: Option<Edge>,
    location: DraftLocation,
    explanation: String,
) {
    diagnostics.push(Diagnostic {
        stage: AdmissionStage::GraphAdmission,
        rule,
        ids,
        edge,
        location,
        explanation,
    });
}

/// Validate in dependency stages without inventing kinds for missing endpoints.
/// Node admission precedes resolution; endpoint contracts, tuple/self checks and
/// slice shape precede symmetry, cycles and eligibility. M021 depends on M008:
/// an invalid endpoint suppresses symmetry for that edge. Independent defects
/// accumulate; all diagnostics sort by rule, semantic tuple/identity, location.
///
/// # Errors
/// Returns deterministic diagnostics for every reached enforced-rule refusal.
pub fn validate(draft: ManifestDraft) -> Result<ValidatedManifest, ValidationErrors> {
    let mut diagnostics = Vec::new();
    let nodes = check_nodes(&draft, &mut diagnostics);
    let edges = check_edges(&draft, &nodes, &mut diagnostics);
    check_coverage(&draft, &nodes, &mut diagnostics);
    check_relations(&edges, &mut diagnostics);
    for (index, node) in draft.nodes.iter().enumerate() {
        if nodes.get(&node.id) == Some(&Some(NodeKind::Claim))
            && let NodePayload::Claim {
                audit: ClaimAudit::Awaiting(Awaiting::Capability(id)),
                ..
            } = node.payload
            && adapter_implemented(id, ENGINE_PIN)
        {
            report(
                &mut diagnostics,
                RuleId::M024,
                vec![node.id],
                None,
                DraftLocation::Capability(index),
                "cannot await an implemented adapter".into(),
            );
        }
    }
    diagnostics.sort_by(|a, b| {
        (a.rule, a.edge, &a.ids, &a.location, &a.explanation).cmp(&(
            b.rule,
            b.edge,
            &b.ids,
            &b.location,
            &b.explanation,
        ))
    });
    if diagnostics.is_empty() {
        Ok(ValidatedManifest::from_checked(draft))
    } else {
        Err(ValidationErrors { diagnostics })
    }
}

// None retains presence while withholding a kind when node admission failed.
fn check_nodes(
    draft: &ManifestDraft,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<NodeId, Option<NodeKind>> {
    let mut counts = BTreeMap::new();
    for node in &draft.nodes {
        *counts.entry(node.id).or_insert(0_usize) += 1;
    }
    let mut nodes = BTreeMap::new();
    for (index, node) in draft.nodes.iter().enumerate() {
        let mut valid = true;
        for (failed, reason) in [
            (counts[&node.id] != 1, "identity occurs more than once"),
            (
                node.id.expected_kind() != node.payload.kind(),
                "payload kind disagrees with identity",
            ),
            (
                node.payload.description().trim().is_empty(),
                "description is blank",
            ),
        ] {
            if failed {
                valid = false;
                report(
                    diagnostics,
                    RuleId::M006,
                    vec![node.id],
                    None,
                    DraftLocation::Node(index),
                    format!("{}: {reason}", node.id.semantic_name()),
                );
            }
        }
        nodes.insert(node.id, valid.then_some(node.payload.kind()));
    }
    nodes
}

fn check_edges(
    draft: &ManifestDraft,
    nodes: &BTreeMap<NodeId, Option<NodeKind>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<Edge, usize> {
    let mut counts = BTreeMap::new();
    for edge in &draft.edges {
        *counts.entry(*edge).or_insert(0_usize) += 1;
    }
    let mut admitted = BTreeMap::new();
    for (index, edge) in draft.edges.iter().enumerate() {
        let location = DraftLocation::Edge(index);
        let mut missing: Vec<_> = [edge.source, edge.target]
            .into_iter()
            .filter(|id| !nodes.contains_key(id))
            .collect();
        missing.sort();
        missing.dedup();
        for id in missing {
            report(
                diagnostics,
                RuleId::M007,
                vec![id],
                Some(*edge),
                location.clone(),
                format!("endpoint {} is absent", id.semantic_name()),
            );
        }
        let self_refused = edge.source == edge.target && !edge.relation.contract().allow_self;
        for (failed, reason) in [
            (counts[edge] != 1, "duplicate edge tuple"),
            (self_refused, "self edge is forbidden"),
        ] {
            if failed {
                report(
                    diagnostics,
                    RuleId::M009,
                    vec![edge.source, edge.target],
                    Some(*edge),
                    location.clone(),
                    reason.into(),
                );
            }
        }
        if let (Some(Some(source)), Some(Some(target))) =
            (nodes.get(&edge.source), nodes.get(&edge.target))
        {
            if !edge
                .relation
                .contract()
                .endpoints
                .contains(&(*source, *target))
            {
                report(
                    diagnostics,
                    RuleId::M008,
                    vec![edge.source, edge.target],
                    Some(*edge),
                    location,
                    format!(
                        "{} does not admit {source:?} -> {target:?}",
                        edge.relation.name()
                    ),
                );
            } else if !self_refused {
                // Multiplicity is already refused; a unique semantic projection
                // still permits independent symmetry and cycle checks.
                admitted.entry(*edge).or_insert(index);
            }
        }
    }
    admitted
}

fn check_coverage(
    draft: &ManifestDraft,
    nodes: &BTreeMap<NodeId, Option<NodeKind>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut counts = BTreeMap::new();
    for id in &draft.coverage.slice {
        *counts.entry(*id).or_insert(0_usize) += 1;
    }
    for (index, id) in draft.coverage.slice.iter().enumerate() {
        let location = DraftLocation::Slice(index);
        let Some(kind) = nodes.get(id) else {
            report(
                diagnostics,
                RuleId::M007,
                vec![*id],
                None,
                location,
                format!("slice member {} is absent", id.semantic_name()),
            );
            continue;
        };
        let Some(kind) = kind else {
            continue;
        };
        if counts[id] != 1 || *kind != NodeKind::Claim {
            report(
                diagnostics,
                RuleId::M013,
                vec![*id],
                None,
                location,
                format!("{} must occur once and be a claim", id.semantic_name()),
            );
        } else if draft.nodes.iter().any(|n| {
            n.id == *id
                && matches!(
                    n.payload,
                    NodePayload::Claim {
                        audit: ClaimAudit::Awaiting(_),
                        ..
                    }
                )
        }) {
            report(
                diagnostics,
                RuleId::M014,
                vec![*id],
                None,
                location,
                format!(
                    "{} awaits AuditBinding and is not audit-eligible",
                    id.semantic_name()
                ),
            );
        }
    }
    if draft.coverage.closed_world.audit
        && draft.nodes.iter().any(|n| {
            nodes.get(&n.id) == Some(&Some(NodeKind::Claim))
                && matches!(
                    n.payload,
                    NodePayload::Claim {
                        audit: ClaimAudit::Awaiting(_),
                        ..
                    }
                )
        })
    {
        report(
            diagnostics,
            RuleId::M015,
            vec![],
            None,
            DraftLocation::Coverage,
            "audit closure requires every present claim Bound".into(),
        );
    }
}

fn check_relations(edges: &BTreeMap<Edge, usize>, diagnostics: &mut Vec<Diagnostic>) {
    for (edge, index) in edges {
        if edge.relation.contract().symmetric && !edges.contains_key(&edge.reverse()) {
            report(
                diagnostics,
                RuleId::M021,
                vec![edge.source, edge.target],
                Some(*edge),
                DraftLocation::Edge(*index),
                format!(
                    "missing reverse {}: {} -> {} with {:?}",
                    edge.relation.name(),
                    edge.target.semantic_name(),
                    edge.source.semantic_name(),
                    edge.guard
                ),
            );
        }
    }
    for relation in RelationKind::ALL {
        if !relation.contract().acyclic {
            continue;
        }
        let mut adjacency: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
        for edge in edges.keys().filter(|edge| edge.relation == relation) {
            adjacency
                .entry(edge.source)
                .or_default()
                .insert(edge.target);
        }
        // Search roots in semantic order. The first successful root is the
        // smallest member of any concrete cycle; successor order is also fixed.
        for start in adjacency.keys() {
            let mut path = vec![*start];
            if cycle_from(*start, *start, &adjacency, &mut path) {
                let positions = path
                    .windows(2)
                    .filter_map(|pair| {
                        edges
                            .iter()
                            .find(|(edge, _)| {
                                edge.relation == relation
                                    && edge.source == pair[0]
                                    && edge.target == pair[1]
                            })
                            .map(|(_, index)| *index)
                    })
                    .collect();
                let names: Vec<_> = path.iter().map(|id| id.semantic_name()).collect();
                report(
                    diagnostics,
                    RuleId::M010,
                    path,
                    None,
                    DraftLocation::Cycle {
                        relation,
                        edges: positions,
                    },
                    format!("{} cycle: {}", relation.name(), names.join(" -> ")),
                );
                break;
            }
        }
    }
}

fn cycle_from(
    start: NodeId,
    current: NodeId,
    adjacency: &BTreeMap<NodeId, BTreeSet<NodeId>>,
    path: &mut Vec<NodeId>,
) -> bool {
    if let Some(successors) = adjacency.get(&current) {
        for next in successors {
            if *next == start {
                path.push(start);
                return true;
            }
            if !path.contains(next) {
                path.push(*next);
                if cycle_from(start, *next, adjacency, path) {
                    return true;
                }
                path.pop();
            }
        }
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionStage {
    GraphAdmission,
    RegistryAdmission,
    ReportConsumption,
}
