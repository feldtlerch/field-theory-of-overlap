use crate::{LabelFileError, NotationRenderError, ProjectionCheckError, UnitLoadError};
use ftoo_manifest::{
    AddressFamily, BodyAddressSide, DisplayKind, DisplaySyntaxReason, TagResolutionFault,
};
use ftoo_manifest::{AssemblyResource, MathSyntaxReason};
use ftoo_manifest::{
    AssemblySite, AssemblyStage, DocumentTarget, ProjectionError, RenderOwner, RuleId, UnitErrors,
    UnitFault, UnitSite, WeldErrors,
};
use std::{fmt, io, ops::Range, path::PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)] // The public fault retains both complete address sides.
pub enum AssemblyFault {
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
    MathNotation {
        cause: NotationRenderError,
    },
    EmissionMismatch {
        missing: Vec<EmissionRecord>,
        unexpected: Vec<EmissionRecord>,
        changed: Vec<EmissionChange>,
    },
    GeneratedAddressSyntax {
        path: String,
        mints: Vec<GeneratedLabelOccurrence>,
        citations: Vec<GeneratedLabelOccurrence>,
    },
    RuntimeTagRejected {
        input: String,
        site_token: String,
        message: String,
    },
    NativeInventory {
        missing: Vec<NativeCase>,
        repeated: Vec<(NativeCase, usize)>,
        unexpected: Vec<NativeCase>,
    },
    NativeRejected {
        case: NativeCase,
        status: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        log: Vec<u8>,
    },
    ArtifactInventory {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    NonCanonicalDocument {
        path: String,
        offset: usize,
        diff: String,
    },
}
impl fmt::Display for AssemblyFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::DisplayTag { cause } = self {
            return cause.fmt(f);
        }
        write!(f, "{self:?}")
    }
}
impl AssemblyFault {
    fn rank(&self) -> u8 {
        match self {
            Self::Limit { .. } => 0,
            Self::UnknownToken { .. } => 1,
            Self::EmptySkeleton { .. } => 2,
            Self::SkeletonMultiplicity { .. } => 3,
            Self::UnknownSkeletonDocument { .. } => 4,
            Self::UnknownUnit { .. } => 5,
            Self::UnitMultiplicity { .. } => 6,
            Self::WrongOwner { .. } => 7,
            Self::UnknownMathCommand { .. } => 8,
            Self::MathArguments { .. } => 9,
            Self::MathSyntax { .. } => 10,
            Self::MathNotation { .. } => 11,
            Self::EmissionMismatch { .. } => 12,
            Self::GeneratedAddressSyntax { .. } => 13,
            Self::RuntimeTagRejected { .. } => 14,
            Self::NativeInventory { .. } => 15,
            Self::NativeRejected { .. } => 16,
            Self::ArtifactInventory { .. } => 17,
            Self::NonCanonicalDocument { .. } => 18,
            Self::DisplayMultiplicity { .. } => 20,
            Self::DisplayHeading { .. } => 21,
            Self::BodyLanguages { .. } => 22,
            Self::DisplaySyntax { .. } => 23,
            Self::TableColumns { .. } => 24,
            Self::Coordinate { .. } => 25,
            Self::DisplayInline { .. } => 26,
            Self::DisplayMathReference { .. } => 27,
            Self::DisplayTag { .. } => 28,
            Self::DisplayCitation { .. } => 29,
            Self::DisplaySetMismatch { .. } => 30,
        }
    }
}
fn location(site: &AssemblySite) -> (&str, &[ftoo_manifest::PathPart], Option<usize>) {
    match site {
        AssemblySite::Source(site) => (
            site.file.as_str(),
            &site.path,
            site.decoded.as_ref().map(|r| r.start),
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
impl From<ftoo_manifest::AssemblyInputFault> for AssemblyFault {
    #[allow(clippy::too_many_lines)] // Exhaustive lossless conversion of the specified fault family.
    fn from(value: ftoo_manifest::AssemblyInputFault) -> Self {
        use ftoo_manifest::AssemblyInputFault as F;
        match value {
            F::DisplayMultiplicity {
                unit,
                expected,
                actual,
                sites,
            } => Self::DisplayMultiplicity {
                unit,
                expected,
                actual,
                sites,
            },
            F::DisplayHeading { unit, display_at } => Self::DisplayHeading { unit, display_at },
            F::BodyLanguages {
                target,
                required,
                missing,
                unexpected,
            } => Self::BodyLanguages {
                target,
                required,
                missing,
                unexpected,
            },
            F::DisplaySyntax {
                kind,
                reason,
                token,
                expected,
            } => Self::DisplaySyntax {
                kind,
                reason,
                token,
                expected,
            },
            F::TableColumns {
                row,
                expected,
                actual,
            } => Self::TableColumns {
                row,
                expected,
                actual,
            },
            F::Coordinate {
                field,
                value,
                minimum,
                maximum,
            } => Self::Coordinate {
                field,
                value,
                minimum,
                maximum,
            },
            F::DisplayInline { cause } => Self::DisplayInline { cause },
            F::DisplayMathReference { name, available } => {
                Self::DisplayMathReference { name, available }
            }
            F::DisplayTag { cause } => Self::DisplayTag { cause },
            F::DisplayCitation {
                label,
                span_index,
                expected,
                actual,
            } => Self::DisplayCitation {
                label,
                span_index,
                expected,
                actual,
            },
            F::DisplaySetMismatch {
                family,
                reference,
                actual,
                missing,
                unexpected,
            } => Self::DisplaySetMismatch {
                family,
                reference,
                actual,
                missing,
                unexpected,
            },

            F::Limit {
                resource,
                limit,
                actual,
            } => Self::Limit {
                resource,
                limit,
                actual,
            },
            F::UnknownToken { value, expected } => Self::UnknownToken { value, expected },
            F::EmptySkeleton { document } => Self::EmptySkeleton { document },
            F::SkeletonMultiplicity {
                document,
                expected,
                actual,
                sites,
            } => Self::SkeletonMultiplicity {
                document,
                expected,
                actual,
                sites,
            },
            F::UnknownSkeletonDocument { name, available } => {
                Self::UnknownSkeletonDocument { name, available }
            }
            F::UnknownUnit { label, available } => Self::UnknownUnit { label, available },
            F::UnitMultiplicity {
                label,
                expected,
                actual,
                sites,
            } => Self::UnitMultiplicity {
                label,
                expected,
                actual,
                sites,
            },
            F::WrongOwner {
                label,
                expected,
                actual,
                declared_at,
            } => Self::WrongOwner {
                label,
                expected,
                actual,
                declared_at,
            },
            F::UnknownMathCommand { command } => Self::UnknownMathCommand { command },
            F::MathArguments {
                command,
                expected,
                actual,
                argument,
            } => Self::MathArguments {
                command,
                expected,
                actual,
                argument,
            },
            F::MathSyntax {
                reason,
                token,
                expected,
            } => Self::MathSyntax {
                reason,
                token,
                expected,
            },
            F::Notation { cause } => Self::MathNotation {
                cause: NotationRenderError::Admission(cause),
            },
        }
    }
}
impl From<ftoo_manifest::AssemblyInputDiagnostic> for AssemblyDiagnostic {
    fn from(d: ftoo_manifest::AssemblyInputDiagnostic) -> Self {
        Self {
            rule: d.rule,
            stage: d.stage,
            document: d.document,
            unit: d.unit,
            language: d.language,
            target: d.target,
            site: d.site,
            fault: d.fault.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyDiagnostic {
    pub rule: RuleId,
    pub stage: AssemblyStage,
    pub document: Option<String>,
    pub unit: Option<String>,
    pub language: Option<String>,
    pub target: Option<DocumentTarget>,
    pub site: AssemblySite,
    pub fault: AssemblyFault,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyErrors(Vec<AssemblyDiagnostic>);
impl AssemblyErrors {
    #[must_use]
    pub fn from_diagnostics(mut diagnostics: Vec<AssemblyDiagnostic>) -> Option<Self> {
        diagnostics.sort_by(|a, b| {
            (
                &a.stage,
                &a.rule,
                &a.document,
                &a.unit,
                &a.language,
                &a.target,
            )
                .cmp(&(
                    &b.stage,
                    &b.rule,
                    &b.document,
                    &b.unit,
                    &b.language,
                    &b.target,
                ))
                .then_with(|| location(&a.site).cmp(&location(&b.site)))
                .then_with(|| a.fault.rank().cmp(&b.fault.rank()))
        });
        (!diagnostics.is_empty()).then_some(Self(diagnostics))
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[AssemblyDiagnostic] {
        &self.0
    }
}
impl fmt::Display for AssemblyDiagnostic {
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
impl fmt::Display for AssemblyErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in &self.0 {
            writeln!(f, "{error}")?;
        }
        Ok(())
    }
}
impl std::error::Error for AssemblyErrors {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmissionKind {
    Heading,
    Paragraph,
    Caption,
    Body,
    Tag,
    Citation,
    Mention,
    Math,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmissionRecord {
    pub kind: EmissionKind,
    pub unit: String,
    pub language: String,
    pub target: DocumentTarget,
    pub occurrence: Option<usize>,
    pub value: String,
    pub site: UnitSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmissionChange {
    pub expected: EmissionRecord,
    pub actual: EmissionRecord,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedLabelOccurrence {
    pub label: String,
    pub bytes: Range<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeCase {
    Document { document: String, language: String },
    Sweep { owner: RenderOwner, case: String },
    Collision { command: String },
    Runtime { site_token: String },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentEntryKind {
    Directory,
    Symlink,
    NonTomlFile,
    NonUtf8Name,
    Other,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentInputFault {
    InvalidSourceName {
        value: String,
    },
    DuplicateSource {
        file: String,
        positions: Vec<usize>,
    },
    EmptySkeletons {
        directory: String,
    },
    UnexpectedEntry {
        path: PathBuf,
        kind: DocumentEntryKind,
    },
    SourceChanged {
        file: String,
        expected: Vec<u8>,
        actual: Option<Vec<u8>>,
    },
    SourceInventoryChanged {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    DisplaysUnsupported {
        files: Vec<String>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentIoOperation {
    Read,
    List,
    Inspect,
    CreateDirectory,
    CreateTemporary,
    Write,
    Rename,
    Remove,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IoFailure {
    pub operation: DocumentIoOperation,
    pub path: PathBuf,
    pub kind: io::ErrorKind,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentError {
    Projection(ProjectionCheckError),
    ProjectionLoad(ProjectionError),
    Units(UnitLoadError),
    Labels(LabelFileError),
    Weld(WeldErrors),
    Notation(NotationRenderError),
    Boundary(UnitErrors),
    Assembly(AssemblyErrors),
    Input(DocumentInputFault),
    Native(NativeAdapterFault),
    Io {
        operation: DocumentIoOperation,
        path: PathBuf,
        kind: io::ErrorKind,
        replaced: Vec<String>,
        removed: Vec<String>,
        cleanup: Vec<IoFailure>,
    },
}
impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Projection(e) => e.fmt(f),
            Self::ProjectionLoad(e) => e.fmt(f),
            Self::Units(e) => e.fmt(f),
            Self::Labels(e) => e.fmt(f),
            Self::Weld(e) => e.fmt(f),
            Self::Notation(e) => e.fmt(f),
            Self::Boundary(e) => e.fmt(f),
            Self::Assembly(e) => e.fmt(f),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for DocumentError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeComponent {
    Executable,
    Bundle,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeOperation {
    Inspect,
    Read,
    Spawn,
    WriteInput,
    Wait,
    ReadOutput,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeConfigurationReason {
    Missing,
    RelativeSupport {
        value: PathBuf,
    },
    OverlappingSupport {
        repository: PathBuf,
        support: PathBuf,
    },
    UnsupportedPlatform {
        os: String,
        architecture: String,
    },
    NonRegularEntry {
        kind: DocumentEntryKind,
    },
    EscapingEntry {
        resolved: PathBuf,
    },
    UnpinnedArtifact {
        url: String,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProtocolReason {
    InvalidRuntimeRecord,
    InvalidUtf8,
    UnexpectedStatus,
    MissingOutput,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeMetadataReason {
    Boundary(UnitFault),
    InvalidValue {
        expected: String,
        actual: String,
    },
    DuplicateMember {
        member: String,
        sites: Vec<UnitSite>,
    },
    OutOfOrder {
        previous: String,
        current: String,
    },
    Limit {
        resource: String,
        maximum: usize,
        actual: usize,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatPhase {
    Seed,
    BeforeCase,
    AfterCase,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatChange {
    pub member: String,
    pub expected: Vec<u8>,
    pub actual: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeAdapterFault {
    FormatState {
        phase: FormatPhase,
        missing: Vec<String>,
        unexpected: Vec<String>,
        changed: Vec<FormatChange>,
    },
    SupportChanged {
        member: String,
        expected: crate::SupportIdentity,
        actual: Option<crate::SupportIdentity>,
    },
    Configuration {
        component: Option<NativeComponent>,
        path: Option<PathBuf>,
        reason: NativeConfigurationReason,
    },
    Integrity {
        component: NativeComponent,
        member: String,
        expected: String,
        actual: String,
    },
    Inventory {
        component: NativeComponent,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    Metadata {
        site: UnitSite,
        reason: NativeMetadataReason,
    },
    VersionMismatch {
        expected: String,
        actual: Vec<u8>,
    },
    Io {
        operation: NativeOperation,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    Protocol {
        reason: NativeProtocolReason,
        status: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        log: Vec<u8>,
    },
}
impl From<NativeAdapterFault> for DocumentError {
    fn from(value: NativeAdapterFault) -> Self {
        Self::Native(value)
    }
}
impl From<crate::NotationAdapterFault> for DocumentError {
    fn from(value: crate::NotationAdapterFault) -> Self {
        Self::Notation(crate::NotationRenderError::Adapter(value))
    }
}
impl From<NotationRenderError> for DocumentError {
    fn from(value: NotationRenderError) -> Self {
        Self::Notation(value)
    }
}
