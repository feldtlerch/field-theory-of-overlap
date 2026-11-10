//! Admission precedes allocation; typed outcomes propagate, invalid construction aborts.
#[cfg(test)]
use crate::Receipt;
#[cfg(test)]
use crate::report::ProducerRun;
use crate::{Measurement, registry};
use ftoo_manifest::{
    CapabilityId, Diagnostic, ENGINE_PIN, NodeId, NodeKind, ProducerDescriptor, RelationKind,
    ValidatedManifest, adapter_implemented,
};
use nrz_cas::Fault;
use std::{error::Error, fmt};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStage {
    Capability,
    Premise,
    Setup,
    Candidate,
    Compose,
    ResidualMember,
    Export,
    Expectation,
    Replay,
    Projection,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunCause {
    CapabilityUnavailable(CapabilityId),
    MissingAcceptedPremise(NodeId),
    Engine(Fault),
    ProjectionMismatch,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunError {
    Admission(Vec<Diagnostic>),
    Execution {
        claim: NodeId,
        stage: RunStage,
        cause: RunCause,
    },
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RunError {}
pub(crate) fn fault(claim: NodeId, stage: RunStage, fault: Fault) -> RunError {
    RunError::Execution {
        claim,
        stage,
        cause: RunCause::Engine(fault),
    }
}
#[cfg(test)]
pub(crate) fn ready(descriptor: ProducerDescriptor, accepted: &[Receipt]) -> Result<(), RunError> {
    for capability in descriptor.capabilities {
        if !adapter_implemented(*capability, ENGINE_PIN) {
            return Err(RunError::Execution {
                claim: descriptor.claim,
                stage: RunStage::Capability,
                cause: RunCause::CapabilityUnavailable(*capability),
            });
        }
    }
    for premise in descriptor
        .dependencies
        .iter()
        .filter(|id| id.expected_kind() == NodeKind::Claim)
    {
        if !accepted.iter().any(|r| r.claim() == *premise) {
            return Err(RunError::Execution {
                claim: descriptor.claim,
                stage: RunStage::Premise,
                cause: RunCause::MissingAcceptedPremise(*premise),
            });
        }
    }
    Ok(())
}
/// Execute selected claims only after global registry admission.
///
/// # Errors
/// Returns accumulated admission diagnostics or the first exact execution cause.
/// No partial run is returned, and accepted receipts cannot promote base status.
#[cfg(test)]
pub(crate) fn run(manifest: &ValidatedManifest) -> Result<ProducerRun<'_>, RunError> {
    execute(manifest, &registry::REGISTRY)
}
#[cfg(test)]
pub(crate) fn execute<'a>(
    manifest: &'a ValidatedManifest,
    roster: &[registry::Entry],
) -> Result<ProducerRun<'a>, RunError> {
    let descriptors: Vec<_> = roster.iter().map(|entry| entry.descriptor).collect();
    let diagnostics = ftoo_manifest::registry_diagnostics(manifest, &descriptors);
    if !diagnostics.is_empty() {
        return Err(RunError::Admission(diagnostics));
    }
    let mut receipts = Vec::new();
    let mut measurements: Vec<Measurement> = Vec::new();
    // DependsOn is acyclic by its closed contract, so this error is unreachable.
    let order = manifest
        .dependency_order(RelationKind::DependsOn)
        .map_err(|_| RunError::Admission(Vec::new()))?;
    for id in order
        .into_iter()
        .filter(|id| manifest.coverage().slice.contains(id))
    {
        let entry = roster
            .iter()
            .find(|entry| entry.descriptor.claim == id)
            .ok_or_else(|| RunError::Admission(Vec::new()))?;
        ready(entry.descriptor, &receipts)?;
        let (receipt, measurement) = (entry.produce)()?;
        receipts.push(receipt);
        measurements.push(measurement);
    }
    Ok(ProducerRun::accepted(manifest, receipts, measurements))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditError {
    Input(Vec<Diagnostic>),
    Report(Vec<Diagnostic>),
    Execution(RunError),
}
impl AuditError {
    #[must_use]
    pub const fn disposition(&self) -> ftoo_manifest::Disposition {
        match self {
            Self::Input(_) => ftoo_manifest::Disposition::InputError,
            Self::Report(_) | Self::Execution(_) => ftoo_manifest::Disposition::SemanticInvalidity,
        }
    }
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.disposition().exit_code()
    }
}
impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for AuditError {}
/// Execute the closed registry and consume its own evidence into an owned report.
/// # Errors
/// Admission errors and invalid construction abort; resource limits remain outcomes.
pub fn audit(manifest: &ValidatedManifest) -> Result<crate::AuditReport, AuditError> {
    audit_entries(manifest, &registry::REGISTRY)
}
pub(crate) fn deferred(
    descriptor: ProducerDescriptor,
    outcomes: &[crate::ClaimOutcome],
) -> Option<ftoo_manifest::DeferredReason> {
    use ftoo_manifest::DeferredReason;
    for capability in descriptor.capabilities {
        if !adapter_implemented(*capability, ENGINE_PIN) {
            return Some(DeferredReason::Capability(*capability));
        }
    }
    for premise in descriptor
        .dependencies
        .iter()
        .filter(|id| id.expected_kind() == NodeKind::Claim)
    {
        if let Some(row) = outcomes.iter().find(|r| r.claim() == *premise)
            && row.verdict().class() != ftoo_manifest::VerdictClass::Certified
        {
            return Some(DeferredReason::Premise {
                claim: *premise,
                evidence: row.verdict().class(),
            });
        }
    }
    None
}
pub(crate) fn interpret(error: RunError) -> Result<crate::Verdict, AuditError> {
    match error {
        RunError::Execution {
            claim,
            stage,
            cause: RunCause::Engine(fault),
        } => match fault.kind {
            nrz_cas::FaultKind::Exhausted { .. } => Ok(crate::Verdict::Undecided {
                stage,
                exhaustion: fault,
            }),
            nrz_cas::FaultKind::Invalid(_) => Err(AuditError::Execution(RunError::Execution {
                claim,
                stage,
                cause: RunCause::Engine(fault),
            })),
        },
        RunError::Admission(diagnostics) => Err(AuditError::Input(diagnostics)),
        RunError::Execution { .. } => Err(AuditError::Execution(error)),
    }
}
pub(crate) fn audit_entries(
    manifest: &ValidatedManifest,
    roster: &[registry::Entry],
) -> Result<crate::AuditReport, AuditError> {
    use crate::{ClaimOutcome, Verdict};
    let descriptors: Vec<_> = roster.iter().map(|e| e.descriptor).collect();
    let diagnostics = ftoo_manifest::registry_diagnostics(manifest, &descriptors);
    if !diagnostics.is_empty() {
        return Err(AuditError::Input(diagnostics));
    }
    let snapshot = ftoo_manifest::Snapshot::capture(manifest);
    let order = manifest
        .dependency_order(RelationKind::DependsOn)
        .map_err(|_| AuditError::Input(Vec::new()))?;
    let mut reached = Vec::new();
    let mut outcomes = Vec::new();
    let mut measurements = Vec::new();
    for id in order {
        let Some(node) = manifest
            .nodes()
            .iter()
            .find(|n| n.id == id && n.payload.kind() == NodeKind::Claim)
        else {
            continue;
        };
        let rows = crate::report::outcome_drafts(manifest, &outcomes)?;
        let verdict = if manifest.coverage().slice.contains(&id) {
            reached.push(id);
            let entry = roster
                .iter()
                .find(|e| e.descriptor.claim == id)
                .ok_or_else(|| AuditError::Input(Vec::new()))?;
            let (verdict, measurement) = dispatch(entry, &outcomes)?;
            measurements.extend(measurement);
            verdict
        } else {
            let reason =
                ftoo_manifest::expected_deferral(manifest, node, &rows).ok_or_else(|| {
                    AuditError::Report(vec![crate::report::report_error(
                        ftoo_manifest::RuleId::M028,
                        "unselected claim needs blocker",
                    )])
                })?;
            Verdict::Deferred(reason)
        };
        outcomes.push(ClaimOutcome { claim: id, verdict });
    }
    crate::AuditReport::from_outcomes(manifest, snapshot, reached, outcomes, measurements)
}

pub(crate) fn dispatch(
    entry: &registry::Entry,
    outcomes: &[crate::ClaimOutcome],
) -> Result<(crate::Verdict, Option<Measurement>), AuditError> {
    use crate::Verdict;
    for premise in entry
        .descriptor
        .dependencies
        .iter()
        .filter(|id| id.expected_kind() == NodeKind::Claim)
    {
        if !outcomes.iter().any(|row| row.claim() == *premise) {
            return Err(AuditError::Report(vec![crate::report::report_error(
                ftoo_manifest::RuleId::M025,
                "missing premise outcome",
            )]));
        }
    }
    if let Some(reason) = deferred(entry.descriptor, outcomes) {
        return Ok((Verdict::Deferred(reason), None));
    }
    match (entry.produce)() {
        Ok((receipt, measurement)) => Ok((
            if receipt.counterstatement().is_some() {
                Verdict::Refuted(receipt)
            } else {
                Verdict::Certified(receipt)
            },
            Some(measurement),
        )),
        Err(error) => {
            if let RunError::Execution { claim, .. } = &error
                && *claim != entry.descriptor.claim
            {
                return Err(AuditError::Report(vec![crate::report::report_error(
                    ftoo_manifest::RuleId::M025,
                    "execution error names another claim",
                )]));
            }
            Ok((interpret(error)?, None))
        }
    }
}
