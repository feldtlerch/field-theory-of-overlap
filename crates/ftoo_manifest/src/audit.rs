//! Closed engine-independent contracts; descriptions never supply mathematics.
use crate::{BudgetId, NodeId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnginePin(pub &'static str);
pub const ENGINE_PIN: EnginePin = EnginePin("598ff683a57caca07cf9d44e3fd28c716afb81ef");
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityId {
    ExactRationalLinearEquivalence,
    FormalTruncatedPowerSeries,
    SymbolicTrigonometry,
    TrigonometricIntegration,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineAvailability {
    Present,
    AbsentWithEvidence,
    NotEstablished,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilitySpec {
    pub id: CapabilityId,
    pub availability: EngineAvailability,
    pub adapter_implemented: bool,
    pub evidence: &'static str,
}
pub const CAPABILITIES: [CapabilitySpec; 4] = [
    CapabilitySpec {
        id: CapabilityId::ExactRationalLinearEquivalence,
        availability: EngineAvailability::Present,
        adapter_implemented: false,
        evidence: "Accepted trial linear-equivalence and coefficient-identity witnesses; no production linear adapter.",
    },
    CapabilitySpec {
        id: CapabilityId::FormalTruncatedPowerSeries,
        availability: EngineAvailability::Present,
        adapter_implemented: true,
        evidence: "Fixed adapter checks structural Defined on Product, PolynomialSeed, evaluated residual and Member without assumptions. Region partition still rejects Product; no general source or reciprocal adapter is exposed.",
    },
    CapabilitySpec {
        id: CapabilityId::SymbolicTrigonometry,
        availability: EngineAvailability::NotEstablished,
        adapter_implemented: false,
        evidence: "No present-pin evidence in this adapter.",
    },
    CapabilitySpec {
        id: CapabilityId::TrigonometricIntegration,
        availability: EngineAvailability::NotEstablished,
        adapter_implemented: false,
        evidence: "No present-pin evidence in this adapter.",
    },
];
#[must_use]
pub fn adapter_implemented(id: CapabilityId, pin: EnginePin) -> bool {
    pin == ENGINE_PIN
        && CAPABILITIES
            .iter()
            .any(|s| s.id == id && s.adapter_implemented)
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProducerId {
    LangevinCoefficientOneValue,
    LangevinCoefficientThreeValue,
}
/// One context, with no exclusive alternatives.
/// ```compile_fail,E0599
/// let route = ftoo_manifest::AuditRoutes::Exclusive;
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditRoutes {
    Unconditional,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionContract {
    AmbientProductXNoAssumptions,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerDescriptor {
    pub producer: ProducerId,
    pub claim: NodeId,
    pub dependencies: &'static [NodeId],
    pub capabilities: &'static [CapabilityId],
    pub budget: BudgetId,
    pub revision: u32,
    pub target: usize,
    pub region: RegionContract,
}
impl ProducerId {
    pub const ALL: [Self; 2] = [
        Self::LangevinCoefficientOneValue,
        Self::LangevinCoefficientThreeValue,
    ];
    #[must_use]
    pub const fn descriptor(self) -> ProducerDescriptor {
        let (claim, dependencies, budget, target): (_, &'static [NodeId], _, _) = match self {
            Self::LangevinCoefficientOneValue => (
                NodeId::LangevinCoefficientOneValue,
                &[NodeId::LangevinCoefficientOne],
                BudgetId::CoefficientOneV1,
                1,
            ),
            Self::LangevinCoefficientThreeValue => (
                NodeId::LangevinCoefficientThreeValue,
                &[
                    NodeId::LangevinCoefficientThree,
                    NodeId::LangevinCoefficientOneValue,
                ],
                BudgetId::CoefficientThreeV1,
                3,
            ),
        };
        ProducerDescriptor {
            producer: self,
            claim,
            dependencies,
            capabilities: &[CapabilityId::FormalTruncatedPowerSeries],
            budget,
            revision: 2,
            target,
            region: RegionContract::AmbientProductXNoAssumptions,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimAudit {
    Awaiting(crate::Awaiting),
    Bound {
        producer: ProducerId,
        budget: BudgetId,
        routes: AuditRoutes,
    },
}

/// Compare duplicate-preserving implementation and budget records before lookup.
/// This checks admission only; it cannot construct accepted execution evidence.
#[must_use]
pub fn registry_diagnostics(
    graph: &crate::ValidatedManifest,
    roster: &[ProducerDescriptor],
) -> Vec<crate::Diagnostic> {
    use crate::{
        AdmissionStage, BudgetSpec, Diagnostic, DraftLocation, NodePayload, RelationKind, RuleId,
    };
    let mut errors = Vec::new();
    let mut fail = |rule, ids, location, explanation: &str| {
        errors.push(Diagnostic {
            stage: AdmissionStage::RegistryAdmission,
            rule,
            ids,
            edge: None,
            location,
            explanation: explanation.into(),
        });
    };
    for (index, node) in graph.nodes().iter().enumerate() {
        let NodePayload::Claim {
            audit: ClaimAudit::Bound {
                producer, budget, ..
            },
            ..
        } = node.payload
        else {
            continue;
        };
        let expected = producer.descriptor();
        let binding_valid = expected.claim == node.id
            && roster.iter().filter(|d| d.claim == node.id).count() == 1
            && roster.contains(&expected);
        if !binding_valid {
            fail(
                RuleId::M020,
                vec![node.id],
                DraftLocation::Binding(index),
                "Bound claim must have exactly its designated implementation",
            );
            continue;
        }
        let actual: std::collections::BTreeSet<_> = graph
            .edges()
            .iter()
            .filter(|e| e.source == node.id && e.relation == RelationKind::DependsOn)
            .map(|e| e.target)
            .collect();
        if actual != expected.dependencies.iter().copied().collect() {
            fail(
                RuleId::M022,
                vec![node.id],
                DraftLocation::Binding(index),
                "direct dependencies must equal the fixed descriptor",
            );
        }
        if budget != expected.budget
            || graph.budgets().iter().filter(|b| b.id == budget).count() != 1
            || !graph.budgets().contains(&BudgetSpec::reviewed(producer))
        {
            fail(
                RuleId::M023,
                vec![node.id],
                DraftLocation::Binding(index),
                "missing, duplicate, swapped or unreviewed budget",
            );
        }
        if graph.coverage().slice.contains(&node.id) {
            // Requiring direct premise closure for every selected claim gives transitive closure.
            for premise in expected
                .dependencies
                .iter()
                .filter(|id| id.expected_kind() == crate::NodeKind::Claim)
            {
                if !graph.coverage().slice.contains(premise) {
                    fail(
                        RuleId::M014,
                        vec![node.id, *premise],
                        DraftLocation::Binding(index),
                        "selected claim premise is not selected",
                    );
                }
            }
        }
    }
    for (index, descriptor) in roster.iter().enumerate() {
        if *descriptor != descriptor.producer.descriptor() || roster.iter().filter(|d| d.producer == descriptor.producer).count() != 1 || !graph.nodes().iter().any(|n| n.id == descriptor.claim && matches!(n.payload, NodePayload::Claim { audit: ClaimAudit::Bound { producer, .. }, .. } if producer == descriptor.producer)) {
            fail(RuleId::M020, vec![descriptor.claim], DraftLocation::Binding(index), "duplicate, swapped, orphan or Awaiting implementation");
        }
    }
    for (index, budget) in graph.budgets().iter().enumerate() {
        if *budget != BudgetSpec::reviewed(budget.producer) || graph.budgets().iter().filter(|b| b.id == budget.id).count() != 1 || !graph.nodes().iter().any(|n| matches!(n.payload, NodePayload::Claim { audit: ClaimAudit::Bound { producer, budget: id, .. }, .. } if producer == budget.producer && id == budget.id)) {
            fail(RuleId::M023, vec![budget.producer.descriptor().claim], DraftLocation::Budget(index), "budget is unused, duplicated or does not match the reviewed pin/profile");
        }
    }
    errors.sort_by(|a, b| {
        (a.rule, &a.ids, &a.location, &a.explanation).cmp(&(
            b.rule,
            &b.ids,
            &b.location,
            &b.explanation,
        ))
    });
    errors
}

/// Untrusted classifications for consistency checking; never proof tokens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerdictClass {
    Certified,
    Refuted,
    Undecided,
    Deferred,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferredReason {
    AuditBinding,
    Capability(CapabilityId),
    OutsideSlice,
    Premise {
        claim: NodeId,
        evidence: VerdictClass,
    },
}
/// Caller-authored report data. Acceptance by a pure checker does not certify it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeDraft {
    pub claim: NodeId,
    pub producer: Option<ProducerId>,
    pub revision: Option<u32>,
    pub budget: Option<BudgetId>,
    pub pin: EnginePin,
    pub verdict: VerdictClass,
    pub deferred: Option<DeferredReason>,
}
impl OutcomeDraft {
    #[must_use]
    pub fn new(
        node: &crate::NodeDraft,
        verdict: VerdictClass,
        deferred: Option<DeferredReason>,
    ) -> Self {
        let binding = match node.payload {
            crate::NodePayload::Claim {
                audit:
                    ClaimAudit::Bound {
                        producer, budget, ..
                    },
                ..
            } => Some((producer, budget)),
            _ => None,
        };
        Self {
            claim: node.id,
            producer: binding.map(|b| b.0),
            revision: binding.map(|b| b.0.descriptor().revision),
            budget: binding.map(|b| b.1),
            pin: ENGINE_PIN,
            verdict,
            deferred,
        }
    }
}
/// Canonical owned semantic input, including editorial fields and every edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub nodes: Vec<crate::NodeDraft>,
    pub edges: Vec<crate::Edge>,
    pub descriptors: Vec<ProducerDescriptor>,
    pub capabilities: Vec<CapabilitySpec>,
    pub coverage: crate::Coverage,
    pub budgets: Vec<crate::BudgetSpec>,
    pub pin: EnginePin,
}
impl Snapshot {
    #[must_use]
    pub fn capture(graph: &crate::ValidatedManifest) -> Self {
        let mut nodes = graph.nodes().to_vec();
        nodes.sort_by_key(|n| n.id);
        let mut edges = graph.edges().to_vec();
        edges.sort();
        let mut budgets = graph.budgets().to_vec();
        budgets.sort_by_key(|b| b.id);
        let mut coverage = graph.coverage().clone();
        coverage.slice.sort();
        let mut descriptors: Vec<_> = nodes
            .iter()
            .filter_map(|n| match n.payload {
                crate::NodePayload::Claim {
                    audit: ClaimAudit::Bound { producer, .. },
                    ..
                } => Some(producer.descriptor()),
                _ => None,
            })
            .collect();
        descriptors.sort_by_key(|d| d.producer);
        Self {
            nodes,
            edges,
            descriptors,
            capabilities: CAPABILITIES.to_vec(),
            coverage,
            budgets,
            pin: ENGINE_PIN,
        }
    }
}
/// Derive a blocker from authority and existing classifications, never from timing.
#[must_use]
pub fn expected_deferral(
    graph: &crate::ValidatedManifest,
    node: &crate::NodeDraft,
    rows: &[OutcomeDraft],
) -> Option<DeferredReason> {
    let crate::NodePayload::Claim { audit, .. } = node.payload else {
        return None;
    };
    let producer = match audit {
        ClaimAudit::Awaiting(crate::Awaiting::AuditBinding) => {
            return Some(DeferredReason::AuditBinding);
        }
        ClaimAudit::Awaiting(crate::Awaiting::Capability(c)) => {
            return Some(DeferredReason::Capability(c));
        }
        ClaimAudit::Bound { producer, .. } => producer,
    };
    if !graph.coverage().slice.contains(&node.id) {
        return Some(DeferredReason::OutsideSlice);
    }
    let descriptor = producer.descriptor();
    if let Some(capability) = descriptor
        .capabilities
        .iter()
        .find(|c| !adapter_implemented(**c, ENGINE_PIN))
    {
        return Some(DeferredReason::Capability(*capability));
    }
    for premise in descriptor
        .dependencies
        .iter()
        .filter(|id| id.expected_kind() == crate::NodeKind::Claim)
    {
        if let Some(row) = rows.iter().find(|r| r.claim == *premise)
            && row.verdict != VerdictClass::Certified
        {
            return Some(DeferredReason::Premise {
                claim: *premise,
                evidence: row.verdict,
            });
        }
    }
    None
}
pub(crate) fn consumption_diagnostic(
    rule: crate::RuleId,
    ids: Vec<NodeId>,
    location: crate::DraftLocation,
    explanation: &str,
) -> crate::Diagnostic {
    crate::Diagnostic {
        stage: crate::AdmissionStage::ReportConsumption,
        rule,
        ids,
        edge: None,
        location,
        explanation: explanation.into(),
    }
}
/// Check untrusted report structure only; no engine evidence or status is produced.
#[must_use]
pub fn report_diagnostics(
    graph: &crate::ValidatedManifest,
    snapshot: &Snapshot,
    reached: &[NodeId],
    rows: &[OutcomeDraft],
) -> Vec<crate::Diagnostic> {
    use crate::{DraftLocation, NodeKind, RuleId};
    use std::collections::BTreeSet;
    let mut errors = Vec::new();
    if *snapshot != Snapshot::capture(graph) {
        errors.push(consumption_diagnostic(
            RuleId::M025,
            vec![],
            DraftLocation::Provenance,
            "semantic snapshot differs from authority",
        ));
    }
    let selected: BTreeSet<_> = graph.coverage().slice.iter().copied().collect();
    let actual: BTreeSet<_> = reached.iter().copied().collect();
    if selected != actual || actual.len() != reached.len() {
        errors.push(consumption_diagnostic(
            RuleId::M014,
            vec![],
            DraftLocation::Coverage,
            "reached IDs must equal the declared slice exactly",
        ));
    }
    let claims: BTreeSet<_> = graph
        .nodes()
        .iter()
        .filter(|n| n.payload.kind() == NodeKind::Claim)
        .map(|n| n.id)
        .collect();
    let row_ids: BTreeSet<_> = rows.iter().map(|r| r.claim).collect();
    let domain_valid = claims == row_ids && rows.len() == row_ids.len();
    if !domain_valid {
        errors.push(consumption_diagnostic(
            RuleId::M025,
            vec![],
            DraftLocation::Provenance,
            "exactly one row per present claim and none for quantities is required",
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        let Some(node) = graph
            .nodes()
            .iter()
            .find(|n| n.id == row.claim && n.payload.kind() == NodeKind::Claim)
        else {
            continue;
        };
        let expected = OutcomeDraft::new(node, row.verdict, row.deferred);
        if *row != expected {
            errors.push(consumption_diagnostic(
                RuleId::M025,
                vec![row.claim],
                DraftLocation::Verdict(index),
                "claim, producer, revision, budget and pin must match",
            ));
            continue;
        }
        // Missing or duplicate premises belong solely to M025, not blocker fallback.
        if domain_valid {
            let blocker = expected_deferral(graph, node, rows);
            if row.deferred != blocker
                || (row.verdict == VerdictClass::Deferred) != blocker.is_some()
            {
                errors.push(consumption_diagnostic(
                    RuleId::M028,
                    vec![row.claim],
                    DraftLocation::Verdict(index),
                    "outcome must carry precisely its derived blocker",
                ));
            }
        }
    }
    errors.sort_by(|a, b| (a.rule, &a.ids, &a.location).cmp(&(b.rule, &b.ids, &b.location)));
    errors
}
