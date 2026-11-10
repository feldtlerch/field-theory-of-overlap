//! A closed Rust-data manifest and its structural validator.
//!
//! Validation establishes identity, relation contracts and conservative coverage,
//! not mathematical truth, operation typing or an audit verdict. The coefficient
//! seed is proposed trial content pending its prose. Claims await audit binding.
//! There is no serialized boundary or calculation-engine dependency here.

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
pub use status::{Calibration, Evidence, Identification, Maturity, Status};
pub use validation::{
    Diagnostic, DraftLocation, RULES, RuleId, RuleSpec, RuleState, ValidationErrors, validate,
};

#[cfg(test)]
mod tests;
