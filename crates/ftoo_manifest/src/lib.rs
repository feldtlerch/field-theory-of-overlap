//! A closed Rust-data manifest and its structural validator.
//!
//! Validation establishes identity, relation contracts and conservative coverage,
//! not mathematical truth, operation typing or an audit verdict. The coefficient
//! seed is proposed trial content pending its prose. Bound claims still require an audit run.
//! There is no serialized boundary or calculation-engine dependency here.

mod audit;
mod budgets;
mod coverage;
mod graph;
mod identity;
mod seed;
mod status;
mod validation;

pub use coverage::{ClosedWorld, Coverage, ReleaseGate};
pub use graph::{
    Awaiting, DependencyOrderError, Edge, Guard, ManifestDraft, NodeDraft, NodePayload,
    RelationContract, RelationKind, ValidatedManifest,
};
pub use identity::{NodeId, NodeKind};
pub use seed::{seed, seed_draft};
pub use status::{Blocker, Calibration, Evidence, Identification, Maturity, Status};
pub use validation::{
    AdmissionStage, Diagnostic, DraftLocation, RULES, RuleId, RuleSpec, RuleState,
    ValidationErrors, validate,
};

#[cfg(test)]
mod tests;

pub use audit::{
    AuditRoutes, CAPABILITIES, CapabilityId, CapabilitySpec, ClaimAudit, ENGINE_PIN,
    EngineAvailability, EnginePin, ProducerDescriptor, ProducerId, RegionContract,
    adapter_implemented, registry_diagnostics,
};
pub use budgets::{BudgetId, BudgetProfile, BudgetSpec};
