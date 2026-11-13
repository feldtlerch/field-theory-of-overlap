//! Engine-neutral evidence descriptions carry data, never notation authority.
use crate::{CapabilityId, DeferredReason, NodeId, VerdictClass};
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceStage {
    Capability,
    Premise,
    Setup,
    Candidate,
    Definedness,
    Source,
    ResidualMember,
    Export,
    Expectation,
    Replay,
    Projection,
}
impl EvidenceStage {
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Premise => "premise",
            Self::Setup => "setup",
            Self::Candidate => "candidate",
            Self::Definedness => "definedness",
            Self::Source => "source",
            Self::ResidualMember => "residual-member",
            Self::Export => "export",
            Self::Expectation => "expectation",
            Self::Replay => "replay",
            Self::Projection => "projection",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceResource {
    ArithmeticSteps,
    BoundaryNodes,
    CaseArms,
    CoefficientMaps,
    CoefficientOps,
    ComponentBits,
    ComponentSplits,
    ConditionAtoms,
    ConditionSteps,
    CostQuota,
    DecimalExponent,
    DeltaOrder,
    DenominatorFactors,
    Depth,
    DiagnosticRecords,
    Edges,
    ExpansionCredits,
    FamilyParameters,
    FrontierLabels,
    GcdSteps,
    HypothesisDepth,
    IntegerExponent,
    IntermediateBits,
    IsolationSteps,
    JetEntries,
    JetOrder,
    LinearPivots,
    LiteralBytes,
    LocalBranches,
    LocalContexts,
    MatchAttempts,
    MatchBatch,
    MatrixEntries,
    NameBytes,
    OrbitMembers,
    OrbitPermutations,
    OrbitTerms,
    OriginalOccurrences,
    OutputBytes,
    PairingSteps,
    PayloadBytes,
    PeakExpressionSize,
    PolynomialDegree,
    PolynomialTerms,
    PolynomialVariables,
    ProgressQuota,
    ProofNodes,
    PullbackZeros,
    QueueEntries,
    RefinementPairs,
    RegionCells,
    RegionNodes,
    ReplayWork,
    ResolventDegree,
    ResolventDepth,
    SelectionSlots,
    SolutionBranches,
    SolveIdentities,
    SourceBytes,
    StagedEdges,
    StagedPayloadBytes,
    StagedVertices,
    SymmetricSteps,
    TestNodes,
    TowerDegree,
    TowerLevels,
    TowerReductions,
    Transitions,
    Vertices,
    WitnessBytes,
    Work,
}
impl EvidenceResource {
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::ArithmeticSteps => "arithmetic-steps",
            Self::BoundaryNodes => "boundary-nodes",
            Self::CaseArms => "case-arms",
            Self::CoefficientMaps => "coefficient-maps",
            Self::CoefficientOps => "coefficient-ops",
            Self::ComponentBits => "component-bits",
            Self::ComponentSplits => "component-splits",
            Self::ConditionAtoms => "condition-atoms",
            Self::ConditionSteps => "condition-steps",
            Self::CostQuota => "cost-quota",
            Self::DecimalExponent => "decimal-exponent",
            Self::DeltaOrder => "delta-order",
            Self::DenominatorFactors => "denominator-factors",
            Self::Depth => "depth",
            Self::DiagnosticRecords => "diagnostic-records",
            Self::Edges => "edges",
            Self::ExpansionCredits => "expansion-credits",
            Self::FamilyParameters => "family-parameters",
            Self::FrontierLabels => "frontier-labels",
            Self::GcdSteps => "gcd-steps",
            Self::HypothesisDepth => "hypothesis-depth",
            Self::IntegerExponent => "integer-exponent",
            Self::IntermediateBits => "intermediate-bits",
            Self::IsolationSteps => "isolation-steps",
            Self::JetEntries => "jet-entries",
            Self::JetOrder => "jet-order",
            Self::LinearPivots => "linear-pivots",
            Self::LiteralBytes => "literal-bytes",
            Self::LocalBranches => "local-branches",
            Self::LocalContexts => "local-contexts",
            Self::MatchAttempts => "match-attempts",
            Self::MatchBatch => "match-batch",
            Self::MatrixEntries => "matrix-entries",
            Self::NameBytes => "name-bytes",
            Self::OrbitMembers => "orbit-members",
            Self::OrbitPermutations => "orbit-permutations",
            Self::OrbitTerms => "orbit-terms",
            Self::OriginalOccurrences => "original-occurrences",
            Self::OutputBytes => "output-bytes",
            Self::PairingSteps => "pairing-steps",
            Self::PayloadBytes => "payload-bytes",
            Self::PeakExpressionSize => "peak-expression-size",
            Self::PolynomialDegree => "polynomial-degree",
            Self::PolynomialTerms => "polynomial-terms",
            Self::PolynomialVariables => "polynomial-variables",
            Self::ProgressQuota => "progress-quota",
            Self::ProofNodes => "proof-nodes",
            Self::PullbackZeros => "pullback-zeros",
            Self::QueueEntries => "queue-entries",
            Self::RefinementPairs => "refinement-pairs",
            Self::RegionCells => "region-cells",
            Self::RegionNodes => "region-nodes",
            Self::ReplayWork => "replay-work",
            Self::ResolventDegree => "resolvent-degree",
            Self::ResolventDepth => "resolvent-depth",
            Self::SelectionSlots => "selection-slots",
            Self::SolutionBranches => "solution-branches",
            Self::SolveIdentities => "solve-identities",
            Self::SourceBytes => "source-bytes",
            Self::StagedEdges => "staged-edges",
            Self::StagedPayloadBytes => "staged-payload-bytes",
            Self::StagedVertices => "staged-vertices",
            Self::SymmetricSteps => "symmetric-steps",
            Self::TestNodes => "test-nodes",
            Self::TowerDegree => "tower-degree",
            Self::TowerLevels => "tower-levels",
            Self::TowerReductions => "tower-reductions",
            Self::Transitions => "transitions",
            Self::Vertices => "vertices",
            Self::WitnessBytes => "witness-bytes",
            Self::Work => "work",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExhaustionReason {
    Limit,
    ArithmeticRange,
    Allocation,
    Capacity,
}
impl ExhaustionReason {
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Limit => "limit",
            Self::ArithmeticRange => "arithmetic-range",
            Self::Allocation => "allocation",
            Self::Capacity => "capacity",
        }
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EvidenceExhaustion {
    pub resource: EvidenceResource,
    pub reason: ExhaustionReason,
    pub limit: u64,
    pub attempted: u64,
    pub operation: String,
    pub logical_id: Option<u64>,
}
impl EvidenceExhaustion {
    #[must_use]
    pub fn description(&self, stage: EvidenceStage) -> String {
        let location = self.logical_id.map_or_else(
            || self.operation.clone(),
            |id| format!("{} (logical id {id})", self.operation),
        );
        format!(
            "{} exhausted {} with limit {} and attempted {} at {location}",
            stage.token(),
            self.resource.token(),
            self.limit,
            self.attempted
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasonRole {
    Subject,
    Premise,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReasonPart {
    Text(String),
    Object { role: ReasonRole, id: NodeId },
}
#[must_use]
pub const fn verdict_word(value: VerdictClass) -> &'static str {
    match value {
        VerdictClass::Certified => "certified",
        VerdictClass::Refuted => "refuted",
        VerdictClass::Undecided => "undecided",
        VerdictClass::Deferred => "deferred",
    }
}
#[must_use]
pub fn deferred_parts(reason: DeferredReason) -> Vec<ReasonPart> {
    use ReasonPart::{Object, Text};
    vec![match reason {
        DeferredReason::AuditBinding => Text("audit binding".into()),
        DeferredReason::OutsideSlice => Text("outside slice".into()),
        DeferredReason::Capability(id) => Text(format!(
            "capability {}",
            match id {
                CapabilityId::ExactRationalLinearEquivalence => "exact-rational-linear-equivalence",
                CapabilityId::FormalTruncatedPowerSeries => "formal-truncated-power-series",
                CapabilityId::SymbolicTrigonometry => "symbolic-trigonometry",
                CapabilityId::TrigonometricIntegration => "trigonometric-integration",
            }
        )),
        DeferredReason::Premise { claim, evidence } => {
            return vec![
                Text("premise ".into()),
                Object {
                    role: ReasonRole::Premise,
                    id: claim,
                },
                Text(format!(" with evidence {}", verdict_word(evidence))),
            ];
        }
    }]
}
