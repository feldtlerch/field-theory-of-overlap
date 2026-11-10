//! Closed engine-independent contracts; descriptions never supply mathematics.
use crate::{BudgetId, NodeId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnginePin(pub &'static str);
pub const ENGINE_PIN: EnginePin = EnginePin("a5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad");
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
        evidence: "Fixed object Compose, evaluated residual and Member. Source/reciprocal are registered but not exposed. Product polynomial Defined is unavailable: source requires it and region partition rejects Product.",
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
            revision: 1,
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
