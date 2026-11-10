//! Admission precedes allocation; the first failed producer terminates the run.
use crate::{Measurement, ProducerRun, Receipt, registry};
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
pub fn run(manifest: &ValidatedManifest) -> Result<ProducerRun<'_>, RunError> {
    execute(manifest, &registry::REGISTRY)
}
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
