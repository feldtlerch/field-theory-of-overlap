//! Owned diagnostic coordinates never infer locations from rendered messages.
use super::{RetentionError, WireRefusal};
use ftoo_manifest::{RuleId, WireLocation};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    Read,
    Syntax,
    Shape,
    Provenance,
    Reproduction,
    Assertions,
    Stability,
    Write,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Site {
    File(String),
    ByteOffset(usize),
    LineColumn {
        line: usize,
        column: usize,
        path: String,
    },
    JsonPath(String),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetentionDiagnostic {
    pub rule: RuleId,
    pub stage: Stage,
    pub site: Site,
    pub fault: RetentionError,
}
impl RetentionError {
    /// Complete owned diagnostic records, preserving nested existing faults.
    /// # Panics
    /// Panics only if the preceding exhaustive dispatch is inconsistent, which
    /// a caller cannot arrange.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<RetentionDiagnostic> {
        if let Self::Multiple(diagnostics) = self {
            return diagnostics.clone();
        }
        if let Self::Domain(errors) = self {
            return errors
                .iter()
                .map(|error| RetentionDiagnostic {
                    rule: error.rule,
                    stage: Stage::Provenance,
                    site: Site::File("generated/audit-report.json".into()),
                    fault: Self::Domain(vec![error.clone()]),
                })
                .collect();
        }
        let (rule, stage, site) = match self {
            Self::Wire(WireRefusal::Wire(error)) => wire_site(error),
            Self::Wire(error) => {
                let (rule, path) = match error {
                    WireRefusal::InvalidHex { path, .. }
                    | WireRefusal::InvalidRational { path, .. }
                    | WireRefusal::ArrayLength { path, .. } => (RuleId::M004, path.as_str()),
                    WireRefusal::Limit { path, .. } => (RuleId::M061, path.as_str()),
                    WireRefusal::NonCanonical { .. } => (RuleId::M061, "$"),
                    WireRefusal::Wire(_) => unreachable!("handled above"),
                };
                (rule, Stage::Shape, Site::JsonPath(path.into()))
            }
            Self::EngineRevisionUnavailable(error) => (
                RuleId::M061,
                Stage::Provenance,
                Site::File(error.path.into()),
            ),
            Self::ProvenanceMismatch {
                component, path, ..
            } => (
                component.rule(),
                Stage::Provenance,
                if path.starts_with('$') {
                    Site::JsonPath(path.clone())
                } else {
                    Site::File(path.clone())
                },
            ),
            Self::Projection { path, error } => (
                projection_rule(error),
                Stage::Provenance,
                if path.starts_with('$') {
                    Site::JsonPath(path.clone())
                } else {
                    Site::File(path.clone())
                },
            ),
            Self::StatusDomain { .. } => (
                RuleId::M012,
                Stage::Provenance,
                Site::JsonPath("$.statuses".into()),
            ),
            Self::StatusAssertion { index, .. } => (
                RuleId::M012,
                Stage::Assertions,
                Site::JsonPath(format!("$.statuses[{index}]")),
            ),
            Self::AssessmentAssertion { .. } => (
                RuleId::M015,
                Stage::Assertions,
                Site::JsonPath("$.assessment".into()),
            ),
            Self::Audit(error) => (
                audit_rule(error),
                Stage::Reproduction,
                Site::File("generated/audit-report.json".into()),
            ),
            Self::UnsupportedComponent { .. }
            | Self::NonReproducibility { .. }
            | Self::DiagnosticRerunFailed { .. } => (
                RuleId::M061,
                Stage::Reproduction,
                Site::File("generated/audit-report.json".into()),
            ),
            Self::Domain(_) | Self::Multiple(_) => unreachable!("handled above"),
        };
        vec![RetentionDiagnostic {
            rule,
            stage,
            site,
            fault: self.clone(),
        }]
    }
}
pub(super) fn projection_rule(error: &ftoo_manifest::ProjectionError) -> RuleId {
    use ftoo_manifest::ProjectionError;
    match error {
        ProjectionError::Wire(error) => error.rule,
        ProjectionError::Graph(errors) => errors
            .diagnostics()
            .first()
            .map_or(RuleId::M037, |error| error.rule),
        ProjectionError::Addressing(errors) => {
            errors.first().map_or(RuleId::M037, |error| error.rule)
        }
        ProjectionError::Assertion(errors) => {
            errors.first().map_or(RuleId::M037, |error| error.rule)
        }
    }
}
impl std::fmt::Display for RetentionDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fault.fmt(f)
    }
}

pub(super) fn domain(
    f: &mut std::fmt::Formatter<'_>,
    error: &ftoo_manifest::Diagnostic,
) -> std::fmt::Result {
    use ftoo_manifest::{AdmissionStage, DraftLocation, Guard};
    let q = |text: &str| serde_json::to_string(text).expect("strings serialize");
    let stage = match error.stage {
        AdmissionStage::GraphAdmission => "graph-admission",
        AdmissionStage::RegistryAdmission => "registry-admission",
        AdmissionStage::ReportConsumption => "report-consumption",
    };
    write!(
        f,
        "retention provenance {:?} at file \"generated/audit-report.json\": domain stage={stage} ids=[",
        error.rule
    )?;
    for (index, id) in error.ids.iter().enumerate() {
        if index != 0 {
            f.write_str(",")?;
        }
        f.write_str(&q(id.semantic_name()))?;
    }
    f.write_str("] edge=")?;
    match error.edge {
        Some(edge) => {
            let guard = match edge.guard {
                Guard::Always => "always",
            };
            write!(
                f,
                "{{\"relation\":{},\"source\":{},\"target\":{},\"guard\":{}}}",
                q(edge.relation.name()),
                q(edge.source.semantic_name()),
                q(edge.target.semantic_name()),
                q(guard)
            )?;
        }
        None => f.write_str("none")?,
    }
    f.write_str(" location=")?;
    match &error.location {
        DraftLocation::Node(index) => write!(f, "node({index})")?,
        DraftLocation::Edge(index) => write!(f, "edge({index})")?,
        DraftLocation::Slice(index) => write!(f, "slice({index})")?,
        DraftLocation::Coverage => f.write_str("coverage")?,
        DraftLocation::Binding(index) => write!(f, "binding({index})")?,
        DraftLocation::Budget(index) => write!(f, "budget({index})")?,
        DraftLocation::Capability(index) => write!(f, "capability({index})")?,
        DraftLocation::Verdict(index) => write!(f, "verdict({index})")?,
        DraftLocation::Provenance => f.write_str("provenance")?,
        DraftLocation::Cycle { relation, edges } => write!(
            f,
            "cycle(relation={},edges={})",
            q(relation.name()),
            serde_json::to_string(edges).expect("indices serialize")
        )?,
    }
    write!(f, " explanation={}", q(&error.explanation))
}

fn wire_site(error: &ftoo_manifest::WireError) -> (RuleId, Stage, Site) {
    let site = match &error.location {
        WireLocation::ByteOffset(offset) => Site::ByteOffset(*offset),
        WireLocation::LineColumn { line, column, path } => Site::LineColumn {
            line: *line,
            column: *column,
            path: path.clone(),
        },
        WireLocation::Path(path) => Site::JsonPath(path.clone()),
    };
    (
        error.rule,
        if matches!(error.rule, RuleId::M001 | RuleId::M002 | RuleId::M003) {
            Stage::Syntax
        } else {
            Stage::Shape
        },
        site,
    )
}

pub(super) fn audit_rule(error: &crate::AuditError) -> RuleId {
    use crate::{AuditError, RunCause, RunError};
    match error {
        AuditError::Input(errors)
        | AuditError::Report(errors)
        | AuditError::Execution(RunError::Admission(errors)) => {
            errors.first().map_or(RuleId::M020, |error| error.rule)
        }
        AuditError::Execution(RunError::Execution { cause, .. }) => match cause {
            RunCause::CapabilityUnavailable(_) => RuleId::M024,
            RunCause::MissingAcceptedPremise(_) => RuleId::M022,
            RunCause::Engine(_) => RuleId::M026,
            RunCause::ProjectionMismatch => RuleId::M025,
        },
    }
}
