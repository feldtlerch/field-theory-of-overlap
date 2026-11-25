//! Closed draft decoding for the first registered result containers.

use super::import::{braced, command_at, container_children, site};
use super::{
    BlockDraft, BlockFields, BlockKind, BlockStatus, ResultGrade, ResultGradeCode, ResultHeader,
    ResultKind, ResultPart, ResultPartKind, Site, TargetDraft,
};

type DecodeFault = (usize, usize, String);

fn fault(at: usize, end: usize, reason: &str) -> DecodeFault {
    (at, end, reason.into())
}

fn part(path: &str, text: &str, start: usize, end: usize, kind: ResultPartKind) -> ResultPart {
    ResultPart {
        kind,
        site: site(path, text, start, end),
    }
}

fn argument<'a>(text: &'a str, at: usize, command: &str) -> Option<(&'a str, usize)> {
    text.get(at..)?.starts_with('\\').then_some(())?;
    let (actual, end) = command_at(text, at);
    (actual == command).then_some(())?;
    braced(text, end)
}

fn is_command(text: &str, at: usize, command: &str) -> bool {
    text.get(at..).is_some_and(|rest| rest.starts_with('\\')) && command_at(text, at).0 == command
}

fn ascii_space(text: &str) -> bool {
    text.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}

/// Decode only registered headers with plain children; later header forms stay refusals.
#[allow(clippy::too_many_lines)] // Header, grade, child, and byte boundaries have separate refusals.
pub(super) fn decode_result(
    path: &str,
    text: &str,
    start: usize,
    closed: usize,
    end: usize,
    kind: ResultKind,
) -> Result<(BlockFields, Vec<BlockDraft>, Vec<TargetDraft>), DecodeFault> {
    let name = kind.name();
    let opening = format!("\\begin{{{name}}}");
    let closing = format!("\\end{{{name}}}");
    let header_at = start + opening.len();
    if !text[start..closed].starts_with(&opening)
        || !text[closed - closing.len()..closed].starts_with(&closing)
    {
        return Err(fault(start, closed, "unsupported result boundary"));
    }
    if text.as_bytes().get(header_at) != Some(&b'[') {
        return Err(fault(header_at, header_at, "unsupported result header"));
    }
    let key_at = header_at + 1;
    let Some((key, key_end)) = argument(text, key_at, "\\hdr") else {
        return Err(fault(key_at, key_at + 1, "unsupported result header"));
    };
    if key.is_empty()
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
        || text.as_bytes().get(key_end) != Some(&b']')
    {
        return Err(fault(key_at, key_end, "unsupported result header"));
    }
    let mut cursor = key_end + 1;
    let header = ResultHeader::Registered {
        key: key.into(),
        site: site(path, text, key_at, key_end),
    };
    let mut parts = vec![part(path, text, start, cursor, ResultPartKind::Opening)];

    let grade = if is_command(text, cursor, "\\grade") {
        let grade_at = cursor;
        let Some((code, grade_end)) = argument(text, cursor, "\\grade") else {
            return Err(fault(
                cursor,
                cursor + "\\grade".len(),
                "unsupported result grade",
            ));
        };
        let Some(code) = ResultGradeCode::from_name(code) else {
            return Err(fault(grade_at, grade_end, "unsupported result grade"));
        };
        cursor = grade_end;
        parts.push(part(path, text, grade_at, cursor, ResultPartKind::Grade));
        Some(ResultGrade {
            code,
            site: site(path, text, grade_at, cursor),
        })
    } else {
        None
    };

    if is_command(text, cursor, "\\PaperGradeGap") {
        let gap_at = cursor;
        let Some((value, gap_end)) = argument(text, cursor, "\\PaperGradeGap") else {
            return Err(fault(
                cursor,
                cursor + "\\PaperGradeGap".len(),
                "unsupported result layout",
            ));
        };
        if grade.is_none() || value != "-2pt" {
            return Err(fault(gap_at, gap_end, "unsupported result layout"));
        }
        cursor = gap_end;
        parts.push(part(path, text, gap_at, cursor, ResultPartKind::GradeGap));
    }

    let (children, targets) = container_children(path, text, start, end);
    if children.is_empty() {
        return Err(fault(cursor, closed, "unsupported result child"));
    }
    for child in &children {
        if child.kind != BlockKind::Paragraph || child.status != BlockStatus::Decoded {
            return Err(fault(
                child.site.start_byte,
                child.site.end_byte,
                "unsupported result child",
            ));
        }
        let at = child.site.start_byte;
        if at < cursor {
            return Err(fault(at, cursor, "overlapping result child"));
        }
        if at > cursor {
            if !ascii_space(&text[cursor..at]) {
                return Err(fault(cursor, at, "unaccounted result byte"));
            }
            parts.push(part(path, text, cursor, at, ResultPartKind::Trivia));
        }
        parts.push(part(
            path,
            text,
            at,
            child.site.end_byte,
            ResultPartKind::Child,
        ));
        cursor = child.site.end_byte;
    }
    let close_at = closed - closing.len();
    if cursor > close_at {
        return Err(fault(close_at, cursor, "overlapping result child"));
    }
    if cursor < close_at {
        if !ascii_space(&text[cursor..close_at]) {
            return Err(fault(cursor, close_at, "unaccounted result byte"));
        }
        parts.push(part(path, text, cursor, close_at, ResultPartKind::Trivia));
    }
    if !ascii_space(&text[closed..end]) {
        return Err(fault(closed, end, "unsupported result suffix"));
    }
    parts.push(part(path, text, close_at, end, ResultPartKind::Closing));
    let fields = BlockFields::Result {
        result_kind: kind,
        header,
        grade,
        parts,
    };
    validate_result_parts(&fields, &children, &site(path, text, start, end), text)
        .map_err(|reason| fault(start, end, reason))?;
    Ok((fields, children, targets))
}

/// Check the draft's complete, ordered source partition and child references.
#[allow(clippy::too_many_lines)] // Each part kind has its own independent source check.
pub(super) fn validate_result_parts(
    fields: &BlockFields,
    children: &[BlockDraft],
    outer: &Site,
    text: &str,
) -> Result<(), &'static str> {
    let BlockFields::Result {
        result_kind,
        header,
        grade,
        parts,
    } = fields
    else {
        return Err("missing result fields");
    };
    if !matches!(
        parts.first(),
        Some(ResultPart {
            kind: ResultPartKind::Opening,
            ..
        })
    ) || !matches!(
        parts.last(),
        Some(ResultPart {
            kind: ResultPartKind::Closing,
            ..
        })
    ) {
        return Err("result boundary order");
    }
    let ResultHeader::Registered {
        key,
        site: header_site,
    } = header;
    let opener = format!("\\begin{{{}}}[", result_kind.name());
    if header_site.path != outer.path
        || header_site.start_byte != outer.start_byte + opener.len()
        || text.get(header_site.start_byte..header_site.end_byte)
            != Some(format!("\\hdr{{{key}}}").as_str())
        || !text
            .get(outer.start_byte..)
            .is_some_and(|source| source.starts_with(&opener))
    {
        return Err("result header mismatch");
    }
    let mut cursor = outer.start_byte;
    let mut child_index = 0;
    let mut grade_parts = 0;
    for (index, entry) in parts.iter().enumerate() {
        if entry.site.path != outer.path
            || entry.site.start_byte != cursor
            || entry.site.end_byte <= cursor
        {
            return Err("unaccounted result byte");
        }
        let source = text
            .get(entry.site.start_byte..entry.site.end_byte)
            .ok_or("result range")?;
        match entry.kind {
            ResultPartKind::Opening if index != 0 => {
                return Err("result boundary order");
            }
            ResultPartKind::Closing
                if index + 1 != parts.len()
                    || !source.starts_with(&format!("\\end{{{}}}", result_kind.name()))
                    || !ascii_space(
                        &source[format!("\\end{{{}}}", result_kind.name()).len()..],
                    ) =>
            {
                return Err("result boundary order");
            }
            ResultPartKind::Trivia if !ascii_space(source) => {
                return Err("unaccounted result byte");
            }
            ResultPartKind::Grade => {
                let Some(value) = grade else {
                    return Err("result grade mismatch");
                };
                if entry.site != value.site || source != format!("\\grade{{{}}}", value.code.name())
                {
                    return Err("result grade mismatch");
                }
                grade_parts += 1;
            }
            ResultPartKind::GradeGap if grade_parts != 1 || source != "\\PaperGradeGap{-2pt}" => {
                return Err("result grade-gap mismatch");
            }
            _ => {}
        }
        if entry.kind == ResultPartKind::Child {
            let Some(child) = children.get(child_index) else {
                return Err("missing result child");
            };
            if entry.site != child.site || child.local_index != child_index {
                return Err("reordered result child");
            }
            if child.kind != BlockKind::Paragraph || child.status != BlockStatus::Decoded {
                return Err("unsupported result child");
            }
            child_index += 1;
        }
        cursor = entry.site.end_byte;
    }
    if cursor != outer.end_byte {
        return Err("unaccounted result byte");
    }
    if child_index != children.len() {
        return Err("missing result child");
    }
    if grade_parts != usize::from(grade.is_some()) {
        return Err("result grade mismatch");
    }
    Ok(())
}
