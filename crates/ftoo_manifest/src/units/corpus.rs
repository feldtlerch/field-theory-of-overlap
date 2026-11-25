//! Diagnostic drafts from declared document source bytes.
//! These values are not admitted units or renderable document plans.
mod import;
#[cfg(test)]
mod tests;

use serde::Serialize;

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
    Unresolved {
        production: String,
    },
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

pub use import::{ensure_unchanged, import_source, validate_accounting};
