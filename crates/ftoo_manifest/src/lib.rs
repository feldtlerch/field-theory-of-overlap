//! A closed Rust-data manifest and its structural validator.
//!
//! Validation establishes identity, relation contracts and coverage declarations,
//! not mathematical truth, operation typing or an audit verdict. The coefficient
//! seed is proposed trial content pending its prose. Bound claims still require an audit run.
//! The strict core projection checks base assertions without an engine dependency.
//!
//! Addressing admits permanent opaque handles and append-only lifecycle history.
//! Live handles denote the declared claim slice and its dependency support;
//! they supply no certification, audit admission or release assessment. Address
//! books borrow the graph without changing its semantic snapshot. Preserving
//! descriptors and prior events across source revisions remains a review duty.
//! Future projection consumers must check the full history and all three stores;
//! rendering and per-site build refusal remain separate consumers.

mod audit;
mod budgets;
mod coverage;
mod graph;
mod identity;
mod seed;
mod status;
mod tag_history;
mod tag_validation;
mod tags;
mod validation;
mod wire;

pub use wire::{
    BudgetExport, CapabilityExport, LoadedProjection, ProducerExport, ProjectionDiagnostic,
    ProjectionError, ProjectionFault, ProjectionValue, SourceDifference, WireError, WireFault,
    WireLocation, load_projection, projection_bytes,
};

pub use tag_history::{HistoryChange, HistoryEvent, RetirementCause};
pub use tag_validation::{
    AddressingDiagnostic, AddressingErrors, AddressingFault, AddressingLocation, AddressingStage,
    StoreState,
};
pub use tags::{
    AddressBook, OutOfSliceCause, OutOfSliceEntry, RetiredEntry, TagDescriptor, TagId,
    TagResolutionFault, addressing,
};

#[cfg(test)]
mod tag_tests;

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
pub use budgets::{BudgetId, BudgetProfile, BudgetSpec, ResolvedLimits};

pub use audit::{
    DeferredReason, OutcomeDraft, Snapshot, VerdictClass, expected_deferral, report_diagnostics,
};
pub use coverage::{AssessmentDraft, Disposition, assess_draft};
