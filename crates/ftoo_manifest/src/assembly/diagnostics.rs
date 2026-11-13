use crate::{NodeId, TagResolutionFault, UnitFault};
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
    DisplaySyntax,
    DisplayResolution,
    DisplayEquality,
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
    BodyBytes,
    BodyRows,
    BodyColumns,
    BodyOccurrences,
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
    ReaderIdentity {
        position: ReaderIdentityPosition,
        origins: Vec<SourceOrigin>,
    },
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReaderIdentityPosition {
    Citation {
        unit: String,
        span: usize,
        target: Option<DocumentTarget>,
    },
    Coverage {
        index: usize,
        object: crate::NodeId,
    },
    Pending {
        unit: String,
        definition: crate::DefinitionId,
        reason: usize,
    },
}
impl fmt::Display for AssemblySite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)] // Keep the specified complete typed payloads.
pub enum AssemblyInputFault {
    DocumentaryTagTarget {
        input: String,
        label: String,
    },
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
    DisplayMultiplicity {
        unit: String,
        expected: usize,
        actual: usize,
        sites: Vec<UnitSite>,
    },
    DisplayHeading {
        unit: String,
        display_at: UnitSite,
    },
    BodyLanguages {
        target: DocumentTarget,
        required: Vec<String>,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    DisplaySyntax {
        kind: DisplayKind,
        reason: DisplaySyntaxReason,
        token: String,
        expected: Vec<String>,
    },
    TableColumns {
        row: usize,
        expected: usize,
        actual: usize,
    },
    Coordinate {
        field: String,
        value: String,
        minimum: i32,
        maximum: i32,
    },
    DisplayInline {
        cause: UnitFault,
    },
    DisplayMathReference {
        name: String,
        available: Vec<String>,
    },
    DisplayTag {
        cause: TagResolutionFault,
    },
    DisplayCitation {
        label: String,
        span_index: usize,
        expected: usize,
        actual: usize,
    },
    DisplaySetMismatch {
        family: AddressFamily,
        reference: BodyAddressSide,
        actual: BodyAddressSide,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    Notation {
        cause: NotationErrors,
    },
}
impl fmt::Display for AssemblyInputFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::DisplayTag { cause } = self {
            return cause.fmt(f);
        }
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
            AssemblySite::ReaderIdentity { .. } => ("", &[], None),
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
            AssemblyInputFault::DisplayMultiplicity { .. } => 20,
            AssemblyInputFault::DisplayHeading { .. } => 21,
            AssemblyInputFault::BodyLanguages { .. } => 22,
            AssemblyInputFault::DisplaySyntax { .. } => 23,
            AssemblyInputFault::TableColumns { .. } => 24,
            AssemblyInputFault::Coordinate { .. } => 25,
            AssemblyInputFault::DisplayInline { .. } => 26,
            AssemblyInputFault::DisplayMathReference { .. } => 27,
            AssemblyInputFault::DisplayTag { .. } => 28,
            AssemblyInputFault::DocumentaryTagTarget { .. } => 35,
            AssemblyInputFault::DisplayCitation { .. } => 29,
            AssemblyInputFault::DisplaySetMismatch { .. } => 30,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayKind {
    Table,
    Figure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplaySyntaxReason {
    EmptyBody,
    BadDelimiter,
    MissingHeader,
    MissingAlignment,
    MissingRow,
    UnknownLayout,
    UnclosedLayout,
    TrailingContent,
    InvalidInteger,
    ZeroDirection,
    ForbiddenContent,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressFamily {
    Tags,
    Citations,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyAddressSide {
    pub target: DocumentTarget,
    pub language: String,
    pub site: UnitSite,
    pub members: Vec<BodyAddressMember>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyAddressMember {
    pub value: String,
    pub target: Option<NodeId>,
    pub sites: Vec<UnitSite>,
}
