//! Borrowed execution receipts, never an audited status or release assessment.
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
    pub increment: Vec<Rational>,
    pub input_orders: [u32; 2],
    pub default_order: bool,
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
#[derive(Debug)]
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
    /// Compose, evaluated residual, then Member with its full premise closure.
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
    pub replay_work: u64,
}
/// Only the fixed executing registry can construct this borrowed run.
/// No rows exist for unselected claims, and base status remains Deferred.
/// ```compile_fail,E0599
/// let run = ftoo_audit::ProducerRun::default();
/// ```
/// ```compile_fail,E0451
/// let run = ftoo_audit::ProducerRun { manifest: &ftoo_manifest::seed().unwrap(), selected: vec![], receipts: vec![], measurements: vec![] };
/// ```
#[derive(Debug)]
pub struct ProducerRun<'a> {
    manifest: &'a ValidatedManifest,
    selected: Vec<NodeId>,
    receipts: Vec<Receipt>,
    measurements: Vec<Measurement>,
}
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
