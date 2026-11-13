//! Effective notation consumes accepted evidence without changing the base catalogue.
use super::{AcceptedRetention, Difference, InfoRecord, RetentionError, Run};
use ftoo_manifest::{
    CheckedNotation, DefinitionId, EvidenceExhaustion, EvidenceResource, EvidenceStage,
    ExhaustionReason, PendingReason, SymbolTier,
};

/// Effective status cannot be supplied by a caller.
/// ```compile_fail
/// let status = ftoo_audit::retention::EffectiveSymbolStatus { reasons: vec![] };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveSymbolStatus {
    reasons: Vec<PendingReason>,
}
impl EffectiveSymbolStatus {
    #[must_use]
    pub fn tier(&self) -> SymbolTier {
        if self.reasons.is_empty() {
            SymbolTier::Authorised
        } else {
            SymbolTier::Pending
        }
    }
    #[must_use]
    pub fn reasons(&self) -> &[PendingReason] {
        &self.reasons
    }
}
/// This view binds the exact catalogue projection to independently accepted evidence.
/// ```compile_fail
/// let view = ftoo_audit::retention::RetainedNotation::default();
/// ```
/// ```compile_fail
/// fn forge(catalogue: &ftoo_manifest::CheckedNotation<'_, '_>) {
///     ftoo_audit::retention::RetainedNotation::new(catalogue, &std::collections::BTreeMap::<ftoo_manifest::NodeId, ftoo_manifest::VerdictClass>::new());
/// }
/// ```
#[derive(Clone, Debug)]
pub struct RetainedNotation<'catalogue, 'book, 'graph> {
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
    accepted: &'catalogue AcceptedRetention,
    statuses: Vec<(DefinitionId, EffectiveSymbolStatus)>,
}
impl<'catalogue, 'book, 'graph> RetainedNotation<'catalogue, 'book, 'graph> {
    /// # Errors
    /// Refuses a different projection or unsupported evidence component.
    /// # Panics
    /// Panics only if an accepted report violates its checked claim domain.
    pub fn new(
        catalogue: &'catalogue CheckedNotation<'book, 'graph>,
        accepted: &'catalogue AcceptedRetention,
    ) -> Result<Self, RetentionError> {
        let projection =
            catalogue
                .projection_bytes()
                .map_err(|error| RetentionError::Projection {
                    path: "notation".into(),
                    error: Box::new(error),
                })?;
        if let Some(difference) = Difference::between(
            accepted.projection(),
            &projection,
            InfoRecord::collect(Run::Retained),
            InfoRecord::collect(Run::Retained),
        ) {
            return Err(RetentionError::ContextMismatch {
                path: "notation".into(),
                difference: Box::new(difference),
            });
        }
        let statuses = catalogue
            .definitions()
            .iter()
            .filter_map(|definition| {
                definition
                    .status()
                    .map(|status| (definition.spec().id(), status))
            })
            .map(|(id, status)| {
                let mut reasons = vec![];
                for reason in status.reasons() {
                    if let PendingReason::ClaimDeferred { object, .. } = reason {
                        let outcome = accepted
                            .report()
                            .outcomes()
                            .iter()
                            .find(|outcome| outcome.claim() == *object)
                            .expect("accepted claim domain");
                        if let Some(reason) = evidence_reason(*object, outcome.verdict())? {
                            reasons.push(reason);
                        }
                    } else {
                        reasons.push(reason.clone());
                    }
                }
                reasons.sort_by(PendingReason::compare);
                reasons.dedup();
                Ok((id, EffectiveSymbolStatus { reasons }))
            })
            .collect::<Result<_, RetentionError>>()?;
        Ok(Self {
            catalogue,
            accepted,
            statuses,
        })
    }
    #[must_use]
    pub const fn base_catalogue(&self) -> &'catalogue CheckedNotation<'book, 'graph> {
        self.catalogue
    }
    #[must_use]
    pub const fn accepted(&self) -> &'catalogue AcceptedRetention {
        self.accepted
    }
    #[must_use]
    pub fn effective_status(&self, id: DefinitionId) -> Option<&EffectiveSymbolStatus> {
        self.statuses
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, status)| status)
    }
}
pub(super) fn evidence_reason(
    object: ftoo_manifest::NodeId,
    verdict: &crate::Verdict,
) -> Result<Option<PendingReason>, RetentionError> {
    Ok(match verdict {
        crate::Verdict::Certified(_) => None,
        crate::Verdict::Refuted(_) => Some(PendingReason::ClaimRefuted { object }),
        crate::Verdict::Deferred(reason) => Some(PendingReason::ClaimAuditDeferred {
            object,
            reason: *reason,
        }),
        crate::Verdict::Undecided { stage, exhaustion } => Some(PendingReason::ClaimUndecided {
            object,
            stage: stage_record(*stage),
            exhaustion: exhaustion_record(exhaustion)?,
        }),
    })
}
#[must_use]
pub fn stage_record(stage: crate::RunStage) -> EvidenceStage {
    match stage {
        crate::RunStage::Capability => EvidenceStage::Capability,
        crate::RunStage::Premise => EvidenceStage::Premise,
        crate::RunStage::Setup => EvidenceStage::Setup,
        crate::RunStage::Candidate => EvidenceStage::Candidate,
        crate::RunStage::Definedness => EvidenceStage::Definedness,
        crate::RunStage::Source => EvidenceStage::Source,
        crate::RunStage::ResidualMember => EvidenceStage::ResidualMember,
        crate::RunStage::Export => EvidenceStage::Export,
        crate::RunStage::Expectation => EvidenceStage::Expectation,
        crate::RunStage::Replay => EvidenceStage::Replay,
        crate::RunStage::Projection => EvidenceStage::Projection,
    }
}
/// # Errors
/// Refuses a fault component not representable as exhaustion.
pub fn exhaustion_record(fault: &nrz_cas::Fault) -> Result<EvidenceExhaustion, RetentionError> {
    let record = super::production::exhaustion_record(fault)?;
    Ok(EvidenceExhaustion {
        resource: match record.resource {
            nrz_cas::limits::BudgetKind::ArithmeticSteps => EvidenceResource::ArithmeticSteps,
            nrz_cas::limits::BudgetKind::BoundaryNodes => EvidenceResource::BoundaryNodes,
            nrz_cas::limits::BudgetKind::CaseArms => EvidenceResource::CaseArms,
            nrz_cas::limits::BudgetKind::CoefficientMaps => EvidenceResource::CoefficientMaps,
            nrz_cas::limits::BudgetKind::CoefficientOps => EvidenceResource::CoefficientOps,
            nrz_cas::limits::BudgetKind::ComponentBits => EvidenceResource::ComponentBits,
            nrz_cas::limits::BudgetKind::ComponentSplits => EvidenceResource::ComponentSplits,
            nrz_cas::limits::BudgetKind::ConditionAtoms => EvidenceResource::ConditionAtoms,
            nrz_cas::limits::BudgetKind::ConditionSteps => EvidenceResource::ConditionSteps,
            nrz_cas::limits::BudgetKind::CostQuota => EvidenceResource::CostQuota,
            nrz_cas::limits::BudgetKind::DecimalExponent => EvidenceResource::DecimalExponent,
            nrz_cas::limits::BudgetKind::DeltaOrder => EvidenceResource::DeltaOrder,
            nrz_cas::limits::BudgetKind::DenominatorFactors => EvidenceResource::DenominatorFactors,
            nrz_cas::limits::BudgetKind::Depth => EvidenceResource::Depth,
            nrz_cas::limits::BudgetKind::DiagnosticRecords => EvidenceResource::DiagnosticRecords,
            nrz_cas::limits::BudgetKind::Edges => EvidenceResource::Edges,
            nrz_cas::limits::BudgetKind::ExpansionCredits => EvidenceResource::ExpansionCredits,
            nrz_cas::limits::BudgetKind::FamilyParameters => EvidenceResource::FamilyParameters,
            nrz_cas::limits::BudgetKind::FrontierLabels => EvidenceResource::FrontierLabels,
            nrz_cas::limits::BudgetKind::GcdSteps => EvidenceResource::GcdSteps,
            nrz_cas::limits::BudgetKind::HypothesisDepth => EvidenceResource::HypothesisDepth,
            nrz_cas::limits::BudgetKind::IntegerExponent => EvidenceResource::IntegerExponent,
            nrz_cas::limits::BudgetKind::IntermediateBits => EvidenceResource::IntermediateBits,
            nrz_cas::limits::BudgetKind::IsolationSteps => EvidenceResource::IsolationSteps,
            nrz_cas::limits::BudgetKind::JetEntries => EvidenceResource::JetEntries,
            nrz_cas::limits::BudgetKind::JetOrder => EvidenceResource::JetOrder,
            nrz_cas::limits::BudgetKind::LinearPivots => EvidenceResource::LinearPivots,
            nrz_cas::limits::BudgetKind::LiteralBytes => EvidenceResource::LiteralBytes,
            nrz_cas::limits::BudgetKind::LocalBranches => EvidenceResource::LocalBranches,
            nrz_cas::limits::BudgetKind::LocalContexts => EvidenceResource::LocalContexts,
            nrz_cas::limits::BudgetKind::MatchAttempts => EvidenceResource::MatchAttempts,
            nrz_cas::limits::BudgetKind::MatchBatch => EvidenceResource::MatchBatch,
            nrz_cas::limits::BudgetKind::MatrixEntries => EvidenceResource::MatrixEntries,
            nrz_cas::limits::BudgetKind::NameBytes => EvidenceResource::NameBytes,
            nrz_cas::limits::BudgetKind::OrbitMembers => EvidenceResource::OrbitMembers,
            nrz_cas::limits::BudgetKind::OrbitPermutations => EvidenceResource::OrbitPermutations,
            nrz_cas::limits::BudgetKind::OrbitTerms => EvidenceResource::OrbitTerms,
            nrz_cas::limits::BudgetKind::OriginalOccurrences => {
                EvidenceResource::OriginalOccurrences
            }
            nrz_cas::limits::BudgetKind::OutputBytes => EvidenceResource::OutputBytes,
            nrz_cas::limits::BudgetKind::PairingSteps => EvidenceResource::PairingSteps,
            nrz_cas::limits::BudgetKind::PayloadBytes => EvidenceResource::PayloadBytes,
            nrz_cas::limits::BudgetKind::PeakExpressionSize => EvidenceResource::PeakExpressionSize,
            nrz_cas::limits::BudgetKind::PolynomialDegree => EvidenceResource::PolynomialDegree,
            nrz_cas::limits::BudgetKind::PolynomialTerms => EvidenceResource::PolynomialTerms,
            nrz_cas::limits::BudgetKind::PolynomialVariables => {
                EvidenceResource::PolynomialVariables
            }
            nrz_cas::limits::BudgetKind::ProgressQuota => EvidenceResource::ProgressQuota,
            nrz_cas::limits::BudgetKind::ProofNodes => EvidenceResource::ProofNodes,
            nrz_cas::limits::BudgetKind::PullbackZeros => EvidenceResource::PullbackZeros,
            nrz_cas::limits::BudgetKind::QueueEntries => EvidenceResource::QueueEntries,
            nrz_cas::limits::BudgetKind::RefinementPairs => EvidenceResource::RefinementPairs,
            nrz_cas::limits::BudgetKind::RegionCells => EvidenceResource::RegionCells,
            nrz_cas::limits::BudgetKind::RegionNodes => EvidenceResource::RegionNodes,
            nrz_cas::limits::BudgetKind::ReplayWork => EvidenceResource::ReplayWork,
            nrz_cas::limits::BudgetKind::ResolventDegree => EvidenceResource::ResolventDegree,
            nrz_cas::limits::BudgetKind::ResolventDepth => EvidenceResource::ResolventDepth,
            nrz_cas::limits::BudgetKind::SelectionSlots => EvidenceResource::SelectionSlots,
            nrz_cas::limits::BudgetKind::SolutionBranches => EvidenceResource::SolutionBranches,
            nrz_cas::limits::BudgetKind::SolveIdentities => EvidenceResource::SolveIdentities,
            nrz_cas::limits::BudgetKind::SourceBytes => EvidenceResource::SourceBytes,
            nrz_cas::limits::BudgetKind::StagedEdges => EvidenceResource::StagedEdges,
            nrz_cas::limits::BudgetKind::StagedPayloadBytes => EvidenceResource::StagedPayloadBytes,
            nrz_cas::limits::BudgetKind::StagedVertices => EvidenceResource::StagedVertices,
            nrz_cas::limits::BudgetKind::SymmetricSteps => EvidenceResource::SymmetricSteps,
            nrz_cas::limits::BudgetKind::TestNodes => EvidenceResource::TestNodes,
            nrz_cas::limits::BudgetKind::TowerDegree => EvidenceResource::TowerDegree,
            nrz_cas::limits::BudgetKind::TowerLevels => EvidenceResource::TowerLevels,
            nrz_cas::limits::BudgetKind::TowerReductions => EvidenceResource::TowerReductions,
            nrz_cas::limits::BudgetKind::Transitions => EvidenceResource::Transitions,
            nrz_cas::limits::BudgetKind::Vertices => EvidenceResource::Vertices,
            nrz_cas::limits::BudgetKind::WitnessBytes => EvidenceResource::WitnessBytes,
            nrz_cas::limits::BudgetKind::Work => EvidenceResource::Work,
        },
        reason: match record.reason {
            nrz_cas::Exhaustion::Limit => ExhaustionReason::Limit,
            nrz_cas::Exhaustion::ArithmeticRange => ExhaustionReason::ArithmeticRange,
            nrz_cas::Exhaustion::Allocation => ExhaustionReason::Allocation,
            nrz_cas::Exhaustion::Capacity => ExhaustionReason::Capacity,
        },
        limit: record.limit,
        attempted: record.attempted,
        operation: record.location.operation,
        logical_id: record.location.logical_id,
    })
}
