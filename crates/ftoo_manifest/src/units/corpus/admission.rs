//! Admission of a bounded ordered corpus slot and its independent source correspondence.

use super::{
    AccountOwner, BlockFields, BlockKind, BlockStatus, ImportFault, ParagraphEdge, ResultGradeCode,
    ResultHeader, ResultKind, ResultPartKind, Run, SourceDraft, validate_accounting,
};
use serde::Serialize;

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
    Result {
        kind: ResultKind,
        target: String,
        key: String,
        grade: Option<ResultGradeCode>,
        grade_gap: bool,
        children: Vec<Vec<SlotRun>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotPaperMint {
    pub selector: String,
    pub key: String,
    pub declaration: crate::UnitSite,
    pub placement: crate::UnitSite,
}

/// Parsed native data, still untrusted until admitted with its child-unit owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotDraft {
    pub schema: u32,
    pub id: String,
    pub document: String,
    pub composition: String,
    pub language: String,
    pub paper_mint: Option<SlotPaperMint>,
    pub blocks: Vec<SlotBlock>,
}

/// A closed, ordered slot; its fields cannot be constructed by a caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSlot {
    draft: SlotDraft,
    source: String,
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

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn paper_mint(&self) -> Option<&SlotPaperMint> {
        self.draft.paper_mint.as_ref()
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

/// One maximal interval admitted by the same native grammar and source transfer check.
#[derive(Clone, Debug, Serialize)]
pub struct SlotInterval {
    pub unit: String,
    pub first: usize,
    pub last: usize,
    pub blocks: Vec<SlotIntervalBlock>,
    pub stopping_condition: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SlotIntervalBlock {
    pub kind: super::BlockKind,
    pub first: usize,
    pub last: usize,
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

fn result_mint(draft: &SlotDraft, target: &str, key: &str, source: &str) -> bool {
    matches!(&draft.paper_mint, Some(mint)
        if mint.selector == target && mint.key == key
            && mint.declaration.file == source && mint.placement.file == source)
}

/// Admit the native grammar and resolve every inherited child without reading old source bytes.
/// # Errors
/// Refuses an unsupported mode, child order, run or missing checked child owner.
#[allow(clippy::too_many_lines)] // Result and heading forms share one admission boundary.
pub fn admit_slot(
    draft: SlotDraft,
    source: &str,
    has_child: impl Fn(&str, &str, &str) -> bool,
) -> Result<CheckedSlot, SlotFault> {
    if draft.schema != 1 {
        return Err(SlotFault::Schema);
    }
    let result_slot = matches!(draft.blocks.as_slice(), [SlotBlock::Result { .. }]);
    let headed = matches!(draft.blocks.first(), Some(SlotBlock::Heading { .. }));
    let paragraph_id = draft.id.strip_prefix("sec:corpus:").is_some_and(|suffix| {
        suffix.len() == 64
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    });
    if !(if result_slot {
        draft.id.starts_with("result:")
            && crate::units::label_name(&draft.id)
            && draft.document == "paper-pilot"
    } else if headed {
        draft.document == "explainer-pilot"
            && draft.id.starts_with("sec:")
            && crate::units::label_name(&draft.id)
    } else {
        paragraph_id
    }) || !matches!(draft.document.as_str(), "paper-pilot" | "explainer-pilot")
        || draft.composition != "corpus-slot"
        || draft.language != "en"
    {
        return Err(SlotFault::Identity);
    }
    if let [
        SlotBlock::Result {
            target,
            key,
            grade,
            grade_gap,
            children,
            ..
        },
    ] = draft.blocks.as_slice()
    {
        if !result_mint(&draft, target, key, source) {
            return Err(SlotFault::SourceMint);
        }
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
            || *grade_gap && grade.is_none()
            || children.is_empty()
        {
            return Err(SlotFault::ChildOrder { index: 0 });
        }
        for (index, runs) in children.iter().enumerate() {
            if !valid_runs(runs, false) {
                return Err(SlotFault::UnsupportedRun { index });
            }
        }
        return Ok(CheckedSlot {
            draft,
            source: source.into(),
        });
    }
    if draft.paper_mint.is_some() {
        return Err(SlotFault::SourceMint);
    }
    if draft.blocks.is_empty()
        || !draft
            .blocks
            .iter()
            .any(|block| matches!(block, SlotBlock::Paragraph { .. }))
    {
        return Err(SlotFault::ChildOrder {
            index: draft.blocks.len(),
        });
    }
    if headed {
        match &draft.blocks[0] {
            SlotBlock::Heading {
                level,
                star,
                title,
                label,
            } if level == "section"
                && !star
                && label == &draft.id
                && matches!(title.as_slice(), [SlotRun::Text(text)] if !text.trim().is_empty()
                && !text.chars().any(|ch| matches!(ch, '{' | '}' | '\\' | '\n' | '\r'))
                && !merges_tex_text(text)) => {}
            _ => return Err(SlotFault::ChildOrder { index: 0 }),
        }
    }
    let mut children = std::collections::BTreeSet::new();
    for (index, block) in draft.blocks.iter().enumerate().skip(usize::from(headed)) {
        match block {
            SlotBlock::Use { unit, role } if role == "paragraph" && unit != &draft.id => {
                if !children.insert(unit) {
                    return Err(SlotFault::ChildOrder { index });
                }
                if !has_child(unit, &draft.document, &draft.language) {
                    return Err(SlotFault::ChildOwner);
                }
            }
            SlotBlock::Paragraph { runs } if !valid_runs(runs, false) => {
                return Err(SlotFault::UnsupportedRun { index });
            }
            SlotBlock::Paragraph { .. } => {}
            _ => return Err(SlotFault::ChildOrder { index }),
        }
    }
    Ok(CheckedSlot {
        draft,
        source: source.into(),
    })
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

fn imported_child(block: &super::BlockDraft, label: &str) -> Option<SlotBlock> {
    if block.status != BlockStatus::Decoded {
        return None;
    }
    match (&block.kind, &block.fields) {
        (
            BlockKind::Heading,
            BlockFields::Heading {
                level,
                star,
                title,
                label_selector: None,
            },
        ) if level == "\\section" && !star => Some(SlotBlock::Heading {
            level: "section".into(),
            star: false,
            title: imported_runs(title, false)?,
            label: label.into(),
        }),
        (
            BlockKind::Paragraph,
            BlockFields::Paragraph {
                continuation: false,
                layout,
                runs,
            },
        ) if layout.is_empty() => Some(SlotBlock::Paragraph {
            runs: imported_runs(runs, true)?,
        }),
        (BlockKind::ExistingNativeInclusion, BlockFields::NativeInclusion { unit, role, .. })
            if role == "paragraph" =>
        {
            Some(SlotBlock::Use {
                unit: unit.clone(),
                role: role.clone(),
            })
        }
        _ => None,
    }
}

fn awaiting_paragraph(children: &[SlotBlock], fault: &SlotFault) -> bool {
    matches!(fault, SlotFault::ChildOrder { index } if *index == children.len())
        && !children
            .iter()
            .any(|child| matches!(child, SlotBlock::Paragraph { .. }))
}

/// Report maximal, nonoverlapping source-boundary and slot-grammar admissible intervals.
#[must_use]
pub fn admitted_intervals(
    source: &SourceDraft,
    bytes: &[u8],
    has_child: impl Fn(&str, &str, &str) -> bool,
    derive_id: impl Fn(&SourceDraft, usize, usize) -> String,
) -> Vec<SlotInterval> {
    if !matches!(source.document.as_str(), "paper" | "explainer") || source.language != "en" {
        return Vec::new();
    }
    let mut intervals = headed_intervals(source, bytes, &has_child);
    let paragraphs = paragraph_intervals(source, bytes, &intervals, &has_child, &derive_id);
    intervals.extend(paragraphs);
    intervals.sort_by_key(|interval| interval.first);
    intervals
}

fn headed_intervals(
    source: &SourceDraft,
    bytes: &[u8],
    has_child: &impl Fn(&str, &str, &str) -> bool,
) -> Vec<SlotInterval> {
    let mut intervals = Vec::new();
    for (heading_index, heading) in source.blocks.iter().enumerate() {
        if source.document != "explainer" {
            break;
        }
        if !matches!(heading.kind, BlockKind::Heading) {
            continue;
        }
        let next_start = source
            .blocks
            .get(heading_index + 1)
            .map_or(bytes.len(), |next| next.site.start_byte);
        let labels: Vec<_> = source
            .metadata_targets
            .iter()
            .filter(|target| {
                target.source_command == "\\label"
                    && target.site.start_byte >= heading.site.start_byte
                    && target.site.start_byte < next_start
                    && target.selector == target.exact_key
            })
            .collect();
        let [label] = labels.as_slice() else {
            continue;
        };
        let Some(first_child) = imported_child(heading, &label.exact_key) else {
            continue;
        };
        let mut children = vec![first_child];
        let mut best = None;
        let mut stop = "end of source".to_owned();
        for index in heading_index + 1..source.blocks.len() {
            let block = &source.blocks[index];
            let Some(child) = imported_child(block, &label.exact_key) else {
                stop = format!(
                    "{}:{} {:?} {:?}",
                    source.path, block.site.start_line, block.kind, block.status
                );
                break;
            };
            children.push(child);
            let last = source.blocks.get(index + 1).map_or_else(
                || bytes.split(|byte| *byte == b'\n').count(),
                |next| next.site.start_line.saturating_sub(1),
            );
            let candidate = SlotDraft {
                schema: 1,
                id: label.exact_key.clone(),
                document: "explainer-pilot".into(),
                composition: "corpus-slot".into(),
                language: "en".into(),
                paper_mint: None,
                blocks: children.clone(),
            };
            let checked = match admit_slot(candidate, &source.path, has_child) {
                Ok(checked) => checked,
                Err(fault) if awaiting_paragraph(&children, &fault) => {
                    continue;
                }
                Err(fault) => {
                    stop = format!("{}:{} {fault}", source.path, block.site.start_line);
                    break;
                }
            };
            if let Err(fault) = check_transfer(
                &checked,
                source,
                &source.path,
                bytes,
                heading.site.start_line,
                last,
            ) {
                stop = format!("{}:{} {fault}", source.path, block.site.start_line);
                break;
            }
            let selected = headed_interval_blocks(source, heading_index, index, last);
            best = Some(SlotInterval {
                unit: label.exact_key.clone(),
                first: heading.site.start_line,
                last,
                blocks: selected,
                stopping_condition: String::new(),
            });
        }
        if let Some(mut interval) = best {
            interval.stopping_condition = stop;
            intervals.push(interval);
        }
    }
    intervals
}

fn headed_interval_blocks(
    source: &SourceDraft,
    heading_index: usize,
    end_index: usize,
    last: usize,
) -> Vec<SlotIntervalBlock> {
    source.blocks[heading_index..=end_index]
        .iter()
        .enumerate()
        .map(|(offset, selected)| SlotIntervalBlock {
            kind: selected.kind.clone(),
            first: selected.site.start_line,
            last: source
                .blocks
                .get(heading_index + offset + 1)
                .map_or(last, |next| next.site.start_line.saturating_sub(1)),
        })
        .collect()
}

fn paragraph_intervals(
    source: &SourceDraft,
    bytes: &[u8],
    intervals: &[SlotInterval],
    has_child: &impl Fn(&str, &str, &str) -> bool,
    derive_id: &impl Fn(&SourceDraft, usize, usize) -> String,
) -> Vec<SlotInterval> {
    let mut selected = Vec::new();
    for index in 0..source.blocks.len() {
        let first_block = &source.blocks[index];
        if first_block.kind != BlockKind::Paragraph
            || inside_intervals(intervals, first_block.site.start_line)
            || inside_intervals(&selected, first_block.site.start_line)
        {
            continue;
        }
        let Some(boundary) = source
            .paragraph_boundaries
            .iter()
            .find(|entry| entry.block_index == index)
        else {
            continue;
        };
        if !matches!(
            boundary.left,
            ParagraphEdge::Blank | ParagraphEdge::HeadingTail
        ) {
            continue;
        }
        let mut children = Vec::new();
        let mut best = None;
        let mut stop = "end of source".to_owned();
        for end_index in index..source.blocks.len() {
            let block = &source.blocks[end_index];
            if inside_intervals(intervals, block.site.start_line) {
                stop = "existing headed interval".into();
                break;
            }
            let Some(child) = imported_child(block, "") else {
                stop = format!(
                    "{}:{} {:?} {:?}",
                    source.path, block.site.start_line, block.kind, block.status
                );
                break;
            };
            children.push(child);
            let first = first_block.site.start_line;
            let last = block.site.end_line;
            let Some(begin) = line_start(bytes, first) else {
                break;
            };
            let end = line_start(bytes, last + 1).unwrap_or(bytes.len());
            let id = derive_id(source, begin, end);
            let candidate = SlotDraft {
                schema: 1,
                id: id.clone(),
                document: format!("{}-pilot", source.document),
                composition: "corpus-slot".into(),
                language: "en".into(),
                paper_mint: None,
                blocks: children.clone(),
            };
            let checked = match admit_slot(candidate, &source.path, has_child) {
                Ok(checked) => checked,
                Err(fault) => {
                    stop = format!("{}:{} {fault}", source.path, block.site.start_line);
                    break;
                }
            };
            if let Err(fault) = check_transfer(&checked, source, &source.path, bytes, first, last) {
                stop = format!("{}:{} {fault}", source.path, block.site.start_line);
                break;
            }
            best = Some(SlotInterval {
                unit: id,
                first,
                last,
                blocks: source.blocks[index..=end_index]
                    .iter()
                    .enumerate()
                    .map(|(offset, child)| SlotIntervalBlock {
                        kind: child.kind.clone(),
                        first: child.site.start_line,
                        last: source
                            .blocks
                            .get(index + offset + 1)
                            .filter(|_| index + offset < end_index)
                            .map_or(child.site.end_line, |next| {
                                next.site.start_line.saturating_sub(1)
                            }),
                    })
                    .collect(),
                stopping_condition: String::new(),
            });
        }
        if let Some(mut interval) = best {
            interval.stopping_condition = stop;
            selected.push(interval);
        }
    }
    selected
}

fn inside_intervals(intervals: &[SlotInterval], line: usize) -> bool {
    intervals
        .iter()
        .any(|interval| interval.first <= line && line <= interval.last)
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
#[allow(clippy::too_many_lines)] // Result children and ordinary slot blocks share the source-accounting traversal.
pub fn check_transfer(
    slot: &CheckedSlot,
    source: &SourceDraft,
    expected_source: &str,
    bytes: &[u8],
    first: usize,
    last: usize,
) -> Result<(), SlotFault> {
    validate_accounting(source, bytes).map_err(SlotFault::Source)?;
    if source.path != expected_source || first > last {
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
                SlotBlock::Result {
                    kind,
                    key,
                    grade,
                    grade_gap,
                    children,
                    ..
                },
                BlockKind::TheoremContainer,
                BlockFields::Result {
                    result_kind,
                    header:
                        ResultHeader::Registered {
                            key: actual_key, ..
                        },
                    grade: actual_grade,
                    parts,
                },
            ) if kind == result_kind && key == actual_key => {
                if grade != &actual_grade.as_ref().map(|value| value.code)
                    || *grade_gap
                        != parts
                            .iter()
                            .any(|part| part.kind == ResultPartKind::GradeGap)
                    || children.len() != block.children.len()
                {
                    return Err(SlotFault::SourceContent { index });
                }
                for (expected, actual) in children.iter().zip(&block.children) {
                    let BlockFields::Paragraph {
                        continuation: false,
                        layout,
                        runs,
                    } = &actual.fields
                    else {
                        return Err(SlotFault::SourceBlock { index });
                    };
                    if actual.kind != BlockKind::Paragraph
                        || actual.status != BlockStatus::Decoded
                        || !layout.is_empty()
                        || imported_runs(runs, true).as_deref() != Some(expected)
                    {
                        return Err(SlotFault::SourceContent { index });
                    }
                }
            }
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
                let boundary = source
                    .paragraph_boundaries
                    .iter()
                    .find(|entry| entry.block_index == block.local_index)
                    .ok_or(SlotFault::SourceBlock { index })?;
                if !matches!(
                    boundary.left,
                    ParagraphEdge::Blank | ParagraphEdge::HeadingTail
                ) || !matches!(
                    boundary.right,
                    ParagraphEdge::Blank | ParagraphEdge::Heading
                ) || boundary.first_token.is_none()
                    || boundary.last_token.is_none()
                {
                    return Err(SlotFault::SourceBlock { index });
                }
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
    let headed = matches!(slot.blocks().first(), Some(SlotBlock::Heading { .. }));
    if (headed
        && (labels != 1
            || !matches!(targets.as_slice(), [target] if target.exact_key == slot.id()
            && target.selector == slot.id() && target.source_command == "\\label")))
        || (!headed && (labels != 0 || !targets.is_empty()))
    {
        return Err(SlotFault::SourceMint);
    }
    Ok(())
}

#[cfg(test)]
mod result_tests {
    use super::*;
    use crate::corpus::{ResultHeader, import_source};

    fn mint(path: &str, key: &str) -> SlotPaperMint {
        let site = crate::UnitSite {
            file: path.into(),
            path: Vec::new(),
            anchor: crate::SourceAnchor::Unavailable,
            decoded: None,
            occurrence: None,
        };
        SlotPaperMint {
            selector: "header".into(),
            key: key.into(),
            declaration: site.clone(),
            placement: site,
        }
    }

    fn live_result(part: &str, key: &str) -> (String, SourceDraft, SlotDraft, usize, usize) {
        let path = format!("documents/paper/parts/{part}");
        let bytes = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../units/fixtures")
                .join(format!(
                    "result-{}-source.tex",
                    if key == "f:pool" { "pool" } else { "modest" }
                )),
        )
        .unwrap();
        let source = import_source(&path, bytes.as_bytes(), "digest".into(), &[]).unwrap();
        let block = source
            .blocks
            .iter()
            .find(|block| {
                matches!(&block.fields, BlockFields::Result {
                header: ResultHeader::Registered { key: actual, .. }, ..
            } if actual == key)
            })
            .unwrap();
        let BlockFields::Result {
            result_kind,
            grade,
            parts,
            ..
        } = &block.fields
        else {
            panic!("result fields")
        };
        let children = block
            .children
            .iter()
            .map(|child| {
                let BlockFields::Paragraph { runs, .. } = &child.fields else {
                    panic!("paragraph child")
                };
                imported_runs(runs, true).unwrap()
            })
            .collect();
        let first = block.site.start_line;
        let last = 1 + bytes[..block.site.end_byte.saturating_sub(1)]
            .matches('\n')
            .count();
        let slot = SlotDraft {
            schema: 1,
            id: format!("result:{key}"),
            document: "paper-pilot".into(),
            composition: "corpus-slot".into(),
            language: "en".into(),
            paper_mint: Some(mint(&source.path, key)),
            blocks: vec![SlotBlock::Result {
                kind: *result_kind,
                target: "header".into(),
                key: key.into(),
                grade: grade.as_ref().map(|value| value.code),
                grade_gap: parts
                    .iter()
                    .any(|part| part.kind == ResultPartKind::GradeGap),
                children,
            }],
        };
        (bytes, source, slot, first, last)
    }

    #[test]
    fn real_plain_results_require_key_grade_gap_and_child_words() {
        for (part, key) in [("partA.tex", "f:pool"), ("partC.tex", "nrm:modest")] {
            let (bytes, source, draft, first, last) = live_result(part, key);
            let checked = admit_slot(draft.clone(), &source.path, |_, _, _| false).unwrap();
            assert!(
                check_transfer(
                    &checked,
                    &source,
                    &source.path,
                    bytes.as_bytes(),
                    first,
                    last
                )
                .is_ok()
            );

            let mut missing_mint = draft.clone();
            missing_mint.paper_mint = None;
            assert_eq!(
                admit_slot(missing_mint, &source.path, |_, _, _| false),
                Err(SlotFault::SourceMint)
            );

            let mut wrong_selector = draft.clone();
            let SlotBlock::Result { target, .. } = &mut wrong_selector.blocks[0] else {
                unreachable!()
            };
            *target = "other".into();
            assert_eq!(
                admit_slot(wrong_selector, &source.path, |_, _, _| false),
                Err(SlotFault::SourceMint)
            );

            let mut changed = draft.clone();
            let SlotBlock::Result { key, .. } = &mut changed.blocks[0] else {
                unreachable!()
            };
            key.push('x');
            changed.paper_mint.as_mut().unwrap().key.push('x');
            let checked = admit_slot(changed, &source.path, |_, _, _| false).unwrap();
            assert_eq!(
                check_transfer(
                    &checked,
                    &source,
                    &source.path,
                    bytes.as_bytes(),
                    first,
                    last
                ),
                Err(SlotFault::SourceBlock { index: 0 })
            );

            let mut changed = draft.clone();
            let SlotBlock::Result { grade_gap, .. } = &mut changed.blocks[0] else {
                unreachable!()
            };
            *grade_gap = !*grade_gap;
            let checked = admit_slot(changed, &source.path, |_, _, _| false).unwrap();
            assert_eq!(
                check_transfer(
                    &checked,
                    &source,
                    &source.path,
                    bytes.as_bytes(),
                    first,
                    last
                ),
                Err(SlotFault::SourceContent { index: 0 })
            );

            let mut changed = draft.clone();
            let SlotBlock::Result { grade, .. } = &mut changed.blocks[0] else {
                unreachable!()
            };
            *grade = None;
            if let Ok(checked) = admit_slot(changed, &source.path, |_, _, _| false) {
                assert_eq!(
                    check_transfer(
                        &checked,
                        &source,
                        &source.path,
                        bytes.as_bytes(),
                        first,
                        last
                    ),
                    Err(SlotFault::SourceContent { index: 0 })
                );
            }

            let mut changed = draft;
            let SlotBlock::Result { children, .. } = &mut changed.blocks[0] else {
                unreachable!()
            };
            children[0].push(SlotRun::Text("changed".into()));
            let checked = admit_slot(changed, &source.path, |_, _, _| false).unwrap();
            assert_eq!(
                check_transfer(
                    &checked,
                    &source,
                    &source.path,
                    bytes.as_bytes(),
                    first,
                    last
                ),
                Err(SlotFault::SourceContent { index: 0 })
            );
        }
    }

    #[test]
    fn ungraded_remark_keeps_absence_and_child_order() {
        let text = "\\begin{remark}[\\hdr{r:plain}]\nFirst.\n\nSecond.\n\\end{remark}\n";
        let path = "documents/paper/parts/partA.tex";
        let source = import_source(path, text.as_bytes(), "digest".into(), &[]).unwrap();
        let block = &source.blocks[0];
        assert_eq!(block.children.len(), 2);
        let children: Vec<_> = block
            .children
            .iter()
            .map(|child| {
                let BlockFields::Paragraph { runs, .. } = &child.fields else {
                    unreachable!()
                };
                imported_runs(runs, true).unwrap()
            })
            .collect();
        let draft = SlotDraft {
            schema: 1,
            id: "result:r:plain".into(),
            document: "paper-pilot".into(),
            composition: "corpus-slot".into(),
            language: "en".into(),
            paper_mint: Some(mint(path, "r:plain")),
            blocks: vec![SlotBlock::Result {
                kind: ResultKind::Remark,
                target: "header".into(),
                key: "r:plain".into(),
                grade: None,
                grade_gap: false,
                children,
            }],
        };
        let checked = admit_slot(draft.clone(), path, |_, _, _| false).unwrap();
        check_transfer(&checked, &source, path, text.as_bytes(), 1, 6).unwrap();

        let mut graded = draft.clone();
        let SlotBlock::Result { grade, .. } = &mut graded.blocks[0] else {
            unreachable!()
        };
        *grade = Some(ResultGradeCode::Nec);
        let checked = admit_slot(graded, path, |_, _, _| false).unwrap();
        assert_eq!(
            check_transfer(&checked, &source, path, text.as_bytes(), 1, 6),
            Err(SlotFault::SourceContent { index: 0 })
        );

        let mut reordered = draft;
        let SlotBlock::Result { children, .. } = &mut reordered.blocks[0] else {
            unreachable!()
        };
        children.swap(0, 1);
        let checked = admit_slot(reordered, path, |_, _, _| false).unwrap();
        assert_eq!(
            check_transfer(&checked, &source, path, text.as_bytes(), 1, 6),
            Err(SlotFault::SourceContent { index: 0 })
        );
    }
}
