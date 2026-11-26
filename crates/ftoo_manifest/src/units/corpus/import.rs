use super::containers::decode_result;
use super::{
    AccountOwner, AccountPiece, BlockDraft, BlockFields, BlockKind, BlockStatus, EventDraft,
    ImportFault, MathDraft, Refusal, ResultKind, Run, Site, SourceDraft, TableCell, TargetDraft,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Production {
    Begin,
    End,
    Heading,
    Header,
    Appendix,
    Wrapper,
    Layout,
    Checking,
    Label,
    ParagraphHook,
    Event,
    Style,
    Tag,
    Handle,
    Reference,
    Citation,
    Spacing,
    Escape,
    Opaque,
}

pub(super) const COMMANDS: &[(&str, Production)] = &[
    ("\\begin", Production::Begin),
    ("\\end", Production::End),
    ("\\part", Production::Heading),
    ("\\chapter", Production::Heading),
    ("\\section", Production::Heading),
    ("\\subsection", Production::Heading),
    ("\\subsubsection", Production::Heading),
    ("\\coda", Production::Heading),
    ("\\appendix", Production::Appendix),
    ("\\documentclass", Production::Wrapper),
    ("\\PaperParagraph", Production::Layout),
    ("\\PaperDisplayParagraph", Production::Layout),
    ("\\secgrade", Production::Checking),
    ("\\grade", Production::Checking),
    ("\\hdr", Production::Header),
    ("\\PaperGradeGap", Production::Layout),
    ("\\spend", Production::Checking),
    ("\\grant", Production::Checking),
    ("\\DeclareQuantity", Production::Checking),
    ("\\label", Production::Label),
    ("\\noindent", Production::ParagraphHook),
    ("\\vspace", Production::Event),
    ("\\par", Production::Event),
    ("\\tableofcontents", Production::Event),
    ("\\printbibliography", Production::Event),
    ("\\ftooindex", Production::Event),
    ("\\clearpage", Production::Event),
    ("\\newpage", Production::Event),
    ("\\pagebreak", Production::Event),
    ("\\nopagebreak", Production::Event),
    ("\\medskip", Production::Event),
    ("\\bigskip", Production::Event),
    ("\\smallskip", Production::Event),
    ("\\begingroup", Production::Event),
    ("\\endgroup", Production::Event),
    ("\\nobreak", Production::Event),
    ("\\emph", Production::Style),
    ("\\textbf", Production::Style),
    ("\\textit", Production::Style),
    ("\\textsc", Production::Style),
    ("\\ftag", Production::Tag),
    ("\\ptag", Production::Tag),
    ("\\handle", Production::Handle),
    ("\\ref", Production::Reference),
    ("\\cref", Production::Reference),
    ("\\ptg", Production::Reference),
    ("\\parencite", Production::Citation),
    ("\\textcite", Production::Citation),
    ("\\enspace", Production::Spacing),
    ("\\verb", Production::Opaque),
    ("\\%", Production::Escape),
    ("\\#", Production::Escape),
    ("\\&", Production::Escape),
    ("\\_", Production::Escape),
];

pub(super) fn production(name: &str) -> Option<Production> {
    COMMANDS
        .iter()
        .find_map(|(entry, kind)| (*entry == name).then_some(*kind))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lexeme<'a> {
    Command(&'a str),
    Comment,
    Open,
    Close,
    Dollar,
    Ampersand,
    Other(char),
}

struct Walker<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> Walker<'a> {
    fn new(text: &'a str, cursor: usize) -> Self {
        Self { text, cursor }
    }

    fn next(&mut self) -> Option<(usize, usize, Lexeme<'a>)> {
        let start = self.cursor;
        let ch = self.text.get(start..)?.chars().next()?;
        let (end, token) = match ch {
            '%' => {
                let end = self.text[start..]
                    .find('\n')
                    .map_or(self.text.len(), |offset| start + offset + 1);
                (end, Lexeme::Comment)
            }
            '\\' => {
                let mut end = start + 1;
                if self
                    .text
                    .as_bytes()
                    .get(end)
                    .is_some_and(u8::is_ascii_alphabetic)
                {
                    while self
                        .text
                        .as_bytes()
                        .get(end)
                        .is_some_and(u8::is_ascii_alphabetic)
                    {
                        end += 1;
                    }
                } else if let Some(next) = self.text[end..].chars().next() {
                    end += next.len_utf8();
                }
                (end, Lexeme::Command(&self.text[start..end]))
            }
            '{' => (start + 1, Lexeme::Open),
            '}' => (start + 1, Lexeme::Close),
            '$' => (start + 1, Lexeme::Dollar),
            '&' => (start + 1, Lexeme::Ampersand),
            other => (start + other.len_utf8(), Lexeme::Other(other)),
        };
        self.cursor = end;
        Some((start, end, token))
    }
}

pub(super) fn site(path: &str, text: &str, start: usize, end: usize) -> Site {
    let last_line = if end > 0 && text.as_bytes()[end - 1] == b'\n' {
        &text[..end - 1]
    } else {
        &text[..end]
    };
    Site {
        path: path.into(),
        start_byte: start,
        end_byte: end,
        start_line: 1 + text[..start].matches('\n').count(),
        end_line: 1 + last_line.matches('\n').count(),
    }
}

pub(super) fn braced(text: &str, start: usize) -> Option<(&str, usize)> {
    let mut walker = Walker::new(text, start);
    if !matches!(walker.next(), Some((_, _, Lexeme::Open))) {
        return None;
    }
    let mut depth = 0usize;
    while let Some((at, end, token)) = walker.next() {
        match token {
            Lexeme::Open => depth += 1,
            Lexeme::Close if depth == 0 => return Some((&text[start + 1..at], end)),
            Lexeme::Close => depth -= 1,
            _ => {}
        }
    }
    None
}

pub(super) fn command_at(text: &str, start: usize) -> (&str, usize) {
    let mut walker = Walker::new(text, start);
    let (_, end, Lexeme::Command(command)) =
        walker.next().expect("command starts at a source boundary")
    else {
        unreachable!("caller checked a command at a source boundary")
    };
    (command, end)
}

type InlineResult = Result<(Vec<Run>, Vec<TargetDraft>), (usize, usize, String)>;

/// One source contribution produced by the inline command walker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocatedPiece {
    pub start: usize,
    pub end: usize,
    pub semantic: String,
    pub control: Option<&'static str>,
}

/// Decode an inline source slice and return its located contributions.
/// # Errors
/// Refuses the same unsupported inline syntax as the diagnostic importer.
pub fn located_inline(
    path: &str,
    whole: &str,
    start: usize,
    end: usize,
) -> Result<Vec<LocatedPiece>, String> {
    let source = whole
        .get(start..end)
        .ok_or_else(|| "inline range is not a source boundary".to_owned())?;
    let mut pieces = Vec::new();
    inline_with_locations(path, whole, source, start, &mut pieces)
        .map_err(|(at, _, reason)| format!("{path}:{at}: {reason}"))?;
    Ok(pieces)
}

fn decoded_text(source: &str) -> String {
    source
        .replace("---", "—")
        .replace("--", "–")
        .replace("``", "“")
        .replace("''", "”")
        .replace('\'', "’")
}

#[allow(clippy::too_many_lines)] // Closed inline cases retain each source boundary and refusal.
fn inline(path: &str, whole: &str, source: &str, base: usize) -> InlineResult {
    inline_with_locations(path, whole, source, base, &mut Vec::new())
}

#[allow(clippy::too_many_lines)] // The located projection follows the closed inline grammar.
fn inline_with_locations(
    path: &str,
    whole: &str,
    source: &str,
    base: usize,
    pieces: &mut Vec<LocatedPiece>,
) -> InlineResult {
    let mut runs = Vec::new();
    let mut targets = Vec::new();
    let mut plain = String::new();
    let mut walker = Walker::new(source, 0);
    while let Some((cursor, end, token)) = walker.next() {
        let command = match token {
            Lexeme::Comment => {
                return Err((
                    base + cursor,
                    base + cursor + 1,
                    "comment in content".into(),
                ));
            }
            Lexeme::Dollar => {
                let mut last = source.len();
                while let Some((_, close, next)) = walker.next() {
                    if next == Lexeme::Dollar {
                        last = close;
                        break;
                    }
                }
                return Err((base + cursor, base + last, "unresolved mathematics".into()));
            }
            Lexeme::Open | Lexeme::Close => {
                return Err((base + cursor, base + end, "unresolved grouping".into()));
            }
            Lexeme::Other(ch) => {
                let (width, semantic) = if source[cursor..].starts_with("---") {
                    (3, "—".to_owned())
                } else if source[cursor..].starts_with("--") {
                    (2, "–".to_owned())
                } else if source[cursor..].starts_with("``") {
                    (2, "“".to_owned())
                } else if source[cursor..].starts_with("''") {
                    (2, "”".to_owned())
                } else if ch == '\'' {
                    (1, "’".to_owned())
                } else {
                    (ch.len_utf8(), ch.to_string())
                };
                pieces.push(LocatedPiece {
                    start: base + cursor,
                    end: base + cursor + width,
                    semantic,
                    control: None,
                });
                if width > ch.len_utf8() {
                    walker.cursor = cursor + width;
                    plain.push_str(&source[cursor..walker.cursor]);
                    continue;
                }
                plain.push(ch);
                continue;
            }
            Lexeme::Ampersand => {
                pieces.push(LocatedPiece {
                    start: base + cursor,
                    end: base + end,
                    semantic: "&".into(),
                    control: None,
                });
                plain.push('&');
                continue;
            }
            Lexeme::Command(command) => command,
        };
        let Some(kind) = production(command) else {
            return Err((
                base + cursor,
                base + end,
                format!("unsupported command {command}"),
            ));
        };
        if kind == Production::Escape {
            let escaped = command.chars().nth(1).expect("escaped command");
            pieces.push(LocatedPiece {
                start: base + cursor,
                end: base + end,
                semantic: escaped.to_string(),
                control: None,
            });
            plain.push(escaped);
            continue;
        }
        if kind == Production::Spacing {
            pieces.push(LocatedPiece {
                start: base + cursor,
                end: base + end,
                semantic: " ".into(),
                control: Some("en-space"),
            });
            if !plain.is_empty() {
                runs.push(Run::Text {
                    decoded: decoded_text(&std::mem::take(&mut plain)),
                    spacing_intent: "source".into(),
                });
            }
            runs.push(Run::Text {
                decoded: " ".into(),
                spacing_intent: "en-space".into(),
            });
            while matches!(source.as_bytes().get(walker.cursor), Some(b' ' | b'\t')) {
                walker.cursor += 1;
            }
            let line_end_consumed = if source[walker.cursor..].starts_with("\r\n") {
                walker.cursor += 2;
                true
            } else if source.as_bytes().get(walker.cursor) == Some(&b'\n') {
                walker.cursor += 1;
                true
            } else {
                false
            };
            if line_end_consumed {
                while matches!(source.as_bytes().get(walker.cursor), Some(b' ' | b'\t')) {
                    walker.cursor += 1;
                }
            }
            if walker.cursor > end {
                pieces.push(LocatedPiece {
                    start: base + end,
                    end: base + walker.cursor,
                    semantic: " ".into(),
                    control: None,
                });
            }
            continue;
        }
        if !matches!(
            kind,
            Production::Style
                | Production::Tag
                | Production::Handle
                | Production::Reference
                | Production::Citation
        ) {
            return Err((
                base + cursor,
                base + end,
                format!("unsupported command {command}"),
            ));
        }
        if !plain.is_empty() {
            let decoded = decoded_text(&std::mem::take(&mut plain));
            runs.push(Run::Text {
                decoded,
                spacing_intent: "source".into(),
            });
        }
        let star = source.as_bytes().get(end) == Some(&b'*');
        let argument_start = end + usize::from(star);
        if source.as_bytes().get(argument_start) == Some(&b'[') {
            return Err((
                base + cursor,
                base + argument_start + 1,
                format!("unsupported optional argument of {command}"),
            ));
        }
        if source.as_bytes().get(argument_start) != Some(&b'{') {
            return Err((
                base + cursor,
                base + argument_start.min(source.len()),
                format!("missing argument of {command}"),
            ));
        }
        let Some((argument, next)) = braced(source, argument_start) else {
            return Err((
                base + cursor,
                base + source.len(),
                format!("unbalanced argument of {command}"),
            ));
        };
        let run = match kind {
            Production::Style => {
                pieces.push(LocatedPiece {
                    start: base + cursor,
                    end: base + argument_start + 1,
                    semantic: String::new(),
                    control: Some(if command == "\\textbf" {
                        "strong-open"
                    } else if command == "\\emph" {
                        "emphasis-open"
                    } else {
                        "unsupported-style"
                    }),
                });
                let (children, nested) = inline_with_locations(
                    path,
                    whole,
                    argument,
                    base + argument_start + 1,
                    pieces,
                )?;
                pieces.push(LocatedPiece {
                    start: base + next - 1,
                    end: base + next,
                    semantic: String::new(),
                    control: Some(if command == "\\textbf" {
                        "strong-close"
                    } else if command == "\\emph" {
                        "emphasis-close"
                    } else {
                        "unsupported-style"
                    }),
                });
                targets.extend(nested);
                Run::Style {
                    style: command[1..].into(),
                    runs: children,
                }
            }
            Production::Tag => {
                targets.push(TargetDraft {
                    selector: argument.into(),
                    exact_key: argument.into(),
                    source_command: format!("{command}{}", if star { "*" } else { "" }),
                    site: site(path, whole, base + cursor, base + next),
                    occurrence: 0,
                });
                Run::Tag {
                    command: command.into(),
                    star,
                    key: argument.into(),
                }
            }
            Production::Handle => {
                targets.push(TargetDraft {
                    selector: argument.into(),
                    exact_key: argument.into(),
                    source_command: format!("{command}{}", if star { "*" } else { "" }),
                    site: site(path, whole, base + cursor, base + next),
                    occurrence: 0,
                });
                Run::Handle {
                    star,
                    selector: argument.into(),
                }
            }
            Production::Reference => Run::Reference {
                command: command.into(),
                star,
                key: argument.into(),
            },
            Production::Citation => Run::Citation {
                command: command.into(),
                ordered_keys: argument
                    .split(',')
                    .map(str::trim)
                    .map(str::to_owned)
                    .collect(),
            },
            _ => unreachable!("closed command set was checked"),
        };
        runs.push(run);
        walker.cursor = next;
    }
    if !plain.is_empty() {
        let decoded = decoded_text(&plain);
        runs.push(Run::Text {
            decoded,
            spacing_intent: "source".into(),
        });
    }
    Ok((runs, targets))
}

fn environment_end(text: &str, start: usize) -> Option<(usize, String)> {
    let mut stack: Vec<String> = Vec::new();
    let mut walker = Walker::new(text, start);
    while let Some((_, end, token)) = walker.next() {
        if let Lexeme::Command(command) = token
            && matches!(
                production(command),
                Some(Production::Begin | Production::End)
            )
        {
            let (name, next) = braced(text, end)?;
            if production(command) == Some(Production::Begin) {
                stack.push(name.into());
            } else if stack.pop().as_deref() != Some(name) {
                return None;
            }
            if stack.is_empty() {
                return Some((next, name.into()));
            }
            walker.cursor = next;
        }
    }
    None
}

fn environment_kind(name: &str) -> BlockKind {
    match name {
        "equation" | "equation*" | "align" | "align*" | "gather" => BlockKind::Display,
        "itemize" | "enumerate" | "description" => BlockKind::List,
        "tabular" | "tabularx" | "longtable" | "table" => BlockKind::Table,
        "theorem" | "lemma" | "proposition" | "proof" | "remark" | "corollary" | "definition"
        | "postulate" | "insight" => BlockKind::TheoremContainer,
        "econ" => BlockKind::EconomicsBox,
        "epist" => BlockKind::EpistemologyBox,
        "statement" | "statementbox" => BlockKind::StatementBox,
        "quote" | "quotation" => BlockKind::Quotation,
        "abstract" => BlockKind::Abstract,
        _ => BlockKind::Refusal,
    }
}

fn table_fields(text: &str, start: usize, closed: usize) -> Option<BlockFields> {
    let header = "\\begin{tabular}";
    let begin = start + header.len();
    let (columns, body_start) = braced(text, begin)?;
    let ending = "\\end{tabular}";
    let body_end = closed.checked_sub(ending.len())?;
    if body_end < body_start || !text[body_end..closed].starts_with(ending) {
        return None;
    }
    let body = &text[body_start..body_end];
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut depth = 0usize;
    let mut walker = Walker::new(body, 0);
    while let Some((_, _, token)) = walker.next() {
        match token {
            Lexeme::Open => depth += 1,
            Lexeme::Close => depth = depth.checked_sub(1)?,
            Lexeme::Command("\\\\") if depth == 0 => {
                row.push(TableCell {
                    text: cell.trim().into(),
                    row_span: 1,
                    column_span: 1,
                    header: false,
                });
                rows.push(std::mem::take(&mut row));
                cell.clear();
            }
            Lexeme::Command(_) | Lexeme::Comment | Lexeme::Dollar => return None,
            Lexeme::Ampersand if depth == 0 => {
                row.push(TableCell {
                    text: cell.trim().into(),
                    row_span: 1,
                    column_span: 1,
                    header: false,
                });
                cell.clear();
            }
            Lexeme::Ampersand => cell.push('&'),
            Lexeme::Other(ch) => cell.push(ch),
        }
    }
    if depth != 0 {
        return None;
    }
    if !cell.trim().is_empty() || !row.is_empty() {
        row.push(TableCell {
            text: cell.trim().into(),
            row_span: 1,
            column_span: 1,
            header: false,
        });
        rows.push(row);
    }
    if rows.is_empty() || rows.iter().any(|row| row.len() != columns.chars().count()) {
        return None;
    }
    Some(BlockFields::Table {
        columns: columns.into(),
        rows,
        caption: None,
    })
}

fn child_hook_end(text: &str, start: usize, line_end: usize) -> Option<usize> {
    if !text[start..line_end].starts_with('\\') {
        return None;
    }
    let (command, command_end) = command_at(text, start);
    let kind = production(command)?;
    if !matches!(
        kind,
        Production::Checking
            | Production::Label
            | Production::ParagraphHook
            | Production::Layout
            | Production::Event
    ) {
        return None;
    }
    let mut end = if text.as_bytes().get(command_end) == Some(&b'{') {
        braced(text, command_end)?.1
    } else if matches!(
        kind,
        Production::Checking | Production::Label | Production::Layout
    ) {
        return None;
    } else {
        command_end
    };
    if command == "\\PaperDisplayParagraph" {
        end = braced(text, end)?.1;
    }
    (end <= line_end).then_some(end)
}

#[allow(clippy::collapsible_if, clippy::too_many_lines)] // Each nested boundary has a distinct failure condition.
pub(super) fn container_children(
    path: &str,
    text: &str,
    start: usize,
    end: usize,
) -> (Vec<BlockDraft>, Vec<TargetDraft>) {
    let (command, command_end) = command_at(text, start);
    if production(command) != Some(Production::Begin) {
        return (Vec::new(), Vec::new());
    }
    let Some((_, mut body_start)) = braced(text, command_end) else {
        return (Vec::new(), Vec::new());
    };
    if text.as_bytes().get(body_start) == Some(&b'[') {
        let mut walker = Walker::new(text, body_start + 1);
        let mut depth = 0usize;
        let mut found = None;
        while let Some((_, next, token)) = walker.next() {
            match token {
                Lexeme::Open => depth += 1,
                Lexeme::Close => depth = depth.saturating_sub(1),
                Lexeme::Other(']') if depth == 0 => {
                    found = Some(next);
                    break;
                }
                _ => {}
            }
        }
        let Some(next) = found else {
            return (Vec::new(), Vec::new());
        };
        body_start = next;
    }
    let Some(close_line) = text[body_start..end]
        .rfind("\\end{")
        .map(|offset| body_start + offset)
    else {
        return (Vec::new(), Vec::new());
    };
    let mut children = Vec::new();
    let mut targets = Vec::new();
    let mut cursor = body_start;
    let mut after_display = false;
    while cursor < close_line {
        let line_end = text[cursor..close_line]
            .find('\n')
            .map_or(close_line, |offset| cursor + offset + 1);
        if text[cursor..line_end].trim().is_empty() {
            after_display = false;
            cursor = line_end;
            continue;
        }
        let indent = text[cursor..line_end].len() - text[cursor..line_end].trim_start().len();
        let mut begin = cursor + indent;
        while let Some(next) = child_hook_end(text, begin, line_end) {
            begin = next;
            while matches!(text.as_bytes().get(begin), Some(b' ' | b'\t')) {
                begin += 1;
            }
        }
        if text[begin..line_end].trim().is_empty() || text[begin..line_end].starts_with('%') {
            cursor = line_end;
            continue;
        }
        if text[begin..line_end].starts_with("\\begin{") {
            if let Some((closed, name)) = environment_end(text, begin) {
                if closed <= close_line {
                    let child_end = text[closed..close_line]
                        .find('\n')
                        .map_or(close_line, |offset| closed + offset + 1);
                    children.push(BlockDraft {
                        local_index: children.len(),
                        site: site(path, text, begin, child_end),
                        kind: environment_kind(&name),
                        fields: BlockFields::Unresolved {
                            production: name.clone(),
                        },
                        children: Vec::new(),
                        events: Vec::new(),
                        status: BlockStatus::Unresolved,
                    });
                    after_display = matches!(environment_kind(&name), BlockKind::Display);
                    cursor = child_end;
                    continue;
                }
            }
        }
        let mut paragraph_end = line_end;
        if !boundary_line(&text[cursor..line_end]) || opening_hook(&text[cursor..line_end]) {
            while paragraph_end < close_line {
                let next_end = text[paragraph_end..close_line]
                    .find('\n')
                    .map_or(close_line, |offset| paragraph_end + offset + 1);
                if boundary_line(&text[paragraph_end..next_end]) {
                    break;
                }
                paragraph_end = next_end;
            }
        }
        let fragment = text[begin..paragraph_end].trim_end();
        let fields = match inline(path, text, fragment, begin) {
            Ok((runs, child_targets)) => {
                targets.extend(child_targets);
                BlockFields::Paragraph {
                    continuation: after_display,
                    layout: Vec::new(),
                    runs,
                }
            }
            Err(_) => BlockFields::Unresolved {
                production: "paragraph".into(),
            },
        };
        let status = if matches!(fields, BlockFields::Unresolved { .. }) {
            BlockStatus::Unresolved
        } else {
            BlockStatus::Decoded
        };
        children.push(BlockDraft {
            local_index: children.len(),
            site: site(path, text, begin, paragraph_end),
            kind: BlockKind::Paragraph,
            fields,
            children: Vec::new(),
            events: Vec::new(),
            status,
        });
        after_display = false;
        cursor = paragraph_end;
    }
    (children, targets)
}

fn heading(text: &str) -> Option<(&str, bool, &str, usize)> {
    if !text.starts_with('\\') {
        return None;
    }
    let (command, end) = command_at(text, 0);
    if matches!(
        production(command),
        Some(Production::Heading | Production::Appendix)
    ) {
        let star = text.as_bytes().get(end) == Some(&b'*');
        let start = end + usize::from(star);
        if let Some((title, end)) = braced(text, start) {
            return Some((command, star, title, end));
        }
    }
    None
}

fn boundary_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.trim().is_empty()
        || trimmed.starts_with('%')
        || (trimmed.starts_with('\\')
            && matches!(
                production(command_at(trimmed, 0).0),
                Some(
                    Production::Begin
                        | Production::End
                        | Production::Heading
                        | Production::Appendix
                        | Production::Wrapper
                        | Production::Layout
                        | Production::Checking
                        | Production::Label
                        | Production::ParagraphHook
                        | Production::Event
                )
            ))
}

fn opening_hook(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('\\')
        && matches!(
            production(command_at(trimmed, 0).0),
            Some(
                Production::Layout
                    | Production::Checking
                    | Production::Label
                    | Production::ParagraphHook
            )
        )
}

fn admitted_layout(value: &str) -> bool {
    value.split(',').all(|part| {
        matches!(
            part,
            "lastline" | "wholewords" | "stretch=2em" | "stretch=4em" | "rightskip=0pt plus 1em"
        )
    })
}

fn admitted_display_layout(value: &str) -> bool {
    let trimmed = value.trim();
    if !trimmed.starts_with('\\') {
        return false;
    }
    let (command, command_end) = command_at(trimmed, 0);
    if command != "\\PaperDisplayParagraph" {
        return false;
    }
    let rest = &trimmed[command_end..];
    let Some((kind, after_kind)) = braced(rest, 0) else {
        return false;
    };
    let Some((options, after_options)) = braced(rest, after_kind) else {
        return false;
    };
    matches!(kind, "equation" | "align")
        && admitted_layout(options)
        && rest[after_options..].trim().is_empty()
}

struct Importer<'a> {
    path: &'a str,
    text: &'a str,
    markers: &'a [(String, String)],
    draft: SourceDraft,
    advance: Option<usize>,
    pending_display: bool,
}
impl Importer<'_> {
    fn math_fault(&mut self, start: usize, end: usize) {
        if self
            .draft
            .math
            .iter()
            .any(|item| item.site.start_byte == start)
        {
            return;
        }
        let index = self.draft.math.len();
        self.draft.math.push(MathDraft {
            local_key: format!("unresolved-{index}"),
            source: self.text[start..end].into(),
            site: site(self.path, self.text, start, end),
            parse_status: "unresolved".into(),
            dependencies: Vec::new(),
            refusals: vec!["unresolved mathematics".into()],
        });
    }

    fn targets(&mut self, targets: Vec<TargetDraft>) {
        for mut target in targets {
            target.occurrence = self.draft.metadata_targets.len();
            self.draft.metadata_targets.push(target);
        }
    }
    fn event(&mut self, start: usize, end: usize, kind: &str) {
        let index = self.draft.events.len();
        self.draft.events.push(EventDraft {
            kind: kind.into(),
            site: site(self.path, self.text, start, end),
            source: self.text[start..end].into(),
        });
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner: AccountOwner::Event(index),
        });
    }
    fn block(
        &mut self,
        start: usize,
        end: usize,
        kind: BlockKind,
        fields: BlockFields,
        native: bool,
    ) {
        let index = self.draft.blocks.len();
        self.draft.blocks.push(BlockDraft {
            local_index: index,
            site: site(self.path, self.text, start, end),
            kind,
            fields,
            children: Vec::new(),
            events: Vec::new(),
            status: BlockStatus::Decoded,
        });
        let owner = if native {
            AccountOwner::ExistingNative(index)
        } else {
            AccountOwner::Content(index)
        };
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner,
        });
    }
    #[allow(clippy::too_many_arguments)] // Accounted range and fault range are independent.
    fn refuse(
        &mut self,
        start: usize,
        end: usize,
        fault_start: usize,
        fault_end: usize,
        kind: &BlockKind,
        command: &str,
        reason: &str,
    ) {
        let mut end = end;
        let mut fault_end = fault_end;
        let mut reason = reason.to_owned();
        if reason.starts_with("unbalanced argument") {
            let open = self.text[fault_start..]
                .find('{')
                .map(|offset| fault_start + offset);
            if let Some((_, closed)) = open.and_then(|open| braced(self.text, open)) {
                fault_end = closed;
                if closed > end {
                    end = self.text[closed..]
                        .find('\n')
                        .map_or(self.text.len(), |offset| closed + offset + 1);
                    reason = "argument crosses block boundary".into();
                }
            } else {
                end = self.text.len();
                fault_end = end;
            }
            self.advance = Some(end);
        }
        if reason == "unresolved mathematics" {
            self.math_fault(fault_start, fault_end);
        }
        let index = self.draft.refusals.len();
        let block_index = self.draft.blocks.len();
        self.draft.refusals.push(Refusal {
            outer_block_index: block_index,
            is_primary: true,
            code: reason.clone(),
            site: site(self.path, self.text, fault_start, fault_end),
            block_kind: kind.clone(),
            command_or_environment: command.into(),
            reason,
        });
        let children = if matches!(
            kind,
            BlockKind::TheoremContainer
                | BlockKind::StatementBox
                | BlockKind::EconomicsBox
                | BlockKind::EpistemologyBox
        ) {
            container_children(self.path, self.text, start, end).0
        } else {
            Vec::new()
        };
        self.draft.blocks.push(BlockDraft {
            local_index: block_index,
            site: site(self.path, self.text, start, end),
            kind: kind.clone(),
            fields: BlockFields::Unresolved {
                production: command.into(),
            },
            children,
            events: Vec::new(),
            status: BlockStatus::Unresolved,
        });
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner: AccountOwner::Refusal(index),
        });
        self.scan_faults(start, end, block_index, kind);
    }

    #[allow(clippy::too_many_lines, clippy::collapsible_if)] // One pass preserves source order across lexical fault kinds.
    fn scan_faults(&mut self, start: usize, end: usize, block_index: usize, kind: &BlockKind) {
        let text = self.text;
        let mut walker = Walker::new(&text[..end], start);
        while let Some((at, next, token)) = walker.next() {
            let fault = match token {
                Lexeme::Comment
                    if at == start
                        && *kind == BlockKind::ExistingNativeInclusion
                        && text[start..end].starts_with("% generated: ") =>
                {
                    None
                }
                Lexeme::Comment => {
                    Some((at, at + 1, "comment in content".to_owned(), "%".to_owned()))
                }
                Lexeme::Dollar => {
                    let mut closed = end;
                    while let Some((_, last, item)) = walker.next() {
                        if item == Lexeme::Dollar {
                            closed = last;
                            break;
                        }
                    }
                    Some((
                        at,
                        closed,
                        "unresolved mathematics".to_owned(),
                        "$".to_owned(),
                    ))
                }
                Lexeme::Command(command) => match production(command) {
                    Some(Production::Begin | Production::End) => {
                        if let Some((name, after)) = braced(self.text, next) {
                            walker.cursor = after;
                            if command == "\\begin"
                                && matches!(
                                    name,
                                    "equation"
                                        | "equation*"
                                        | "align"
                                        | "align*"
                                        | "gather"
                                        | "gathered"
                                        | "split"
                                        | "array"
                                        | "cases"
                                        | "pmatrix"
                                )
                            {
                                if let Some((closed, _)) = environment_end(self.text, at) {
                                    if closed <= end {
                                        let code = "unresolved mathematics";
                                        self.math_fault(at, closed);
                                        if at != start
                                            && !self.draft.refusals.iter().any(|item| {
                                                item.outer_block_index == block_index
                                                    && item.site.start_byte == at
                                                    && item.code == code
                                            })
                                        {
                                            self.draft.refusals.push(Refusal {
                                                outer_block_index: block_index,
                                                is_primary: false,
                                                code: code.into(),
                                                site: site(self.path, self.text, at, closed),
                                                block_kind: kind.clone(),
                                                command_or_environment: name.into(),
                                                reason: code.into(),
                                            });
                                        }
                                        walker.cursor = closed;
                                    }
                                }
                            }
                            (command == "\\begin" && at != start).then(|| {
                                (
                                    at,
                                    after,
                                    format!("unsupported environment {name}"),
                                    name.to_owned(),
                                )
                            })
                        } else {
                            Some((
                                at,
                                next,
                                "unbalanced environment".to_owned(),
                                command.to_owned(),
                            ))
                        }
                    }
                    Some(
                        Production::Style
                        | Production::Tag
                        | Production::Handle
                        | Production::Reference
                        | Production::Citation,
                    ) => {
                        let argument_start =
                            next + usize::from(self.text.as_bytes().get(next) == Some(&b'*'));
                        if self.text.as_bytes().get(argument_start) == Some(&b'[') {
                            Some((
                                at,
                                argument_start + 1,
                                format!("unsupported optional argument of {command}"),
                                command.to_owned(),
                            ))
                        } else if self.text.as_bytes().get(argument_start) != Some(&b'{') {
                            Some((
                                at,
                                argument_start.min(end),
                                format!("missing argument of {command}"),
                                command.to_owned(),
                            ))
                        } else {
                            None
                        }
                    }
                    Some(Production::Opaque) => {
                        if let Some(delimiter) = self.text[next..end].chars().next()
                            && let Some(offset) =
                                self.text[next + delimiter.len_utf8()..end].find(delimiter)
                        {
                            walker.cursor =
                                next + delimiter.len_utf8() + offset + delimiter.len_utf8();
                        }
                        Some((
                            at,
                            next,
                            format!("unsupported command {command}"),
                            command.to_owned(),
                        ))
                    }
                    Some(_) => None,
                    None => Some((
                        at,
                        next,
                        format!("unsupported command {command}"),
                        command.to_owned(),
                    )),
                },
                _ => None,
            };
            let Some((fault_start, fault_end, code, command)) = fault else {
                continue;
            };
            if code == "unresolved mathematics" {
                self.math_fault(fault_start, fault_end);
            }
            if self.draft.refusals.iter().any(|item| {
                item.outer_block_index == block_index
                    && item.site.start_byte == fault_start
                    && (item.code == code || !item.is_primary)
            }) {
                continue;
            }
            self.draft.refusals.push(Refusal {
                outer_block_index: block_index,
                is_primary: false,
                code: code.clone(),
                site: site(self.path, self.text, fault_start, fault_end),
                block_kind: kind.clone(),
                command_or_environment: command,
                reason: code,
            });
        }
    }
    #[allow(clippy::too_many_lines, clippy::collapsible_if)] // One located source line has closed lexical cases.
    fn line(&mut self, start: usize, end: usize) {
        let raw = &self.text[start..end];
        let trimmed = raw.trim_start();
        let prefix = raw.len() - trimmed.len();
        if prefix > 0 {
            self.event(start, start + prefix, "lexical-trivia");
        }
        let body_start = start + prefix;
        if body_start == end {
            return;
        }
        let body = &self.text[body_start..end];
        if body.starts_with('%') {
            let first_end = body
                .find('\n')
                .map_or(end, |offset| body_start + offset + 1);
            if let Some((unit, role)) = self.markers.iter().find(|(unit, _)| {
                prefix == 0
                    && (body_start == 0 || self.text.as_bytes().get(body_start - 1) == Some(&b'\n'))
                    && self.text[body_start..first_end]
                        .strip_suffix('\n')
                        .unwrap_or(&self.text[body_start..first_end])
                        == format!("% generated: {unit}")
            }) {
                self.block(
                    body_start,
                    first_end,
                    BlockKind::ExistingNativeInclusion,
                    BlockFields::NativeInclusion {
                        marker: self.text[body_start..first_end]
                            .trim_end_matches('\n')
                            .into(),
                        unit: unit.clone(),
                        role: role.clone(),
                    },
                    true,
                );
            } else if body.starts_with("% generated: ") {
                self.refuse(
                    body_start,
                    first_end,
                    body_start,
                    first_end,
                    &BlockKind::ExistingNativeInclusion,
                    "% generated:",
                    "unowned generated marker",
                );
            } else {
                self.event(body_start, first_end, "lexical-trivia");
            }
            if first_end < end {
                self.line(first_end, end);
            }
            return;
        }
        if (body.starts_with('\\')
            && production(command_at(body, 0).0) == Some(Production::Wrapper))
            || body.starts_with("\\end{document}")
        {
            self.event(body_start, end, "document-wrapper");
            return;
        }
        if body.starts_with('\\') && command_at(body, 0).0 == "\\PaperParagraph" {
            let rest = &body["\\PaperParagraph".len()..];
            if let Some((options, end_option)) = braced(rest, 0) {
                if admitted_layout(options) {
                    let consumed = "\\PaperParagraph".len() + end_option;
                    self.event(body_start, body_start + consumed, "layout-instruction");
                    if body_start + consumed < end {
                        self.line(body_start + consumed, end);
                    }
                    return;
                }
            }
            self.refuse(
                body_start,
                end,
                body_start,
                end,
                &BlockKind::Paragraph,
                "\\PaperParagraph",
                "unsupported layout option",
            );
            return;
        }
        if let Some((level, star, title, consumed)) = heading(body) {
            if body[consumed..].trim().is_empty() {
                self.pending_display = false;
                let title_start = body_start + consumed - title.len() - 1;
                match inline(self.path, self.text, title, title_start) {
                    Ok((runs, targets)) => {
                        self.targets(targets);
                        self.block(
                            body_start,
                            end,
                            BlockKind::Heading,
                            BlockFields::Heading {
                                level: level.into(),
                                star,
                                title: runs,
                                label_selector: None,
                            },
                            false,
                        );
                    }
                    Err((a, b, reason)) => {
                        self.refuse(body_start, end, a, b, &BlockKind::Heading, level, &reason);
                    }
                }
                return;
            }
        }
        if body.starts_with('\\') && production(command_at(body, 0).0) == Some(Production::Begin) {
            self.refuse(
                body_start,
                end,
                body_start,
                end,
                &BlockKind::Refusal,
                "\\begin",
                "unbalanced environment",
            );
            return;
        }
        if body.starts_with('\\') {
            let (command, command_end) = command_at(body, 0);
            if production(command) == Some(Production::Label) {
                if let Some((key, consumed)) = braced(body, command_end) {
                    self.targets(vec![TargetDraft {
                        selector: key.into(),
                        exact_key: key.into(),
                        source_command: command.into(),
                        site: site(self.path, self.text, body_start, body_start + consumed),
                        occurrence: 0,
                    }]);
                    self.event(body_start, body_start + consumed, "label-event");
                    if body_start + consumed < end {
                        self.line(body_start + consumed, end);
                    }
                } else {
                    self.refuse(
                        body_start,
                        end,
                        body_start,
                        end,
                        &BlockKind::Refusal,
                        command,
                        "malformed label",
                    );
                }
                return;
            }
            if matches!(
                production(command),
                Some(
                    Production::Checking
                        | Production::Event
                        | Production::ParagraphHook
                        | Production::Appendix
                )
            ) {
                let consumed = if body.as_bytes().get(command_end) == Some(&b'{') {
                    let Some((_, next)) = braced(body, command_end) else {
                        self.refuse(
                            body_start,
                            end,
                            body_start,
                            end,
                            &BlockKind::Refusal,
                            command,
                            &format!("unbalanced argument of {command}"),
                        );
                        return;
                    };
                    next
                } else {
                    command_end
                };
                self.event(
                    body_start,
                    body_start + consumed,
                    if command == "\\appendix" {
                        "appendix-event"
                    } else if production(command) == Some(Production::Checking) {
                        "checking-effect"
                    } else {
                        "source-event"
                    },
                );
                if body_start + consumed < end {
                    self.line(body_start + consumed, end);
                }
                return;
            }
        }
        match inline(self.path, self.text, body, body_start) {
            Ok((runs, targets)) => {
                self.targets(targets);
                let continuation = std::mem::take(&mut self.pending_display);
                self.block(
                    body_start,
                    end,
                    BlockKind::Paragraph,
                    BlockFields::Paragraph {
                        continuation,
                        layout: Vec::new(),
                        runs,
                    },
                    false,
                );
            }
            Err((a, b, reason)) => {
                self.pending_display = false;
                self.refuse(
                    body_start,
                    end,
                    a,
                    b,
                    &BlockKind::Paragraph,
                    &body[a - body_start..b - body_start],
                    &reason,
                );
            }
        }
    }
}

/// Decode bounded forms and account for all supplied bytes without admitting them.
/// # Errors
/// Refuses invalid UTF-8 or an invalid independent accounting partition.
/// # Panics
/// Panics only if a line loses an opener already found in it or a block just inserted cannot be accessed; supplied bytes cannot arrange either condition.
#[allow(clippy::too_many_lines, clippy::single_match_else)] // The source-order pass closes each block before accounting it.
pub fn import_source(
    path: &str,
    bytes: &[u8],
    digest: String,
    markers: &[(String, String)],
) -> Result<SourceDraft, ImportFault> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| ImportFault::Encoding { path: path.into() })?;
    let mut importer = Importer {
        path,
        text,
        markers,
        draft: SourceDraft {
            path: path.into(),
            digest,
            document: if path.contains("/explainer/") {
                "explainer"
            } else {
                "paper"
            }
            .into(),
            language: "en".into(),
            proposed_unit_id: None,
            metadata_targets: Vec::new(),
            math: Vec::new(),
            blocks: Vec::new(),
            events: Vec::new(),
            accounting: Vec::new(),
            refusals: Vec::new(),
            byte_length: bytes.len(),
        },
        advance: None,
        pending_display: false,
    };
    let mut cursor = 0usize;
    while cursor < text.len() {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset + 1);
        let line = &text[cursor..line_end];
        let trimmed = line.trim_start();
        if trimmed.trim().is_empty() {
            importer.pending_display = false;
        }
        if trimmed.starts_with("\\begin{document}") {
            importer.event(cursor, line_end, "document-wrapper");
            cursor = line_end;
        } else if trimmed.starts_with('\\')
            && command_at(trimmed, 0).0 == "\\PaperDisplayParagraph"
            && trimmed.contains("\\begin{")
        {
            let begin = cursor + line.find("\\begin{").expect("checked line contains begin");
            if admitted_display_layout(&text[cursor..begin]) {
                importer.event(cursor, begin, "layout-instruction");
            } else {
                importer.refuse(
                    cursor,
                    begin,
                    cursor,
                    begin,
                    &BlockKind::Display,
                    "\\PaperDisplayParagraph",
                    "unsupported layout option",
                );
            }
            if let Some((closed, name)) = environment_end(text, begin) {
                let end = text[closed..]
                    .find('\n')
                    .map_or(text.len(), |offset| closed + offset + 1);
                importer.refuse(
                    begin,
                    end,
                    begin,
                    closed,
                    &environment_kind(&name),
                    &name,
                    &format!("unsupported environment {name}"),
                );
                importer.pending_display = matches!(environment_kind(&name), BlockKind::Display);
                cursor = end;
            } else {
                importer.refuse(
                    begin,
                    text.len(),
                    begin,
                    text.len(),
                    &BlockKind::Refusal,
                    "\\begin",
                    "unbalanced environment",
                );
                cursor = text.len();
            }
        } else if trimmed.starts_with("\\begin{") {
            let begin = cursor + line.len() - trimmed.len();
            if begin > cursor {
                importer.event(cursor, begin, "lexical-trivia");
            }
            match environment_end(text, begin) {
                Some((closed, name)) => {
                    let end = text[closed..]
                        .find('\n')
                        .map_or(text.len(), |offset| closed + offset + 1);
                    if name == "tabular" {
                        if text[closed..end].trim().is_empty()
                            && let Some(fields) = table_fields(text, begin, closed)
                        {
                            importer.block(begin, end, BlockKind::Table, fields, false);
                        } else {
                            importer.refuse(
                                begin,
                                end,
                                begin,
                                closed,
                                &BlockKind::Table,
                                &name,
                                &format!("unsupported environment {name}"),
                            );
                        }
                    } else if let Some(result_kind) = ResultKind::from_name(&name) {
                        match decode_result(path, text, begin, closed, end, result_kind) {
                            Ok((fields, children, targets)) => {
                                importer.targets(targets);
                                importer.block(
                                    begin,
                                    end,
                                    BlockKind::TheoremContainer,
                                    fields,
                                    false,
                                );
                                importer
                                    .draft
                                    .blocks
                                    .last_mut()
                                    .expect("block just inserted")
                                    .children = children;
                            }
                            Err((fault_start, fault_end, reason)) => importer.refuse(
                                begin,
                                end,
                                fault_start,
                                fault_end,
                                &BlockKind::TheoremContainer,
                                &name,
                                &reason,
                            ),
                        }
                    } else {
                        importer.refuse(
                            begin,
                            end,
                            begin,
                            closed,
                            &environment_kind(&name),
                            &name,
                            &format!("unsupported environment {name}"),
                        );
                    }
                    importer.pending_display =
                        matches!(environment_kind(&name), BlockKind::Display);
                    cursor = end;
                }
                None => {
                    importer.refuse(
                        begin,
                        text.len(),
                        begin,
                        text.len(),
                        &BlockKind::Refusal,
                        "\\begin",
                        "unbalanced environment",
                    );
                    cursor = text.len();
                }
            }
        } else {
            let mut end = line_end;
            if !boundary_line(line) || opening_hook(line) {
                while end < text.len() {
                    let next_end = text[end..]
                        .find('\n')
                        .map_or(text.len(), |offset| end + offset + 1);
                    if boundary_line(&text[end..next_end]) {
                        break;
                    }
                    end = next_end;
                }
            }
            importer.line(cursor, end);
            cursor = importer.advance.take().unwrap_or(end);
        }
    }
    validate_accounting(&importer.draft, bytes)?;
    Ok(importer.draft)
}

/// Verify the byte partition independently of the importer that produced it.
/// # Errors
/// Refuses holes, overlaps, split UTF-8 boundaries and unknown owners.
pub fn validate_accounting(draft: &SourceDraft, bytes: &[u8]) -> Result<(), ImportFault> {
    let text = std::str::from_utf8(bytes).map_err(|_| ImportFault::Encoding {
        path: draft.path.clone(),
    })?;
    let mut cursor = 0usize;
    for piece in &draft.accounting {
        let reason = if piece.start_byte > cursor {
            Some("hole")
        } else if piece.start_byte < cursor {
            Some("overlap")
        } else if piece.end_byte <= piece.start_byte || piece.end_byte > bytes.len() {
            Some("range")
        } else if !text.is_char_boundary(piece.start_byte) || !text.is_char_boundary(piece.end_byte)
        {
            Some("unicode boundary")
        } else {
            match piece.owner {
                AccountOwner::Content(index) | AccountOwner::ExistingNative(index)
                    if index >= draft.blocks.len() =>
                {
                    Some("missing owner")
                }
                AccountOwner::Event(index) if index >= draft.events.len() => Some("missing owner"),
                AccountOwner::Refusal(index) if index >= draft.refusals.len() => {
                    Some("missing owner")
                }
                AccountOwner::Refusal(index) if !draft.refusals[index].is_primary => {
                    Some("secondary refusal owner")
                }
                _ => None,
            }
        };
        if let Some(reason) = reason {
            return Err(ImportFault::Accounting {
                path: draft.path.clone(),
                at: cursor,
                reason,
            });
        }
        cursor = piece.end_byte;
    }
    if cursor != bytes.len() {
        return Err(ImportFault::Accounting {
            path: draft.path.clone(),
            at: cursor,
            reason: "hole",
        });
    }
    Ok(())
}

/// Confirm that the bytes used for a diagnostic draft are still the bytes at its consumer.
/// # Errors
/// Refuses any changed byte, including a changed or missing suffix.
pub fn ensure_unchanged(path: &str, captured: &[u8], current: &[u8]) -> Result<(), ImportFault> {
    if captured == current {
        Ok(())
    } else {
        Err(ImportFault::InputChanged { path: path.into() })
    }
}
