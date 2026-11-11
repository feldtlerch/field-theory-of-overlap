use crate::{NotationErrors, RuleId, UnitSite};
use std::{fmt, ops::Range};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DocumentTarget {
    Markdown,
    Latex,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AssemblyStage {
    Source,
    Shape,
    Ownership,
    Math,
    Emission,
    Native,
    Bytes,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyResource {
    SourceBytes,
    SkeletonDepth,
    UnitPositions,
    MathBytes,
    MathTokens,
    MathDepth,
    MathNodes,
    DocumentBytes,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathSyntaxReason {
    UnexpectedToken,
    ExpectedAtom,
    EmptyGroup,
    UnclosedGroup,
    MismatchedCloser,
    RepeatedSubscript,
    RepeatedSuperscript,
    ChainedEquality,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathUseSite {
    pub language: String,
    pub target: Option<DocumentTarget>,
    pub site: UnitSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOrigin {
    pub unit: Option<String>,
    pub language: Option<String>,
    pub target: Option<DocumentTarget>,
    pub site: UnitSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssemblySite {
    Source(UnitSite),
    Math {
        definition: UnitSite,
        key: String,
        bytes: Range<usize>,
        uses: Vec<MathUseSite>,
    },
    Generated {
        path: String,
        bytes: Range<usize>,
        origins: Vec<SourceOrigin>,
    },
    Inventory {
        path: String,
    },
}
impl fmt::Display for AssemblySite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssemblyInputFault {
    Limit {
        resource: AssemblyResource,
        limit: usize,
        actual: usize,
    },
    UnknownToken {
        value: String,
        expected: Vec<String>,
    },
    EmptySkeleton {
        document: String,
    },
    SkeletonMultiplicity {
        document: String,
        expected: usize,
        actual: usize,
        sites: Vec<UnitSite>,
    },
    UnknownSkeletonDocument {
        name: String,
        available: Vec<String>,
    },
    UnknownUnit {
        label: String,
        available: Vec<String>,
    },
    UnitMultiplicity {
        label: String,
        expected: usize,
        actual: usize,
        sites: Vec<UnitSite>,
    },
    WrongOwner {
        label: String,
        expected: String,
        actual: String,
        declared_at: UnitSite,
    },
    UnknownMathCommand {
        command: String,
    },
    MathArguments {
        command: String,
        expected: u8,
        actual: usize,
        argument: usize,
    },
    MathSyntax {
        reason: MathSyntaxReason,
        token: String,
        expected: Vec<String>,
    },
    Notation {
        cause: NotationErrors,
    },
}
impl fmt::Display for AssemblyInputFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyInputDiagnostic {
    pub rule: RuleId,
    pub stage: AssemblyStage,
    pub document: Option<String>,
    pub unit: Option<String>,
    pub language: Option<String>,
    pub target: Option<DocumentTarget>,
    pub site: AssemblySite,
    pub fault: AssemblyInputFault,
}
impl fmt::Display for AssemblyInputDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} document={:?} unit={:?} language={:?} target={:?} at {}: {}",
            self.rule,
            self.stage,
            self.document,
            self.unit,
            self.language,
            self.target,
            self.site,
            self.fault
        )
    }
}
pub(super) fn sort(diagnostics: &mut [AssemblyInputDiagnostic]) {
    fn location(site: &AssemblySite) -> (&str, &[crate::PathPart], Option<usize>) {
        match site {
            AssemblySite::Source(s) => (
                s.file.as_str(),
                &s.path,
                s.decoded.as_ref().map(|r| r.start),
            ),
            AssemblySite::Math {
                definition, bytes, ..
            } => (
                definition.file.as_str(),
                &definition.path,
                Some(bytes.start),
            ),
            AssemblySite::Generated { path, bytes, .. } => (path, &[], Some(bytes.start)),
            AssemblySite::Inventory { path } => (path, &[], None),
        }
    }
    fn rank(fault: &AssemblyInputFault) -> u8 {
        match fault {
            AssemblyInputFault::Limit { .. } => 0,
            AssemblyInputFault::UnknownToken { .. } => 1,
            AssemblyInputFault::EmptySkeleton { .. } => 2,
            AssemblyInputFault::SkeletonMultiplicity { .. } => 3,
            AssemblyInputFault::UnknownSkeletonDocument { .. } => 4,
            AssemblyInputFault::UnknownUnit { .. } => 5,
            AssemblyInputFault::UnitMultiplicity { .. } => 6,
            AssemblyInputFault::WrongOwner { .. } => 7,
            AssemblyInputFault::UnknownMathCommand { .. } => 8,
            AssemblyInputFault::MathArguments { .. } => 9,
            AssemblyInputFault::MathSyntax { .. } => 10,
            AssemblyInputFault::Notation { .. } => 11,
        }
    }
    diagnostics.sort_by(|a, b| {
        (
            &a.stage,
            &a.rule,
            &a.document,
            &a.unit,
            &a.language,
            &a.target,
            location(&a.site),
            rank(&a.fault),
        )
            .cmp(&(
                &b.stage,
                &b.rule,
                &b.document,
                &b.unit,
                &b.language,
                &b.target,
                location(&b.site),
                rank(&b.fault),
            ))
    });
}
