use crate::NodeId;

/// Explicit completeness assertions, not exemptions from reference checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosedWorld {
    pub referents: bool,
    pub relations: bool,
    pub audit: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseGate {
    ExplicitPartial,
    Complete,
}

/// The slice names claims reached by manifest-bound audit, not registered claims.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Coverage {
    pub slice: Vec<NodeId>,
    pub closed_world: ClosedWorld,
    pub release_gate: ReleaseGate,
}

/// Code 5 extends ADR 007: incomplete coverage need not be mathematical failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    Pass,
    InputError,
    SemanticInvalidity,
    MathematicalFailure,
    Incomplete,
}
impl Disposition {
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::InputError => 2,
            Self::SemanticInvalidity => 3,
            Self::MathematicalFailure => 4,
            Self::Incomplete => 5,
        }
    }
}
/// Untrusted pure calculation, not a release authorization or audited status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssessmentDraft {
    pub disposition: Disposition,
    pub diagnostics: Vec<crate::Diagnostic>,
}
#[must_use]
pub fn assess_draft(
    graph: &crate::ValidatedManifest,
    snapshot: &crate::Snapshot,
    reached: &[NodeId],
    rows: &[crate::OutcomeDraft],
) -> AssessmentDraft {
    let mut diagnostics = crate::report_diagnostics(graph, snapshot, reached, rows);
    if !diagnostics.is_empty() {
        return AssessmentDraft {
            disposition: Disposition::SemanticInvalidity,
            diagnostics,
        };
    }
    let coverage = graph.coverage();
    let refuted = rows
        .iter()
        .any(|r| coverage.slice.contains(&r.claim) && r.verdict == crate::VerdictClass::Refuted);
    let complete = coverage.closed_world.referents
        && coverage.closed_world.relations
        && coverage.closed_world.audit
        && !rows.is_empty()
        && rows.len() == coverage.slice.len()
        && rows
            .iter()
            .all(|r| r.verdict == crate::VerdictClass::Certified);
    let disposition = if refuted {
        Disposition::MathematicalFailure
    } else if coverage.release_gate == ReleaseGate::Complete && !complete {
        Disposition::Incomplete
    } else {
        Disposition::Pass
    };
    if disposition != Disposition::Pass {
        diagnostics.push(crate::audit::consumption_diagnostic(
            crate::RuleId::M015,
            vec![],
            crate::DraftLocation::Coverage,
            "release predicate is not satisfied",
        ));
    }
    AssessmentDraft {
        disposition,
        diagnostics,
    }
}
