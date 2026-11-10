//! Engine-backed audited evidence and release assessment for finite Langevin jets.
//! ```
//! let manifest = ftoo_manifest::seed()?;
//! let report = ftoo_audit::audit(&manifest)?;
//! let status = report.status(ftoo_manifest::NodeId::LangevinCoefficientOneValue).unwrap();
//! assert_eq!(status.evidence(),ftoo_audit::AuditedEvidence::Certified);
//! assert_eq!(status.base().evidence(),ftoo_manifest::Evidence::Deferred);
//! assert_eq!(report.assessment().exit_code(),0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
mod engine;
mod registry;
mod report;
mod runner;
mod theory;
pub use report::{
    AuditReport, AuditedEvidence, AuditedStatus, ClaimOutcome, Counterstatement, GatedReport,
    JetSummary, Measurement, Receipt, ReleaseAssessment, Verdict,
};
#[cfg(test)]
use runner::run;
pub use runner::{AuditError, RunCause, RunError, RunStage, audit};
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
