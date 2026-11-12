//! Checked receipts feed protected audited statuses, provenance and release assessment.
use ftoo_manifest::{
    BudgetId, ENGINE_PIN, EnginePin, NodeId, ProducerDescriptor, ProducerId, RegionContract,
    ValidatedManifest,
};
use nrz_cas::{Rational, limits::Limits};

/// Fixed theorem scope, not a claim of global analytic equality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JetSummary {
    pub subject: &'static str,
    pub jet: Vec<Rational>,
    pub source: &'static str,
    pub clause: &'static str,
    pub order: u32,
    pub derivative_order: u32,
    pub residual_order: u32,
    pub equation_degrees: [u32; 4],
    pub ode: &'static str,
    pub region: RegionContract,
    pub analytic_scope: &'static str,
}
/// Accepted portable evidence is immutable and has no public constructor.
/// This private-fields diagnostic has no error code, so the example is uncoded.
/// ```compile_fail
/// let receipt = ftoo_audit::Receipt {};
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    claim: NodeId,
    producer: ProducerId,
    revision: u32,
    budget: BudgetId,
    limits: Limits,
    pin: EnginePin,
    target: usize,
    summary: JetSummary,
    evidence: [Vec<u8>; 3],
    pub(crate) contrary: Option<Counterstatement>,
}
impl Receipt {
    pub(crate) fn accepted(
        descriptor: ProducerDescriptor,
        limits: Limits,
        summary: JetSummary,
        evidence: [Vec<u8>; 3],
    ) -> Self {
        Self {
            claim: descriptor.claim,
            producer: descriptor.producer,
            revision: descriptor.revision,
            budget: descriptor.budget,
            limits,
            pin: ENGINE_PIN,
            target: descriptor.target,
            summary,
            evidence,
            contrary: None,
        }
    }
    #[must_use]
    pub const fn claim(&self) -> NodeId {
        self.claim
    }
    #[must_use]
    pub const fn producer(&self) -> ProducerId {
        self.producer
    }
    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }
    #[must_use]
    pub const fn budget(&self) -> BudgetId {
        self.budget
    }
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }
    #[must_use]
    pub const fn pin(&self) -> EnginePin {
        self.pin
    }
    #[must_use]
    pub const fn target(&self) -> usize {
        self.target
    }
    #[must_use]
    pub fn coefficient(&self) -> &Rational {
        &self.summary.jet[self.target]
    }
    #[must_use]
    pub const fn summary(&self) -> &JetSummary {
        &self.summary
    }
    /// Source including Defined, evaluated residual, then Member with its full premise closure.
    #[must_use]
    pub const fn evidence(&self) -> &[Vec<u8>; 3] {
        &self.evidence
    }
}
/// Observations are separate from the accepted mathematical evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Measurement {
    pub claim: NodeId,
    pub member_work: u64,
    pub total_work: u64,
    /// Cumulative Work spent across all replay calls in this audit.
    pub cumulative_replay_work: u64,
    /// Largest single replay requirement; the `ReplayWork` ceiling is per call.
    pub max_replay_work: u64,
    pub(crate) execution: u64,
}
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ProducerRun<'a> {
    manifest: &'a ValidatedManifest,
    selected: Vec<NodeId>,
    receipts: Vec<Receipt>,
    measurements: Vec<Measurement>,
}
#[cfg(test)]
impl<'a> ProducerRun<'a> {
    pub(crate) fn accepted(
        manifest: &'a ValidatedManifest,
        receipts: Vec<Receipt>,
        measurements: Vec<Measurement>,
    ) -> Self {
        Self {
            manifest,
            selected: receipts.iter().map(Receipt::claim).collect(),
            receipts,
            measurements,
        }
    }
    #[must_use]
    pub const fn manifest(&self) -> &'a ValidatedManifest {
        self.manifest
    }
    #[must_use]
    pub fn selected(&self) -> &[NodeId] {
        &self.selected
    }
    #[must_use]
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }
    #[must_use]
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }
}

/// A checked nonzero difference for the same scoped jet projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counterstatement {
    pub(crate) claimed: Rational,
    pub(crate) witness: Vec<u8>,
}
impl Counterstatement {
    #[must_use]
    pub const fn claimed(&self) -> &Rational {
        &self.claimed
    }
    #[must_use]
    pub fn witness(&self) -> &[u8] {
        &self.witness
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    Certified(Receipt),
    Refuted(Receipt),
    Undecided {
        stage: crate::RunStage,
        exhaustion: nrz_cas::Fault,
    },
    Deferred(ftoo_manifest::DeferredReason),
}
impl Verdict {
    #[must_use]
    pub const fn class(&self) -> ftoo_manifest::VerdictClass {
        use ftoo_manifest::VerdictClass;
        match self {
            Self::Certified(_) => VerdictClass::Certified,
            Self::Refuted(_) => VerdictClass::Refuted,
            Self::Undecided { .. } => VerdictClass::Undecided,
            Self::Deferred(_) => VerdictClass::Deferred,
        }
    }
}
impl Receipt {
    #[must_use]
    pub const fn counterstatement(&self) -> Option<&Counterstatement> {
        self.contrary.as_ref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimOutcome {
    pub(crate) claim: NodeId,
    pub(crate) verdict: Verdict,
}
impl ClaimOutcome {
    #[must_use]
    pub const fn claim(&self) -> NodeId {
        self.claim
    }
    #[must_use]
    pub const fn verdict(&self) -> &Verdict {
        &self.verdict
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditedEvidence {
    NotAClaim,
    Certified,
    Refuted,
    Undecided,
    Deferred,
}
/// Only checked report construction derives audited evidence.
/// ```compile_fail,E0451
/// let base=ftoo_manifest::seed().unwrap().status(ftoo_manifest::NodeId::LangevinCoefficientOneValue).unwrap();
/// let status=ftoo_audit::AuditedStatus {base,evidence:ftoo_audit::AuditedEvidence::Certified};
/// ```
/// ```compile_fail,E0624
/// let graph=ftoo_manifest::seed().unwrap();
/// let draft=ftoo_manifest::OutcomeDraft::new(&graph.nodes()[1],ftoo_manifest::VerdictClass::Certified,None);
/// let base=graph.status(draft.claim).unwrap();
/// let status=ftoo_audit::AuditedStatus::derive(base,Some(draft.verdict));
/// ```
/// ```compile_fail,E0277
/// let base=ftoo_manifest::seed().unwrap().status(ftoo_manifest::NodeId::LangevinCoefficientOneValue).unwrap();
/// let status:ftoo_audit::AuditedStatus=base.into();
/// ```
/// ```compile_fail,E0277
/// let draft=ftoo_manifest::AssessmentDraft {disposition:ftoo_manifest::Disposition::Pass,diagnostics:vec![]};
/// let status:ftoo_audit::AuditedStatus=draft.into();
/// ```
/// ```compile_fail,E0599
/// let status=ftoo_audit::AuditedStatus::default();
/// ```
/// ```compile_fail,E0616
/// let graph=ftoo_manifest::seed().unwrap();
/// let report=ftoo_audit::audit(&graph).unwrap();
/// let mut status=report.status(ftoo_manifest::NodeId::LangevinCoefficientOneValue).unwrap().clone();
/// status.evidence=ftoo_audit::AuditedEvidence::Certified;
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditedStatus {
    base: ftoo_manifest::Status,
    evidence: AuditedEvidence,
}
impl AuditedStatus {
    fn derive(base: ftoo_manifest::Status, verdict: Option<ftoo_manifest::VerdictClass>) -> Self {
        use ftoo_manifest::VerdictClass as V;
        let evidence = match verdict {
            None => AuditedEvidence::NotAClaim,
            Some(V::Certified) => AuditedEvidence::Certified,
            Some(V::Refuted) => AuditedEvidence::Refuted,
            Some(V::Undecided) => AuditedEvidence::Undecided,
            Some(V::Deferred) => AuditedEvidence::Deferred,
        };
        Self { base, evidence }
    }
    #[must_use]
    pub const fn base(&self) -> ftoo_manifest::Status {
        self.base
    }
    #[must_use]
    pub const fn evidence(&self) -> AuditedEvidence {
        self.evidence
    }
}
/// An assessment of this report's execution, never caller-authored verdicts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAssessment {
    disposition: ftoo_manifest::Disposition,
    diagnostics: Vec<ftoo_manifest::Diagnostic>,
}
impl ReleaseAssessment {
    #[must_use]
    pub const fn disposition(&self) -> ftoo_manifest::Disposition {
        self.disposition
    }
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.disposition.exit_code()
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[ftoo_manifest::Diagnostic] {
        &self.diagnostics
    }
}
/// All semantic output, including portable evidence; observations are separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatedReport {
    snapshot: ftoo_manifest::Snapshot,
    reached: Vec<NodeId>,
    outcomes: Vec<ClaimOutcome>,
    statuses: Vec<(NodeId, AuditedStatus)>,
    assessment: ReleaseAssessment,
}
/// Owned checked results, constructed only from the audit's own execution.
/// ```compile_fail,E0624
/// let graph=ftoo_manifest::seed().unwrap();
/// let report=ftoo_audit::AuditReport::from_outcomes(&graph,ftoo_manifest::Snapshot::capture(&graph),vec![],vec![],vec![]);
/// ```
/// ```compile_fail,E0277
/// let graph=ftoo_manifest::seed().unwrap();
/// let draft=ftoo_manifest::OutcomeDraft::new(&graph.nodes()[1],ftoo_manifest::VerdictClass::Certified,None);
/// let report:ftoo_audit::AuditReport=vec![draft].into();
/// ```
/// No public fields exist; the empty literal diagnostic has no error code.
/// ```compile_fail
/// let report=ftoo_audit::AuditReport {};
/// ```
#[derive(Debug)]
pub struct AuditReport {
    gated: GatedReport,
    measurements: Vec<Measurement>,
}
impl AuditReport {
    pub(crate) fn from_outcomes(
        graph: &ValidatedManifest,
        snapshot: ftoo_manifest::Snapshot,
        mut reached: Vec<NodeId>,
        mut outcomes: Vec<ClaimOutcome>,
        measurements: Vec<Measurement>,
    ) -> Result<Self, crate::AuditError> {
        outcomes.sort_by_key(|r| r.claim);
        reached.sort();
        let rows = outcome_drafts(graph, &outcomes)?;
        let assessment = ftoo_manifest::assess_draft(graph, &snapshot, &reached, &rows);
        if assessment.disposition == ftoo_manifest::Disposition::SemanticInvalidity {
            return Err(crate::AuditError::Report(assessment.diagnostics));
        }
        let identities: std::collections::BTreeSet<_> =
            measurements.iter().map(|m| m.execution).collect();
        if identities.len() != measurements.len() {
            return Err(crate::AuditError::Report(vec![report_error(
                ftoo_manifest::RuleId::M027,
                "executions must use distinct fresh stores and budgets",
            )]));
        }
        let mut statuses: Vec<_> = graph
            .nodes()
            .iter()
            .map(|node| {
                let base = graph.status(node.id).ok_or_else(|| {
                    crate::AuditError::Report(vec![report_error(
                        ftoo_manifest::RuleId::M025,
                        "missing base status",
                    )])
                })?;
                Ok((
                    node.id,
                    AuditedStatus::derive(
                        base,
                        rows.iter().find(|r| r.claim == node.id).map(|r| r.verdict),
                    ),
                ))
            })
            .collect::<Result<_, crate::AuditError>>()?;
        statuses.sort_by_key(|(id, _)| *id);
        Ok(Self {
            gated: GatedReport {
                snapshot,
                reached,
                outcomes,
                statuses,
                assessment: ReleaseAssessment {
                    disposition: assessment.disposition,
                    diagnostics: assessment.diagnostics,
                },
            },
            measurements,
        })
    }
    #[must_use]
    pub const fn gated(&self) -> &GatedReport {
        &self.gated
    }
    #[must_use]
    pub const fn snapshot(&self) -> &ftoo_manifest::Snapshot {
        &self.gated.snapshot
    }
    #[must_use]
    pub fn reached(&self) -> &[NodeId] {
        &self.gated.reached
    }
    #[must_use]
    pub fn outcomes(&self) -> &[ClaimOutcome] {
        &self.gated.outcomes
    }
    #[must_use]
    pub fn status(&self, id: NodeId) -> Option<&AuditedStatus> {
        self.gated
            .statuses
            .iter()
            .find(|(node, _)| *node == id)
            .map(|(_, status)| status)
    }
    #[must_use]
    pub const fn assessment(&self) -> &ReleaseAssessment {
        &self.gated.assessment
    }
    #[must_use]
    pub fn measurements(&self) -> &[Measurement] {
        &self.measurements
    }
}
pub(crate) fn report_error(
    rule: ftoo_manifest::RuleId,
    message: &str,
) -> ftoo_manifest::Diagnostic {
    ftoo_manifest::Diagnostic {
        stage: ftoo_manifest::AdmissionStage::ReportConsumption,
        rule,
        ids: Vec::new(),
        edge: None,
        location: ftoo_manifest::DraftLocation::Provenance,
        explanation: message.into(),
    }
}
pub(crate) fn outcome_drafts(
    graph: &ValidatedManifest,
    outcomes: &[ClaimOutcome],
) -> Result<Vec<ftoo_manifest::OutcomeDraft>, crate::AuditError> {
    outcomes
        .iter()
        .map(|row| {
            let node = graph
                .nodes()
                .iter()
                .find(|n| n.id == row.claim)
                .ok_or_else(|| {
                    crate::AuditError::Report(vec![report_error(
                        ftoo_manifest::RuleId::M025,
                        "unknown outcome identity",
                    )])
                })?;
            let deferred = if let Verdict::Deferred(reason) = row.verdict {
                Some(reason)
            } else {
                None
            };
            let mut draft = ftoo_manifest::OutcomeDraft::new(node, row.verdict.class(), deferred);
            if let Verdict::Undecided { exhaustion, .. } = &row.verdict {
                match exhaustion.kind {
                    nrz_cas::FaultKind::Exhausted { .. } => {}
                    nrz_cas::FaultKind::Invalid(_) => {
                        return Err(crate::AuditError::Report(vec![report_error(
                            ftoo_manifest::RuleId::M026,
                            "only typed resource faults yield Undecided",
                        )]));
                    }
                }
            }
            if let Verdict::Certified(receipt) | Verdict::Refuted(receipt) = &row.verdict {
                if receipt.limits()
                    != &crate::theory::limits(ftoo_manifest::BudgetProfile::PinnedTrialLimitsV2)
                    || receipt.target() != receipt.producer().descriptor().target
                {
                    return Err(crate::AuditError::Report(vec![report_error(
                        ftoo_manifest::RuleId::M025,
                        "receipt limits or target differ from contract",
                    )]));
                }
                if (matches!(row.verdict, Verdict::Refuted(_))) != receipt.contrary.is_some() {
                    return Err(crate::AuditError::Report(vec![report_error(
                        ftoo_manifest::RuleId::M026,
                        "verdict must match checked evidence",
                    )]));
                }
                draft.claim = receipt.claim();
                draft.producer = Some(receipt.producer());
                draft.revision = Some(receipt.revision());
                draft.budget = Some(receipt.budget());
                draft.pin = receipt.pin();
                if draft.claim != row.claim {
                    return Err(crate::AuditError::Report(vec![report_error(
                        ftoo_manifest::RuleId::M025,
                        "receipt claim differs from row",
                    )]));
                }
            }
            Ok(draft)
        })
        .collect()
}
