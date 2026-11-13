use crate::{RULES, RuleId, RuleSpec, RuleState};

#[test]
fn retention_register_has_complete_independent_records() {
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
