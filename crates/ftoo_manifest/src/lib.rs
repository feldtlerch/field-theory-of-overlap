//! A closed Rust-data manifest and its structural validator.
//!
//! Validation establishes identity, relation contracts and coverage declarations,
//! not mathematical truth, operation typing or an audit verdict. The coefficient
//! seed is proposed trial content pending its prose. Bound claims still require an audit run.
//! The strict complete projection checks base assertions without an engine dependency.
//!
//! Addressing admits permanent opaque handles and append-only lifecycle history.
//! Live handles denote the declared claim slice and its dependency support;
//! they supply no certification, audit admission or release assessment. Address
//! books borrow the graph without changing its semantic snapshot. Preserving
//! descriptors and prior events across source revisions remains a review duty.
//! Projection loading checks the full history and all three store assertions,
//! retaining derived addressing beside the graph. Rendering and per-site build
//! refusal remain separate consumers.

mod assembly;
mod audit;
pub use assembly::*;
mod budgets;
mod coverage;
mod graph;
mod identity;
mod notation;
pub use notation::{
    AliasTargetReason, Callee, CheckedDefinition, CheckedExpression, CheckedNotation,
    DefinitionCategory, DefinitionId, DefinitionKind, DefinitionShapeReason, DefinitionSpec,
    ExprDraft, LiteralReason, MAX_AST_NODES, MAX_DEPTH, MAX_EXPANSION_NODES, MAX_LITERAL_BYTES,
    MathSort, NotationDiagnostic, NotationErrors, NotationFault, NotationOrigin, NotationPathPart,
    NotationResource, NotationSite, NotationStage, PendingReason, PrimitiveId, PrimitiveOperation,
    PrimitiveSpec, RegistryEntry, Signature, SymbolLookupFault, SymbolStatus, SymbolTier,
    SymbolUse, check_expression, notation,
};
pub use notation::{RenderOwner, RenderTarget};
mod labels;
pub use labels::{
    CheckedMarkdown, LabelDiagnostic, LabelErrors, LabelFault, LabelIntegratedUnits,
    LabelOccurrence, LabelSite, LabelSourceError, LabelSourceFault, LabelStage, MarkdownSource,
    MintKind, MintOrigin, ReferenceDomain, check_markdown_sources, integrate_labels,
};
mod seed;
mod status;
mod tag_history;
mod tag_validation;
mod tags;
mod units;
mod weld;
pub use units::LabelSpanReason;
pub use weld::{
    LabelMember, LabelSide, LabelUseSite, ResolvedUnitCitation, TagMember, TagSide, WeldDiagnostic,
    WeldErrors, WeldFault, WeldStage, WeldedLanguage, WeldedUnit, WeldedUnits, weld_units,
};
mod validation;
mod wire;
pub use units::{
    CheckedText, CheckedUnit, DocumentDraft, DocumentEntryDraft, InsertionReason,
    LanguageCheckedUnits, LocatedEntryDraft, LocatedString, MathOccurrence, NameDomain, PathPart,
    ResolvedUnitTag, ResolvedUnits, ShapeKind, SourceAnchor, TagOccurrence, UnitDiagnostic,
    UnitDraft, UnitErrors, UnitFault, UnitLabel, UnitSite, UnitStage, admit_unit_drafts,
    resolve_unit_tags,
};

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
