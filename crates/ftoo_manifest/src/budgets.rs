//! Pin-specific immutable profiles, resolved and exhaustively checked by the adapter.
use crate::{ENGINE_PIN, EnginePin, ProducerId};
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BudgetId {
    CoefficientOneV1,
    CoefficientThreeV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetProfile {
    PinnedTrialLimitsV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetSpec {
    pub id: BudgetId,
    pub producer: ProducerId,
    pub profile: BudgetProfile,
    pub pin: EnginePin,
}
impl BudgetSpec {
    #[must_use]
    pub const fn reviewed(producer: ProducerId) -> Self {
        Self {
            id: producer.descriptor().budget,
            producer,
            profile: BudgetProfile::PinnedTrialLimitsV1,
            pin: ENGINE_PIN,
        }
    }
}

impl BudgetProfile {
    /// Complete-route measurement at `ENGINE_PIN`: 6,073,177 for both targets.
    /// Round up to 500,000 (6,500,000), then step by 500,000 until headroom is at least 25%.
    #[must_use]
    pub const fn work(self) -> u64 {
        match self {
            Self::PinnedTrialLimitsV1 => 8_000_000,
        }
    }
    /// Combined replay measurement: 3,465,362 for both targets.
    /// Round up to 500,000 (3,500,000), then step by 500,000 until headroom is at least 25%.
    #[must_use]
    pub const fn replay_work(self) -> u64 {
        match self {
            Self::PinnedTrialLimitsV1 => 4_500_000,
        }
    }
}

/// Complete immutable expansion of the reviewed profile at the engine pin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedLimits {
    arithmetic_steps: u64,
    boundary_nodes: u64,
    case_arms: u64,
    coefficient_maps: u64,
    coefficient_ops: u64,
    component_bits: u64,
    component_splits: u64,
    condition_atoms: u64,
    condition_steps: u64,
    cost_quota: u64,
    decimal_exponent: u64,
    delta_order: u64,
    denominator_factors: u64,
    depth: u64,
    diagnostic_records: u64,
    edges: u64,
    expansion_credits: u64,
    family_parameters: u64,
    frontier_labels: u64,
    gcd_steps: u64,
    hypothesis_depth: u64,
    integer_exponent: u64,
    intermediate_bits: u64,
    isolation_steps: u64,
    jet_entries: u64,
    jet_order: u64,
    linear_pivots: u64,
    literal_bytes: u64,
    local_branches: u64,
    local_contexts: u64,
    match_attempts: u64,
    match_batch: u64,
    matrix_entries: u64,
    name_bytes: u64,
    orbit_members: u64,
    orbit_permutations: u64,
    orbit_terms: u64,
    original_occurrences: u64,
    output_bytes: u64,
    pairing_steps: u64,
    payload_bytes: u64,
    peak_expression_size: u64,
    polynomial_degree: u64,
    polynomial_terms: u64,
    polynomial_variables: u64,
    progress_quota: u64,
    proof_nodes: u64,
    pullback_zeros: u64,
    queue_entries: u64,
    refinement_pairs: u64,
    region_cells: u64,
    region_nodes: u64,
    replay_work: u64,
    resolvent_degree: u64,
    resolvent_depth: u64,
    selection_slots: u64,
    solution_branches: u64,
    solve_identities: u64,
    source_bytes: u64,
    staged_edges: u64,
    staged_payload_bytes: u64,
    staged_vertices: u64,
    symmetric_steps: u64,
    test_nodes: u64,
    tower_degree: u64,
    tower_levels: u64,
    tower_reductions: u64,
    transitions: u64,
    vertices: u64,
    witness_bytes: u64,
    work: u64,
}
impl ResolvedLimits {
    #[must_use]
    pub const fn arithmetic_steps(self) -> u64 {
        self.arithmetic_steps
    }
    #[must_use]
    pub const fn boundary_nodes(self) -> u64 {
        self.boundary_nodes
    }
    #[must_use]
    pub const fn case_arms(self) -> u64 {
        self.case_arms
    }
    #[must_use]
    pub const fn coefficient_maps(self) -> u64 {
        self.coefficient_maps
    }
    #[must_use]
    pub const fn coefficient_ops(self) -> u64 {
        self.coefficient_ops
    }
    #[must_use]
    pub const fn component_bits(self) -> u64 {
        self.component_bits
    }
    #[must_use]
    pub const fn component_splits(self) -> u64 {
        self.component_splits
    }
    #[must_use]
    pub const fn condition_atoms(self) -> u64 {
        self.condition_atoms
    }
    #[must_use]
    pub const fn condition_steps(self) -> u64 {
        self.condition_steps
    }
    #[must_use]
    pub const fn cost_quota(self) -> u64 {
        self.cost_quota
    }
    #[must_use]
    pub const fn decimal_exponent(self) -> u64 {
        self.decimal_exponent
    }
    #[must_use]
    pub const fn delta_order(self) -> u64 {
        self.delta_order
    }
    #[must_use]
    pub const fn denominator_factors(self) -> u64 {
        self.denominator_factors
    }
    #[must_use]
    pub const fn depth(self) -> u64 {
        self.depth
    }
    #[must_use]
    pub const fn diagnostic_records(self) -> u64 {
        self.diagnostic_records
    }
    #[must_use]
    pub const fn edges(self) -> u64 {
        self.edges
    }
    #[must_use]
    pub const fn expansion_credits(self) -> u64 {
        self.expansion_credits
    }
    #[must_use]
    pub const fn family_parameters(self) -> u64 {
        self.family_parameters
    }
    #[must_use]
    pub const fn frontier_labels(self) -> u64 {
        self.frontier_labels
    }
    #[must_use]
    pub const fn gcd_steps(self) -> u64 {
        self.gcd_steps
    }
    #[must_use]
    pub const fn hypothesis_depth(self) -> u64 {
        self.hypothesis_depth
    }
    #[must_use]
    pub const fn integer_exponent(self) -> u64 {
        self.integer_exponent
    }
    #[must_use]
    pub const fn intermediate_bits(self) -> u64 {
        self.intermediate_bits
    }
    #[must_use]
    pub const fn isolation_steps(self) -> u64 {
        self.isolation_steps
    }
    #[must_use]
    pub const fn jet_entries(self) -> u64 {
        self.jet_entries
    }
    #[must_use]
    pub const fn jet_order(self) -> u64 {
        self.jet_order
    }
    #[must_use]
    pub const fn linear_pivots(self) -> u64 {
        self.linear_pivots
    }
    #[must_use]
    pub const fn literal_bytes(self) -> u64 {
        self.literal_bytes
    }
    #[must_use]
    pub const fn local_branches(self) -> u64 {
        self.local_branches
    }
    #[must_use]
    pub const fn local_contexts(self) -> u64 {
        self.local_contexts
    }
    #[must_use]
    pub const fn match_attempts(self) -> u64 {
        self.match_attempts
    }
    #[must_use]
    pub const fn match_batch(self) -> u64 {
        self.match_batch
    }
    #[must_use]
    pub const fn matrix_entries(self) -> u64 {
        self.matrix_entries
    }
    #[must_use]
    pub const fn name_bytes(self) -> u64 {
        self.name_bytes
    }
    #[must_use]
    pub const fn orbit_members(self) -> u64 {
        self.orbit_members
    }
    #[must_use]
    pub const fn orbit_permutations(self) -> u64 {
        self.orbit_permutations
    }
    #[must_use]
    pub const fn orbit_terms(self) -> u64 {
        self.orbit_terms
    }
    #[must_use]
    pub const fn original_occurrences(self) -> u64 {
        self.original_occurrences
    }
    #[must_use]
    pub const fn output_bytes(self) -> u64 {
        self.output_bytes
    }
    #[must_use]
    pub const fn pairing_steps(self) -> u64 {
        self.pairing_steps
    }
    #[must_use]
    pub const fn payload_bytes(self) -> u64 {
        self.payload_bytes
    }
    #[must_use]
    pub const fn peak_expression_size(self) -> u64 {
        self.peak_expression_size
    }
    #[must_use]
    pub const fn polynomial_degree(self) -> u64 {
        self.polynomial_degree
    }
    #[must_use]
    pub const fn polynomial_terms(self) -> u64 {
        self.polynomial_terms
    }
    #[must_use]
    pub const fn polynomial_variables(self) -> u64 {
        self.polynomial_variables
    }
    #[must_use]
    pub const fn progress_quota(self) -> u64 {
        self.progress_quota
    }
    #[must_use]
    pub const fn proof_nodes(self) -> u64 {
        self.proof_nodes
    }
    #[must_use]
    pub const fn pullback_zeros(self) -> u64 {
        self.pullback_zeros
    }
    #[must_use]
    pub const fn queue_entries(self) -> u64 {
        self.queue_entries
    }
    #[must_use]
    pub const fn refinement_pairs(self) -> u64 {
        self.refinement_pairs
    }
    #[must_use]
    pub const fn region_cells(self) -> u64 {
        self.region_cells
    }
    #[must_use]
    pub const fn region_nodes(self) -> u64 {
        self.region_nodes
    }
    #[must_use]
    pub const fn replay_work(self) -> u64 {
        self.replay_work
    }
    #[must_use]
    pub const fn resolvent_degree(self) -> u64 {
        self.resolvent_degree
    }
    #[must_use]
    pub const fn resolvent_depth(self) -> u64 {
        self.resolvent_depth
    }
    #[must_use]
    pub const fn selection_slots(self) -> u64 {
        self.selection_slots
    }
    #[must_use]
    pub const fn solution_branches(self) -> u64 {
        self.solution_branches
    }
    #[must_use]
    pub const fn solve_identities(self) -> u64 {
        self.solve_identities
    }
    #[must_use]
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }
    #[must_use]
    pub const fn staged_edges(self) -> u64 {
        self.staged_edges
    }
    #[must_use]
    pub const fn staged_payload_bytes(self) -> u64 {
        self.staged_payload_bytes
    }
    #[must_use]
    pub const fn staged_vertices(self) -> u64 {
        self.staged_vertices
    }
    #[must_use]
    pub const fn symmetric_steps(self) -> u64 {
        self.symmetric_steps
    }
    #[must_use]
    pub const fn test_nodes(self) -> u64 {
        self.test_nodes
    }
    #[must_use]
    pub const fn tower_degree(self) -> u64 {
        self.tower_degree
    }
    #[must_use]
    pub const fn tower_levels(self) -> u64 {
        self.tower_levels
    }
    #[must_use]
    pub const fn tower_reductions(self) -> u64 {
        self.tower_reductions
    }
    #[must_use]
    pub const fn transitions(self) -> u64 {
        self.transitions
    }
    #[must_use]
    pub const fn vertices(self) -> u64 {
        self.vertices
    }
    #[must_use]
    pub const fn witness_bytes(self) -> u64 {
        self.witness_bytes
    }
    #[must_use]
    pub const fn work(self) -> u64 {
        self.work
    }
    pub(crate) fn fields(self) -> [(&'static str, u64); 71] {
        [
            ("arithmetic-steps", self.arithmetic_steps),
            ("boundary-nodes", self.boundary_nodes),
            ("case-arms", self.case_arms),
            ("coefficient-maps", self.coefficient_maps),
            ("coefficient-ops", self.coefficient_ops),
            ("component-bits", self.component_bits),
            ("component-splits", self.component_splits),
            ("condition-atoms", self.condition_atoms),
            ("condition-steps", self.condition_steps),
            ("cost-quota", self.cost_quota),
            ("decimal-exponent", self.decimal_exponent),
            ("delta-order", self.delta_order),
            ("denominator-factors", self.denominator_factors),
            ("depth", self.depth),
            ("diagnostic-records", self.diagnostic_records),
            ("edges", self.edges),
            ("expansion-credits", self.expansion_credits),
            ("family-parameters", self.family_parameters),
            ("frontier-labels", self.frontier_labels),
            ("gcd-steps", self.gcd_steps),
            ("hypothesis-depth", self.hypothesis_depth),
            ("integer-exponent", self.integer_exponent),
            ("intermediate-bits", self.intermediate_bits),
            ("isolation-steps", self.isolation_steps),
            ("jet-entries", self.jet_entries),
            ("jet-order", self.jet_order),
            ("linear-pivots", self.linear_pivots),
            ("literal-bytes", self.literal_bytes),
            ("local-branches", self.local_branches),
            ("local-contexts", self.local_contexts),
            ("match-attempts", self.match_attempts),
            ("match-batch", self.match_batch),
            ("matrix-entries", self.matrix_entries),
            ("name-bytes", self.name_bytes),
            ("orbit-members", self.orbit_members),
            ("orbit-permutations", self.orbit_permutations),
            ("orbit-terms", self.orbit_terms),
            ("original-occurrences", self.original_occurrences),
            ("output-bytes", self.output_bytes),
            ("pairing-steps", self.pairing_steps),
            ("payload-bytes", self.payload_bytes),
            ("peak-expression-size", self.peak_expression_size),
            ("polynomial-degree", self.polynomial_degree),
            ("polynomial-terms", self.polynomial_terms),
            ("polynomial-variables", self.polynomial_variables),
            ("progress-quota", self.progress_quota),
            ("proof-nodes", self.proof_nodes),
            ("pullback-zeros", self.pullback_zeros),
            ("queue-entries", self.queue_entries),
            ("refinement-pairs", self.refinement_pairs),
            ("region-cells", self.region_cells),
            ("region-nodes", self.region_nodes),
            ("replay-work", self.replay_work),
            ("resolvent-degree", self.resolvent_degree),
            ("resolvent-depth", self.resolvent_depth),
            ("selection-slots", self.selection_slots),
            ("solution-branches", self.solution_branches),
            ("solve-identities", self.solve_identities),
            ("source-bytes", self.source_bytes),
            ("staged-edges", self.staged_edges),
            ("staged-payload-bytes", self.staged_payload_bytes),
            ("staged-vertices", self.staged_vertices),
            ("symmetric-steps", self.symmetric_steps),
            ("test-nodes", self.test_nodes),
            ("tower-degree", self.tower_degree),
            ("tower-levels", self.tower_levels),
            ("tower-reductions", self.tower_reductions),
            ("transitions", self.transitions),
            ("vertices", self.vertices),
            ("witness-bytes", self.witness_bytes),
            ("work", self.work),
        ]
    }
}
impl BudgetProfile {
    #[must_use]
    pub const fn resolved(self) -> ResolvedLimits {
        ResolvedLimits {
            arithmetic_steps: 500_000,
            boundary_nodes: 128,
            case_arms: 256,
            coefficient_maps: 256,
            coefficient_ops: 2_000_000,
            component_bits: 4096,
            component_splits: 128,
            condition_atoms: 256,
            condition_steps: 100_000,
            cost_quota: 4,
            decimal_exponent: 128,
            delta_order: 8,
            denominator_factors: 256,
            depth: 128,
            diagnostic_records: 128,
            edges: 200_000,
            expansion_credits: 20_000,
            family_parameters: 64,
            frontier_labels: 8192,
            gcd_steps: 100_000,
            hypothesis_depth: 32,
            integer_exponent: 256,
            intermediate_bits: 8192,
            isolation_steps: 100_000,
            jet_entries: 4096,
            jet_order: 8,
            linear_pivots: 256,
            literal_bytes: 256,
            local_branches: 64,
            local_contexts: 128,
            match_attempts: 50_000,
            match_batch: 32,
            matrix_entries: 4096,
            name_bytes: 256,
            orbit_members: 256,
            orbit_permutations: 40_320,
            orbit_terms: 262_144,
            original_occurrences: 10_000,
            output_bytes: 1_048_576,
            pairing_steps: 4096,
            payload_bytes: 16_777_216,
            peak_expression_size: 50_000,
            polynomial_degree: 64,
            polynomial_terms: 4096,
            polynomial_variables: 16,
            progress_quota: 1,
            proof_nodes: 20_000,
            pullback_zeros: 64,
            queue_entries: 16_384,
            refinement_pairs: 4096,
            region_cells: 256,
            region_nodes: 2048,
            replay_work: self.replay_work(),
            resolvent_degree: 32,
            resolvent_depth: 8,
            selection_slots: 64,
            solution_branches: 256,
            solve_identities: 8192,
            source_bytes: 262_144,
            staged_edges: 200_000,
            staged_payload_bytes: 16_777_216,
            staged_vertices: 50_000,
            symmetric_steps: 4096,
            test_nodes: 256,
            tower_degree: 256,
            tower_levels: 8,
            tower_reductions: 100_000,
            transitions: 20_000,
            vertices: 50_000,
            witness_bytes: 1_048_576,
            work: self.work(),
        }
    }
}
