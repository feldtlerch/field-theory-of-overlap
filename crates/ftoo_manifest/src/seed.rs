use crate::{
    Awaiting, ClosedWorld, Coverage, Edge, Guard, ManifestDraft, Maturity, NodeDraft, NodeId,
    NodePayload, RelationKind, ReleaseGate, ValidatedManifest, ValidationErrors, validate,
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
                    awaits: Awaiting::AuditBinding,
                },
            },
            NodeDraft {
                id: ThreeValue,
                maturity: Maturity::Proposed,
                payload: NodePayload::Claim {
                    description: "The third coefficient is exactly -1/45.".into(),
                    awaits: Awaiting::AuditBinding,
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
        coverage: Coverage {
            slice: vec![],
            closed_world: ClosedWorld {
                referents: false,
                relations: false,
                audit: false,
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
