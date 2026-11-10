//! Closed manifest-bound execution of finite Langevin jets.
//! Receipts establish acceptance of the scoped identity-composition/residual
//! obligation, not an audited status, provenance report or release assessment.
//!
//! ```
//! let manifest = ftoo_manifest::seed()?;
//! let run = ftoo_audit::run(&manifest)?;
//! assert_eq!(run.receipts().len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
mod engine;
mod registry;
mod report;
mod runner;
mod theory;
pub use report::{JetSummary, Measurement, ProducerRun, Receipt};
pub use runner::{RunCause, RunError, RunStage, run};
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
