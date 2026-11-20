//! Closed identities for the paper and explainer's checked reading occurrences.
//!
//! Allocations name existing passages and quotations without assigning an outline,
//! a mathematical verdict, or a new source of document order.

mod construction;
mod diagnostics;
mod explainer;
mod identity;
mod paper;
#[cfg(test)]
mod tests;

pub use construction::{CheckedReadingStructure, ReadingDeclaration, catalogue};
pub use diagnostics::{ReadingDiagnostic, ReadingErrors, ReadingFault, ReadingStage};
pub use identity::{ReadingDocument, ReadingKey, ReadingRole};
