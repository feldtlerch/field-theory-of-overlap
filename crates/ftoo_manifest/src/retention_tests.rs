use crate::{RULES, RuleId, RuleSpec, RuleState};

#[test]
#[allow(clippy::too_many_lines)] // One table pins all new reason alternatives and roles.
fn retained_reason_parts_keep_roles_and_complete_evidence_text() {
    use crate::{
        DeferredReason, EvidenceExhaustion, EvidenceResource, EvidenceStage, ExhaustionReason,
        NodeId, PendingReason, ReasonPart, ReasonRole, VerdictClass,
    };
    let object = NodeId::LangevinCoefficientThreeValue;
    let subject = ReasonPart::Object {
        role: ReasonRole::Subject,
        id: object,
    };
    let premise = ReasonPart::Object {
        role: ReasonRole::Premise,
        id: NodeId::LangevinCoefficientOneValue,
    };
    let cases = [
        (
            PendingReason::ClaimRefuted { object },
            vec![
                ReasonPart::Text("claim evidence ".into()),
                subject.clone(),
                ReasonPart::Text(" is refuted".into()),
            ],
            "claim evidence claim:langevin:coefficient-three-value is refuted",
        ),
        (
            PendingReason::ClaimAuditDeferred {
                object,
                reason: DeferredReason::AuditBinding,
            },
            vec![
                ReasonPart::Text("claim evidence ".into()),
                subject.clone(),
                ReasonPart::Text(" awaits audit binding".into()),
            ],
            "claim evidence claim:langevin:coefficient-three-value awaits audit binding",
        ),
        (
            PendingReason::ClaimAuditDeferred {
                object,
                reason: DeferredReason::OutsideSlice,
            },
            vec![
                ReasonPart::Text("claim evidence ".into()),
                subject.clone(),
                ReasonPart::Text(" awaits outside slice".into()),
            ],
            "claim evidence claim:langevin:coefficient-three-value awaits outside slice",
        ),
        (
            PendingReason::ClaimAuditDeferred {
                object,
                reason: DeferredReason::Premise {
                    claim: NodeId::LangevinCoefficientOneValue,
                    evidence: VerdictClass::Undecided,
                },
            },
            vec![
                ReasonPart::Text("claim evidence ".into()),
                subject.clone(),
                ReasonPart::Text(" awaits premise ".into()),
                premise,
                ReasonPart::Text(" with evidence undecided".into()),
            ],
            "claim evidence claim:langevin:coefficient-three-value awaits premise claim:langevin:coefficient-one-value with evidence undecided",
        ),
    ];
    for (reason, parts, display) in cases {
        assert_eq!(reason.text_parts(), parts);
        assert_eq!(reason.to_string(), display);
    }
    for (logical_id, suffix) in [
        (None, "typed witness DAG"),
        (Some(17), "typed witness DAG (logical id 17)"),
    ] {
        let reason = PendingReason::ClaimUndecided {
            object,
            stage: EvidenceStage::Replay,
            exhaustion: EvidenceExhaustion {
                resource: EvidenceResource::ReplayWork,
                reason: ExhaustionReason::Limit,
                limit: 9,
                attempted: 10,
                operation: "typed witness DAG".into(),
                logical_id,
            },
        };
        assert_eq!(
            reason.text_parts(),
            vec![
                ReasonPart::Text("claim evidence ".into()),
                subject.clone(),
                ReasonPart::Text(format!(
                    " is undecided: replay exhausted replay-work with limit 9 and attempted 10 at {suffix}"
                ))
            ]
        );
        assert_eq!(
            reason.to_string(),
            format!(
                "claim evidence claim:langevin:coefficient-three-value is undecided: replay exhausted replay-work with limit 9 and attempted 10 at {suffix}"
            )
        );
    }
}

#[test]
fn retention_register_has_complete_independent_records() {
    assert_eq!(
        RULES[48],
        RuleSpec {
            number: RuleId::M049,
            statement: "Symbol tiers are derived from the supplied graph, an accepted retained audit context where supplied, and transitive symbol uses; an alias takes the lesser of its own binding tier and its target tier, retaining every pending reason.",
            scope: "Base checked notation catalogues, accepted retained notation views and all exposed symbol-status values.",
            state: RuleState::Structural,
            deferred_reason: None
        }
    );
    assert_eq!(
        RULES[60..],
        [
            RuleSpec {
                number: RuleId::M061,
                statement: "Retained audit bytes bind the exact current projection, audit/codec contract revision and linked engine source revision, and no retained assertion is accepted before complete gated re-derivation and byte equality.",
                scope: "Strict retained-report admission, provenance, bounds, canonical equality and every retained evidence context supplied to a consumer.",
                state: RuleState::Enforced,
                deferred_reason: None,
            },
            RuleSpec {
                number: RuleId::M062,
                statement: "Report generation and check use one canonical writer; a release claiming retained audit evidence requires a stable reread accepted report and matching generated artifacts.",
                scope: "Retained-report file lifecycle, committed report byte gate and release-retention consumer.",
                state: RuleState::Enforced,
                deferred_reason: None,
            },
        ]
    );
    assert_eq!(
        (
            RULES.len(),
            RULES
                .iter()
                .filter(|rule| rule.state == RuleState::Enforced)
                .count(),
            RULES
                .iter()
                .filter(|rule| rule.state == RuleState::Structural)
                .count(),
            RULES
                .iter()
                .filter(|rule| rule.deferred_reason.is_some())
                .count()
        ),
        (62, 53, 4, 5)
    );
}
