use crate::{Edge, ManifestDraft, NodeId, NodeKind, RelationKind, ValidatedManifest};
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuleState {
    Structural,
    Enforced,
    DeferredM1b,
    DeferredM2,
    DeferredM3,
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

pub const RULES: [RuleSpec; 21] = [
    RuleSpec {
        number: RuleId::M001,
        statement: "Input is valid UTF-8 without a BOM.",
        scope: "Every byte load.",
        state: RuleState::DeferredM1b,
        deferred_reason: Some("The serialized boundary enters with its M1b projection consumer."),
    },
    RuleSpec {
        number: RuleId::M002,
        statement: "Decoded object keys are unique and input contains exactly one document.",
        scope: "Every object at every depth.",
        state: RuleState::DeferredM1b,
        deferred_reason: Some("No byte parser is admitted before M1b."),
    },
    RuleSpec {
        number: RuleId::M003,
        statement: "Numeric tokens are integers and the wire schema is exactly integer 1.",
        scope: "Every numeric token at the byte boundary.",
        state: RuleState::DeferredM1b,
        deferred_reason: Some("The wire schema is deferred to M1b."),
    },
    RuleSpec {
        number: RuleId::M004,
        statement: "Wire fields have exact required shapes and exclusive variant fields.",
        scope: "Every wire object and scalar.",
        state: RuleState::DeferredM1b,
        deferred_reason: Some("Typed wire admission is deferred to M1b."),
    },
    RuleSpec {
        number: RuleId::M005,
        statement: "Identifiers, kinds, relations, guards and status values are closed; payload determines kind.",
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
        statement: "Only maturity is authored; other status axes derive from admitted identity and payload.",
        scope: "Every validated node.",
        state: RuleState::Structural,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M012,
        statement: "Loaded status assertions equal derivation and claims name the admitted blocker.",
        scope: "Every loaded status assertion.",
        state: RuleState::DeferredM1b,
        deferred_reason: Some("Assertions enter only through the M1b wire boundary."),
    },
    RuleSpec {
        number: RuleId::M013,
        statement: "Slice members are unique claims.",
        scope: "Resolved members; shape precedes eligibility.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M014,
        statement: "Every slice claim is reached by implemented manifest audit binding.",
        scope: "Members passing presence and shape; AuditBinding is ineligible.",
        state: RuleState::Enforced,
        deferred_reason: None,
    },
    RuleSpec {
        number: RuleId::M015,
        statement: "All completeness assertions are false and release is explicit partial.",
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
        statement: "Claims and producers bind exclusively in both directions; routes, budgets, readiness and evidence are checked.",
        scope: "Bound claims, producers and consumed verdicts.",
        state: RuleState::DeferredM3,
        deferred_reason: Some("M3 must admit audit binding and the complete-release consumer."),
    },
    RuleSpec {
        number: RuleId::M021,
        statement: "Every symmetric edge has its reverse with the same relation and guard.",
        scope: "Edges passing M007, M008 and self-edge policy; compatibility only.",
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
    Cycle {
        relation: RelationKind,
        edges: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
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
        } else {
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
    if !draft.coverage.has_partial_assertions() {
        report(diagnostics, RuleId::M015, vec![], None, DraftLocation::Coverage,
            "referents, relations and audit closure must all be false; release gate must be ExplicitPartial".into());
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
