//! Located failures of the joint label boundary.
use crate::{NodeId, RuleId, UnitSite};
use std::{fmt, ops::Range};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStage {
    Encoding,
    Markdown,
    Namespace,
    RustCitation,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelSite {
    Markdown {
        file: String,
        bytes: Range<usize>,
    },
    RustIdentity {
        file: String,
        identity: NodeId,
        field: String,
    },
    RustHistory {
        file: String,
        ordinal: u32,
        field: String,
    },
    Unit {
        unit: String,
        site: UnitSite,
    },
}
impl fmt::Display for LabelSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Markdown { file, bytes } => {
                write!(f, "markdown {file:?} bytes {}..{}", bytes.start, bytes.end)
            }
            Self::RustIdentity {
                file,
                identity,
                field,
            } => write!(
                f,
                "rust-identity {file:?} {} field {field:?}",
                identity.semantic_name()
            ),
            Self::RustHistory {
                file,
                ordinal,
                field,
            } => write!(f, "rust-history {file:?} event {ordinal} field {field:?}"),
            Self::Unit { unit, site } => write!(f, "unit {unit:?} at {site}"),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MintKind {
    RustIdentity,
    Markdown,
    UnitMetadata,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MintOrigin {
    pub kind: MintKind,
    pub site: LabelSite,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceDomain {
    MarkdownOnly,
    Joint,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelFault {
    InvalidUtf8 {
        offset: usize,
        error_len: Option<usize>,
    },
    Bom,
    DuplicateMint {
        label: String,
        expected: usize,
        actual: usize,
        origins: Vec<MintOrigin>,
    },
    UnresolvedMarkdownCitation {
        label: String,
        expected: usize,
        actual: usize,
        domain: ReferenceDomain,
    },
    UnresolvedRustCitation {
        label: String,
        expected: usize,
        actual: usize,
        domain: ReferenceDomain,
    },
}
impl fmt::Display for LabelFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelDiagnostic {
    pub rule: RuleId,
    pub stage: LabelStage,
    pub unit: Option<String>,
    pub language: Option<String>,
    pub site: LabelSite,
    pub fault: LabelFault,
}
impl fmt::Display for LabelDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} unit={:?} language={:?} at {}: {}",
            self.rule, self.stage, self.unit, self.language, self.site, self.fault
        )
    }
}
/// A nonempty, immutable ordered failure collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelErrors(pub(super) Vec<LabelDiagnostic>);
impl LabelErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[LabelDiagnostic] {
        &self.0
    }
}
impl fmt::Display for LabelErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.0 {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for LabelErrors {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelSourceFault {
    InvalidSourceName { value: String },
    DuplicateSource { file: String, positions: Vec<usize> },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelSourceError {
    Input(LabelSourceFault),
    Admission(LabelErrors),
}
impl fmt::Display for LabelSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(_) => write!(f, "{self:?}"),
            Self::Admission(errors) => write!(f, "{errors}"),
        }
    }
}
impl std::error::Error for LabelSourceError {}
