//! Admission of a bounded ordered corpus slot and its independent source correspondence.

use super::{
    AccountOwner, BlockFields, BlockKind, BlockStatus, ImportFault, Run, SourceDraft,
    validate_accounting,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotStyle {
    Strong,
    Emphasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotRun {
    Text(String),
    Style {
        kind: SlotStyle,
        children: Vec<Self>,
    },
    EnSpace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotBlock {
    Heading {
        level: String,
        star: bool,
        title: Vec<SlotRun>,
        label: String,
    },
    Use {
        unit: String,
        role: String,
    },
    Paragraph {
        runs: Vec<SlotRun>,
    },
}

/// Parsed native data, still untrusted until admitted with its child-unit owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotDraft {
    pub schema: u32,
    pub id: String,
    pub document: String,
    pub composition: String,
    pub language: String,
    pub blocks: Vec<SlotBlock>,
}

/// A closed, ordered slot; its fields cannot be constructed by a caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSlot {
    draft: SlotDraft,
}

impl CheckedSlot {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.draft.id
    }

    #[must_use]
    pub fn document(&self) -> &str {
        &self.draft.document
    }

    #[must_use]
    pub fn language(&self) -> &str {
        &self.draft.language
    }

    #[must_use]
    pub fn blocks(&self) -> &[SlotBlock] {
        &self.draft.blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlotFault {
    Schema,
    Identity,
    ChildOwner,
    ChildOrder { index: usize },
    UnsupportedRun { index: usize },
    Source(ImportFault),
    SourceSelection,
    SourceAccounting,
    SourceEvent,
    SourceMint,
    SourceBlock { index: usize },
    SourceContent { index: usize },
}

impl std::fmt::Display for SlotFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SlotFault {}

fn merges_tex_text(text: &str) -> bool {
    text.chars().zip(text.chars().skip(1)).any(|(left, right)| {
        (matches!(left, '-' | '—') && matches!(right, '-' | '—'))
            || (matches!(left, '\'' | '’') && matches!(right, '\'' | '’' | '”'))
            || (left == '`' && matches!(right, '`' | '“'))
            || (matches!(left, '!' | '?') && matches!(right, '`' | '“'))
    })
}

fn valid_runs(runs: &[SlotRun], inside_style: bool) -> bool {
    if runs.is_empty() {
        return false;
    }
    let mut adjacent_text = String::new();
    for run in runs {
        match run {
            SlotRun::Text(text) => {
                if text.is_empty() || text.chars().any(char::is_control) {
                    return false;
                }
                adjacent_text.push_str(text);
            }
            SlotRun::Style { children, .. } => {
                if merges_tex_text(&adjacent_text) || inside_style || !valid_runs(children, true) {
                    return false;
                }
                adjacent_text.clear();
            }
            SlotRun::EnSpace => {
                if merges_tex_text(&adjacent_text) || inside_style {
                    return false;
                }
                adjacent_text.clear();
            }
        }
    }
    !merges_tex_text(&adjacent_text)
}

/// Admit the native grammar and resolve its one inherited child without reading old source bytes.
/// # Errors
/// Refuses an unsupported mode, child order, run or missing checked child owner.
pub fn admit_slot(
    draft: SlotDraft,
    has_child: impl Fn(&str, &str, &str) -> bool,
) -> Result<CheckedSlot, SlotFault> {
    if draft.schema != 1 {
        return Err(SlotFault::Schema);
    }
    if draft.id != "sec:explainer:plain"
        || draft.document != "explainer-pilot"
        || draft.composition != "corpus-slot"
        || draft.language != "en"
    {
        return Err(SlotFault::Identity);
    }
    if draft.blocks.len() != 6 {
        return Err(SlotFault::ChildOrder {
            index: draft.blocks.len(),
        });
    }
    match &draft.blocks[0] {
        SlotBlock::Heading {
            level,
            star,
            title,
            label,
        } if level == "section"
            && !star
            && label == &draft.id
            && matches!(title.as_slice(), [SlotRun::Text(text)] if text == "The Whole Idea, in Plain Words") =>
            {}
        _ => return Err(SlotFault::ChildOrder { index: 0 }),
    }
    match &draft.blocks[1] {
        SlotBlock::Use { unit, role }
            if unit == "sec:reading:plain-intro" && role == "paragraph" =>
        {
            if !has_child(unit, &draft.document, &draft.language) {
                return Err(SlotFault::ChildOwner);
            }
        }
        _ => return Err(SlotFault::ChildOrder { index: 1 }),
    }
    for (index, block) in draft.blocks.iter().enumerate().skip(2) {
        let SlotBlock::Paragraph { runs } = block else {
            return Err(SlotFault::ChildOrder { index });
        };
        if !valid_runs(runs, false) {
            return Err(SlotFault::UnsupportedRun { index });
        }
    }
    Ok(CheckedSlot { draft })
}

fn decoded_text(value: &str, final_run: bool) -> String {
    let mut output = String::new();
    let mut whitespace = false;
    for ch in value.chars() {
        if ch.is_ascii_whitespace() {
            if !whitespace {
                output.push(' ');
            }
            whitespace = true;
        } else {
            output.push(ch);
            whitespace = false;
        }
    }
    if final_run && output.ends_with(' ') {
        output.pop();
    }
    output
}

fn imported_runs(runs: &[Run], final_paragraph: bool) -> Option<Vec<SlotRun>> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| match run {
            Run::Text {
                decoded,
                spacing_intent,
            } if spacing_intent == "en-space" && decoded == " " => Some(SlotRun::EnSpace),
            Run::Text {
                decoded,
                spacing_intent,
            } if spacing_intent == "source" => Some(SlotRun::Text(decoded_text(
                decoded,
                final_paragraph && index + 1 == runs.len(),
            ))),
            Run::Style { style, runs } => Some(SlotRun::Style {
                kind: match style.as_str() {
                    "textbf" => SlotStyle::Strong,
                    "emph" => SlotStyle::Emphasis,
                    _ => return None,
                },
                children: imported_runs(runs, false)?,
            }),
            _ => None,
        })
        .collect()
}

fn line_start(bytes: &[u8], line: usize) -> Option<usize> {
    if line == 1 {
        return Some(0);
    }
    bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .nth(line - 2)
        .map(|(index, _)| index + 1)
}

/// Compare every selected draft block and accounting piece with the independently authored slot.
/// # Errors
/// Refuses changed child order, words, styles, spacing, mint, event or interval accounting.
pub fn check_transfer(
    slot: &CheckedSlot,
    source: &SourceDraft,
    bytes: &[u8],
    first: usize,
    last: usize,
) -> Result<(), SlotFault> {
    validate_accounting(source, bytes).map_err(SlotFault::Source)?;
    if source.path != "documents/paper/explainer/explainer.tex" || first > last {
        return Err(SlotFault::SourceSelection);
    }
    let begin = line_start(bytes, first).ok_or(SlotFault::SourceSelection)?;
    let end = line_start(bytes, last + 1).unwrap_or(bytes.len());
    if begin >= end {
        return Err(SlotFault::SourceSelection);
    }
    let blocks: Vec<_> = source
        .blocks
        .iter()
        .filter(|block| block.site.start_byte >= begin && block.site.start_byte < end)
        .collect();
    if blocks.len() != slot.blocks().len() {
        return Err(SlotFault::ChildOrder {
            index: blocks.len().min(slot.blocks().len()),
        });
    }
    for (index, (block, proposed)) in blocks.iter().zip(slot.blocks()).enumerate() {
        if block.site.end_byte > end || block.status != BlockStatus::Decoded {
            return Err(SlotFault::SourceBlock { index });
        }
        match (proposed, &block.kind, &block.fields) {
            (
                SlotBlock::Heading {
                    level, star, title, ..
                },
                BlockKind::Heading,
                BlockFields::Heading {
                    level: actual,
                    star: actual_star,
                    title: actual_title,
                    label_selector: None,
                },
            ) if actual == &format!("\\{level}") && star == actual_star => {
                if imported_runs(actual_title, false).as_deref() != Some(title) {
                    return Err(SlotFault::SourceContent { index });
                }
            }
            (
                SlotBlock::Use { unit, role },
                BlockKind::ExistingNativeInclusion,
                BlockFields::NativeInclusion {
                    unit: actual,
                    role: actual_role,
                    marker,
                },
            ) if unit == actual
                && role == actual_role
                && marker == &format!("% generated: {unit}") => {}
            (
                SlotBlock::Paragraph { runs },
                BlockKind::Paragraph,
                BlockFields::Paragraph {
                    continuation: false,
                    layout,
                    runs: actual,
                },
            ) if layout.is_empty() => {
                if imported_runs(actual, true).as_deref() != Some(runs) {
                    return Err(SlotFault::SourceContent { index });
                }
            }
            _ => return Err(SlotFault::SourceBlock { index }),
        }
    }
    check_selected_accounting(slot, source, &blocks, begin, end)
}

fn check_selected_accounting(
    slot: &CheckedSlot,
    source: &SourceDraft,
    blocks: &[&super::BlockDraft],
    begin: usize,
    end: usize,
) -> Result<(), SlotFault> {
    let mut cursor = begin;
    let mut labels = 0usize;
    for piece in source
        .accounting
        .iter()
        .filter(|piece| piece.start_byte < end && piece.end_byte > begin)
    {
        if piece.start_byte != cursor || piece.end_byte > end {
            return Err(SlotFault::SourceAccounting);
        }
        match piece.owner {
            AccountOwner::Content(index) | AccountOwner::ExistingNative(index) => {
                let block = source
                    .blocks
                    .get(index)
                    .ok_or(SlotFault::SourceAccounting)?;
                if block.site.start_byte != piece.start_byte
                    || block.site.end_byte != piece.end_byte
                    || !blocks.iter().any(|selected| selected.local_index == index)
                {
                    return Err(SlotFault::SourceAccounting);
                }
            }
            AccountOwner::Event(index) => {
                let event = source
                    .events
                    .get(index)
                    .ok_or(SlotFault::SourceAccounting)?;
                if event.site.start_byte != piece.start_byte
                    || event.site.end_byte != piece.end_byte
                {
                    return Err(SlotFault::SourceAccounting);
                }
                if event.kind == "label-event"
                    && event.source == format!("\\label{{{}}}", slot.id())
                {
                    labels += 1;
                } else if event.kind != "lexical-trivia"
                    || !event.source.chars().all(char::is_whitespace)
                {
                    return Err(SlotFault::SourceEvent);
                }
            }
            AccountOwner::Refusal(_) => return Err(SlotFault::SourceBlock { index: 0 }),
        }
        cursor = piece.end_byte;
    }
    if cursor != end {
        return Err(SlotFault::SourceAccounting);
    }
    let targets: Vec<_> = source
        .metadata_targets
        .iter()
        .filter(|target| target.site.start_byte >= begin && target.site.start_byte < end)
        .collect();
    if labels != 1
        || !matches!(targets.as_slice(), [target] if target.exact_key == slot.id()
            && target.selector == slot.id() && target.source_command == "\\label")
    {
        return Err(SlotFault::SourceMint);
    }
    Ok(())
}
