use super::*;
use ftoo_manifest::{DefinitionId, Maturity, NodeId, PendingReason, SymbolTier};

#[test]
#[allow(clippy::too_many_lines)] // One restoration control pins the full status and mismatch families.
fn retained_notation_preserves_transitive_formulation_and_refuses_foreign_projection() {
    for retained in [false, true] {
        let mut draft = ftoo_manifest::seed_draft();
        if retained {
            for node in &mut draft.nodes {
                node.maturity = Maturity::Retained;
            }
        }
        let graph = ftoo_manifest::validate(draft).unwrap();
        let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
        let production = produce(projection.clone()).unwrap();
        let accepted = accept(projection.clone(), production.bytes().to_vec()).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let catalogue = ftoo_manifest::notation(&book).unwrap();
        let view = RetainedNotation::new(&catalogue, &accepted).unwrap();
        for (id, objects) in [
            (
                DefinitionId::CoefficientOne,
                vec![NodeId::LangevinCoefficientOne],
            ),
            (
                DefinitionId::FirstCoefficient,
                vec![NodeId::LangevinCoefficientOne],
            ),
            (
                DefinitionId::CoefficientThree,
                vec![NodeId::LangevinCoefficientThree],
            ),
            (
                DefinitionId::CoefficientOneValue,
                vec![
                    NodeId::LangevinCoefficientOneValue,
                    NodeId::LangevinCoefficientOne,
                ],
            ),
            (
                DefinitionId::CoefficientThreeValue,
                vec![
                    NodeId::LangevinCoefficientThreeValue,
                    NodeId::LangevinCoefficientThree,
                ],
            ),
        ] {
            let status = view.effective_status(id).unwrap();
            let expected: Vec<_> = if retained {
                vec![]
            } else {
                objects
                    .into_iter()
                    .map(|object| PendingReason::FormulationProposed { object })
                    .collect()
            };
            assert_eq!(status.reasons(), expected);
            assert_eq!(
                status.tier(),
                if retained {
                    SymbolTier::Authorised
                } else {
                    SymbolTier::Pending
                }
            );
        }
        assert_eq!(view.effective_status(DefinitionId::Product), None);
        assert_eq!(
            catalogue
                .definition(DefinitionId::CoefficientOneValue)
                .status()
                .unwrap()
                .tier(),
            SymbolTier::Pending
        );
        let mut foreign = ftoo_manifest::seed_draft();
        foreign.nodes[0].maturity = if retained {
            Maturity::Proposed
        } else {
            Maturity::Retained
        };
        let foreign = ftoo_manifest::validate(foreign).unwrap();
        let foreign_bytes = ftoo_manifest::projection_bytes(&foreign).unwrap();
        let foreign_book = ftoo_manifest::addressing(&foreign).unwrap();
        let foreign_catalogue = ftoo_manifest::notation(&foreign_book).unwrap();
        let difference = matrix_tests::expected_difference(
            &projection,
            &foreign_bytes,
            InfoRecord::collect(Run::Retained),
            InfoRecord::collect(Run::Retained),
        );
        let expected_text = format!(
            "retention provenance M061 at file \"notation\": context-mismatch {}",
            classifier_tests::difference_display_labels(&difference, "accepted", "current")
        );
        let expected = RetentionError::ContextMismatch {
            path: "notation".into(),
            difference: Box::new(difference),
        };
        let actual = RetainedNotation::new(&foreign_catalogue, &accepted).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected_text);
        assert_eq!(
            RetainedNotation::new(&catalogue, &accepted)
                .unwrap()
                .effective_status(DefinitionId::CoefficientOneValue),
            view.effective_status(DefinitionId::CoefficientOneValue)
        );
    }
}

pub(super) fn genuine_fixture(
    graph: &ftoo_manifest::ValidatedManifest,
    accepted: &AcceptedRetention,
) {
    let book = ftoo_manifest::addressing(graph).unwrap();
    let catalogue = ftoo_manifest::notation(&book).unwrap();
    let view = RetainedNotation::new(&catalogue, accepted).unwrap();
    for outcome in accepted.report().outcomes() {
        let definition = catalogue
            .object_symbol(outcome.claim())
            .unwrap()
            .spec()
            .id();
        let mut expected = catalogue
            .definition(definition)
            .status()
            .unwrap()
            .reasons()
            .iter()
            .filter(|reason| !matches!(reason, PendingReason::ClaimDeferred { .. }))
            .cloned()
            .collect::<Vec<_>>();
        match outcome.verdict() {
            crate::Verdict::Certified(_) => (),
            crate::Verdict::Refuted(_) => expected.push(PendingReason::ClaimRefuted {
                object: outcome.claim(),
            }),
            crate::Verdict::Deferred(reason) => expected.push(PendingReason::ClaimAuditDeferred {
                object: outcome.claim(),
                reason: *reason,
            }),
            crate::Verdict::Undecided { stage, exhaustion } => {
                let nrz_cas::FaultKind::Exhausted {
                    reason,
                    budget,
                    limit,
                    needed,
                } = exhaustion.kind
                else {
                    panic!("genuine exhaustion");
                };
                assert_eq!(stage, &crate::RunStage::Setup);
                assert_eq!(reason, nrz_cas::Exhaustion::Limit);
                let resource = match budget {
                    nrz_cas::limits::BudgetKind::Work => ftoo_manifest::EvidenceResource::Work,
                    nrz_cas::limits::BudgetKind::ReplayWork => {
                        ftoo_manifest::EvidenceResource::ReplayWork
                    }
                    _ => panic!("fixture resource"),
                };
                expected.push(PendingReason::ClaimUndecided {
                    object: outcome.claim(),
                    stage: ftoo_manifest::EvidenceStage::Setup,
                    exhaustion: ftoo_manifest::EvidenceExhaustion {
                        resource,
                        reason: ftoo_manifest::ExhaustionReason::Limit,
                        limit,
                        attempted: needed,
                        operation: "draft validation".into(),
                        logical_id: None,
                    },
                });
            }
        }
        assert_eq!(
            view.effective_status(definition).unwrap().reasons(),
            expected
        );
    }
}
