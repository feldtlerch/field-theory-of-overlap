use crate::{
    AuditRoutes, BudgetSpec, ClaimAudit, ClosedWorld, Coverage, Edge, Guard, ManifestDraft,
    Maturity, NodeDraft, NodeId, NodePayload, ProducerId, RelationKind, ReleaseGate,
    ValidatedManifest, ValidationErrors, validate,
};

/// Four proposed objects grounded in the dimension-three Langevin audit trial.
/// Their descriptions neither certify values nor supply a mathematical language.
#[must_use]
pub fn seed_draft() -> ManifestDraft {
    use NodeId::{
        LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
        LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as ThreeValue,
    };
    use RelationKind::{CompatibleWith, DependsOn};
    ManifestDraft {
        nodes: vec![
            NodeDraft {
                id: One,
                maturity: Maturity::Proposed,
                payload: NodePayload::Quantity {
                    description:
                        "The coefficient of k in the three-dimensional Langevin expansion.".into(),
                },
            },
            NodeDraft {
                id: Three,
                maturity: Maturity::Proposed,
                payload: NodePayload::Quantity {
                    description: "The coefficient of k cubed in the same expansion.".into(),
                },
            },
            NodeDraft {
                id: OneValue,
                maturity: Maturity::Proposed,
                payload: NodePayload::Claim {
                    description: "The first coefficient is exactly 1/3.".into(),
                    audit: ClaimAudit::Bound {
                        producer: ProducerId::LangevinCoefficientOneValue,
                        budget: ProducerId::LangevinCoefficientOneValue.descriptor().budget,
                        routes: AuditRoutes::Unconditional,
                    },
                },
            },
            NodeDraft {
                id: ThreeValue,
                maturity: Maturity::Proposed,
                payload: NodePayload::Claim {
                    description: "The third coefficient is exactly -1/45.".into(),
                    audit: ClaimAudit::Bound {
                        producer: ProducerId::LangevinCoefficientThreeValue,
                        budget: ProducerId::LangevinCoefficientThreeValue
                            .descriptor()
                            .budget,
                        routes: AuditRoutes::Unconditional,
                    },
                },
            },
        ],
        edges: [
            (DependsOn, Three, One),
            (DependsOn, OneValue, One),
            (DependsOn, ThreeValue, Three),
            (DependsOn, ThreeValue, OneValue),
            (CompatibleWith, One, Three),
            (CompatibleWith, Three, One),
        ]
        .into_iter()
        .map(|(relation, source, target)| Edge {
            relation,
            source,
            target,
            guard: Guard::Always,
        })
        .collect(),
        budgets: ProducerId::ALL
            .into_iter()
            .map(BudgetSpec::reviewed)
            .collect(),
        coverage: Coverage {
            slice: vec![OneValue, ThreeValue],
            closed_world: ClosedWorld {
                referents: false,
                relations: false,
                audit: true,
            },
            release_gate: ReleaseGate::ExplicitPartial,
        },
    }
}

/// Validate the seed through the same path as every authored draft.
///
/// # Errors
/// Returns structural diagnostics if the seed violates a registered rule.
pub fn seed() -> Result<ValidatedManifest, ValidationErrors> {
    validate(seed_draft())
}
