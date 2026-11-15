//! Located failures of open-question store admission.
use super::{QuestionStateKind, ReferenceKind};
use crate::{RuleId, UnitSite};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionStage {
    Store,
    References,
    Lifecycle,
}
/// The authored texts of one record.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionText {
    Title,
    Question,
    SettledWhen,
    Answer,
    WithdrawnBecause,
}
impl QuestionText {
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Question => "question",
            Self::SettledWhen => "settled-when",
            Self::Answer => "answer",
            Self::WithdrawnBecause => "withdrawn-because",
        }
    }
}
/// The reference lists of one record.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionList {
    About,
    Blocks,
    RestsOn,
    SettledBy,
    SucceededBy,
}
impl QuestionList {
    #[must_use]
    pub const fn field(self) -> &'static str {
        match self {
            Self::About => "about",
            Self::Blocks => "blocks",
            Self::RestsOn => "rests-on",
            Self::SettledBy => "settled-by",
            Self::SucceededBy => "succeeded-by",
        }
    }
}
/// Why an authored text is not one admissible sentence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextReason {
    Blank,
    Padded,
    ControlCharacter,
    ForbiddenCharacter,
    Terminal,
    SeveralSentences,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuestionFault {
    InvalidLabel {
        label: String,
    },
    DuplicateLabel {
        label: String,
        first: usize,
    },
    LabelOrder {
        label: String,
        previous: String,
    },
    Text {
        text: QuestionText,
        value: String,
        reason: TextReason,
    },
    DuplicateTitle {
        title: String,
        first: usize,
    },
    DuplicateQuestion {
        question: String,
        first: usize,
    },
    EmptyList {
        list: QuestionList,
    },
    RepeatedReference {
        list: QuestionList,
        kind: ReferenceKind,
        value: String,
    },
    UnresolvedReference {
        list: QuestionList,
        kind: ReferenceKind,
        value: String,
    },
    MalformedPath {
        list: QuestionList,
        value: String,
    },
    SelfReference {
        list: QuestionList,
    },
    OpenInBlocks {
        value: String,
    },
    RestsOnClosed {
        label: String,
        state: QuestionStateKind,
    },
    Cycle {
        list: QuestionList,
        labels: Vec<String>,
    },
    SettlementKind {
        kind: ReferenceKind,
    },
    SuccessionKind {
        kind: ReferenceKind,
    },
}
impl fmt::Display for QuestionFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionDiagnostic {
    pub rule: RuleId,
    pub stage: QuestionStage,
    pub site: UnitSite,
    pub fault: QuestionFault,
}
impl fmt::Display for QuestionDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} at {}: {}",
            self.rule, self.stage, self.site, self.fault
        )
    }
}
/// A nonempty, immutable ordered collection of store admission faults.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionErrors(pub(super) Vec<QuestionDiagnostic>);
impl QuestionErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[QuestionDiagnostic] {
        &self.0
    }
    #[must_use]
    pub fn from_diagnostics(diagnostics: Vec<QuestionDiagnostic>) -> Option<Self> {
        (!diagnostics.is_empty()).then_some(Self(diagnostics))
    }
}
impl fmt::Display for QuestionErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.0 {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for QuestionErrors {}
