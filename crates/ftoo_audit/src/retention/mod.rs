//! Retained audit wire assertions never construct trusted evidence.
mod acceptance;
mod diagnostic;
mod difference;
mod info;
mod production;
mod revision;
mod syntax;
mod wire;
const CONTRACT_REVISION: u32 = 1;

#[cfg(test)]
mod classifier_tests;
#[cfg(test)]
mod file_tests;
#[cfg(test)]
mod matrix_tests;
#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod revision_tests;
#[cfg(test)]
mod staleness_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod verdict_tests;
#[cfg(test)]
mod wire_tests;

pub use acceptance::{
    AcceptedRetention, Basis, Component, InputKey, Produced, ProvenanceDifference, ProvenanceValue,
    RetentionError, StatusAxis, accept, accept_after, check_current_engine, produce,
};
pub use diagnostic::{RetentionDiagnostic, Site, Stage};
pub use difference::{Difference, Slot, Window};
pub use info::{Closure, Compiler, Identity, InfoRecord, Package, Run, Unavailable};
pub use revision::{
    Reason as EngineRevisionReason, Source as EngineRevisionSource,
    Unavailable as EngineRevisionUnavailable,
};
pub use wire::{Assessment as AssessmentAssertion, Status as StatusAssertion};
pub use wire::{Rational as RationalAssertion, Summary as SummaryAssertion};

pub use wire::{Dimension, WireRefusal};
