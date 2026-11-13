//! Owned locations and faults for the authored unit boundary.
use crate::{RuleId, TagResolutionFault};
use std::{fmt, ops::Range};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PathPart {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceAnchor {
    Bytes(usize, usize),
    Parent(usize, usize),
    WholeFile,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitSite {
    pub file: String,
    pub path: Vec<PathPart>,
    pub anchor: SourceAnchor,
    pub decoded: Option<Range<usize>>,
    pub occurrence: Option<usize>,
}

impl UnitSite {
    #[must_use]
    pub fn child(&self, key: &str) -> Self {
        let mut site = self.clone();
        site.path.push(PathPart::Key(key.to_owned()));
        site
    }
    #[must_use]
    pub fn parent(&self) -> Self {
        let mut site = self.clone();
        if let SourceAnchor::Bytes(a, b) = site.anchor {
            site.anchor = SourceAnchor::Parent(a, b);
        }
        site.decoded = None;
        site.occurrence = None;
        site
    }
}

impl fmt::Display for UnitSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} $", self.file)?;
        for part in &self.path {
            match part {
                PathPart::Key(key) => write!(f, "[{key:?}]")?,
                PathPart::Index(index) => write!(f, "[{index}]")?,
            }
        }
        match self.anchor {
            SourceAnchor::Bytes(a, b) => write!(f, " bytes {a}..{b}")?,
            SourceAnchor::Parent(a, b) => write!(f, " parent {a}..{b}")?,
            SourceAnchor::WholeFile => write!(f, " whole-file")?,
            SourceAnchor::Unavailable => write!(f, " unavailable")?,
        }
        if let Some(range) = &self.decoded {
            write!(f, " decoded {}..{}", range.start, range.end)?;
        }
        if let Some(index) = self.occurrence {
            write!(f, " occurrence {index}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitStage {
    Encoding,
    Syntax,
    Numeric,
    Shape,
    Metadata,
    Language,
    Content,
    Resolution,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeKind {
    Table,
    Array,
    String,
    Integer,
    Float,
    Boolean,
    Datetime,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameDomain {
    UnitLabel,
    DocumentName,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertionReason {
    UnknownForm,
    Unclosed,
    NestedBrace,
    BadMathName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitFault {
    UnknownToken {
        value: String,
        expected: Vec<String>,
    },
    DocumentaryTagTarget {
        input: String,
        label: String,
    },
    InvalidUtf8,
    Bom,
    TomlSyntax {
        message: String,
    },
    NonIntegerNumber {
        token: String,
    },
    SchemaValue {
        expected: String,
        actual: String,
    },
    UnknownField {
        name: String,
        expected: Vec<String>,
    },
    MissingField {
        name: String,
    },
    WrongType {
        expected: ShapeKind,
        actual: ShapeKind,
    },
    InvalidName {
        domain: NameDomain,
        value: String,
    },
    DuplicateUnit {
        id: String,
        sites: Vec<UnitSite>,
    },
    UnknownDocument {
        name: String,
        available: Vec<String>,
    },
    MissingMain,
    InvalidLanguage {
        value: String,
    },
    EmptyRequiredLanguages {
        document: String,
    },
    DuplicateRequiredLanguage {
        document: String,
        language: String,
        sites: Vec<UnitSite>,
    },
    MainLanguages {
        actual: Vec<String>,
    },
    MissingLanguage {
        document: String,
        required_at: UnitSite,
    },
    UndeclaredLanguage {
        document: String,
        required: Vec<String>,
    },
    BlankText,
    UnsupportedTextCharacter {
        value: String,
    },
    MalformedInsertion {
        reason: InsertionReason,
        fragment: String,
    },
    InvalidMathName {
        value: String,
    },
    BlankMath {
        name: String,
    },
    UnsupportedMathSyntax {
        name: String,
        token: String,
    },
    UnknownMath {
        name: String,
        available: Vec<String>,
    },
    UnusedMath {
        name: String,
    },
    ResolutionFault {
        cause: TagResolutionFault,
    },
    MalformedLabelSpan {
        span_index: usize,
        reason: LabelSpanReason,
        fragment: String,
    },
    TextMintForbidden {
        span_index: usize,
        label: String,
    },
}
impl UnitFault {
    fn rank(&self) -> usize {
        match self {
            Self::InvalidUtf8 => 0,
            Self::Bom => 1,
            Self::TomlSyntax { .. } => 2,
            Self::NonIntegerNumber { .. } => 3,
            Self::SchemaValue { .. } => 4,
            Self::UnknownField { .. } => 5,
            Self::MissingField { .. } => 6,
            Self::WrongType { .. } => 7,
            Self::InvalidName { .. } => 8,
            Self::DuplicateUnit { .. } => 9,
            Self::UnknownDocument { .. } => 10,
            Self::MissingMain => 11,
            Self::InvalidLanguage { .. } => 12,
            Self::EmptyRequiredLanguages { .. } => 13,
            Self::DuplicateRequiredLanguage { .. } => 14,
            Self::MainLanguages { .. } => 15,
            Self::MissingLanguage { .. } => 16,
            Self::UndeclaredLanguage { .. } => 17,
            Self::BlankText => 18,
            Self::UnsupportedTextCharacter { .. } => 19,
            Self::MalformedInsertion { .. } => 20,
            Self::InvalidMathName { .. } => 21,
            Self::BlankMath { .. } => 22,
            Self::UnsupportedMathSyntax { .. } => 23,
            Self::UnknownMath { .. } => 24,
            Self::UnusedMath { .. } => 25,
            Self::ResolutionFault { .. } => 26,
            Self::DocumentaryTagTarget { .. } => 50,
            Self::UnknownToken { .. } => 51,
            Self::MalformedLabelSpan { .. } => 27,
            Self::TextMintForbidden { .. } => 28,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelSpanReason {
    DelimiterWidth,
    MismatchedDelimiter,
    Unclosed,
    EmptyBody,
    InvalidLabel,
    ForbiddenCharacter,
}
impl fmt::Display for UnitFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResolutionFault { cause } => write!(f, "{cause}"),
            _ => write!(f, "{self:?}"),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitDiagnostic {
    pub rule: RuleId,
    pub stage: UnitStage,
    pub unit: Option<String>,
    pub language: Option<String>,
    pub site: UnitSite,
    pub fault: UnitFault,
}
impl fmt::Display for UnitDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} unit={:?} language={:?} at {}: {}",
            self.rule, self.stage, self.unit, self.language, self.site, self.fault
        )
    }
}
/// A nonempty, immutable ordered collection of admission faults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitErrors(Vec<UnitDiagnostic>);
impl UnitErrors {
    /// Preserve a byte phase's file order; an empty collection has no error.
    #[must_use]
    pub fn from_diagnostics(diagnostics: Vec<UnitDiagnostic>) -> Option<Self> {
        (!diagnostics.is_empty()).then_some(Self(diagnostics))
    }
    pub(super) fn semantic(mut diagnostics: Vec<UnitDiagnostic>) -> Option<Self> {
        diagnostics.sort_by(|a, b| {
            (
                &a.rule,
                &a.unit,
                &a.language,
                &a.site.file,
                &a.site.path,
                a.site.decoded.as_ref().map(|r| r.start),
                a.fault.rank(),
            )
                .cmp(&(
                    &b.rule,
                    &b.unit,
                    &b.language,
                    &b.site.file,
                    &b.site.path,
                    b.site.decoded.as_ref().map(|r| r.start),
                    b.fault.rank(),
                ))
        });
        Self::from_diagnostics(diagnostics)
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[UnitDiagnostic] {
        &self.0
    }
}
impl fmt::Display for UnitErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.0 {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for UnitErrors {}
