//! Diagnostic drafts from declared document source bytes.
//! These values are not admitted units or renderable document plans.
mod admission;
mod containers;
mod import;
#[cfg(test)]
mod tests;

use serde::Serialize;

pub use import::{LocatedPiece, located_inline};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Site {
    pub path: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockKind {
    Paragraph,
    Heading,
    TheoremContainer,
    StatementBox,
    EconomicsBox,
    EpistemologyBox,
    List,
    Table,
    Display,
    Refrain,
    Quotation,
    FormalNote,
    AuditNote,
    FrontMatter,
    Abstract,
    AppendixEvent,
    Contents,
    DerivedView,
    ExistingNativeInclusion,
    Refusal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Run {
    Text {
        decoded: String,
        spacing_intent: String,
    },
    Style {
        style: String,
        runs: Vec<Run>,
    },
    Math {
        local_key: String,
    },
    Tag {
        command: String,
        star: bool,
        key: String,
    },
    Handle {
        star: bool,
        selector: String,
    },
    Reference {
        command: String,
        star: bool,
        key: String,
    },
    Citation {
        command: String,
        ordered_keys: Vec<String>,
    },
    Virtue {
        command: String,
        word: String,
    },
    Unresolved {
        site: Site,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BlockFields {
    Paragraph {
        continuation: bool,
        layout: Vec<String>,
        runs: Vec<Run>,
    },
    Heading {
        level: String,
        star: bool,
        title: Vec<Run>,
        label_selector: Option<String>,
    },
    Table {
        columns: String,
        rows: Vec<Vec<TableCell>>,
        caption: Option<Vec<Run>>,
    },
    NativeInclusion {
        marker: String,
        unit: String,
        role: String,
    },
    Result {
        result_kind: ResultKind,
        header: ResultHeader,
        grade: Option<ResultGrade>,
        parts: Vec<ResultPart>,
    },
    Unresolved {
        production: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultKind {
    Insight,
    Remark,
    Definition,
}

impl ResultKind {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "insight" => Some(Self::Insight),
            "remark" => Some(Self::Remark),
            "definition" => Some(Self::Definition),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Insight => "insight",
            Self::Remark => "remark",
            Self::Definition => "definition",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "form", rename_all = "kebab-case")]
pub enum ResultHeader {
    Registered { key: String, site: Site },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResultGrade {
    pub code: ResultGradeCode,
    pub site: Site,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultGradeCode {
    Nec,
    ContAnch,
    Feas,
    ContComm,
    Cont,
}

impl ResultGradeCode {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nec" => Some(Self::Nec),
            "cont-anch" => Some(Self::ContAnch),
            "feas" => Some(Self::Feas),
            "cont-comm" => Some(Self::ContComm),
            "cont" => Some(Self::Cont),
            _ => None,
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Nec => "nec",
            Self::ContAnch => "cont-anch",
            Self::Feas => "feas",
            Self::ContComm => "cont-comm",
            Self::Cont => "cont",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultPartKind {
    Opening,
    Grade,
    GradeGap,
    Trivia,
    Child,
    Closing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResultPart {
    pub kind: ResultPartKind,
    pub site: Site,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TableCell {
    pub text: String,
    pub row_span: usize,
    pub column_span: usize,
    pub header: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockStatus {
    Decoded,
    Unresolved,
    ResolutionNeeded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BlockDraft {
    pub local_index: usize,
    pub site: Site,
    pub kind: BlockKind,
    pub fields: BlockFields,
    pub children: Vec<BlockDraft>,
    pub events: Vec<usize>,
    pub status: BlockStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EventDraft {
    pub kind: String,
    pub site: Site,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TargetDraft {
    pub selector: String,
    pub exact_key: String,
    pub source_command: String,
    pub site: Site,
    pub occurrence: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MathDraft {
    pub local_key: String,
    pub source: String,
    pub site: Site,
    pub parse_status: String,
    pub dependencies: Vec<String>,
    pub refusals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Refusal {
    pub outer_block_index: usize,
    pub is_primary: bool,
    pub code: String,
    pub site: Site,
    pub block_kind: BlockKind,
    pub command_or_environment: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "index", rename_all = "kebab-case")]
pub enum AccountOwner {
    Content(usize),
    Event(usize),
    ExistingNative(usize),
    Refusal(usize),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AccountPiece {
    pub start_byte: usize,
    pub end_byte: usize,
    pub owner: AccountOwner,
}

/// Captured source evidence for one outer prose paragraph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParagraphBoundary {
    pub block_index: usize,
    pub left: ParagraphEdge,
    pub right: ParagraphEdge,
    pub first_token: Option<String>,
    pub last_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParagraphEdge {
    Blank,
    HeadingTail,
    Heading,
    Unproved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceDraft {
    pub path: String,
    pub digest: String,
    pub document: String,
    pub language: String,
    pub proposed_unit_id: Option<String>,
    pub metadata_targets: Vec<TargetDraft>,
    pub math: Vec<MathDraft>,
    pub blocks: Vec<BlockDraft>,
    pub events: Vec<EventDraft>,
    pub accounting: Vec<AccountPiece>,
    pub paragraph_boundaries: Vec<ParagraphBoundary>,
    pub refusals: Vec<Refusal>,
    pub byte_length: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CorpusDraft {
    pub schema: u32,
    pub source_order: Vec<String>,
    pub source_list_digest: String,
    pub sources: Vec<SourceDraft>,
    pub transfer_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportFault {
    Encoding {
        path: String,
    },
    Accounting {
        path: String,
        at: usize,
        reason: &'static str,
    },
    InputChanged {
        path: String,
    },
}
impl std::fmt::Display for ImportFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ImportFault {}

pub use admission::{
    CheckedSlot, SlotBlock, SlotDraft, SlotFault, SlotInterval, SlotIntervalBlock, SlotPaperMint,
    SlotRun, SlotStyle, admit_slot, admitted_intervals, check_transfer,
};
pub use import::{ensure_unchanged, import_source, validate_accounting};
