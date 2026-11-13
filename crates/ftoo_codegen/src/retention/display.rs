use super::{FileIdentity, IoFailure, Operation, RetentionCheckError};
use std::fmt;

fn q(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize")
}
impl Operation {
    const fn token(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Metadata => "metadata",
            Self::Create => "create",
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Sync => "sync",
            Self::Rename => "rename",
            Self::DirectorySync => "directory-sync",
            Self::Reread => "reread",
            Self::Remove => "remove",
        }
    }
}
impl fmt::Display for IoFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "path={} operation={} kind={} raw-os-error=",
            q(&self.path),
            self.operation.token(),
            q(&self.kind.to_string())
        )?;
        match self.raw_os_error {
            Some(code) => write!(f, "{code}"),
            None => f.write_str("none"),
        }
    }
}
impl fmt::Display for FileIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{length={},modified=", self.length)?;
        if let Some(modified) = self.modified {
            match modified.duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => write!(f, "{}.{:09}", duration.as_secs(), duration.subsec_nanos())?,
                Err(error) => write!(
                    f,
                    "-{}.{:09}",
                    error.duration().as_secs(),
                    error.duration().subsec_nanos()
                )?,
            }
        } else {
            f.write_str("none")?;
        }
        write!(
            f,
            ",device={},inode={},mode={},changed={}.{:09}}}",
            self.device, self.inode, self.mode, self.changed.0, self.changed.1
        )
    }
}
fn paths(values: &[String]) -> String {
    serde_json::to_string(values).expect("strings serialize")
}
fn release(assessment: &ftoo_audit::ReleaseAssessment, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    use ftoo_manifest::Disposition;
    let disposition = match assessment.disposition() {
        Disposition::Pass => "pass",
        Disposition::InputError => "input-error",
        Disposition::SemanticInvalidity => "semantic-invalidity",
        Disposition::MathematicalFailure => "mathematical-failure",
        Disposition::Incomplete => "incomplete",
    };
    write!(
        f,
        "retention assertions M015 at file \"generated/audit-report.json\": release-rejected disposition={disposition} exit-code={}",
        assessment.exit_code()
    )?;
    if !assessment.diagnostics().is_empty() {
        write!(
            f,
            "\n{}",
            ftoo_audit::retention::RetentionError::Domain(assessment.diagnostics().to_vec())
        )?;
    }
    Ok(())
}
impl fmt::Display for RetentionCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Projection(error) => {
                f.write_str("retention provenance")?;
                if let Some(rule) = projection_rule(error) {
                    write!(f, " {rule:?}")?;
                }
                write!(f, " at file \"generated/manifest.json\": {error}")
            }
            Self::Audit(error) => error.fmt(f),
            Self::Documents(error) => write!(
                f,
                "retention assertions at file \"generated/documents\": {error}"
            ),
            Self::Io {
                primary,
                cleanup,
                replaced,
            } => {
                write!(
                    f,
                    "retention {} M062 at file {}: io {primary} replaced={}",
                    if matches!(
                        primary.operation,
                        Operation::Read | Operation::Metadata | Operation::Reread
                    ) {
                        "read"
                    } else {
                        "write"
                    },
                    q(&primary.path),
                    paths(replaced)
                )?;
                for failure in cleanup {
                    write!(f, "\ncleanup {failure}")?;
                }
                Ok(())
            }
            Self::NotRegular { path, identity } => write!(
                f,
                "retention read M062 at file {}: not-regular identity={identity}",
                q(path)
            ),
            Self::InputChanged {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "retention stability M062 at file {}: input-changed expected={expected} actual=",
                    q(path)
                )?;
                match actual {
                    Some(identity) => identity.fmt(f),
                    None => f.write_str("missing"),
                }
            }
            Self::Limit {
                path,
                limit,
                actual,
            } => write!(
                f,
                "retention read M061 at file {}: limit dimension=input-bytes limit={limit} actual={actual}",
                q(path)
            ),
            Self::ReleaseRejected(assessment) => release(assessment, f),
            Self::AfterReplacement { error, replaced } => {
                write!(f, "{error}\nreplaced={}", paths(replaced))
            }
            Self::Cleanup { error, failures } => {
                error.fmt(f)?;
                for failure in failures {
                    write!(f, "\ncleanup {failure}")?;
                }
                Ok(())
            }
        }
    }
}
impl std::error::Error for RetentionCheckError {}

fn projection_rule(error: &crate::ProjectionCheckError) -> Option<ftoo_manifest::RuleId> {
    use crate::ProjectionCheckError;
    use ftoo_manifest::ProjectionError;
    match error {
        ProjectionCheckError::SourceGraph(errors)
        | ProjectionCheckError::Projection(ProjectionError::Graph(errors)) => {
            errors.diagnostics().first().map(|error| error.rule)
        }
        ProjectionCheckError::Registry(errors) => errors.first().map(|error| error.rule),
        ProjectionCheckError::Projection(ProjectionError::Wire(error)) => Some(error.rule),
        ProjectionCheckError::Projection(ProjectionError::Addressing(errors)) => {
            errors.first().map(|error| error.rule)
        }
        ProjectionCheckError::Projection(ProjectionError::Assertion(errors)) => {
            errors.first().map(|error| error.rule)
        }
        ProjectionCheckError::ByteGate { rule, .. } => Some(*rule),
        ProjectionCheckError::AfterReplacement(error) => projection_rule(error),
        ProjectionCheckError::MissingArtifact { .. } | ProjectionCheckError::Io { .. } => None,
    }
}
