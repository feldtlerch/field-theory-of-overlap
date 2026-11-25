use super::diagnostics::{
    AssemblyDiagnostic, AssemblyErrors, AssemblyFault, DocumentError, EconReason, EmissionKind,
    EmissionRecord, GapPosition, RefrainRole,
};
use crate::RenderableNotation;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, CheckedSkeletons, DocumentTarget, MathUseSite, RenderTarget,
    RuleId, SourceOrigin, UnitSite, WeldedUnits,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StyleKind {
    Emphasis,
    Strong,
    Smallcaps,
    Regular,
}

#[cfg(test)]
mod econ_controls {
    use super::*;
    use std::{fs, path::Path};

    #[test]
    #[allow(clippy::too_many_lines)] // Each mutation shares the same checked unit and source evidence.
    fn real_econ_unit_refuses_changed_title_owner_and_body_forms() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let declaration = crate::UnitSource {
            file: "units/documents.toml".into(),
            bytes: fs::read(root.join("units/documents.toml")).unwrap(),
        };
        let source = "units/statements/coda-flourishing-pole.toml";
        let original = fs::read_to_string(root.join(source)).unwrap();
        let load = |text: &str| {
            crate::load_unit_sources(
                declaration.clone(),
                vec![crate::UnitSource {
                    file: source.into(),
                    bytes: text.as_bytes().to_vec(),
                }],
            )
            .unwrap()
            .units()[0]
                .clone()
        };
        let unit = load(&original);
        let graph = ftoo_manifest::seed().unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let lookup = super::super::registry::CheckedPaperLookup::check(&root).unwrap();
        let mut inputs = BTreeMap::new();
        for path in [
            "documents/paper-build/sources.tsv",
            "documents/paper/econ_membrane.tex",
            "documents/paper/data/ftoo_manifest.tex",
        ] {
            inputs.insert(path.into(), fs::read(root.join(path)).unwrap());
        }
        for path in fs::read_to_string(root.join("documents/paper-build/sources.tsv"))
            .unwrap()
            .lines()
        {
            inputs.insert(path.into(), fs::read(root.join(path)).unwrap());
        }
        let site = unit.econ().unwrap().citations[0].site.clone();
        let entry = super::super::bibliography::Entry {
            key: "a:sencapability".into(),
            author: "Sen, Amartya".into(),
            year: "1999".into(),
            title: "Development as Freedom".into(),
            details: super::super::bibliography::Details::Book {
                publisher: "Oxford University Press".into(),
                address: "Oxford".into(),
            },
            raw: String::new(),
            label: "A. Sen 1999".into(),
            anchor: "bibliography-sen-1999".into(),
        };
        let citation = || Inline::BibliographyCitation {
            kind: ftoo_manifest::BibliographyCitationKind::Parenthetical,
            entries: vec![entry.clone()],
            label: "(A. Sen 1999)".into(),
            site: site.clone(),
        };
        let runs = vec![Inline::Text("The flourishing pole is ".into()), citation()];
        econ_plan(&unit, "en", &runs, &book, Some(&lookup), &inputs).unwrap();
        let changed_title = load(&original.replace(
            "title = { en = \"The flourishing pole\" }",
            "title = { en = \"Another title\" }",
        ));
        assert!(
            format!(
                "{:?}",
                econ_plan(&changed_title, "en", &runs, &book, Some(&lookup), &inputs).unwrap_err()
            )
            .contains("TitleMismatch")
        );
        let changed_owner = load(&original.replace(
            "id = \"sec:coda:flourishing-pole\"",
            "id = \"sec:coda:another-pole\"",
        ));
        assert!(
            format!(
                "{:?}",
                econ_plan(&changed_owner, "en", &runs, &book, Some(&lookup), &inputs).unwrap_err()
            )
            .contains("TargetOwnerMismatch")
        );
        let unsupported = vec![
            Inline::Tag {
                handle: "op:vmoral".into(),
                label: "op:vmoral".into(),
                token: "u0-t0".into(),
                site: site.clone(),
            },
            citation(),
        ];
        assert!(
            format!(
                "{:?}",
                econ_plan(&unit, "en", &unsupported, &book, Some(&lookup), &inputs).unwrap_err()
            )
            .contains("UnsupportedInline")
        );
        let twice = vec![citation(), citation()];
        assert!(
            format!(
                "{:?}",
                econ_plan(&unit, "en", &twice, &book, Some(&lookup), &inputs).unwrap_err()
            )
            .contains("CitationMismatch")
        );
    }
}
impl StyleKind {
    fn from_checked(value: &str) -> Self {
        match value {
            "emphasis" => Self::Emphasis,
            "strong" => Self::Strong,
            "smallcaps" => Self::Smallcaps,
            "regular" => Self::Regular,
            _ => unreachable!("checked style kind"),
        }
    }
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Emphasis => "emphasis",
            Self::Strong => "strong",
            Self::Smallcaps => "smallcaps",
            Self::Regular => "regular",
        }
    }
}
impl std::fmt::Display for StyleKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
#[cfg(test)]
#[path = "tests/citations.rs"]
pub(super) mod citation_tests;
#[cfg(test)]
#[path = "tests/reader.rs"]
mod reader_tests;
#[cfg(test)]
#[path = "tests/retained.rs"]
mod retained_tests;

#[derive(Clone)]
#[allow(clippy::large_enum_variant)] // Citation identity remains inline in the shared run representation.
pub(super) enum Inline {
    Text(String),
    Tag {
        handle: String,
        label: String,
        token: String,
        site: UnitSite,
    },
    PaperTag {
        value: ftoo_manifest::PaperTagValue,
        mint: Option<String>,
        site: UnitSite,
    },
    See {
        label: String,
        locator: String,
        owner: String,
        language: String,
        fragment: String,
        site: UnitSite,
    },
    Virtue {
        word: String,
        companion: bool,
        key: String,
        semantic: String,
        owner: String,
        language: String,
        fragment: String,
        site: UnitSite,
    },
    Math {
        definition_site: UnitSite,
        key: String,
        markdown: String,
        latex: String,
        pending: Vec<crate::PendingEntry>,
        site: UnitSite,
    },
    Citation {
        identity: super::CitationIdentity,
        text: String,
        site: UnitSite,
    },
    BibliographyCitation {
        kind: ftoo_manifest::BibliographyCitationKind,
        entries: Vec<super::bibliography::Entry>,
        label: String,
        site: UnitSite,
    },
    Mention {
        text: String,
        site: UnitSite,
    },
    Styled {
        kind: StyleKind,
        children: Vec<Inline>,
        child_sites: Vec<UnitSite>,
        site: UnitSite,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum Recovered {
    Text(String),
    Tag {
        handle: String,
        label: String,
        token: Option<String>,
    },
    Math(String),
    Citation {
        handle: String,
        label: String,
        token: Option<String>,
    },
    BibliographyCitation {
        kind: ftoo_manifest::BibliographyCitationKind,
        keys: Vec<String>,
        label: String,
        anchors: Vec<String>,
    },
    Mention(String),
    See {
        label: Option<String>,
        locator: Option<String>,
        href: Option<String>,
    },
    Virtue {
        word: String,
        key: Option<String>,
        href: Option<String>,
        companion: Option<bool>,
    },
    Styled {
        kind: StyleKind,
        children: Vec<Recovered>,
    },
}

fn entity(text: &str, code: bool) -> Option<(char, usize)> {
    let end = text.find(';')?;
    let name = &text[1..end];
    let c = match name {
        "amp" if code => '&',
        "lt" if code => '<',
        "gt" if code => '>',
        _ => char::from_u32(name.strip_prefix('#')?.parse().ok()?)?,
    };
    Some((c, end + 1))
}
fn decode_code(text: &str) -> Option<String> {
    let mut result = String::new();
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with('&') {
            let (c, n) = entity(&text[at..], true)?;
            result.push(c);
            at += n;
        } else {
            let c = text[at..].chars().next()?;
            result.push(c);
            at += c.len_utf8();
        }
    }
    Some(result)
}
fn flush_text(output: &mut Vec<Recovered>, text: &mut String) {
    if !text.is_empty() {
        output.push(Recovered::Text(std::mem::take(text)));
    }
}
fn recover_bibliography_markdown(source: &str) -> Option<(Recovered, usize)> {
    let (kind, mut rest) = if let Some(rest) = source.strip_prefix("(<a href=\"#") {
        (ftoo_manifest::BibliographyCitationKind::Parenthetical, rest)
    } else {
        (
            ftoo_manifest::BibliographyCitationKind::Textual,
            source.strip_prefix("<a href=\"#")?,
        )
    };
    let mut anchors = Vec::new();
    let mut labels = Vec::new();
    loop {
        let (anchor, tail) = rest.split_once("\">")?;
        if !anchor.starts_with("bibliography-") {
            return None;
        }
        let (label, tail) = tail.split_once("</a>")?;
        anchors.push(anchor.to_owned());
        labels.push(decode_code(label)?);
        if let Some(next) = tail.strip_prefix("; <a href=\"#") {
            rest = next;
        } else {
            rest = tail;
            break;
        }
    }
    if kind == ftoo_manifest::BibliographyCitationKind::Parenthetical {
        rest = rest.strip_prefix(')')?;
    }
    let after = rest.strip_prefix("<!-- bibcite ")?;
    let (keys, rest) = after.split_once(" -->")?;
    let keys = keys.split(',').map(str::to_owned).collect::<Vec<_>>();
    if keys.len() != anchors.len() {
        return None;
    }
    let label = if kind == ftoo_manifest::BibliographyCitationKind::Parenthetical {
        format!("({})", labels.join("; "))
    } else {
        labels.join("; ")
    };
    let consumed = source.len() - rest.len();
    Some((
        Recovered::BibliographyCitation {
            kind,
            keys,
            label,
            anchors,
        },
        consumed,
    ))
}
fn inline_handle(run: &Inline) -> Option<&str> {
    match run {
        Inline::Tag { handle, .. }
        | Inline::PaperTag {
            value: ftoo_manifest::PaperTagValue {
                spelling: handle, ..
            },
            ..
        }
        | Inline::Citation {
            identity: super::CitationIdentity { handle, .. },
            ..
        } => Some(handle),
        _ => None,
    }
}
fn inline_handles(run: &Inline) -> Vec<&str> {
    match run {
        Inline::Styled { children, .. } => children.iter().flat_map(inline_handles).collect(),
        _ => inline_handle(run).into_iter().collect(),
    }
}
#[allow(clippy::too_many_lines)] // One pass preserves the order of all admitted inline forms.
fn recover_markdown<'a>(
    body: &str,
    handles: impl Iterator<Item = &'a str>,
) -> Option<Vec<Recovered>> {
    let handles: Vec<_> = handles.collect();
    let mut output = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    while at < body.len() {
        let tail = &body[at..];
        if tail.starts_with("<!-- label ")
            || tail.starts_with("<!-- cite ")
            || tail.starts_with("<!-- handle ")
        {
            let citation = tail.starts_with("<!-- cite ");
            let mint = tail.starts_with("<!-- handle ");
            let comment_prefix = if citation {
                10
            } else if mint {
                12
            } else {
                11
            };
            let tail = &tail[comment_prefix..];
            let end = tail.find(" -->")?;
            let (selector, label) = if mint {
                let (selector, key) = tail[..end].split_once(' ')?;
                (Some(selector), key)
            } else {
                (None, &tail[..end])
            };
            let handle = handles
                .iter()
                .copied()
                .filter(|handle| text.ends_with(handle))
                .max_by_key(|handle| handle.len())?;
            let prefix = text.strip_suffix(handle)?.to_owned();
            text = prefix;
            flush_text(&mut output, &mut text);
            output.push(if citation {
                Recovered::Citation {
                    handle: handle.into(),
                    label: label.into(),
                    token: None,
                }
            } else {
                Recovered::Tag {
                    handle: handle.into(),
                    label: label.into(),
                    token: selector.map(str::to_owned),
                }
            });
            at += comment_prefix + end + 4;
        } else if tail.starts_with("(<a href=\"")
            || tail
                .strip_prefix("<a href=\"")
                .and_then(|_| tail.split_once("</a>"))
                .is_some_and(|(_, after)| {
                    after.starts_with("<!-- bibcite ")
                        || after.starts_with(")<!-- bibcite ")
                        || after.starts_with("; <a href=\"")
                })
        {
            flush_text(&mut output, &mut text);
            let (citation, consumed) = recover_bibliography_markdown(tail)?;
            output.push(citation);
            at += consumed;
        } else if tail
            .strip_prefix("<a href=\"")
            .and_then(|after| after.split_once("\">"))
            .is_some_and(|(_, body)| body.starts_with("<strong><em>"))
        {
            let end = tail.find(" -->")? + 4;
            let value = &tail[..end];
            let value = value.strip_prefix("<a href=\"")?;
            let (href, value) = value.split_once("\"><strong><em>")?;
            let (word, value) = value.split_once("</em></strong></a><!-- virtue ")?;
            let key = value.strip_suffix(" -->")?;
            if word.is_empty() || !word.bytes().all(|byte| byte.is_ascii_lowercase()) {
                return None;
            }
            flush_text(&mut output, &mut text);
            output.push(Recovered::Virtue {
                word: word.into(),
                key: Some(key.into()),
                href: Some(href.into()),
                companion: None,
            });
            at += end;
        } else if let Some(after) = tail.strip_prefix("<a href=\"") {
            let (inside, after) = after.split_once("</a>")?;
            let (href, locator) = inside.split_once("\">→ ")?;
            if locator.contains('<') || locator.contains('>') {
                return None;
            }
            let label = after.strip_prefix("<!-- see ")?.split_once(" -->")?.0;
            flush_text(&mut output, &mut text);
            output.push(Recovered::See {
                label: Some(label.into()),
                locator: Some(locator.into()),
                href: Some(href.into()),
            });
            at += 9
                + href.len()
                + "\">→ ".len()
                + locator.len()
                + "</a><!-- see ".len()
                + label.len()
                + 4;
        } else if tail.starts_with('*') {
            flush_text(&mut output, &mut text);
            let (kind, delimiter) = if tail.starts_with("**") {
                (StyleKind::Strong, "**")
            } else {
                (StyleKind::Emphasis, "*")
            };
            let inside = tail.strip_prefix(delimiter)?;
            let end = inside.find(delimiter)?;
            if end == 0 {
                return None;
            }
            let children = recover_markdown(&inside[..end], handles.iter().copied())?;
            output.push(Recovered::Styled { kind, children });
            at += delimiter.len() * 2 + end;
        } else if let Some(tail) = tail.strip_prefix("<code>") {
            flush_text(&mut output, &mut text);
            let end = tail.find("</code>")?;
            let value = decode_code(&tail[..end])?;
            if super::markdown::code(&value) != body[at..at + 6 + end + 7] {
                return None;
            }
            at += 6 + end + 7;
            output.push(Recovered::Mention(value));
        } else if let Some(tail) = tail.strip_prefix("$`") {
            flush_text(&mut output, &mut text);
            let end = tail.find("`$")?;
            output.push(Recovered::Math(tail[..end].into()));
            at += end + 4;
        } else if tail.starts_with('$') {
            return None;
        } else if tail.starts_with('&') {
            let (c, n) = entity(tail, false)?;
            text.push(c);
            at += n;
        } else {
            let c = tail.chars().next()?;
            // This accepts raw ASCII punctuation that markdown::text encodes in authored text.
            // Heading, pending-notation, coverage and refrain frames share this path; strict refusal needs separate authored and frame bytes.
            if matches!(c, '<' | '>' | '`' | '*' | '\n' | '\r') {
                return None;
            }
            text.push(c);
            at += c.len_utf8();
        }
    }
    flush_text(&mut output, &mut text);
    (!output.iter().any(literal_recovered)).then_some(output)
}
fn literal_recovered(run: &Recovered) -> bool {
    match run {
        Recovered::Text(text) | Recovered::Mention(text) => {
            !ftoo_manifest::reader_label_literals(text).is_empty()
        }
        Recovered::Styled { children, .. } => children.iter().any(literal_recovered),
        _ => false,
    }
}
fn tex_literal(tail: &str) -> Option<(char, usize)> {
    for (spelling, c) in [
        (r"\space{}", ' '),
        (r"\textbackslash{}", '\\'),
        (r"\{", '{'),
        (r"\}", '}'),
        (r"\%", '%'),
        (r"\#", '#'),
        (r"\$", '$'),
        (r"\&", '&'),
        (r"\_", '_'),
        (r"\textasciicircum{}", '^'),
        (r"\textasciitilde{}", '~'),
    ] {
        if tail.starts_with(spelling) {
            return Some((c, spelling.len()));
        }
    }
    None
}
#[derive(Clone, Copy, PartialEq)]
enum TexLine {
    NewLine,
    MidLine,
    SkipBlanks,
}
// Literal coverage prose has no TeX whitespace normalization or control spaces.
fn exact_literal(mut encoded: &str, expected: &str) -> bool {
    for scalar in expected.chars() {
        if encoded.starts_with('\\') {
            let Some((decoded, size)) = tex_literal(encoded) else {
                return false;
            };
            if decoded != scalar || decoded == ' ' {
                return false;
            }
            encoded = &encoded[size..];
        } else {
            if matches!(
                scalar,
                '\\' | '{' | '}' | '%' | '_' | '#' | '$' | '&' | '^' | '~'
            ) {
                return false;
            }
            let Some(rest) = encoded.strip_prefix(scalar) else {
                return false;
            };
            encoded = rest;
        }
    }
    encoded.is_empty()
}
// Read escaped identity bytes and validate their discretionary boundaries from
// decoded scalar positions, independently of the writer's segment traversal.
fn recover_identity(source: &str) -> Option<String> {
    let mut value = String::new();
    let mut boundaries = Vec::new();
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix(r"\allowbreak{}") {
            boundaries.push(value.chars().count());
            rest = tail;
        } else if rest.starts_with('\\') {
            let (scalar, size) = tex_literal(rest)?;
            value.push(scalar);
            rest = &rest[size..];
        } else {
            let scalar = rest.chars().next()?;
            if matches!(scalar, '{' | '}' | '%' | '_' | '#' | '$' | '&' | '^' | '~') {
                return None;
            }
            value.push(scalar);
            rest = &rest[scalar.len_utf8()..];
        }
    }
    let scalars: Vec<_> = value.chars().collect();
    let separator = |c: &char| matches!(c, ':' | '-' | '/' | '.');
    let expected: Vec<_> = (1..scalars.len())
        .filter(|&at| {
            if separator(&scalars[at - 1]) {
                return true;
            }
            let left = scalars[..at]
                .iter()
                .rposition(separator)
                .map_or(0, |i| i + 1);
            let right = scalars[at..]
                .iter()
                .position(separator)
                .map_or(scalars.len(), |i| at + i);
            at < right && right - left > 24
        })
        .collect();
    (boundaries == expected).then_some(value)
}

// Diagnostic inventories retain semantic names. Check their paragraph framing
// independently of the sweep writer, including the scope of paragraph layout.
pub(super) fn check_sweep_pending(source: &str, entries: &[crate::PendingEntry]) -> bool {
    if entries.is_empty() {
        return source.is_empty();
    }
    let Some(mut rest) = source
        .strip_prefix("{\\raggedright\nPending notation: ")
        .and_then(|body| body.strip_suffix(".\\par}\n\n"))
    else {
        return false;
    };
    for (index, entry) in entries.iter().enumerate() {
        if index != 0 {
            let Some(tail) = rest.strip_prefix(". ") else {
                return false;
            };
            rest = tail;
        }
        let Some(tail) = rest
            .strip_prefix("\\(")
            .and_then(|tail| tail.strip_prefix(entry.body_macro()))
            .and_then(|tail| tail.strip_prefix("\\): "))
        else {
            return false;
        };
        rest = tail;
        for (reason_index, reason) in entry.reasons().iter().enumerate() {
            if reason_index != 0 {
                let Some(tail) = rest.strip_prefix("; ") else {
                    return false;
                };
                rest = tail;
            }
            let expected = reason.to_string();
            for (word_index, word) in expected.split(' ').enumerate() {
                if word_index != 0 {
                    let Some(tail) = rest.strip_prefix(' ') else {
                        return false;
                    };
                    rest = tail;
                }
                // Consume exactly one encoded token by trying UTF-8 boundaries;
                // the decoder also verifies every discretionary break position.
                let Some(end) = rest
                    .char_indices()
                    .map(|(at, scalar)| at + scalar.len_utf8())
                    .find(|&end| recover_identity(&rest[..end]).as_deref() == Some(word))
                else {
                    return false;
                };
                rest = &rest[end..];
            }
        }
    }
    rest.is_empty()
}
// TeX's input processor precedes the closed-grammar recognizer. Opaque inline
// commands emit a nonblank token; comments discard their endline. Explicit
// control spaces expand independently of the input processor's blank state.
fn recover_latex(body: &str) -> Option<Vec<Recovered>> {
    recover_tex(body, false, TexLine::NewLine)
}
fn recover_prose_latex(body: &str) -> Option<Vec<Recovered>> {
    recover_tex(body, true, TexLine::NewLine)
}
fn tex_group_end(body: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut chars = body.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '\\'
            && chars
                .peek()
                .is_some_and(|(_, next)| matches!(next, '{' | '}' | '%'))
        {
            chars.next();
        } else if c == '%' {
            while chars.peek().is_some_and(|(_, next)| *next != '\n') {
                chars.next();
            }
        } else if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }
    None
}
#[allow(clippy::too_many_lines)] // One TeX decoder keeps style boundaries and child runs in order.
fn recover_tex(body: &str, prose: bool, initial_line: TexLine) -> Option<Vec<Recovered>> {
    let mut output = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    let mut line = initial_line;
    while at < body.len() {
        let tail = &body[at..];
        if let Some((kind, prefix)) = [
            (StyleKind::Emphasis, r"\emph{"),
            (StyleKind::Strong, r"\textbf{"),
            (StyleKind::Smallcaps, r"{\scshape "),
            (StyleKind::Regular, r"{\normalfont "),
        ]
        .into_iter()
        .find(|(_, prefix)| tail.starts_with(prefix))
        {
            flush_text(&mut output, &mut text);
            let inside = &tail[prefix.len()..];
            let end = tex_group_end(inside)?;
            let children = recover_tex(&inside[..end], prose, TexLine::MidLine)?;
            output.push(Recovered::Styled { kind, children });
            at += prefix.len() + end + 1;
            line = TexLine::MidLine;
        } else if tail.starts_with(r"\parencite{") || tail.starts_with(r"\textcite{") {
            flush_text(&mut output, &mut text);
            let (kind, prefix) = if tail.starts_with(r"\parencite{") {
                (
                    ftoo_manifest::BibliographyCitationKind::Parenthetical,
                    r"\parencite{",
                )
            } else {
                (
                    ftoo_manifest::BibliographyCitationKind::Textual,
                    r"\textcite{",
                )
            };
            let body = &tail[prefix.len()..];
            let end = body.find('}')?;
            let keys = body[..end].split(',').map(str::to_owned).collect();
            output.push(Recovered::BibliographyCitation {
                kind,
                keys,
                label: String::new(),
                anchors: Vec::new(),
            });
            at += prefix.len() + end + 1;
            line = TexLine::MidLine;
        } else if let Some(tail) = tail.strip_prefix(r"\FtdocTag{") {
            flush_text(&mut output, &mut text);
            let (handle, end) = recover_handle(tail)?;
            let rest = tail[end..].strip_prefix('{')?;
            let token_end = rest.find('}')?;
            let after = &rest[token_end + 1..];
            let (citation, comment, prefix) = if let Some(comment) = after.strip_prefix("% cite ") {
                (true, comment, 7)
            } else {
                (false, after.strip_prefix("% label ")?, 8)
            };
            let comment_end = comment.find('\n')?;
            let label = comment[..comment_end].into();
            let token = Some(rest[..token_end].into());
            output.push(if citation {
                Recovered::Citation {
                    handle,
                    label,
                    token,
                }
            } else {
                Recovered::Tag {
                    handle,
                    label,
                    token,
                }
            });
            at += 10 + end + 1 + token_end + 1 + prefix + comment_end + 1;
            line = TexLine::NewLine;
        } else if let Some(tail) = tail.strip_prefix(r"\(") {
            flush_text(&mut output, &mut text);
            let end = tail.find(r"\)")?;
            output.push(Recovered::Math(tail[..end].into()));
            at += 2 + end + 2;
            line = TexLine::MidLine;
        } else if tail.starts_with(r"\virtue{") || tail.starts_with(r"\papervirtue{") {
            let (companion, prefix) = if tail.starts_with(r"\papervirtue{") {
                (true, r"\papervirtue{")
            } else {
                (false, r"\virtue{")
            };
            let rest = &tail[prefix.len()..];
            let end = rest.find('}')?;
            if !rest[..end].bytes().all(|byte| byte.is_ascii_lowercase()) || end == 0 {
                return None;
            }
            flush_text(&mut output, &mut text);
            output.push(Recovered::Virtue {
                word: rest[..end].into(),
                key: None,
                href: None,
                companion: Some(companion),
            });
            at += prefix.len() + end + 1;
            line = TexLine::MidLine;
        } else if let Some(rest) = tail.strip_prefix(r"\handle{") {
            let end = rest.find('}')?;
            if rest[..end].contains('{') {
                return None;
            }
            flush_text(&mut output, &mut text);
            output.push(Recovered::Tag {
                handle: rest[..end].into(),
                label: rest[..end].into(),
                token: None,
            });
            at += r"\handle{".len() + end + 1;
            line = TexLine::MidLine;
        } else if let Some(rest) = tail.strip_prefix(r"\see{") {
            let end = rest.find('}')?;
            if rest[..end].contains('{') {
                return None;
            }
            flush_text(&mut output, &mut text);
            output.push(Recovered::See {
                label: Some(rest[..end].into()),
                locator: None,
                href: None,
            });
            at += 5 + end + 1;
            line = TexLine::MidLine;
        } else if let Some(tail) = tail.strip_prefix(r"\texttt{") {
            flush_text(&mut output, &mut text);
            let mut value = String::new();
            let mut end = 0;
            while !tail[end..].starts_with('}') {
                if tail[end..].starts_with(r"\allowbreak{}") {
                    end += r"\allowbreak{}".len();
                } else if tail[end..].starts_with('\\') {
                    let (c, n) = tex_literal(&tail[end..])?;
                    value.push(c);
                    end += n;
                } else {
                    let c = tail[end..].chars().next()?;
                    value.push(c);
                    end += c.len_utf8();
                }
            }
            at += 8 + end + 1;
            let encoded = &tail[..end];
            if encoded.contains(r"\allowbreak{}") {
                return None;
            }
            output.push(Recovered::Mention(value));
            line = TexLine::MidLine;
        } else if tail.starts_with('\\') {
            let (c, n) = tex_literal(tail)?;
            text.push(c);
            at += n;
            line = TexLine::MidLine;
        } else {
            let c = tail.chars().next()?;
            if matches!(c, '{' | '}' | '%' | '\r') {
                return None;
            }
            if matches!(c, ' ' | '\t' | '\n') {
                if c == '\n' && line == TexLine::NewLine {
                    // A blank line is a paragraph token, outside an inline region.
                    return None;
                }
                if line == TexLine::MidLine {
                    text.push(' ');
                    line = TexLine::SkipBlanks;
                }
                if c == '\n' {
                    line = TexLine::NewLine;
                }
            } else {
                // Straight prose apostrophes share the TeX spelling of curved ones until admission narrows.
                text.push(if prose && c == '\'' { '’' } else { c });
                line = TexLine::MidLine;
            }
            at += c.len_utf8();
        }
    }
    flush_text(&mut output, &mut text);
    (!output.iter().any(literal_recovered)).then_some(output)
}
fn expected_runs(block: &Block, target: DocumentTarget) -> Vec<Recovered> {
    expected_runs_in_context(block, target, false)
}
fn expected_prose_runs(block: &Block, target: DocumentTarget) -> Vec<Recovered> {
    expected_runs_in_context(block, target, true)
}
fn expected_runs_in_context(block: &Block, target: DocumentTarget, prose: bool) -> Vec<Recovered> {
    block
        .runs
        .iter()
        .filter_map(|run| expected_run(run, target, prose))
        .collect()
}
fn expected_run(run: &Inline, target: DocumentTarget, prose: bool) -> Option<Recovered> {
    Some(match run {
        Inline::Text(text) if text.is_empty() => return None,
        Inline::Text(text) => {
            // Recovery compares reader characters while legacy straight prose remains admitted.
            Recovered::Text(if prose && target == DocumentTarget::Latex {
                text.replace('\'', "’")
            } else {
                text.clone()
            })
        }
        Inline::Tag {
            handle,
            label,
            token,
            ..
        } => Recovered::Tag {
            handle: handle.clone(),
            label: label.clone(),
            token: (target == DocumentTarget::Latex).then(|| token.clone()),
        },
        Inline::PaperTag { value, mint, .. } => Recovered::Tag {
            handle: value.spelling.clone(),
            label: value.target.clone(),
            token: (target == DocumentTarget::Markdown)
                .then(|| mint.clone())
                .flatten(),
        },
        Inline::See {
            label,
            locator,
            owner,
            language,
            fragment,
            ..
        } => Recovered::See {
            label: Some(label.clone()),
            locator: (target == DocumentTarget::Markdown).then(|| locator.clone()),
            href: (target == DocumentTarget::Markdown)
                .then(|| markdown_document_href(owner, language, fragment)),
        },
        Inline::Virtue {
            word,
            key,
            companion,
            owner,
            language,
            fragment,
            ..
        } => Recovered::Virtue {
            word: word.clone(),
            key: (target == DocumentTarget::Markdown).then(|| key.clone()),
            href: (target == DocumentTarget::Markdown)
                .then(|| markdown_document_href(owner, language, fragment)),
            companion: (target == DocumentTarget::Latex).then_some(*companion),
        },
        Inline::Math {
            markdown, latex, ..
        } => Recovered::Math(
            if target == DocumentTarget::Markdown {
                markdown
            } else {
                latex
            }
            .clone(),
        ),
        Inline::Citation { text, identity, .. } => Recovered::Citation {
            handle: identity.handle.clone(),
            label: text.clone(),
            token: (target == DocumentTarget::Latex).then(|| identity.token.clone()),
        },
        Inline::BibliographyCitation { kind, entries, .. } => Recovered::BibliographyCitation {
            kind: *kind,
            keys: entries.iter().map(|entry| entry.key.clone()).collect(),
            label: if target == DocumentTarget::Markdown {
                super::bibliography::citation_label(*kind, entries)
            } else {
                String::new()
            },
            anchors: if target == DocumentTarget::Markdown {
                entries.iter().map(|entry| entry.anchor.clone()).collect()
            } else {
                Vec::new()
            },
        },
        Inline::Mention { text, .. } => Recovered::Mention(text.clone()),
        Inline::Styled { kind, children, .. } => Recovered::Styled {
            kind: *kind,
            children: children
                .iter()
                .filter_map(|child| expected_run(child, target, prose))
                .collect(),
        },
    })
}
#[allow(clippy::too_many_lines)] // Compare one complete candidate inventory, including captions and bodies.
pub(super) fn check_emission(
    plan: &DocumentPlan,
    source: &str,
    target: DocumentTarget,
) -> Result<(), DocumentError> {
    check_coverage(plan, source, target)?;
    if target == DocumentTarget::Markdown && plan.screen.is_some() {
        return check_excerpt_emission(plan, source);
    }
    let marker = if target == DocumentTarget::Markdown {
        "<!-- unit "
    } else {
        "% unit "
    };
    let positions: Vec<_> = source
        .match_indices(marker)
        .filter(|(at, _)| {
            envelope(plan, source, target) && (*at == 0 || source.as_bytes()[at - 1] == b'\n')
        })
        .map(|(at, _)| at)
        .collect();
    let expected = plan.records(target);
    let mut missing = Vec::new();
    let mut unexpected = Vec::new();
    let mut changed = Vec::new();
    let mut actual_records = Vec::new();
    let mut first_range = None;
    for (index, &start) in positions.iter().enumerate() {
        let end = positions.get(index + 1).copied().unwrap_or(source.len());
        let chunk = &source[start..end];
        let newline = chunk.find('\n').unwrap_or(chunk.len());
        let metadata = &chunk[..newline];
        let block = plan
            .blocks
            .iter()
            .find(|block| metadata == block_metadata(plan, block, target));
        let Some(block) = block else {
            actual_records.push(unknown_block(plan, target, metadata, chunk, start..end));
            first_range.get_or_insert(start..end);
            continue;
        };
        let records: Vec<_> = expected
            .iter()
            .filter(|r| r.unit == block.unit)
            .cloned()
            .collect();
        let mut body = &chunk[newline + 1..];
        if target == DocumentTarget::Latex && index + 1 == positions.len() {
            body = body.strip_suffix("\\end{document}\n").unwrap_or(body);
            if !super::bibliography::entries(plan).is_empty() {
                body = body.strip_suffix("\\printbibliography\n").unwrap_or(body);
            }
        }
        let separator = if target == DocumentTarget::Markdown && index + 1 == positions.len() {
            "\n"
        } else {
            "\n\n"
        };
        let framed = body.strip_suffix(separator);
        let (framed, explanation) =
            match framed.and_then(|body| body.split_once("\n\nPending notation: ")) {
                Some((main, explanation)) => (Some(main), Some(explanation)),
                None => (framed, None),
            };
        let pending_expected = pending_records(plan, block, target);
        let pending_actual = recover_pending(plan, block, target, explanation, &pending_expected);
        if pending_actual != pending_expected {
            first_range.get_or_insert(start..end);
        }
        let records: Vec<_> = records
            .into_iter()
            .filter(|r| !matches!(r.kind, EmissionKind::PendingExplanation(_)))
            .collect();
        if let Some(display) = block.displays.get(&target) {
            let actual = framed
                .and_then(|body| recover_display_block(body, block, display, &records, target));
            if let Some(actual) = actual {
                if actual != records {
                    first_range.get_or_insert(start..end);
                }
                actual_records.extend(actual);
            } else {
                let text = framed.unwrap_or(body);
                if let Some((caption, display)) = text.split_once("\n\n")
                    && let Some(recovered) = recover_block(caption, block, target)
                {
                    let count = records
                        .iter()
                        .position(|r| r.kind == EmissionKind::Body)
                        .expect("display record");
                    actual_records.extend(recovered_records(
                        block,
                        &recovered,
                        &records[..count],
                        target,
                    ));
                    let mut record = records[count].clone();
                    record.value = display.into();
                    actual_records.push(record);
                } else {
                    let mut actual = records[0].clone();
                    actual.value = text.into();
                    actual_records.push(actual);
                }
                first_range.get_or_insert(start..end);
            }
            actual_records.extend(pending_actual);
            continue;
        }
        let recovered = framed.and_then(|body| recover_block(body, block, target));
        if let Some(recovered) = recovered {
            let mut actual = recovered_records(block, &recovered, &records, target);
            if actual == records && recovered != expected_prose_runs(block, target) {
                actual[0].value = chunk.into();
            }
            if actual != records
                || plan
                    .blocks
                    .get(index)
                    .is_none_or(|expected| expected.unit != block.unit)
            {
                first_range.get_or_insert(start..end);
            }
            actual_records.extend(actual);
        } else {
            let mut actual = records[0].clone();
            actual.value = framed.unwrap_or(body).into();
            actual_records.push(actual);
            first_range.get_or_insert(start..end);
        }
        actual_records.extend(pending_actual);
    }
    difference(
        &expected,
        &actual_records,
        &mut missing,
        &mut unexpected,
        &mut changed,
    );
    if !missing.is_empty() || !unexpected.is_empty() || !changed.is_empty() {
        return Err(emission_failure(
            plan,
            target,
            first_range.unwrap_or(source.len()..source.len()),
            AssemblyFault::EmissionMismatch {
                missing,
                unexpected,
                changed,
            },
        ));
    }
    Ok(())
}
fn excerpt_text(source: &str) -> Option<String> {
    let recovered = recover_markdown(source, std::iter::empty())?;
    let [Recovered::Text(value)] = recovered.as_slice() else {
        return None;
    };
    Some(value.clone())
}
fn excerpt_paragraph(source: &str, block: &Block) -> Option<String> {
    let actual = recover_markdown(source, block.runs.iter().flat_map(inline_handles))?;
    (actual == expected_runs(block, DocumentTarget::Markdown))
        .then(|| block.runs.iter().map(canonical_run).collect())
}
fn excerpt_econ(source: &str, block: &Block, econ: &EconPlan) -> Option<Vec<Recovered>> {
    let header = format!(
        "<table>\n<tr>\n<td>\n<p><a id=\"{}\"></a><strong>Economics {}. {}</strong> (on {}<!-- label {} -->)</p><!-- econ {} {} -->\n<p>",
        econ.fragment,
        econ.number,
        super::markdown::text(&econ.title),
        super::markdown::text(&econ.reference),
        econ.reference_identity,
        block.unit,
        econ.target
    );
    let footer = format!(
        "</p>\n<p align=\"right\">{}<!-- label {} --> · d:{}<!-- label {} --></p>\n</td>\n</tr>\n</table>",
        super::markdown::text(&econ.target),
        econ.target_identity,
        super::markdown::text(&econ.departure),
        econ.departure_identity
    );
    let body = source.strip_prefix(&header)?.strip_suffix(&footer)?;
    let actual = recover_markdown(body, block.runs.iter().flat_map(inline_handles))?;
    (actual == expected_runs(block, DocumentTarget::Markdown)).then_some(actual)
}
fn excerpt_unit(source: &str) -> Option<(&str, &str, &str, &str)> {
    let (metadata, body) = source.split_once('\n')?;
    let fields = metadata.strip_prefix("<!-- unit ")?.strip_suffix(" -->")?;
    let mut fields = fields.split_ascii_whitespace();
    let unit = fields.next()?;
    let key = fields.next()?;
    let document = fields.next()?;
    let language_key = fields.next()?;
    let language = fields.next()?;
    (key == "document" && language_key == "language" && fields.next().is_none())
        .then_some((unit, document, language, body))
}
fn excerpt_refrain(source: &str) -> Option<(String, Vec<String>, String, String, RefrainRole)> {
    let mut rest = source.strip_prefix("<p align=\"center\">")?;
    let mut anchors = Vec::new();
    while let Some(after) = rest.strip_prefix("<a id=\"") {
        let (fragment, tail) = after.split_once("\"></a>")?;
        anchors.push(fragment.into());
        rest = tail;
    }
    let rest = rest.strip_prefix("<em>")?;
    let (prose, rest) = rest.split_once("</em></p>\n<p align=\"right\">")?;
    if let Some((handle, tail)) = rest.split_once("<!-- label ") {
        let label = tail.strip_suffix(" --></p>")?;
        return Some((
            excerpt_text(prose)?,
            anchors,
            excerpt_text(handle)?,
            label.into(),
            RefrainRole::Owned,
        ));
    }
    if !anchors.is_empty() {
        return None;
    }
    let rest = rest.strip_prefix("<a href=\"")?;
    let (href, rest) = rest.split_once("\">")?;
    let (handle, rest) = rest.split_once("</a><!-- cite ")?;
    let label = rest.strip_suffix(" --></p>")?;
    Some((
        excerpt_text(prose)?,
        vec![href.into()],
        excerpt_text(handle)?,
        label.into(),
        RefrainRole::Quoted,
    ))
}
#[allow(clippy::too_many_lines)] // The excerpt's ordered blocks and their source ranges form one recovery pass.
fn check_excerpt_emission(plan: &DocumentPlan, source: &str) -> Result<(), DocumentError> {
    let sections = end_matter(plan);
    let entries = sections
        .iter()
        .find_map(|section| match section {
            EndMatter::References(entries) => Some(*entries),
            EndMatter::Unavailable(_) => None,
        })
        .unwrap_or(&[]);
    let base_source;
    let source = if entries.is_empty() {
        if source.contains("\n\n## References\n\n") {
            return Err(emission_failure(
                plan,
                DocumentTarget::Markdown,
                0..source.len(),
                AssemblyFault::EmissionMismatch {
                    missing: vec![],
                    unexpected: vec![],
                    changed: vec![],
                },
            ));
        }
        source
    } else {
        let marker = "\n\n## References\n\n";
        let Some((prefix, section)) = source.split_once(marker) else {
            return Err(emission_failure(
                plan,
                DocumentTarget::Markdown,
                0..source.len(),
                AssemblyFault::EmissionMismatch {
                    missing: plan.records(DocumentTarget::Markdown),
                    unexpected: vec![],
                    changed: vec![],
                },
            ));
        };
        if !super::bibliography::recover_screen(section, entries) {
            return Err(emission_failure(
                plan,
                DocumentTarget::Markdown,
                prefix.len()..source.len(),
                AssemblyFault::EmissionMismatch {
                    missing: plan.records(DocumentTarget::Markdown),
                    unexpected: vec![],
                    changed: vec![],
                },
            ));
        }
        base_source = format!("{prefix}\n");
        &base_source
    };
    let screen = plan.screen.as_ref().expect("screen plan");
    let expected = plan.records(DocumentTarget::Markdown);
    if !envelope(plan, source, DocumentTarget::Markdown) {
        return Err(emission_failure(
            plan,
            DocumentTarget::Markdown,
            source.len()..source.len(),
            AssemblyFault::EmissionMismatch {
                missing: expected,
                unexpected: vec![],
                changed: vec![],
            },
        ));
    }
    let mut actual = Vec::new();
    let mut first_range = None;
    let start = source.find("<!-- unit ").unwrap_or(source.len());
    let mut cursor = start;
    let mut body_index = 0;
    for chunk in source[start..]
        .strip_suffix('\n')
        .expect("checked envelope")
        .split("\n\n")
    {
        let range = cursor..cursor + chunk.len();
        cursor = range.end + 2;
        let before = actual.len();
        if let Some((unit, document, language, body)) = excerpt_unit(chunk) {
            if document != plan.document {
                actual.push(unknown_block(
                    plan,
                    DocumentTarget::Markdown,
                    chunk.split_once('\n').map_or(chunk, |(head, _)| head),
                    chunk,
                    range.clone(),
                ));
                first_range.get_or_insert(range);
                continue;
            }
            if let Some((index, block)) = plan
                .blocks
                .iter()
                .enumerate()
                .find(|(_, block)| block.unit == unit)
            {
                let kind = if index == 0 {
                    EmissionKind::Heading
                } else {
                    EmissionKind::Paragraph
                };
                let econ_recovered = block
                    .econ
                    .as_ref()
                    .and_then(|econ| excerpt_econ(body, block, econ));
                let text = if index == 0 {
                    body.strip_prefix("# ").and_then(excerpt_text)
                } else if block.econ.is_some() {
                    econ_recovered
                        .as_ref()
                        .map(|_| block.runs.iter().map(canonical_run).collect())
                } else if block.paragraph {
                    excerpt_paragraph(body, block)
                } else if block.equation {
                    recover_block(body, block, DocumentTarget::Markdown)
                        .map(|_| block.runs.iter().map(canonical_run).collect())
                } else {
                    excerpt_refrain(body).map(|value| value.0)
                };
                actual.push(EmissionRecord {
                    kind,
                    unit: unit.into(),
                    language: language.into(),
                    target: DocumentTarget::Markdown,
                    occurrence: None,
                    value: text.unwrap_or_else(|| body.into()),
                    site: block.site.clone(),
                });
                if index > 0 {
                    if block.paragraph || block.equation {
                        if let Some(found) = if block.econ.is_some() {
                            econ_recovered.clone()
                        } else if block.equation {
                            recover_block(body, block, DocumentTarget::Markdown)
                        } else {
                            recover_markdown(body, block.runs.iter().flat_map(inline_handles))
                        } {
                            for (run, recovered) in block.runs.iter().zip(found) {
                                let (kind, value, site) = match (run, recovered) {
                                    (
                                        Inline::PaperTag { site, .. },
                                        Recovered::Tag { label, .. },
                                    ) => (EmissionKind::Tag, label, site),
                                    (
                                        Inline::Math {
                                            key,
                                            site,
                                            markdown,
                                            ..
                                        },
                                        Recovered::Math(body),
                                    ) if body == *markdown => {
                                        (EmissionKind::Math, key.clone(), site)
                                    }
                                    (
                                        Inline::See { label, site, .. },
                                        Recovered::See {
                                            locator: Some(locator),
                                            href: Some(href),
                                            ..
                                        },
                                    ) => (
                                        EmissionKind::See {
                                            target: label.clone(),
                                        },
                                        format!("{locator}\t{href}"),
                                        site,
                                    ),
                                    (
                                        Inline::Virtue { key, site, .. },
                                        Recovered::Virtue {
                                            word,
                                            key: Some(found_key),
                                            href: Some(href),
                                            ..
                                        },
                                    ) if key == &found_key => (
                                        EmissionKind::Virtue { key: key.clone() },
                                        format!("{word}\t{href}"),
                                        site,
                                    ),
                                    (
                                        Inline::BibliographyCitation { site, .. },
                                        Recovered::BibliographyCitation {
                                            kind,
                                            keys,
                                            label,
                                            anchors,
                                        },
                                    ) => (
                                        EmissionKind::BibliographyCitation {
                                            kind,
                                            keys,
                                            anchors,
                                        },
                                        label,
                                        site,
                                    ),
                                    _ => continue,
                                };
                                actual.push(EmissionRecord {
                                    kind,
                                    unit: unit.into(),
                                    language: language.into(),
                                    target: DocumentTarget::Markdown,
                                    occurrence: site.occurrence,
                                    value,
                                    site: site.clone(),
                                });
                            }
                        }
                    } else if let Some((_, destinations, handle, label, role)) =
                        excerpt_refrain(body)
                    {
                        let site = screen.refrains
                            [refrain_index(&plan.blocks, index).expect("refrain")]
                        .site
                        .clone();
                        for destination in destinations {
                            let kind = if role == RefrainRole::Quoted {
                                let (owner, language, fragment, _) = parse_public_href(
                                    &destination,
                                )
                                .unwrap_or(("", "", "", String::new()));
                                EmissionKind::QuotationLink {
                                    owner_document: owner.into(),
                                    language: language.into(),
                                    reader_path: format!(
                                        "generated/documents/{owner}/{language}.md"
                                    ),
                                    fragment: fragment.into(),
                                }
                            } else {
                                EmissionKind::PublicAnchor {
                                    fragment: destination.clone(),
                                }
                            };
                            actual.push(EmissionRecord {
                                kind,
                                unit: unit.into(),
                                language: language.into(),
                                target: DocumentTarget::Markdown,
                                occurrence: refrain_index(&plan.blocks, index),
                                value: destination,
                                site: site.clone(),
                            });
                        }
                        actual.push(EmissionRecord {
                            kind: EmissionKind::RefrainHandle {
                                role,
                                handle: handle.clone(),
                                bound_label: label,
                            },
                            unit: unit.into(),
                            language: language.into(),
                            target: DocumentTarget::Markdown,
                            occurrence: refrain_index(&plan.blocks, index),
                            value: handle,
                            site,
                        });
                    }
                    body_index += 1;
                }
            } else {
                actual.push(unknown_block(
                    plan,
                    DocumentTarget::Markdown,
                    chunk.split_once('\n').map_or(chunk, |(head, _)| head),
                    chunk,
                    range.clone(),
                ));
            }
        } else if chunk == "## References to unavailable content" {
            actual.push(EmissionRecord {
                kind: EmissionKind::IndexHeading,
                unit: String::new(),
                language: plan.language.clone(),
                target: DocumentTarget::Markdown,
                occurrence: None,
                value: chunk.into(),
                site: plan.blocks[0].site.clone(),
            });
        } else if let Some(rest) = chunk.strip_prefix("<a id=\"") {
            let entry = rest.split_once("\"></a>");
            let (fragment, value) = entry.unwrap_or(("", chunk));
            actual.push(EmissionRecord {
                kind: EmissionKind::IndexEntry {
                    fragment: fragment.into(),
                },
                unit: String::new(),
                language: plan.language.clone(),
                target: DocumentTarget::Markdown,
                occurrence: None,
                value: excerpt_text(value).unwrap_or_else(|| value.into()),
                site: plan.blocks[0].site.clone(),
            });
        } else {
            let gap_index = body_index;
            let position = if gap_index == 0 {
                GapPosition::Before
            } else if gap_index + 1 == plan.blocks.len() {
                GapPosition::After
            } else {
                GapPosition::Between
            };
            let site = screen
                .gaps
                .get(gap_index)
                .unwrap_or(&screen.gaps[0])
                .site
                .clone();
            actual.push(EmissionRecord {
                kind: EmissionKind::Gap {
                    index: gap_index,
                    position,
                },
                unit: String::new(),
                language: plan.language.clone(),
                target: DocumentTarget::Markdown,
                occurrence: Some(gap_index),
                value: excerpt_text(chunk).unwrap_or_else(|| chunk.into()),
                site,
            });
        }
        if expected.get(before..actual.len()) != Some(&actual[before..]) {
            first_range.get_or_insert(range);
        }
    }
    let (mut missing, mut unexpected, mut changed) = (vec![], vec![], vec![]);
    difference(
        &expected,
        &actual,
        &mut missing,
        &mut unexpected,
        &mut changed,
    );
    if missing.is_empty() && unexpected.is_empty() && changed.is_empty() {
        return Ok(());
    }
    Err(emission_failure(
        plan,
        DocumentTarget::Markdown,
        first_range.unwrap_or(start..source.len()),
        AssemblyFault::EmissionMismatch {
            missing,
            unexpected,
            changed,
        },
    ))
}
fn emission_failure(
    plan: &DocumentPlan,
    target: DocumentTarget,
    bytes: std::ops::Range<usize>,
    fault: AssemblyFault,
) -> DocumentError {
    super::failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Generated {
            path: format!(
                "generated/documents/{}/{}.{}",
                plan.document,
                plan.language,
                if target == DocumentTarget::Markdown {
                    "labels.md"
                } else {
                    "tex"
                }
            ),
            bytes,
            origins: plan.origins(target),
        },
        fault,
    )
}
fn unknown_block(
    plan: &DocumentPlan,
    target: DocumentTarget,
    metadata: &str,
    chunk: &str,
    bytes: std::ops::Range<usize>,
) -> EmissionRecord {
    let mut site = super::input::site(&format!(
        "generated/documents/{}/{}.{}",
        plan.document,
        plan.language,
        if target == DocumentTarget::Markdown {
            "labels.md"
        } else {
            "tex"
        }
    ));
    site.anchor = ftoo_manifest::SourceAnchor::Bytes(bytes.start, bytes.end);
    let fields = metadata
        .strip_prefix("<!-- unit ")
        .or_else(|| metadata.strip_prefix("% unit "))
        .unwrap_or("");
    EmissionRecord {
        kind: EmissionKind::Paragraph,
        unit: fields.split_ascii_whitespace().next().unwrap_or("").into(),
        language: fields.split_ascii_whitespace().nth(4).unwrap_or("").into(),
        target,
        occurrence: None,
        value: chunk.into(),
        site,
    }
}
fn block_metadata(plan: &DocumentPlan, block: &Block, target: DocumentTarget) -> String {
    if target == DocumentTarget::Markdown {
        format!(
            "<!-- unit {} document {} language {} -->",
            block.unit, plan.document, plan.language
        )
    } else {
        format!(
            "% unit {} document {} language {}",
            block.unit, plan.document, plan.language
        )
    }
}
fn envelope(plan: &DocumentPlan, source: &str, target: DocumentTarget) -> bool {
    source.starts_with(&coverage_prefix(plan, target))
        && if target == DocumentTarget::Markdown {
            source.ends_with('\n') && !source.ends_with("\n\n")
        } else if !super::bibliography::entries(plan).is_empty() {
            source.ends_with("\\printbibliography\n\\end{document}\n")
        } else {
            source.ends_with("\\end{document}\n")
        }
}

pub(super) struct DisplayBody {
    pub source: String,
    pub layout: ftoo_manifest::DisplayLayout,
    pub regions: Vec<Block>,
    pub site: UnitSite,
}
pub(super) struct Block {
    pub pending_names:
        BTreeMap<(DocumentTarget, ftoo_manifest::DefinitionId), Vec<super::PendingReaderName>>,
    pub displays: BTreeMap<DocumentTarget, DisplayBody>,
    pub unit: String,
    pub depth: Option<usize>,
    pub quotation: bool,
    pub paragraph: bool,
    pub equation: bool,
    pub econ: Option<EconPlan>,
    pub site: UnitSite,
    pub runs: Vec<Inline>,
}
#[derive(Clone, Debug)]
pub(super) struct EconPlan {
    pub target: String,
    pub target_identity: String,
    pub title: String,
    pub number: String,
    pub reference: String,
    pub reference_identity: String,
    pub departure: String,
    pub departure_identity: String,
    pub stops_at: String,
    pub citations: Vec<String>,
    pub fragment: String,
}
impl Block {
    pub(super) fn equation_pair(&self) -> Option<(&str, &str)> {
        if !self.equation {
            return None;
        }
        match self.runs.as_slice() {
            [
                Inline::Math {
                    markdown, latex, ..
                },
                Inline::Text(stop),
            ] if stop == "." => Some((markdown, latex)),
            _ => None,
        }
    }
    pub fn map_pending(
        &mut self,
        book: &ftoo_manifest::AddressBook<'_>,
        index: usize,
        document: &str,
        language: &str,
    ) -> Result<(), DocumentError> {
        let mut mapped = BTreeMap::new();
        let mut errors = vec![];
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let mut entries = BTreeMap::new();
            for run in self.target_runs(target) {
                if let Inline::Math { pending, .. } = run {
                    for entry in pending {
                        entries.entry(entry.order()).or_insert(entry);
                    }
                }
            }
            for (order, entry) in entries {
                let shared = self.runs.iter().any(|run| matches!(run, Inline::Math { pending, .. } if pending.iter().any(|e| e.definition() == entry.definition())));
                let selected_target = (!shared).then_some(target);
                let origins = self.pending_origins(entry.definition(), language, selected_target);
                let mut names = vec![];
                for (reason, value) in entry.reasons().iter().enumerate() {
                    for part in value.text_parts() {
                        let ftoo_manifest::ReasonPart::Object { role, id: object } = part else {
                            continue;
                        };
                        let handle = match book.reader_identity(object) {
                            Ok(handle) => handle,
                            Err(fault) => {
                                collect_reader_error(
                                    &mut errors,
                                    reader_failure(
                                        fault,
                                        document,
                                        language,
                                        Some(&self.unit),
                                        selected_target,
                                        ftoo_manifest::ReaderIdentityPosition::Pending {
                                            unit: self.unit.clone(),
                                            definition: entry.definition(),
                                            reason,
                                        },
                                        origins.clone(),
                                    ),
                                );
                                continue;
                            }
                        };
                        names.push(super::PendingReaderName {
                            reason,
                            role,
                            object,
                            handle,
                            label: object.semantic_name().into(),
                            token: format!(
                                "u{index}-p{order}-r{reason}{}",
                                if role == ftoo_manifest::ReasonRole::Premise {
                                    "-premise"
                                } else {
                                    ""
                                }
                            ),
                        });
                    }
                }
                mapped.insert((target, entry.definition()), names);
            }
        }
        if let Some(errors) = AssemblyErrors::from_diagnostics(errors) {
            return Err(DocumentError::Assembly(errors));
        }
        self.pending_names = mapped;
        Ok(())
    }
    pub(super) fn pending_origins(
        &self,
        definition: ftoo_manifest::DefinitionId,
        language: &str,
        target: Option<DocumentTarget>,
    ) -> Vec<SourceOrigin> {
        let mut regions = vec![(None, self.runs.as_slice())];
        for (&format, display) in &self.displays {
            if target.is_none_or(|target| target == format) {
                regions.extend(
                    display
                        .regions
                        .iter()
                        .map(|region| (Some(format), region.runs.as_slice())),
                );
            }
        }
        let mut origins = vec![];
        for (target, runs) in regions {
            for run in runs {
                if let Inline::Math {
                    pending,
                    site,
                    definition_site,
                    ..
                } = run
                    && pending.iter().any(|entry| entry.definition() == definition)
                {
                    origins.extend([
                        SourceOrigin {
                            unit: Some(self.unit.clone()),
                            language: Some(language.into()),
                            target: None,
                            site: definition_site.clone(),
                        },
                        SourceOrigin {
                            unit: Some(self.unit.clone()),
                            language: Some(language.into()),
                            target,
                            site: site.clone(),
                        },
                    ]);
                }
            }
        }
        origins
    }
    pub fn target_runs(&self, target: DocumentTarget) -> impl Iterator<Item = &Inline> {
        let mut flattened = Vec::new();
        flatten_runs(&self.runs, &mut flattened);
        for region in self
            .displays
            .get(&target)
            .into_iter()
            .flat_map(|body| body.regions.iter())
        {
            flatten_runs(&region.runs, &mut flattened);
        }
        flattened.into_iter()
    }
}

pub(super) fn check_equation_shape(block: &Block) -> Result<(), DocumentError> {
    if block.equation && block.equation_pair().is_none() {
        return Err(super::failure(
            RuleId::M054,
            AssemblyStage::Ownership,
            AssemblySite::Source(block.site.clone()),
            AssemblyFault::UnknownToken {
                value: block.unit.clone(),
                expected: vec![
                    "one display mathematics insertion followed by one terminal stop".into(),
                ],
            },
        ));
    }
    Ok(())
}

// The forge consumes a backslash before punctuation before handing a display
// to its mathematics renderer. The dimensioned command carries the observed
// thick-space width through that transport.
pub(super) fn markdown_display_spelling(source: &str) -> String {
    source.replace(r"\;", r"\hspace{0.2778em}")
}
pub(super) fn flatten_runs<'a>(runs: &'a [Inline], output: &mut Vec<&'a Inline>) {
    for run in runs {
        output.push(run);
        if let Inline::Styled {
            children,
            child_sites,
            ..
        } = run
        {
            debug_assert_eq!(children.len(), child_sites.len());
            flatten_runs(children, output);
        }
    }
}
#[derive(Clone)]
pub(super) struct Coverage {
    pub declaration: String,
    pub audit: Vec<Vec<Inline>>,
    pub runs: Vec<Inline>,
    pub objects: Vec<ftoo_manifest::NodeId>,
    pub site: UnitSite,
}
impl Coverage {
    pub(super) fn all_runs(&self) -> impl Iterator<Item = &Inline> {
        self.runs.iter().chain(self.audit.iter().flatten())
    }
    #[cfg(test)]
    pub fn fixture(declaration: &str, slice: &str) -> Self {
        Self {
            declaration: declaration.into(),
            audit: vec![],
            runs: vec![Inline::Text(slice.into())],
            objects: vec![],
            site: coverage_site(None),
        }
    }
    fn build(
        book: &ftoo_manifest::AddressBook<'_>,
        declaration: &str,
        document: &str,
        language: &str,
    ) -> Result<Self, DocumentError> {
        let mut objects = book.graph().coverage().slice.clone();
        objects.sort_by_key(|object| object.semantic_name());
        let mut errors = vec![];
        let mut runs = vec![Inline::Text("Declared slice: ".into())];
        if objects.is_empty() {
            runs.push(Inline::Text("(empty)".into()));
        }
        for (index, &object) in objects.iter().enumerate() {
            if index > 0 {
                runs.push(Inline::Text(", ".into()));
            }
            let site = coverage_site(Some(index));
            let handle = match book.reader_identity(object) {
                Ok(handle) => handle,
                Err(fault) => {
                    collect_reader_error(
                        &mut errors,
                        reader_failure(
                            fault,
                            document,
                            language,
                            None,
                            None,
                            ftoo_manifest::ReaderIdentityPosition::Coverage { index, object },
                            vec![SourceOrigin {
                                unit: None,
                                language: Some(language.into()),
                                target: None,
                                site: site.clone(),
                            }],
                        ),
                    );
                    continue;
                }
            };
            runs.push(Inline::Tag {
                handle,
                label: object.semantic_name().into(),
                token: format!("g-slice-{index}"),
                site,
            });
        }
        runs.push(Inline::Text(".".into()));
        if let Some(errors) = AssemblyErrors::from_diagnostics(errors) {
            return Err(DocumentError::Assembly(errors));
        }
        Ok(Self {
            declaration: declaration.into(),
            audit: vec![],
            runs,
            objects,
            site: coverage_site(None),
        })
    }
    fn block(&self) -> Block {
        Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: String::new(),
            depth: None,
            quotation: false,
            paragraph: false,
            equation: false,
            econ: None,
            site: self.site.clone(),
            runs: self.runs.clone(),
        }
    }
}
pub(super) fn coverage_site(index: Option<usize>) -> UnitSite {
    let mut site = super::input::site("generated/manifest.json");
    site.anchor = ftoo_manifest::SourceAnchor::Unavailable;
    site.path = vec![
        ftoo_manifest::PathPart::Key("coverage".into()),
        ftoo_manifest::PathPart::Key("slice".into()),
    ];
    if let Some(index) = index {
        site.path.push(ftoo_manifest::PathPart::Index(index));
    }
    site
}
#[allow(clippy::too_many_arguments)] // Complete generated position and context are retained together.
pub(super) fn reader_failure(
    fault: ftoo_manifest::ReaderIdentityFault,
    document: &str,
    language: &str,
    unit: Option<&str>,
    target: Option<DocumentTarget>,
    position: ftoo_manifest::ReaderIdentityPosition,
    origins: Vec<SourceOrigin>,
) -> DocumentError {
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: Some(document.into()),
            language: Some(language.into()),
            unit: unit.map(Into::into),
            target,
            site: AssemblySite::ReaderIdentity { position, origins },
            fault: AssemblyFault::ReaderIdentity {
                target: fault.target,
                reason: fault.reason,
                candidates: fault.candidates,
            },
        }])
        .expect("one reader diagnostic"),
    )
}
fn coverage_prefix(plan: &DocumentPlan, target: DocumentTarget) -> String {
    if target == DocumentTarget::Markdown {
        format!(
            "<!-- generated document {} language {} -->\n",
            plan.document, plan.language
        )
    } else {
        super::latex::start_plan(plan)
    }
}
fn coverage_records(
    plan: &DocumentPlan,
    target: DocumentTarget,
    declaration: String,
    recovered: Option<&[Recovered]>,
    raw: &str,
) -> Vec<EmissionRecord> {
    let record = |kind, value, site| EmissionRecord {
        kind,
        unit: String::new(),
        language: plan.language.clone(),
        target,
        occurrence: None,
        value,
        site,
    };
    let mut records = vec![record(
        EmissionKind::CoverageDeclaration,
        declaration,
        plan.coverage.site.clone(),
    )];
    let mut text = String::new();
    let mut tags = Vec::new();
    if let Some(recovered) = recovered {
        for run in recovered {
            match run {
                Recovered::Text(value) => text.push_str(value),
                Recovered::Tag {
                    handle,
                    label,
                    token,
                } => {
                    use std::fmt::Write;
                    write!(text, "@tag{{{handle}}}").expect("string write");
                    let index = tags.len();
                    let object = plan
                        .coverage
                        .objects
                        .iter()
                        .copied()
                        .find(|object| object.semantic_name() == label);
                    let position = plan.coverage.objects.get(index).map(|&object| {
                        ftoo_manifest::ReaderIdentityPosition::Coverage { index, object }
                    });
                    tags.push(record(
                        EmissionKind::ReaderIdentity {
                            position,
                            object,
                            handle: handle.clone(),
                            token: token.clone().unwrap_or_else(|| format!("g-slice-{index}")),
                        },
                        label.clone(),
                        coverage_site(Some(index)),
                    ));
                }
                _ => text.push_str(raw),
            }
        }
    } else {
        text = raw.into();
    }
    records.push(record(
        EmissionKind::CoverageSlice,
        text,
        plan.coverage.site.clone(),
    ));
    records.extend(tags);
    records
}
fn check_coverage(
    plan: &DocumentPlan,
    source: &str,
    target: DocumentTarget,
) -> Result<(), DocumentError> {
    let prefix = coverage_prefix(plan, target);
    // Envelope failures retain the existing whole-document inventory refusal.
    let Some(rest) = source.strip_prefix(&prefix) else {
        return Ok(());
    };
    let separator = if target == DocumentTarget::Latex {
        "\n\n"
    } else {
        "<br>\n"
    };
    let (declaration, rest) = rest.split_once(separator).unwrap_or(("", rest));
    let (slice, audit_rest) = rest
        .split_once(if plan.coverage.audit.is_empty() {
            "\n\n"
        } else {
            separator
        })
        .unwrap_or((rest, ""));
    check_audit_paragraphs(
        plan,
        target,
        audit_rest,
        separator,
        source.len() - audit_rest.len(),
    )?;
    let block = plan.coverage.block();
    let expected_runs = expected_runs(&block, target);
    let expected = coverage_records(
        plan,
        target,
        plan.coverage.declaration.clone(),
        Some(&expected_runs),
        "",
    );
    let recovered = if target == DocumentTarget::Latex {
        recover_latex(slice)
    } else {
        recover_markdown(slice, block.runs.iter().flat_map(inline_handles))
    };
    let declaration = if target == DocumentTarget::Latex
        && exact_literal(declaration, &plan.coverage.declaration)
    {
        plan.coverage.declaration.clone()
    } else {
        declaration.into()
    };
    let actual = coverage_records(plan, target, declaration, recovered.as_deref(), slice);
    let (mut missing, mut unexpected, mut changed) = (vec![], vec![], vec![]);
    difference(
        &expected,
        &actual,
        &mut missing,
        &mut unexpected,
        &mut changed,
    );
    if missing.is_empty() && unexpected.is_empty() && changed.is_empty() {
        return Ok(());
    }
    Err(emission_failure(
        plan,
        target,
        prefix.len()..source.len() - rest.len() + slice.len(),
        AssemblyFault::EmissionMismatch {
            missing,
            unexpected,
            changed,
        },
    ))
}
fn check_audit_paragraphs(
    plan: &DocumentPlan,
    target: DocumentTarget,
    mut rest: &str,
    separator: &str,
    mut at: usize,
) -> Result<(), DocumentError> {
    for (index, runs) in plan.coverage.audit.iter().enumerate() {
        let ending = if target == DocumentTarget::Markdown && index + 1 == plan.coverage.audit.len()
        {
            if plan.blocks.is_empty() { "\n" } else { "\n\n" }
        } else {
            separator
        };
        let (source, next) = rest.split_once(ending).unwrap_or((rest, ""));
        rest = next;
        let begin = at;
        at += source.len() + ending.len();
        let mut block = plan.coverage.block();
        block.runs.clone_from(runs);
        let expected = expected_runs(&block, target);
        let actual = if target == DocumentTarget::Latex {
            recover_latex(source)
        } else {
            recover_markdown(source, runs.iter().flat_map(inline_handles))
        };
        {
            let records = |recovered: Option<&[Recovered]>, raw: &str| {
                let mut records = coverage_records(plan, target, String::new(), recovered, raw);
                let tags: Vec<_> = runs
                    .iter()
                    .filter(|run| matches!(run, Inline::Tag { .. }))
                    .collect();
                for (index, record) in records.iter_mut().skip(2).enumerate() {
                    if let EmissionKind::ReaderIdentity {
                        position,
                        object,
                        token,
                        ..
                    } = &mut record.kind
                    {
                        *object = ftoo_manifest::NodeId::ALL
                            .into_iter()
                            .find(|id| id.semantic_name() == record.value);
                        if let Some(Inline::Tag {
                            token: expected_token,
                            label,
                            site,
                            ..
                        }) = tags.get(index).copied()
                        {
                            if target == DocumentTarget::Markdown {
                                token.clone_from(expected_token);
                            }
                            record.site.clone_from(site);
                            let outcome_index = site
                                .path
                                .iter()
                                .find_map(|part| match part {
                                    ftoo_manifest::PathPart::Index(value) => Some(*value),
                                    ftoo_manifest::PathPart::Key(_) => None,
                                })
                                .unwrap_or(index);
                            *position = ftoo_manifest::NodeId::ALL
                                .into_iter()
                                .find(|id| id.semantic_name() == label)
                                .map(|object| ftoo_manifest::ReaderIdentityPosition::Coverage {
                                    index: outcome_index,
                                    object,
                                });
                        }
                    }
                }
                records
            };
            let (mut missing, mut unexpected, mut changed) = (vec![], vec![], vec![]);
            difference(
                &records(Some(&expected), ""),
                &records(actual.as_deref(), source),
                &mut missing,
                &mut unexpected,
                &mut changed,
            );
            if missing.is_empty() && unexpected.is_empty() && changed.is_empty() {
                continue;
            }
            return Err(emission_failure(
                plan,
                target,
                begin..begin + source.len(),
                AssemblyFault::EmissionMismatch {
                    missing,
                    unexpected,
                    changed,
                },
            ));
        }
    }
    Ok(())
}
pub(super) struct DocumentPlan {
    pub document: String,
    pub language: String,
    pub coverage: Coverage,
    pub blocks: Vec<Block>,
    pub screen: Option<ScreenPlan>,
}
#[derive(Clone, Debug)]
pub(super) struct ScreenPlan {
    pub gaps: Vec<ScreenGap>,
    pub refrains: Vec<ScreenRefrain>,
    pub index: Vec<super::destinations::IndexEntry>,
    pub html_gaps: BTreeMap<usize, ScreenGap>,
    pub end_matter: Vec<ScreenEndMatter>,
}
#[derive(Clone, Debug)]
pub(super) enum ScreenEndMatter {
    Unavailable(Vec<super::destinations::IndexEntry>),
    References(Vec<super::bibliography::Entry>),
}

pub(super) enum EndMatter<'a> {
    Unavailable(&'a [super::destinations::IndexEntry]),
    References(&'a [super::bibliography::Entry]),
}

pub(super) fn end_matter(plan: &DocumentPlan) -> Vec<EndMatter<'_>> {
    plan.screen
        .as_ref()
        .into_iter()
        .flat_map(|screen| &screen.end_matter)
        .map(|section| match section {
            ScreenEndMatter::Unavailable(entries) => EndMatter::Unavailable(entries),
            ScreenEndMatter::References(entries) => EndMatter::References(entries),
        })
        .collect()
}
pub(super) fn refrain_index(blocks: &[Block], index: usize) -> Option<usize> {
    (index > 0 && !blocks[index].paragraph && !blocks[index].equation).then(|| {
        blocks[1..index]
            .iter()
            .filter(|block| !block.paragraph && !block.equation)
            .count()
    })
}
#[derive(Clone, Debug)]
pub(super) struct ScreenGap {
    pub position: GapPosition,
    pub message: String,
    pub site: UnitSite,
    pub boundaries: Vec<UnitSite>,
}
#[derive(Clone, Debug)]
pub(super) struct ScreenRefrain {
    pub handle: String,
    pub bound_label: String,
    pub fragment: String,
    pub quotation: Option<ScreenQuotation>,
    pub site: UnitSite,
    pub binding_site: Option<UnitSite>,
}
#[derive(Clone, Debug)]
pub(super) struct ScreenQuotation {
    pub owner_document: String,
    pub language: String,
    pub reader_path: String,
    pub fragment: String,
    pub href: String,
    pub owner_site: UnitSite,
}
pub(super) fn check_public_anchors(plan: &DocumentPlan) -> Result<(), DocumentError> {
    let Some(screen) = &plan.screen else {
        return Ok(());
    };
    let mut seen = BTreeSet::from([
        "references".to_owned(),
        "references-to-unavailable-content".to_owned(),
    ]);
    for refrain in &screen.refrains {
        if refrain.quotation.is_some() {
            continue;
        }
        if refrain.fragment.starts_with("bibliography-") || !seen.insert(refrain.fragment.clone()) {
            return Err(screen_failure(
                plan,
                &refrain.site,
                AssemblyFault::DuplicatePublicAnchor {
                    fragment: refrain.fragment.clone(),
                },
            ));
        }
    }
    for econ in plan.blocks.iter().filter_map(|block| block.econ.as_ref()) {
        if !seen.insert(econ.fragment.clone()) {
            return Err(screen_failure(
                plan,
                &plan.blocks[0].site,
                AssemblyFault::DuplicatePublicAnchor {
                    fragment: econ.fragment.clone(),
                },
            ));
        }
    }
    for section in &screen.end_matter {
        match section {
            ScreenEndMatter::Unavailable(entries) => {
                for entry in entries {
                    if entry.fragment.starts_with("bibliography-")
                        || !seen.insert(entry.fragment.clone())
                    {
                        return Err(screen_failure(
                            plan,
                            &plan.blocks[0].site,
                            AssemblyFault::DuplicatePublicAnchor {
                                fragment: entry.fragment.clone(),
                            },
                        ));
                    }
                }
            }
            ScreenEndMatter::References(entries) => {
                for entry in entries {
                    if !seen.insert(entry.anchor.clone()) {
                        return Err(screen_failure(
                            plan,
                            &plan.coverage.site,
                            AssemblyFault::DuplicatePublicAnchor {
                                fragment: entry.anchor.clone(),
                            },
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
fn screen_failure(plan: &DocumentPlan, site: &UnitSite, fault: AssemblyFault) -> DocumentError {
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: Some(plan.document.clone()),
            unit: None,
            language: Some(plan.language.clone()),
            target: Some(DocumentTarget::Markdown),
            site: AssemblySite::Source(site.clone()),
            fault,
        }])
        .expect("one screen fault"),
    )
}
fn gap_template(language: &str, position: GapPosition) -> Option<&'static str> {
    (language == "en").then_some(match position {
        GapPosition::Before => "Not yet converted: earlier material is omitted from this excerpt.",
        GapPosition::Between => {
            "Not yet converted: intervening material is omitted from this excerpt."
        }
        GapPosition::After => "Not yet converted: later material is omitted from this excerpt.",
    })
}
pub(super) fn screen_gap(
    plan: &DocumentPlan,
    site: &UnitSite,
    position: GapPosition,
) -> Result<ScreenGap, DocumentError> {
    let Some(message) = gap_template(&plan.language, position) else {
        return Err(screen_failure(
            plan,
            site,
            AssemblyFault::MissingScreenTemplates {
                language: plan.language.clone(),
            },
        ));
    };
    Ok(ScreenGap {
        position,
        message: message.into(),
        site: site.clone(),
        boundaries: Vec::new(),
    })
}
pub(super) fn check_excerpt_shape(
    plan: &DocumentPlan,
    site: &UnitSite,
) -> Result<(), DocumentError> {
    let shape = plan.blocks.len() >= 2
        && plan.blocks[0].depth == Some(1)
        && plan.blocks[1..].iter().all(|block| {
            block.depth.is_none()
                && block.displays.is_empty()
                && if block.equation {
                    block.equation_pair().is_some()
                } else if block.paragraph {
                    !block.runs.is_empty()
                        && block.runs.iter().all(|run| match run {
                            Inline::Text(_)
                            | Inline::PaperTag { .. }
                            | Inline::BibliographyCitation { .. }
                            | Inline::See { .. }
                            | Inline::Virtue { .. } => true,
                            Inline::Math { latex, .. } => native_paragraph_command(latex),
                            Inline::Styled {
                                kind: StyleKind::Emphasis,
                                children,
                                ..
                            } => children
                                .iter()
                                .all(|child| matches!(child, Inline::Text(_))),
                            _ => false,
                        })
                } else {
                    matches!(block.runs.as_slice(), [Inline::Text(_)])
                }
        });
    if shape {
        Ok(())
    } else {
        Err(screen_failure(
            plan,
            site,
            AssemblyFault::ExcerptShape {
                document: plan.document.clone(),
                positions: plan.blocks.len(),
            },
        ))
    }
}
pub(super) fn native_paragraph_command(latex: &str) -> bool {
    matches!(latex, "\\comovfield{}" | "\\willvirtue{}" | "\\gensym{}")
}

fn source_material_between(
    first: &super::input::Inclusion,
    second: &super::input::Inclusion,
    inputs: &BTreeMap<String, Vec<u8>>,
) -> bool {
    if first.source != second.source {
        return true;
    }
    let bytes = &inputs[&first.source];
    bytes
        .split(|byte| *byte == b'\n')
        .skip(first.line)
        .take(second.line.saturating_sub(first.line + 1))
        .any(|line| line.iter().any(|byte| !byte.is_ascii_whitespace()))
}
pub(super) fn public_fragment(
    plan: &DocumentPlan,
    site: &UnitSite,
    handle: &str,
) -> Result<String, DocumentError> {
    encode_fragment(handle).map_err(|(character, offset)| {
        screen_failure(
            plan,
            site,
            AssemblyFault::UnsupportedPublicHandle {
                handle: handle.into(),
                character,
                offset,
            },
        )
    })
}
fn encode_fragment(handle: &str) -> Result<String, (char, usize)> {
    let mut fragment = String::new();
    if handle.is_empty() {
        return Err(('\0', 0));
    }
    for (offset, scalar) in handle.char_indices() {
        match scalar {
            'a'..='z' | '0'..='9' => fragment.push(scalar),
            '-' => fragment.push_str("--"),
            ':' => fragment.push_str("-c"),
            'A'..='Z' => {
                fragment.push_str("-u");
                fragment.push(scalar.to_ascii_lowercase());
            }
            _ => return Err((scalar, offset)),
        }
    }
    Ok(fragment)
}
pub(super) fn inverse_fragment(fragment: &str) -> Option<String> {
    let mut handle = String::new();
    let mut chars = fragment.chars().peekable();
    while let Some(scalar) = chars.next() {
        match scalar {
            'a'..='z' | '0'..='9' => handle.push(scalar),
            '-' => match chars.next()? {
                '-' => handle.push('-'),
                'c' => handle.push(':'),
                'u' => {
                    let letter = chars.next()?;
                    if !letter.is_ascii_lowercase() {
                        return None;
                    }
                    handle.push(letter.to_ascii_uppercase());
                }
                _ => return None,
            },
            _ => return None,
        }
    }
    (!handle.is_empty() && encode_fragment(&handle).ok().as_deref() == Some(fragment))
        .then_some(handle)
}
pub(super) fn parse_public_href(href: &str) -> Option<(&str, &str, &str, String)> {
    if !href.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'-' | b'.' | b'/' | b'#')
    }) {
        return None;
    }
    let (path, fragment) = href.split_once('#')?;
    let (owner, filename) = path.strip_prefix("../")?.split_once('/')?;
    let language = filename.strip_suffix(".md")?;
    let component = |value: &str| {
        value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && !value.ends_with('-')
            && !value.contains("--")
    };
    (component(owner) && component(language))
        .then(|| inverse_fragment(fragment).map(|handle| (owner, language, fragment, handle)))?
}
pub(super) fn markdown_document_href(owner: &str, language: &str, fragment: &str) -> String {
    format!("../{owner}/{language}.md#{fragment}")
}

fn public_href(
    plan: &DocumentPlan,
    site: &UnitSite,
    owner: &str,
    language: &str,
    fragment: &str,
) -> Result<String, DocumentError> {
    let href = markdown_document_href(owner, language, fragment);
    if parse_public_href(&href).is_none() {
        return Err(screen_failure(
            plan,
            site,
            AssemblyFault::InvalidQuotationHref { href },
        ));
    }
    Ok(href)
}
fn valid_see_href(owner: &str, language: &str, fragment: &str) -> bool {
    parse_public_href(&markdown_document_href(owner, language, "a")).is_some()
        && super::destinations::valid_fragment(fragment)
}
#[derive(Clone)]
struct OwnedDestination {
    document: String,
    refrain: ScreenRefrain,
}
pub(super) fn html_gaps(plan: &DocumentPlan) -> BTreeMap<usize, ScreenGap> {
    plan.blocks
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, block)| {
            !if block.equation {
                super::html::realizable_equation(block)
            } else {
                super::html::realizable(&block.runs, block.paragraph)
            }
        })
        .map(|(index, block)| {
            (
                index,
                ScreenGap {
                    position: GapPosition::Between,
                    message: "This passage is not yet available in this rendering.".into(),
                    site: block.site.clone(),
                    boundaries: vec![block.site.clone()],
                },
            )
        })
        .collect()
}
fn missing_markdown_owner(
    plan: &DocumentPlan,
    site: &UnitSite,
    owner: &str,
    unit: &str,
    document_sites: &BTreeMap<String, UnitSite>,
) -> DocumentError {
    screen_failure(
        plan,
        site,
        AssemblyFault::MissingMarkdownOwnerTarget {
            owner: owner.into(),
            language: plan.language.clone(),
            unit: unit.into(),
            owner_site: document_sites[owner].clone(),
        },
    )
}
fn reading(document: &str) -> Option<super::destinations::Reading> {
    match document {
        "paper-pilot" => Some(super::destinations::Reading::Paper),
        "explainer-pilot" => Some(super::destinations::Reading::Explainer),
        _ => None,
    }
}
fn bind_see(
    runs: &mut [Inline],
    links: &BTreeMap<String, (String, String, String, String)>,
    virtues: &BTreeMap<String, super::registry::VirtueBinding>,
) -> Result<(), (String, UnitSite)> {
    for run in runs {
        match run {
            Inline::See {
                label,
                locator,
                owner,
                language,
                fragment,
                site,
                ..
            } => {
                let Some((number, document, lang, anchor)) = links.get(label) else {
                    return Err((label.clone(), site.clone()));
                };
                if number.is_empty() || document.is_empty() || lang.is_empty() || anchor.is_empty()
                {
                    return Err((label.clone(), site.clone()));
                }
                locator.clone_from(number);
                owner.clone_from(document);
                language.clone_from(lang);
                fragment.clone_from(anchor);
            }
            Inline::Virtue {
                word,
                key,
                semantic,
                owner,
                language,
                fragment,
                site,
                ..
            } => {
                let label = format!("v:{word}");
                let Some(binding) = virtues.get(word) else {
                    return Err((label, site.clone()));
                };
                let Some((_, document, lang, anchor)) = links.get(&label) else {
                    return Err((label, site.clone()));
                };
                key.clone_from(&binding.key);
                semantic.clone_from(&binding.target);
                owner.clone_from(document);
                language.clone_from(lang);
                fragment.clone_from(anchor);
            }
            Inline::Styled { children, .. } => bind_see(children, links, virtues)?,
            _ => {}
        }
    }
    Ok(())
}
type SeeLinks = BTreeMap<String, BTreeMap<String, (String, String, String, String)>>;
type SeeIndexes = BTreeMap<String, Vec<super::destinations::IndexEntry>>;
#[allow(clippy::too_many_lines)] // Resolution retains each reference's source plan and site through one checked destination pass.
fn resolve_see(
    plans: &[DocumentPlan],
    selected: &BTreeMap<String, Vec<ftoo_manifest::DocumentOutput>>,
    inputs: &BTreeMap<String, Vec<u8>>,
    mints: &BTreeMap<String, ftoo_manifest::MintOrigin>,
    anchors: &BTreeSet<(String, String, String)>,
    virtues: &BTreeMap<String, super::registry::VirtueBinding>,
) -> Result<(SeeLinks, SeeIndexes), DocumentError> {
    use super::destinations::{self, Reference, Target};
    let mut by_language: BTreeMap<String, Vec<(Reference, UnitSite, usize)>> = BTreeMap::new();
    for (plan_index, plan) in plans.iter().enumerate() {
        if !selected[&plan.document].contains(&ftoo_manifest::DocumentOutput::Markdown) {
            for block in &plan.blocks {
                let mut runs = Vec::new();
                flatten_runs(&block.runs, &mut runs);
                if let Some(Inline::Virtue { word, site, .. }) = runs
                    .into_iter()
                    .find(|run| matches!(run, Inline::Virtue { .. }))
                {
                    return Err(screen_failure(
                        plan,
                        site,
                        AssemblyFault::ReferenceDestination {
                            label: format!("v:{word}"),
                            reason: "virtue requires a screen document".into(),
                        },
                    ));
                }
            }
            continue;
        }
        for block in &plan.blocks {
            let mut runs = Vec::new();
            flatten_runs(&block.runs, &mut runs);
            for run in runs {
                if let Inline::See { label, site, .. } = run {
                    if label.starts_with("v:") {
                        return Err(screen_failure(
                            plan,
                            site,
                            AssemblyFault::ReferenceDestination {
                                label: label.clone(),
                                reason: "virtue requires its typed occurrence".into(),
                            },
                        ));
                    }
                    let Some(document) = reading(&plan.document) else {
                        return Err(screen_failure(
                            plan,
                            site,
                            AssemblyFault::ReferenceDestination {
                                label: label.clone(),
                                reason: "unsupported reading".into(),
                            },
                        ));
                    };
                    let references = by_language.entry(plan.language.clone()).or_default();
                    references.push((
                        Reference {
                            document,
                            position: references.len(),
                            target: label.clone(),
                        },
                        site.clone(),
                        plan_index,
                    ));
                } else if let Inline::Virtue { word, site, .. } = run {
                    let label = format!("v:{word}");
                    if plan.language != "en" {
                        return Err(screen_failure(
                            plan,
                            site,
                            AssemblyFault::ReferenceDestination {
                                label,
                                reason: "virtue requires an English paragraph".into(),
                            },
                        ));
                    }
                    if !virtues.contains_key(word) {
                        return Err(screen_failure(
                            plan,
                            site,
                            AssemblyFault::ReferenceDestination {
                                label,
                                reason: "unknown virtue roster word".into(),
                            },
                        ));
                    }
                    let Some(document) = reading(&plan.document) else {
                        return Err(screen_failure(
                            plan,
                            site,
                            AssemblyFault::ReferenceDestination {
                                label,
                                reason: "unsupported virtue reading".into(),
                            },
                        ));
                    };
                    let references = by_language.entry(plan.language.clone()).or_default();
                    references.push((
                        Reference {
                            document,
                            position: references.len(),
                            target: label,
                        },
                        site.clone(),
                        plan_index,
                    ));
                }
            }
        }
    }
    let mut links = BTreeMap::new();
    let mut indexes = BTreeMap::new();
    for (language, located) in by_language {
        let fault = |label: &str, reason: String| {
            let (_, site, plan_index) = located
                .iter()
                .find(|(reference, _, _)| reference.target == label)
                .unwrap_or(&located[0]);
            screen_failure(
                &plans[*plan_index],
                site,
                AssemblyFault::ReferenceDestination {
                    label: label.into(),
                    reason,
                },
            )
        };
        let mut targets = BTreeMap::<String, Target>::new();
        for (reference, _, _) in &located {
            if targets.contains_key(&reference.target) {
                continue;
            }
            let mut target = if let Some(word) = reference.target.strip_prefix("v:") {
                let binding = virtues
                    .get(word)
                    .ok_or_else(|| fault(&reference.target, "unknown virtue roster word".into()))?;
                Target {
                    document: destinations::Reading::Paper,
                    label: binding.key.clone(),
                    decoded_title: binding.word.clone(),
                    address: destinations::Address::Virtue,
                    available: false,
                    locator: String::new(),
                }
            } else {
                destinations::target_from_inputs(&reference.target, mints, inputs)
                    .map_err(|error| fault(&reference.target, format!("{error:?}")))?
            };
            let fragment = destinations::fragment(&target)
                .map_err(|error| fault(&reference.target, format!("{error:?}")))?;
            target.available =
                anchors.contains(&("paper-pilot".into(), language.clone(), fragment));
            targets.insert(reference.target.clone(), target);
        }
        let body: Vec<_> = anchors
            .iter()
            .filter(|(_, lang, _)| lang == &language)
            .filter_map(|(document, _, fragment)| {
                reading(document).map(|reading| (reading, fragment.clone()))
            })
            .collect();
        let targets: Vec<_> = targets.into_values().collect();
        let references: Vec<_> = located
            .iter()
            .map(|(reference, _, _)| reference.clone())
            .collect();
        let entries =
            destinations::resolve(&targets, &references, &body, &language).map_err(|error| {
                let label = match &error {
                    destinations::DestinationError::UnknownTarget(label)
                    | destinations::DestinationError::DuplicateTarget(label)
                    | destinations::DestinationError::InvalidFragment(label)
                    | destinations::DestinationError::MissingHeading(label)
                    | destinations::DestinationError::UnsupportedOutline(label) => label.as_str(),
                    destinations::DestinationError::MissingBody { fragment, .. }
                    | destinations::DestinationError::DuplicateFragment { fragment, .. } => targets
                        .iter()
                        .find_map(|target| {
                            (destinations::fragment(target).ok().as_deref()
                                == Some(fragment.as_str()))
                            .then_some(target.label.as_str())
                        })
                        .unwrap_or(references[0].target.as_str()),
                    destinations::DestinationError::ReaderLabel(value) => targets
                        .iter()
                        .find(|target| target.decoded_title.contains(value))
                        .map_or(references[0].target.as_str(), |target| {
                            target.label.as_str()
                        }),
                    _ => references[0].target.as_str(),
                };
                fault(label, format!("{error:?}"))
            })?;
        for entry in &entries {
            let owner = match entry.document {
                destinations::Reading::Paper => "paper-pilot",
                destinations::Reading::Explainer => "explainer-pilot",
            };
            let placed = plans.iter().any(|plan| {
                plan.document == owner
                    && plan.language == language
                    && selected.get(owner).is_some_and(|outputs| {
                        outputs.contains(&ftoo_manifest::DocumentOutput::Markdown)
                            && outputs.contains(&ftoo_manifest::DocumentOutput::Fragment)
                    })
            });
            if !placed {
                let label = targets
                    .iter()
                    .find(|target| {
                        target.document == entry.document
                            && destinations::fragment(target).ok().as_deref()
                                == Some(entry.fragment.as_str())
                    })
                    .map_or(references[0].target.as_str(), |target| {
                        target.label.as_str()
                    });
                return Err(fault(
                    label,
                    "unavailable target has no owning screen".into(),
                ));
            }
        }
        let mut resolved = BTreeMap::new();
        for target in &targets {
            let fragment = destinations::fragment(target)
                .map_err(|error| fault(&target.label, format!("{error:?}")))?;
            let owner = match target.document {
                destinations::Reading::Paper => "paper-pilot",
                destinations::Reading::Explainer => "explainer-pilot",
            };
            if !valid_see_href(owner, &language, &fragment) {
                return Err(fault(&target.label, "invalid public href".into()));
            }
            resolved.insert(
                target.label.clone(),
                (
                    target.locator.clone(),
                    owner.into(),
                    language.clone(),
                    fragment,
                ),
            );
        }
        links.insert(language.clone(), resolved);
        indexes.insert(language, entries);
    }
    Ok((links, indexes))
}
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // Both passes share checked ownership and destination identities.
pub(super) fn plan_screens(
    plans: &mut [DocumentPlan],
    selected: &BTreeMap<String, Vec<ftoo_manifest::DocumentOutput>>,
    book: &ftoo_manifest::AddressBook<'_>,
    skeletons: &CheckedSkeletons<'_>,
    document_sites: &BTreeMap<String, UnitSite>,
    inclusions: &[super::input::Inclusion],
    inputs: &BTreeMap<String, Vec<u8>>,
    mints: &BTreeMap<String, ftoo_manifest::MintOrigin>,
    lookup: Option<&super::registry::CheckedPaperLookup>,
) -> Result<(), DocumentError> {
    let empty_virtues = BTreeMap::new();
    let virtues = lookup.map_or(&empty_virtues, super::registry::CheckedPaperLookup::virtues);
    let mut owned = BTreeMap::<(String, String), OwnedDestination>::new();
    let mut anchors = BTreeSet::new();
    for plan in plans.iter() {
        let targets = &selected[&plan.document];
        if !targets.contains(&ftoo_manifest::DocumentOutput::Markdown)
            || !targets.contains(&ftoo_manifest::DocumentOutput::Fragment)
        {
            continue;
        }
        let positions = &skeletons.documents()[&plan.document];
        check_excerpt_shape(plan, positions[0].site())?;
        if gap_template(&plan.language, GapPosition::Before).is_none() {
            return Err(screen_failure(
                plan,
                &document_sites[&plan.document],
                AssemblyFault::MissingScreenTemplates {
                    language: plan.language.clone(),
                },
            ));
        }
        for (index, block) in plan.blocks.iter().enumerate().skip(1) {
            if let Some(econ) = &block.econ
                && !anchors.insert((
                    plan.document.clone(),
                    plan.language.clone(),
                    econ.fragment.clone(),
                ))
            {
                return Err(screen_failure(
                    plan,
                    &block.site,
                    AssemblyFault::DuplicatePublicAnchor {
                        fragment: econ.fragment.clone(),
                    },
                ));
            }
            if block.quotation || block.paragraph || block.equation {
                continue;
            }
            let site = positions[index].site();
            let target = ftoo_manifest::AddressTarget::Documentary(block.unit.clone());
            let handle = book.reader_identity(target).map_err(|fault| {
                screen_failure(
                    plan,
                    site,
                    AssemblyFault::ReaderIdentity {
                        target: fault.target,
                        reason: fault.reason,
                        candidates: fault.candidates,
                    },
                )
            })?;
            let fragment = public_fragment(plan, site, &handle)?;
            if !anchors.insert((
                plan.document.clone(),
                plan.language.clone(),
                fragment.clone(),
            )) {
                return Err(screen_failure(
                    plan,
                    site,
                    AssemblyFault::DuplicatePublicAnchor { fragment },
                ));
            }
            let binding_site = book
                .common_descriptors()
                .find(|descriptor| descriptor.handle() == handle.as_str())
                .and_then(|descriptor| match descriptor.origin() {
                    ftoo_manifest::CommonOrigin::Documentary { admitted_at, .. } => {
                        Some(admitted_at.clone())
                    }
                    ftoo_manifest::CommonOrigin::Theory(_) => None,
                });
            let refrain = ScreenRefrain {
                handle,
                bound_label: block.unit.clone(),
                fragment,
                quotation: None,
                site: site.clone(),
                binding_site,
            };
            let duplicate_fragment = refrain.fragment.clone();
            if owned
                .insert(
                    (block.unit.clone(), plan.language.clone()),
                    OwnedDestination {
                        document: plan.document.clone(),
                        refrain,
                    },
                )
                .is_some()
            {
                return Err(screen_failure(
                    plan,
                    site,
                    AssemblyFault::DuplicatePublicAnchor {
                        fragment: duplicate_fragment,
                    },
                ));
            }
        }
    }
    let (links, indexes) = resolve_see(plans, selected, inputs, mints, &anchors, virtues)?;
    for plan in plans.iter_mut() {
        let targets = &selected[&plan.document];
        if !targets.contains(&ftoo_manifest::DocumentOutput::Markdown)
            || !targets.contains(&ftoo_manifest::DocumentOutput::Fragment)
        {
            continue;
        }
        let positions = &skeletons.documents()[&plan.document];
        let empty_links = BTreeMap::new();
        let link = links.get(&plan.language).unwrap_or(&empty_links);
        for block in &mut plan.blocks {
            if let Err((label, site)) = bind_see(&mut block.runs, link, virtues) {
                return Err(screen_failure(
                    plan,
                    &site,
                    AssemblyFault::ReferenceDestination {
                        label,
                        reason: "unbound reference destination".into(),
                    },
                ));
            }
        }
        let mut gaps = Vec::new();
        for index in 0..plan.blocks.len() {
            let position = if index == 0 {
                GapPosition::Before
            } else if index + 1 == plan.blocks.len() {
                GapPosition::After
            } else {
                GapPosition::Between
            };
            let mut gap = screen_gap(plan, positions[0].site(), position)?;
            if position == GapPosition::Between {
                let fragment =
                    |block: &Block| super::input::fragment_path(&plan.document, &block.unit);
                let first = inclusions
                    .iter()
                    .find(|inclusion| inclusion.fragment == fragment(&plan.blocks[index]))
                    .expect("checked excerpt inclusion");
                let second = inclusions
                    .iter()
                    .find(|inclusion| inclusion.fragment == fragment(&plan.blocks[index + 1]))
                    .expect("checked excerpt inclusion");
                if !source_material_between(first, second, inputs) {
                    gap.message.clear();
                }
            }
            if index > 0 {
                gap.boundaries.push(positions[index].site().clone());
            }
            if index + 1 < positions.len() {
                gap.boundaries.push(positions[index + 1].site().clone());
            }
            gaps.push(gap);
        }
        let mut refrains = Vec::new();
        for (index, block) in plan.blocks.iter().enumerate().skip(1) {
            if block.paragraph || block.equation {
                continue;
            }
            let source_document = skeletons
                .corpus()
                .units()
                .iter()
                .find(|unit| unit.label().as_str() == block.unit)
                .expect("checked skeleton unit")
                .document();
            if !block.quotation && source_document != plan.document {
                return Err(screen_failure(
                    plan,
                    positions[index].site(),
                    AssemblyFault::ExcerptShape {
                        document: plan.document.clone(),
                        positions: plan.blocks.len(),
                    },
                ));
            }
            if block.quotation
                && (!selected.get(source_document).is_some_and(|targets| {
                    targets.contains(&ftoo_manifest::DocumentOutput::Markdown)
                }) || !skeletons.corpus().documents()[source_document].contains(&plan.language))
            {
                return Err(missing_markdown_owner(
                    plan,
                    positions[index].site(),
                    source_document,
                    &block.unit,
                    document_sites,
                ));
            }
            let Some(destination) = owned.get(&(block.unit.clone(), plan.language.clone())) else {
                return Err(missing_markdown_owner(
                    plan,
                    positions[index].site(),
                    source_document,
                    &block.unit,
                    document_sites,
                ));
            };
            if destination.document != source_document {
                return Err(missing_markdown_owner(
                    plan,
                    positions[index].site(),
                    source_document,
                    &block.unit,
                    document_sites,
                ));
            }
            let mut refrain = destination.refrain.clone();
            if block.quotation {
                let reader_path =
                    format!("generated/documents/{source_document}/{}.md", plan.language);
                let href = public_href(
                    plan,
                    positions[index].site(),
                    source_document,
                    &plan.language,
                    &refrain.fragment,
                )?;
                refrain.quotation = Some(ScreenQuotation {
                    owner_document: source_document.into(),
                    language: plan.language.clone(),
                    reader_path,
                    fragment: refrain.fragment.clone(),
                    href,
                    owner_site: destination.refrain.site.clone(),
                });
                refrain.site = positions[index].site().clone();
            }
            refrains.push(refrain);
        }
        let reading = reading(&plan.document);
        let index: Vec<_> = indexes
            .get(&plan.language)
            .into_iter()
            .flatten()
            .filter(|entry| Some(entry.document) == reading)
            .cloned()
            .collect();
        let html_gaps = html_gaps(plan);
        let entries = super::bibliography::entries(plan);
        let mut end_matter = Vec::new();
        if !index.is_empty() {
            end_matter.push(ScreenEndMatter::Unavailable(index.clone()));
        }
        if !entries.is_empty() {
            end_matter.push(ScreenEndMatter::References(entries));
        }
        plan.screen = Some(ScreenPlan {
            gaps,
            refrains,
            index,
            html_gaps,
            end_matter,
        });
        check_public_anchors(plan)?;
    }
    for plan in plans.iter() {
        require_bibliography_screen(plan)?;
    }
    Ok(())
}

fn require_bibliography_screen(plan: &DocumentPlan) -> Result<(), DocumentError> {
    if plan.screen.is_some() {
        return Ok(());
    }
    for block in &plan.blocks {
        if let Some(Inline::BibliographyCitation { site, .. }) = block
            .runs
            .iter()
            .find(|run| matches!(run, Inline::BibliographyCitation { .. }))
        {
            return Err(super::failure(
                RuleId::M055,
                AssemblyStage::Ownership,
                AssemblySite::Source(site.clone()),
                AssemblyFault::UnknownToken {
                    value: "bibliography citation".into(),
                    expected: vec!["screen bibliography consumer".into()],
                },
            ));
        }
    }
    Ok(())
}
impl DocumentPlan {
    #[allow(clippy::too_many_lines)] // Every target's records retain one source-ordered inventory.
    pub fn records(&self, target: DocumentTarget) -> Vec<EmissionRecord> {
        let mut records = Vec::new();
        for (index, block) in self.blocks.iter().enumerate() {
            records.push(EmissionRecord {
                kind: if block.depth.is_some() {
                    EmissionKind::Heading
                } else if !block.displays.is_empty() {
                    EmissionKind::Caption
                } else {
                    EmissionKind::Paragraph
                },
                unit: block.unit.clone(),
                language: self.language.clone(),
                target,
                occurrence: None,
                value: {
                    // Only prose text shares apostrophe spellings; handles and mentions stay literal.
                    block
                        .runs
                        .iter()
                        .map(|run| match run {
                            Inline::Text(text) if target == DocumentTarget::Latex => {
                                text.replace('\'', "’")
                            }
                            _ => canonical_run(run),
                        })
                        .collect::<String>()
                },
                site: block.site.clone(),
            });
            let mut inline_runs = Vec::new();
            flatten_runs(&block.runs, &mut inline_runs);
            for run in inline_runs {
                let (kind, value, site) = match run {
                    Inline::Text(_) | Inline::Styled { .. } => continue,
                    Inline::Tag { label, site, .. } => (EmissionKind::Tag, label, site),
                    Inline::PaperTag { value, site, .. } => {
                        (EmissionKind::Tag, &value.target, site)
                    }
                    Inline::See {
                        label,
                        locator,
                        owner,
                        language,
                        fragment,
                        site,
                    } => {
                        records.push(EmissionRecord {
                            kind: EmissionKind::See {
                                target: label.clone(),
                            },
                            unit: block.unit.clone(),
                            language: self.language.clone(),
                            target,
                            occurrence: site.occurrence,
                            value: if target == DocumentTarget::Markdown {
                                format!(
                                    "{locator}\t{}",
                                    markdown_document_href(owner, language, fragment)
                                )
                            } else {
                                label.clone()
                            },
                            site: site.clone(),
                        });
                        continue;
                    }
                    Inline::Virtue {
                        word,
                        key,
                        owner,
                        language,
                        fragment,
                        site,
                        ..
                    } => {
                        records.push(EmissionRecord {
                            kind: EmissionKind::Virtue { key: key.clone() },
                            unit: block.unit.clone(),
                            language: self.language.clone(),
                            target,
                            occurrence: site.occurrence,
                            value: if target == DocumentTarget::Markdown {
                                format!(
                                    "{word}\t{}",
                                    markdown_document_href(owner, language, fragment)
                                )
                            } else {
                                word.clone()
                            },
                            site: site.clone(),
                        });
                        continue;
                    }
                    Inline::Math { key, site, .. } => (EmissionKind::Math, key, site),
                    Inline::Citation {
                        text,
                        site,
                        identity,
                    } => (
                        EmissionKind::Citation(Box::new(identity.clone())),
                        text,
                        site,
                    ),
                    Inline::BibliographyCitation {
                        kind,
                        entries,
                        label,
                        site,
                    } => (
                        EmissionKind::BibliographyCitation {
                            kind: *kind,
                            keys: entries.iter().map(|entry| entry.key.clone()).collect(),
                            anchors: if target == DocumentTarget::Markdown {
                                entries.iter().map(|entry| entry.anchor.clone()).collect()
                            } else {
                                Vec::new()
                            },
                        },
                        label,
                        site,
                    ),
                    Inline::Mention { text, site } => (EmissionKind::Mention, text, site),
                };
                records.push(EmissionRecord {
                    kind,
                    unit: block.unit.clone(),
                    language: self.language.clone(),
                    target,
                    occurrence: site.occurrence,
                    value: value.clone(),
                    site: site.clone(),
                });
            }
            if let Some(body) = block.displays.get(&target) {
                records.push(EmissionRecord {
                    kind: EmissionKind::Body,
                    unit: block.unit.clone(),
                    language: self.language.clone(),
                    target,
                    occurrence: None,
                    value: body.source.clone(),
                    site: body.site.clone(),
                });
                for region in &body.regions {
                    let plan = DocumentPlan {
                        document: self.document.clone(),
                        language: self.language.clone(),
                        coverage: self.coverage.clone(),
                        blocks: Vec::new(),
                        screen: None,
                    };
                    records.extend(region_records(region, &plan, target));
                }
            }
            records.extend(pending_records(self, block, target));
            if target == DocumentTarget::Markdown
                && let Some(screen) = &self.screen
            {
                if let Some(refrain_index) = refrain_index(&self.blocks, index) {
                    let refrain = &screen.refrains[refrain_index];
                    if let Some(quotation) = &refrain.quotation {
                        records.push(EmissionRecord {
                            kind: EmissionKind::QuotationLink {
                                owner_document: quotation.owner_document.clone(),
                                language: quotation.language.clone(),
                                reader_path: quotation.reader_path.clone(),
                                fragment: quotation.fragment.clone(),
                            },
                            unit: block.unit.clone(),
                            language: self.language.clone(),
                            target,
                            occurrence: Some(refrain_index),
                            value: quotation.href.clone(),
                            site: refrain.site.clone(),
                        });
                    } else {
                        records.push(EmissionRecord {
                            kind: EmissionKind::PublicAnchor {
                                fragment: refrain.fragment.clone(),
                            },
                            unit: block.unit.clone(),
                            language: self.language.clone(),
                            target,
                            occurrence: Some(refrain_index),
                            value: refrain.fragment.clone(),
                            site: refrain.site.clone(),
                        });
                    }
                    records.push(EmissionRecord {
                        kind: EmissionKind::RefrainHandle {
                            role: if refrain.quotation.is_some() {
                                RefrainRole::Quoted
                            } else {
                                RefrainRole::Owned
                            },
                            handle: refrain.handle.clone(),
                            bound_label: refrain.bound_label.clone(),
                        },
                        unit: block.unit.clone(),
                        language: self.language.clone(),
                        target,
                        occurrence: Some(refrain_index),
                        value: refrain.handle.clone(),
                        site: refrain.site.clone(),
                    });
                }
                let gap = &screen.gaps[index];
                if !gap.message.is_empty() {
                    records.push(EmissionRecord {
                        kind: EmissionKind::Gap {
                            index,
                            position: gap.position,
                        },
                        unit: String::new(),
                        language: self.language.clone(),
                        target,
                        occurrence: Some(index),
                        value: gap.message.clone(),
                        site: gap.site.clone(),
                    });
                }
            }
        }
        if target == DocumentTarget::Markdown {
            for section in end_matter(self) {
                let EndMatter::Unavailable(entries) = section else {
                    continue;
                };
                records.push(EmissionRecord {
                    kind: EmissionKind::IndexHeading,
                    unit: String::new(),
                    language: self.language.clone(),
                    target,
                    occurrence: None,
                    value: "## References to unavailable content".into(),
                    site: self.blocks[0].site.clone(),
                });
                for entry in entries {
                    records.push(EmissionRecord {
                        kind: EmissionKind::IndexEntry {
                            fragment: entry.fragment.clone(),
                        },
                        unit: String::new(),
                        language: self.language.clone(),
                        target,
                        occurrence: None,
                        value: super::destinations::index_text(entry),
                        site: self.blocks[0].site.clone(),
                    });
                }
            }
        }
        records
    }
    pub fn origins(&self, target: DocumentTarget) -> Vec<SourceOrigin> {
        self.records(target)
            .into_iter()
            .flat_map(|record| {
                if let EmissionKind::Citation(identity) = &record.kind {
                    citation_origins(identity, &record.site, &record.unit, &record.language)
                } else if let EmissionKind::PendingExplanation(entry) = record.kind {
                    entry
                        .occurrences
                        .into_iter()
                        .flat_map(|occurrence| [occurrence.definition, occurrence.source])
                        .collect::<Vec<_>>()
                } else if let EmissionKind::Gap { index, .. } = record.kind {
                    std::iter::once(record.site)
                        .chain(
                            self.screen
                                .as_ref()
                                .into_iter()
                                .flat_map(|screen| screen.gaps[index].boundaries.iter().cloned()),
                        )
                        .map(|site| SourceOrigin {
                            unit: None,
                            language: Some(record.language.clone()),
                            target: Some(target),
                            site,
                        })
                        .collect::<Vec<_>>()
                } else if matches!(
                    &record.kind,
                    EmissionKind::RefrainHandle { .. } | EmissionKind::QuotationLink { .. }
                ) {
                    let quotation_link = matches!(&record.kind, EmissionKind::QuotationLink { .. });
                    let mut sites = vec![record.site];
                    if let Some(screen) = &self.screen
                        && let Some(index) = record.occurrence
                    {
                        if let Some(binding) = &screen.refrains[index].binding_site {
                            sites.push(binding.clone());
                        }
                        if quotation_link && let Some(quotation) = &screen.refrains[index].quotation
                        {
                            sites.push(quotation.owner_site.clone());
                        }
                    }
                    sites
                        .into_iter()
                        .map(|site| SourceOrigin {
                            unit: Some(record.unit.clone()),
                            language: Some(record.language.clone()),
                            target: Some(target),
                            site,
                        })
                        .collect::<Vec<_>>()
                } else {
                    vec![SourceOrigin {
                        unit: Some(record.unit),
                        language: Some(record.language),
                        target: Some(target),
                        site: record.site,
                    }]
                }
            })
            .chain(self.coverage.all_runs().filter_map(|run| match run {
                Inline::Tag { site, .. } => Some(SourceOrigin {
                    unit: None,
                    language: Some(self.language.clone()),
                    target: Some(target),
                    site: site.clone(),
                }),
                _ => None,
            }))
            .collect()
    }
}
fn collect_reader_error(errors: &mut Vec<AssemblyDiagnostic>, error: DocumentError) {
    let DocumentError::Assembly(found) = error else {
        unreachable!("reader identity fault")
    };
    for diagnostic in found.diagnostics() {
        // Shared statement runs are mapped for both targets, but have one site.
        if !errors.contains(diagnostic) {
            errors.push(diagnostic.clone());
        }
    }
}
pub(super) fn check_handle_position(
    document: &str,
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    paragraph: bool,
    paper_documents: &BTreeSet<String>,
    errors: &mut Vec<AssemblyDiagnostic>,
) {
    if paragraph && paper_documents.contains(document) {
        return;
    }
    for placement in unit.texts()[language].handle_occurrences() {
        errors.push(AssemblyDiagnostic {
            rule: RuleId::M039,
            stage: AssemblyStage::Emission,
            document: Some(document.into()),
            unit: Some(unit.label().as_str().into()),
            language: Some(language.into()),
            target: None,
            site: AssemblySite::Source(placement.site().clone()),
            fault: AssemblyFault::UnsupportedParagraphBody {
                reason: "auxiliary handle is outside an owning paper paragraph".into(),
            },
        });
    }
}

#[allow(clippy::too_many_lines)] // Document positions, recovery sites and screen inventories share one pass.
#[allow(clippy::too_many_arguments)] // All checked document inputs remain explicit at this boundary.
pub(super) fn build(
    skeletons: &CheckedSkeletons<'_>,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    notation: &RenderableNotation<'_, '_, '_>,
    metadata: &BTreeMap<String, UnitSite>,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
    bibliography: &str,
    paper_lookup: Option<&super::registry::CheckedPaperLookup>,
    inputs: &BTreeMap<String, Vec<u8>>,
    paper_documents: &BTreeSet<String>,
) -> Result<Vec<DocumentPlan>, DocumentError> {
    let corpus = skeletons.corpus();
    let book = welded.integrated().resolved().book();
    let equation_units: BTreeSet<_> = skeletons
        .documents()
        .values()
        .flatten()
        .filter(|position| position.equation())
        .map(|position| position.unit().to_owned())
        .collect();
    let equations = equations(corpus, notation, displays, &equation_units)?;
    let coverage = book.graph().coverage();
    let gate = match coverage.release_gate {
        ftoo_manifest::ReleaseGate::ExplicitPartial => "explicit-partial",
        ftoo_manifest::ReleaseGate::Complete => "complete",
    };
    let closed = coverage.closed_world;
    let first = format!(
        "Coverage declaration: {gate}. Closed world: referents={}; relations={}; audit={}. Evidence context: {}.",
        closed.referents,
        closed.relations,
        closed.audit,
        if notation.retained().is_some() {
            "retained audit accepted"
        } else {
            "unaudited base graph"
        }
    );
    let mut plans = Vec::new();
    let mut errors = vec![];
    for (document, positions) in skeletons.documents() {
        for language in &corpus.documents()[document] {
            let coverage = match Coverage::build(book, &first, document, language) {
                Ok(coverage) => Some(coverage),
                Err(error) => {
                    collect_reader_error(&mut errors, error);
                    None
                }
            };
            let mut blocks = Vec::new();
            for position in positions {
                let (index, unit) = corpus
                    .units()
                    .iter()
                    .enumerate()
                    .find(|(_, u)| u.label().as_str() == position.unit())
                    .expect("checked skeleton unit");
                check_reader_literals(unit, language, displays, &mut errors);
                check_handle_position(
                    document,
                    unit,
                    language,
                    position.paragraph(),
                    paper_documents,
                    &mut errors,
                );
                let runs = runs(
                    index,
                    unit,
                    language,
                    welded,
                    &equations,
                    bibliography,
                    paper_lookup,
                )?;
                let econ = if position.econ() {
                    Some(econ_plan(
                        unit,
                        language,
                        &runs,
                        book,
                        paper_lookup,
                        inputs,
                    )?)
                } else {
                    None
                };
                let bodies = display_plans(index, unit, language, welded, &equations, displays);
                let mut block = Block {
                    pending_names: BTreeMap::new(),
                    displays: bodies,
                    unit: position.unit().into(),
                    depth: position.depth(),
                    quotation: position.quotation(),
                    paragraph: position.paragraph(),
                    equation: position.equation(),
                    econ,
                    site: if position.paragraph() || position.equation() {
                        unit.texts()[language].site().clone()
                    } else {
                        metadata[position.unit()].clone()
                    },
                    runs,
                };
                check_equation_shape(&block)?;
                if let Err(error) = block.map_citations(book, document, language) {
                    collect_reader_error(&mut errors, error);
                }
                if let Err(error) = block.map_pending(book, index, document, language) {
                    collect_reader_error(&mut errors, error);
                }
                blocks.push(block);
            }
            if let Some(mut coverage) = coverage {
                if let Some(view) = notation.retained() {
                    coverage.audit =
                        super::retained::coverage(book, view, &blocks, document, language)?;
                }
                plans.push(DocumentPlan {
                    document: document.clone(),
                    language: language.clone(),
                    coverage,
                    blocks,
                    screen: None,
                });
            }
        }
    }
    if let Some(errors) = AssemblyErrors::from_diagnostics(errors) {
        return Err(DocumentError::Assembly(errors));
    }
    Ok(plans)
}

#[allow(clippy::too_many_lines)] // Bind each econ field and refusal to its source site in one plan boundary.
fn econ_plan(
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    runs: &[Inline],
    book: &ftoo_manifest::AddressBook<'_>,
    lookup: Option<&super::registry::CheckedPaperLookup>,
    inputs: &BTreeMap<String, Vec<u8>>,
) -> Result<EconPlan, DocumentError> {
    let metadata = unit.econ().expect("econ role requires metadata");
    let site = &metadata.target.site;
    let refuse = |site: &UnitSite, reason: EconReason| {
        super::failure(
            RuleId::M055,
            AssemblyStage::Ownership,
            AssemblySite::Source(site.clone()),
            AssemblyFault::Econ { reason },
        )
    };
    let lookup = lookup.ok_or_else(|| refuse(site, EconReason::MissingPaperEvidence))?;
    let value = |key: &str, field_site: &UnitSite| {
        match book.resolve(key) {
            Err(ftoo_manifest::TagResolutionFault::UnknownTag { .. }) => {}
            _ => {
                return Err(refuse(
                    field_site,
                    EconReason::HandleOutsidePaper { key: key.into() },
                ));
            }
        }
        match lookup.resolve(key) {
            Some(ftoo_manifest::PaperResolution::Available(value)) => Ok(value.target),
            _ => Err(refuse(
                field_site,
                EconReason::MissingBinding { key: key.into() },
            )),
        }
    };
    let target_identity = value(&metadata.target.value, &metadata.target.site)?;
    let reference_identity = value(&metadata.reference.value, &metadata.reference.site)?;
    let departure_handle = format!("d:{}", metadata.departure.value);
    let checked_departure = value(&departure_handle, &metadata.departure.site)?;
    let departure_identity = lookup
        .departure(&metadata.departure.value)
        .ok_or_else(|| refuse(&metadata.departure.site, EconReason::MissingDeparture))?
        .to_owned();
    if departure_identity != checked_departure {
        return Err(refuse(
            &metadata.departure.site,
            EconReason::DepartureMismatch,
        ));
    }
    let (owner, origin) = lookup
        .native_target(&metadata.target.value)
        .ok_or_else(|| refuse(site, EconReason::MissingTarget))?;
    if owner != unit.label().as_str() || origin.file != metadata.target.site.file {
        return Err(refuse(site, EconReason::TargetOwnerMismatch));
    }
    let title = metadata.title[language].value.value.clone();
    let inherited_title =
        super::destinations::economics_title(&target_identity, inputs).map_err(|error| {
            refuse(
                &metadata.title[language].value.site,
                EconReason::PaperTitle {
                    detail: format!("{error:?}"),
                },
            )
        })?;
    if title != inherited_title {
        return Err(refuse(
            &metadata.title[language].value.site,
            EconReason::TitleMismatch,
        ));
    }
    let number =
        super::destinations::economics_number(unit.label().as_str(), inputs).map_err(|error| {
            refuse(
                site,
                EconReason::CounterContext {
                    detail: format!("{error:?}"),
                },
            )
        })?;
    let mut citations = Vec::new();
    let mut citation_sites = Vec::new();
    for run in runs {
        match run {
            Inline::Text(_) => {}
            Inline::BibliographyCitation {
                kind: ftoo_manifest::BibliographyCitationKind::Parenthetical,
                entries,
                site: citation_site,
                ..
            } => {
                citations.push(
                    entries
                        .iter()
                        .map(|entry| entry.key.clone())
                        .collect::<Vec<_>>(),
                );
                citation_sites.push(citation_site);
            }
            _ => {
                let source = match run {
                    Inline::Tag { site, .. }
                    | Inline::PaperTag { site, .. }
                    | Inline::See { site, .. }
                    | Inline::Math { site, .. }
                    | Inline::Citation { site, .. }
                    | Inline::Virtue { site, .. }
                    | Inline::BibliographyCitation { site, .. }
                    | Inline::Mention { site, .. }
                    | Inline::Styled { site, .. } => site,
                    Inline::Text(_) => unreachable!("admitted econ text"),
                };
                return Err(refuse(source, EconReason::UnsupportedInline));
            }
        }
    }
    let option_citations = metadata
        .citations
        .iter()
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>();
    if citation_sites.len() > 1 {
        return Err(refuse(citation_sites[1], EconReason::CitationMismatch));
    }
    if citations.len() != 1 || citations[0] != option_citations {
        return Err(refuse(
            &metadata.citations[0].site,
            EconReason::CitationMismatch,
        ));
    }
    let fragment = super::destinations::econ_fragment(&title).ok_or_else(|| {
        refuse(
            &metadata.title[language].value.site,
            EconReason::InvalidPublicFragment,
        )
    })?;
    Ok(EconPlan {
        target: metadata.target.value.clone(),
        target_identity,
        title,
        number,
        reference: metadata.reference.value.clone(),
        reference_identity,
        departure: metadata.departure.value.clone(),
        departure_identity,
        stops_at: metadata.stops_at[language].value.value.clone(),
        citations: option_citations,
        fragment,
    })
}
type Equations = BTreeMap<(String, String), (String, String, Vec<crate::PendingEntry>)>;
pub(super) fn check_equation_mode(
    key: &str,
    source_len: usize,
    requested: bool,
    unicode_returned: bool,
    macro_returned: bool,
) -> Result<(), (std::ops::Range<usize>, AssemblyFault)> {
    if unicode_returned == requested && macro_returned == requested {
        Ok(())
    } else {
        Err((
            0..source_len,
            AssemblyFault::UnknownToken {
                value: key.into(),
                expected: vec!["matching rendered display mode".into()],
            },
        ))
    }
}

#[allow(clippy::too_many_arguments)] // One diagnostic retains its document, unit, source and mathematical use sites.
pub(super) fn equation_diagnostic(
    document: &str,
    unit: &str,
    site: &UnitSite,
    key: &str,
    uses: Vec<MathUseSite>,
    bytes: std::ops::Range<usize>,
    fault: AssemblyFault,
) -> AssemblyDiagnostic {
    AssemblyDiagnostic {
        rule: RuleId::M053,
        stage: AssemblyStage::Math,
        document: Some(document.into()),
        unit: Some(unit.into()),
        language: None,
        target: None,
        site: AssemblySite::Math {
            definition: site.clone(),
            key: key.into(),
            bytes,
            uses,
        },
        fault,
    }
}
#[allow(clippy::too_many_lines)] // Each source is checked for either inline or display mode at the same use sites.
fn equations(
    corpus: &ftoo_manifest::LanguageCheckedUnits,
    notation: &RenderableNotation<'_, '_, '_>,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
    equation_units: &BTreeSet<String>,
) -> Result<Equations, DocumentError> {
    let mut equations = BTreeMap::new();
    let mut errors = Vec::new();
    for unit in corpus.units() {
        for (key, source) in unit.math() {
            let mut uses: Vec<_> = unit
                .texts()
                .iter()
                .flat_map(|(language, text)| {
                    text.occurrences()
                        .iter()
                        .filter(|o| o.name() == key)
                        .map(|o| MathUseSite {
                            language: language.clone(),
                            target: None,
                            site: o.site().clone(),
                        })
                })
                .collect();
            if let Some(bodies) = displays.bodies().get(unit.label().as_str()) {
                for ((target, language), body) in bodies {
                    for region in body.regions() {
                        uses.extend(region.math().iter().filter(|o| o.name() == key).map(|o| {
                            MathUseSite {
                                language: language.clone(),
                                target: Some(*target),
                                site: region.site(o.site()),
                            }
                        }));
                    }
                }
            }
            let is_display = equation_units.contains(unit.label().as_str());
            let parsed = if is_display {
                ftoo_manifest::parse_assembly_display_math(notation.catalogue(), &source.value)
            } else {
                ftoo_manifest::parse_assembly_math(notation.catalogue(), &source.value)
            };
            let outcome = match parsed {
                Ok(parsed) => {
                    let ranges = parsed.ranges().to_vec();
                    (if is_display {
                        notation.check_document_expression(parsed.into_draft())
                    } else {
                        notation.check_expression(parsed.into_draft())
                    })
                    .and_then(|expr| {
                        let unicode = if is_display {
                            expr.render_display(RenderTarget::Unicode)?
                        } else {
                            expr.render(RenderTarget::Unicode)?
                        };
                        let macro_form = if is_display {
                            expr.render_display(RenderTarget::Macro)?
                        } else {
                            expr.render(RenderTarget::Macro)?
                        };
                        let markdown_returned = if is_display {
                            Some(notation.check_markdown_display_source(&format!(
                                "{}.",
                                markdown_display_spelling(unicode.source())
                            ))?)
                        } else {
                            None
                        };
                        Ok((
                            unicode.source().to_owned(),
                            macro_form.source().to_owned(),
                            unicode.pending_entries().to_vec(),
                            unicode.is_display(),
                            macro_form.is_display(),
                            markdown_returned,
                        ))
                    })
                    .map_err(|cause| {
                        (
                            notation_bytes(&cause, &ranges, source.value.len()),
                            AssemblyFault::MathNotation { cause },
                        )
                    })
                    .and_then(
                        |(
                            unicode,
                            macro_form,
                            pending,
                            unicode_returned,
                            macro_returned,
                            markdown_returned,
                        )| {
                            check_equation_mode(
                                key,
                                source.value.len(),
                                is_display,
                                unicode_returned,
                                macro_returned,
                            )?;
                            if let Some(markdown_returned) = markdown_returned {
                                check_equation_mode(
                                    key,
                                    source.value.len(),
                                    is_display,
                                    markdown_returned,
                                    markdown_returned,
                                )?;
                            }
                            Ok((unicode, macro_form, pending))
                        },
                    )
                }
                Err((bytes, fault)) => Err((bytes, fault.into())),
            };
            match outcome {
                Ok(pair) => {
                    equations.insert((unit.label().as_str().to_owned(), key.clone()), pair);
                }
                Err((bytes, fault)) => errors.push(equation_diagnostic(
                    unit.document(),
                    unit.label().as_str(),
                    &source.site,
                    key,
                    uses,
                    bytes,
                    fault,
                )),
            }
        }
    }
    if let Some(errors) = AssemblyErrors::from_diagnostics(errors) {
        return Err(DocumentError::Assembly(errors));
    }
    Ok(equations)
}
pub(super) fn notation_bytes(
    cause: &crate::NotationRenderError,
    ranges: &[ftoo_manifest::MathSourceRange],
    source_length: usize,
) -> std::ops::Range<usize> {
    use ftoo_manifest::{NotationPathPart, NotationSite};
    let crate::NotationRenderError::Admission(errors) = cause else {
        return 0..source_length;
    };
    let path = errors
        .diagnostics()
        .iter()
        .find_map(|diagnostic| match &diagnostic.site {
            NotationSite::Expression { path } => Some(path),
            NotationSite::Rendered { origins, .. } => origins
                .iter()
                .find(|origin| origin.definition.is_none())
                .map(|origin| &origin.path),
            NotationSite::Catalogue { .. } | NotationSite::Registry { .. } => None,
        });
    let Some(path) = path else {
        return 0..source_length;
    };
    let arguments: Vec<_> = path
        .iter()
        .filter_map(|part| match part {
            NotationPathPart::Argument(index) => Some(*index),
            NotationPathPart::Field(_) => None,
        })
        .collect();
    ranges
        .iter()
        .find(|range| range.path == arguments)
        .map_or(0..source_length, |range| range.bytes.clone())
}
pub(super) fn checked_handle_binding(
    key: &str,
    site: &UnitSite,
    resolve: impl Fn(&str) -> Option<ftoo_manifest::PaperResolution>,
) -> Result<ftoo_manifest::PaperTagValue, DocumentError> {
    let value = resolve(key)
        .and_then(|resolution| match resolution {
            ftoo_manifest::PaperResolution::Available(value) => Some(value),
            ftoo_manifest::PaperResolution::Unavailable { .. } => None,
        })
        .ok_or_else(|| {
            DocumentError::PaperLookup(super::registry::PaperLookupError {
                rule: ftoo_manifest::RuleId::M039,
                site: site.to_string(),
                fault: super::registry::PaperLookupFault::ProtocolInvalid {
                    reason: format!(
                        "auxiliary paper target {key:?} has no available checked binding"
                    ),
                },
            })
        })?;
    if value.key != key || value.spelling != key || value.alias_site.is_some() {
        return Err(DocumentError::PaperLookup(
            super::registry::PaperLookupError {
                rule: ftoo_manifest::RuleId::M039,
                site: site.to_string(),
                fault: super::registry::PaperLookupFault::ProtocolInvalid {
                    reason: format!(
                        "auxiliary paper target {key:?} is not a canonical printed key"
                    ),
                },
            },
        ));
    }
    Ok(value)
}

#[allow(clippy::too_many_lines)] // One replacement inventory retains source order across inline kinds.
fn runs(
    index: usize,
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    equations: &Equations,
    bibliography: &str,
    paper_lookup: Option<&super::registry::CheckedPaperLookup>,
) -> Result<Vec<Inline>, DocumentError> {
    let text = &unit.texts()[language];
    let mut replacements = Vec::new();
    for occurrence in text.tag_occurrences() {
        let range = occurrence
            .site()
            .decoded
            .clone()
            .expect("admitted occurrence");
        let target = welded
            .integrated()
            .resolved()
            .tags()
            .iter()
            .find(|tag| {
                tag.unit() == unit.label()
                    && tag.language() == language
                    && tag.occurrence().site() == occurrence.site()
            })
            .expect("resolved occurrence from checked corpus");
        replacements.push((
            range,
            match target.resolved_target() {
                ftoo_manifest::ResolvedTagTarget::Theory(node) => Inline::Tag {
                    handle: occurrence.input().into(),
                    label: node.semantic_name().into(),
                    token: format!(
                        "u{index}-t{}",
                        occurrence.site().occurrence.expect("insertion index")
                    ),
                    site: occurrence.site().clone(),
                },
                ftoo_manifest::ResolvedTagTarget::Paper(value) => Inline::PaperTag {
                    value: value.clone(),
                    mint: None,
                    site: occurrence.site().clone(),
                },
            },
        ));
    }
    for placement in text.handle_occurrences() {
        let declaration = &unit.documentary_targets()[placement.selector()];
        let key = &declaration.value;
        let mut value = checked_handle_binding(key, placement.site(), |key| {
            paper_lookup.and_then(|lookup| lookup.resolve(key))
        })?;
        value.target.clone_from(key);
        replacements.push((
            placement
                .site()
                .decoded
                .clone()
                .expect("admitted placement"),
            Inline::PaperTag {
                value,
                mint: Some(placement.selector().to_owned()),
                site: placement.site().clone(),
            },
        ));
    }
    for occurrence in text.occurrences() {
        let (markdown, latex, pending) =
            &equations[&(unit.label().as_str().into(), occurrence.name().into())];
        replacements.push((
            occurrence
                .site()
                .decoded
                .clone()
                .expect("admitted occurrence"),
            Inline::Math {
                definition_site: unit.math()[occurrence.name()].site.clone(),
                key: occurrence.name().into(),
                markdown: markdown.clone(),
                latex: latex.clone(),
                pending: pending.clone(),
                site: occurrence.site().clone(),
            },
        ));
    }
    for occurrence in text.reference_occurrences() {
        replacements.push((
            occurrence
                .site()
                .decoded
                .clone()
                .expect("admitted occurrence"),
            Inline::See {
                label: occurrence.label().into(),
                locator: String::new(),
                owner: String::new(),
                language: String::new(),
                fragment: String::new(),
                site: occurrence.site().clone(),
            },
        ));
    }
    for occurrence in text.virtue_occurrences() {
        replacements.push((
            occurrence
                .site()
                .decoded
                .clone()
                .expect("admitted occurrence"),
            Inline::Virtue {
                word: occurrence.word().into(),
                companion: unit.document() == "explainer-pilot",
                key: String::new(),
                semantic: String::new(),
                owner: String::new(),
                language: String::new(),
                fragment: String::new(),
                site: occurrence.site().clone(),
            },
        ));
    }
    for occurrence in text.bibliography_citations() {
        let derived_bibliography = paper_lookup.map_or(
            "",
            super::registry::CheckedPaperLookup::derived_bibliography,
        );
        let entries = occurrence
            .keys()
            .iter()
            .map(|key| {
                super::bibliography::resolve_with_derived(
                    bibliography,
                    derived_bibliography,
                    key,
                    occurrence.site(),
                    occurrence.kind(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let label = super::bibliography::citation_label(occurrence.kind(), &entries);
        replacements.push((
            occurrence
                .site()
                .decoded
                .clone()
                .expect("admitted occurrence"),
            Inline::BibliographyCitation {
                kind: occurrence.kind(),
                entries,
                label,
                site: occurrence.site().clone(),
            },
        ));
    }
    spans(
        index,
        unit.label().as_str(),
        language,
        text,
        welded,
        &mut replacements,
    );
    replacements.sort_by_key(|(r, _)| r.start);
    let styles: Vec<_> = text
        .style_occurrences()
        .map(|(kind, body, site)| (StyleKind::from_checked(kind), body, site.clone()))
        .collect();
    let mut runs = scoped_runs(text.text(), 0..text.text().len(), &replacements, &styles);
    normalize(&mut runs);
    Ok(runs)
}
fn scoped_runs(
    source: &str,
    scope: std::ops::Range<usize>,
    replacements: &[(std::ops::Range<usize>, Inline)],
    styles: &[(StyleKind, std::ops::Range<usize>, UnitSite)],
) -> Vec<Inline> {
    scoped_runs_with_sites(source, scope, replacements, styles, None).0
}
fn scoped_runs_with_sites(
    source: &str,
    scope: std::ops::Range<usize>,
    replacements: &[(std::ops::Range<usize>, Inline)],
    styles: &[(StyleKind, std::ops::Range<usize>, UnitSite)],
    parent: Option<&UnitSite>,
) -> (Vec<Inline>, Vec<UnitSite>) {
    let mut runs = Vec::new();
    let mut sites = Vec::new();
    let mut at = scope.start;
    while at < scope.end {
        if let Some((kind, body, site)) = styles
            .iter()
            .find(|(_, _, site)| site.decoded.as_ref().is_some_and(|range| range.start == at))
        {
            let (children, child_sites) =
                scoped_runs_with_sites(source, body.clone(), replacements, &[], Some(site));
            runs.push(Inline::Styled {
                kind: *kind,
                children,
                child_sites,
                site: site.clone(),
            });
            sites.push(site.clone());
            at = site.decoded.as_ref().expect("checked style span").end;
            continue;
        }
        if let Some((range, run)) = replacements.iter().find(|(range, _)| range.start == at) {
            runs.push(run.clone());
            let mut site = match run {
                Inline::Tag { site, .. }
                | Inline::PaperTag { site, .. }
                | Inline::See { site, .. }
                | Inline::Virtue { site, .. }
                | Inline::Math { site, .. }
                | Inline::Citation { site, .. }
                | Inline::BibliographyCitation { site, .. }
                | Inline::Mention { site, .. }
                | Inline::Styled { site, .. } => site.clone(),
                Inline::Text(_) => unreachable!("replacement is not text"),
            };
            site.decoded = Some(range.clone());
            sites.push(site);
            at = range.end;
            continue;
        }
        let next = replacements
            .iter()
            .filter(|(range, _)| range.start > at)
            .map(|(range, _)| range.start)
            .chain(
                styles
                    .iter()
                    .filter_map(|(_, _, site)| site.decoded.as_ref().map(|range| range.start))
                    .filter(|start| *start > at),
            )
            .min()
            .unwrap_or(scope.end)
            .min(scope.end);
        runs.push(Inline::Text(source[at..next].replace("@@", "@")));
        let mut site = parent.cloned().unwrap_or_else(|| {
            let mut site = super::input::site("generated/manifest.json");
            site.decoded = Some(at..next);
            site
        });
        site.decoded = Some(at..next);
        site.occurrence = None;
        sites.push(site);
        at = next;
    }
    (runs, sites)
}
fn spans(
    index: usize,
    unit: &str,
    language: &str,
    text: &ftoo_manifest::CheckedText,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    replacements: &mut Vec<(std::ops::Range<usize>, Inline)>,
) {
    let welded_language = welded
        .units()
        .iter()
        .find(|u| u.unit() == unit)
        .expect("same weld")
        .languages()
        .iter()
        .find(|l| l.language() == language)
        .expect("same language");
    for (span, (value, site, citation)) in ftoo_manifest::assembly_text_spans(text)
        .into_iter()
        .enumerate()
    {
        assert!(if citation {
            welded_language
                .citations()
                .iter()
                .any(|c| c.label() == value && c.site().site == site)
        } else {
            welded_language.mentions().iter().any(|m| m.site == site)
        });
        replacements.push((
            site.decoded.clone().expect("span range"),
            if citation {
                citation_run(
                    value,
                    site,
                    format!("u{index}-c{span}"),
                    unit,
                    span,
                    None,
                    welded,
                )
            } else {
                Inline::Mention { text: value, site }
            },
        ));
    }
}
fn normalize(runs: &mut [Inline]) {
    let mut whitespace = false;
    for run in runs.iter_mut() {
        if let Inline::Styled { children, .. } = run {
            normalize(children);
            whitespace = false;
        } else if let Inline::Text(text) = run {
            let mut output = String::new();
            for c in text.chars() {
                if c.is_whitespace() {
                    if !whitespace {
                        output.push(' ');
                    }
                    whitespace = true;
                } else {
                    output.push(c);
                    whitespace = false;
                }
            }
            *text = output;
        } else {
            whitespace = false;
        }
    }
    if let Some(Inline::Text(text)) = runs.first_mut() {
        *text = text.trim_start_matches(' ').into();
    }
    if let Some(Inline::Text(text)) = runs.last_mut() {
        text.truncate(text.trim_end_matches(' ').len());
    }
}

fn recover_block(body: &str, block: &Block, target: DocumentTarget) -> Option<Vec<Recovered>> {
    if block.equation {
        let (markdown, latex) = block.equation_pair()?;
        let expected = if target == DocumentTarget::Markdown {
            format!("$$\n{}.\n$$", markdown_display_spelling(markdown))
        } else {
            format!("\\[\n{latex}.\n\\]")
        };
        return (body == expected).then(|| {
            vec![
                Recovered::Math(
                    if target == DocumentTarget::Markdown {
                        markdown
                    } else {
                        latex
                    }
                    .into(),
                ),
                Recovered::Text(".".into()),
            ]
        });
    }
    if target == DocumentTarget::Markdown {
        let heading = block
            .depth
            .map_or_else(String::new, |d| format!("{} ", "#".repeat(d)));
        body.strip_prefix(&heading)
            .and_then(|body| recover_markdown(body, block.runs.iter().flat_map(inline_handles)))
    } else {
        let framed = if let Some(depth) = block.depth {
            let prefix = match depth {
                1 => r"\section*{",
                2 => r"\subsection*{",
                3 => r"\subsubsection*{",
                4 => r"\paragraph*{",
                5 => r"\subparagraph*{",
                _ => r"{\noindent\textbf{",
            };
            body.strip_prefix(prefix)
                .and_then(|b| b.strip_suffix(if depth == 6 { r"}\par}" } else { "}" }))
        } else {
            Some(body)
        };
        framed.and_then(recover_prose_latex)
    }
}

fn canonical_run(run: &Inline) -> String {
    match run {
        Inline::Text(text) => text.clone(),
        Inline::Tag { handle, .. } => format!("@tag{{{handle}}}"),
        Inline::PaperTag { value, mint, .. } => mint.as_ref().map_or_else(
            || format!("@tag{{{}}}", value.key),
            |selector| format!("@handle{{{selector}}}"),
        ),
        Inline::See { label, .. } => format!("@see{{{label}}}"),
        Inline::Virtue { word, .. } => format!("@virtue{{{word}}}"),
        Inline::Math { key, .. } => format!("@math{{{key}}}"),
        Inline::Citation { text, .. } => format!("`{text}`"),
        Inline::BibliographyCitation { kind, entries, .. } => {
            let form = match kind {
                ftoo_manifest::BibliographyCitationKind::Parenthetical => "parencite",
                ftoo_manifest::BibliographyCitationKind::Textual => "textcite",
            };
            format!(
                "@{form}{{{}}}",
                entries
                    .iter()
                    .map(|entry| entry.key.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Inline::Mention { text, .. } => format!("``{text}``"),
        Inline::Styled { kind, children, .. } => format!(
            "@{kind}{{{}}}",
            children.iter().map(canonical_run).collect::<String>()
        ),
    }
}
fn recovered_tag_site(
    block: &Block,
    run: &Recovered,
    records: &[EmissionRecord],
) -> Option<UnitSite> {
    let mut flattened = Vec::new();
    flatten_runs(&block.runs, &mut flattened);
    let tag_sites: Vec<_> = if let Recovered::Tag { handle, .. } = run {
        flattened
            .into_iter()
            .filter_map(|run| match run {
                Inline::Tag {
                    handle: expected,
                    site,
                    ..
                } if expected == handle => Some(site),
                Inline::PaperTag {
                    value,
                    mint: Some(_),
                    site,
                } if value.spelling == *handle => Some(site),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    };
    let used_tag_sites = records
        .iter()
        .filter(|record| tag_sites.contains(&&record.site))
        .count();
    tag_sites
        .get(used_tag_sites)
        .or_else(|| tag_sites.last())
        .copied()
        .cloned()
}
fn recovered_citation_site(
    block: &Block,
    run: &Recovered,
    records: &[EmissionRecord],
) -> Option<UnitSite> {
    if matches!(run, Recovered::Citation { .. }) {
        let mut flattened = Vec::new();
        flatten_runs(&block.runs, &mut flattened);
        let sites = flattened
            .into_iter()
            .filter_map(|run| {
                if let Inline::Citation { site, .. } = run {
                    Some(site)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let used = records
            .iter()
            .filter(|r| matches!(r.kind, EmissionKind::Citation(_)))
            .count();
        sites.get(used).or_else(|| sites.last()).copied().cloned()
    } else {
        None
    }
}
enum RecoveryPart<'a> {
    Open(StyleKind),
    Close,
    Run(&'a Recovered),
}
fn recovery_parts<'a>(runs: &'a [Recovered], output: &mut Vec<RecoveryPart<'a>>) {
    for run in runs {
        if let Recovered::Styled { kind, children } = run {
            output.push(RecoveryPart::Open(*kind));
            recovery_parts(children, output);
            output.push(RecoveryPart::Close);
        } else {
            output.push(RecoveryPart::Run(run));
        }
    }
}
#[allow(clippy::too_many_lines)] // One record walk retains nested boundaries and identities.
fn recovered_records(
    block: &Block,
    recovered: &[Recovered],
    expected: &[EmissionRecord],
    target: DocumentTarget,
) -> Vec<EmissionRecord> {
    use std::fmt::Write;
    let mut records = vec![expected[0].clone()];
    let mut body = String::new();
    let mut parts = Vec::new();
    recovery_parts(recovered, &mut parts);
    for part in parts {
        let run = match part {
            RecoveryPart::Open(kind) => {
                write!(body, "@{kind}{{").expect("string write");
                continue;
            }
            RecoveryPart::Close => {
                body.push('}');
                continue;
            }
            RecoveryPart::Run(run) => run,
        };
        let (kind, value) = match run {
            Recovered::Text(text) => {
                body.push_str(text);
                continue;
            }
            Recovered::Tag {
                handle,
                label,
                token,
            } => {
                let selector = block
                    .runs
                    .iter()
                    .flat_map(inline_handles)
                    .find(|candidate| *candidate == handle)
                    .and_then(|_| {
                        let mut flattened = Vec::new();
                        flatten_runs(&block.runs, &mut flattened);
                        flattened.into_iter().find_map(|run| match run {
                            Inline::PaperTag {
                                value,
                                mint: Some(selector),
                                ..
                            } if value.spelling == *handle => Some(selector.as_str()),
                            _ => None,
                        })
                    });
                if let Some(selector) = selector {
                    if target == DocumentTarget::Markdown && token.as_deref() != Some(selector) {
                        body.push_str("@invalid-handle");
                    } else {
                        write!(body, "@handle{{{selector}}}").expect("string write");
                    }
                } else {
                    write!(body, "@tag{{{handle}}}").expect("string write");
                }
                (EmissionKind::Tag, label.clone())
            }
            Recovered::Math(source) => {
                let candidates = block
                    .runs
                    .iter()
                    .filter_map(|run| match run {
                        Inline::Math {
                            key,
                            markdown,
                            latex,
                            ..
                        } if source
                            == if target == DocumentTarget::Markdown {
                                markdown
                            } else {
                                latex
                            } =>
                        {
                            Some(key.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let used = records
                    .iter()
                    .filter(|record| {
                        record.kind == EmissionKind::Math && candidates.contains(&record.value)
                    })
                    .count();
                let key = candidates
                    .get(used)
                    .or_else(|| candidates.last())
                    .cloned()
                    .unwrap_or_else(|| source.clone());
                write!(body, "@math{{{key}}}").expect("string write");
                (EmissionKind::Math, key)
            }
            Recovered::Citation {
                handle,
                label,
                token,
            } => {
                write!(body, "`{label}`").expect("string write");
                let index = records
                    .iter()
                    .filter(|record| matches!(record.kind, EmissionKind::Citation(_)))
                    .count();
                let identity = recovered_citation(block, index, handle, label, token.as_deref());
                (EmissionKind::Citation(Box::new(identity)), label.clone())
            }
            Recovered::BibliographyCitation {
                kind,
                keys,
                label,
                anchors,
            } => {
                let form = match kind {
                    ftoo_manifest::BibliographyCitationKind::Parenthetical => "parencite",
                    ftoo_manifest::BibliographyCitationKind::Textual => "textcite",
                };
                write!(body, "@{form}{{{}}}", keys.join(",")).expect("string write");
                let value = if target == DocumentTarget::Markdown {
                    label.clone()
                } else {
                    let mut flattened = Vec::new();
                    flatten_runs(&block.runs, &mut flattened);
                    flattened
                        .into_iter()
                        .find_map(|run| match run {
                            Inline::BibliographyCitation {
                                kind: expected_kind,
                                entries,
                                label,
                                ..
                            } if expected_kind == kind
                                && entries.iter().map(|entry| &entry.key).eq(keys.iter()) =>
                            {
                                Some(label.clone())
                            }
                            _ => None,
                        })
                        .unwrap_or_default()
                };
                (
                    EmissionKind::BibliographyCitation {
                        kind: *kind,
                        keys: keys.clone(),
                        anchors: anchors.clone(),
                    },
                    value,
                )
            }
            Recovered::Mention(text) => {
                write!(body, "``{text}``").expect("string write");
                (EmissionKind::Mention, text.clone())
            }
            Recovered::See {
                label,
                locator,
                href,
            } => {
                let reference = label.as_deref().unwrap_or("");
                write!(body, "@see{{{reference}}}").expect("string write");
                (
                    EmissionKind::See {
                        target: reference.into(),
                    },
                    if target == DocumentTarget::Markdown {
                        format!(
                            "{}\t{}",
                            locator.as_deref().unwrap_or(""),
                            href.as_deref().unwrap_or("")
                        )
                    } else {
                        reference.into()
                    },
                )
            }
            Recovered::Virtue {
                word, key, href, ..
            } => {
                write!(body, "@virtue{{{word}}}").expect("string write");
                (
                    EmissionKind::Virtue {
                        key: key.clone().unwrap_or_else(|| format!("v:{word}")),
                    },
                    if target == DocumentTarget::Markdown {
                        format!("{word}\t{}", href.as_deref().unwrap_or(""))
                    } else {
                        word.clone()
                    },
                )
            }
            Recovered::Styled { .. } => unreachable!("flattened recovery part"),
        };
        let index = records.iter().filter(|r| r.kind == kind).count();
        let matching = expected
            .iter()
            .filter(|r| r.kind == kind && r.value == value)
            .collect::<Vec<_>>();
        let repeated = records
            .iter()
            .filter(|r| r.kind == kind && r.value == value)
            .count();
        let citation_site = recovered_citation_site(block, run, &records);
        let site = citation_site
            .or_else(|| recovered_tag_site(block, run, &records))
            .unwrap_or_else(|| {
                matching
                    .get(repeated)
                    .or_else(|| matching.last())
                    .copied()
                    .or_else(|| expected.iter().filter(|r| r.kind == kind).nth(index))
                    .map_or_else(|| block.site.clone(), |r| r.site.clone())
            });
        records.push(EmissionRecord {
            kind,
            unit: block.unit.clone(),
            language: expected[0].language.clone(),
            target,
            occurrence: site.occurrence,
            value,
            site,
        });
    }
    records[0].value = body;
    records
}
pub(super) fn difference(
    expected: &[EmissionRecord],
    actual: &[EmissionRecord],
    missing: &mut Vec<EmissionRecord>,
    unexpected: &mut Vec<EmissionRecord>,
    changed: &mut Vec<super::diagnostics::EmissionChange>,
) {
    let (mut left, mut right) = (0, 0);
    while left < expected.len() && right < actual.len() {
        if expected[left] == actual[right] {
            left += 1;
            right += 1;
        } else if expected[left + 1..].contains(&actual[right]) {
            missing.push(expected[left].clone());
            left += 1;
        } else if actual[right + 1..].contains(&expected[left]) {
            unexpected.push(actual[right].clone());
            right += 1;
        } else if expected[left].kind == actual[right].kind
            && expected[left].site == actual[right].site
            && expected[left].unit == actual[right].unit
            && expected[left].language == actual[right].language
            && expected[left].target == actual[right].target
        {
            changed.push(super::diagnostics::EmissionChange {
                expected: expected[left].clone(),
                actual: actual[right].clone(),
            });
            left += 1;
            right += 1;
        } else {
            missing.push(expected[left].clone());
            unexpected.push(actual[right].clone());
            left += 1;
            right += 1;
        }
    }
    missing.extend_from_slice(&expected[left..]);
    unexpected.extend_from_slice(&actual[right..]);
}

fn region_records(
    block: &Block,
    plan: &DocumentPlan,
    target: DocumentTarget,
) -> Vec<EmissionRecord> {
    let mut inline_runs = Vec::new();
    flatten_runs(&block.runs, &mut inline_runs);
    inline_runs
        .into_iter()
        .filter_map(|run| {
            let (kind, value, site) = match run {
                Inline::Text(_) | Inline::Styled { .. } | Inline::See { .. } => return None,
                Inline::Virtue {
                    word, key, site, ..
                } => (EmissionKind::Virtue { key: key.clone() }, word, site),
                Inline::Tag { label, site, .. } => (EmissionKind::Tag, label, site),
                Inline::PaperTag { value, site, .. } => (EmissionKind::Tag, &value.target, site),
                Inline::Math { key, site, .. } => (EmissionKind::Math, key, site),
                Inline::Citation {
                    text,
                    site,
                    identity,
                } => (
                    EmissionKind::Citation(Box::new(identity.clone())),
                    text,
                    site,
                ),
                Inline::BibliographyCitation {
                    kind,
                    entries,
                    label,
                    site,
                } => (
                    EmissionKind::BibliographyCitation {
                        kind: *kind,
                        keys: entries.iter().map(|entry| entry.key.clone()).collect(),
                        anchors: if target == DocumentTarget::Markdown {
                            entries.iter().map(|entry| entry.anchor.clone()).collect()
                        } else {
                            Vec::new()
                        },
                    },
                    label,
                    site,
                ),
                Inline::Mention { text, site } => (EmissionKind::Mention, text, site),
            };
            Some(EmissionRecord {
                kind,
                unit: block.unit.clone(),
                language: plan.language.clone(),
                target,
                occurrence: site.occurrence,
                value: value.clone(),
                site: site.clone(),
            })
        })
        .collect()
}
#[allow(clippy::too_many_lines)] // One source-ordered walk retains whole-body insertion and citation coordinates.
pub(super) fn display_plans(
    index: usize,
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    equations: &Equations,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
) -> BTreeMap<DocumentTarget, DisplayBody> {
    let mut result = BTreeMap::new();
    let Some(bodies) = displays.bodies().get(unit.label().as_str()) else {
        return result;
    };
    for ((target, body_language), body) in bodies {
        if language != body_language {
            continue;
        }
        let mut regions = Vec::new();
        let mut span = 0;
        for region in body.regions() {
            let text = &region.source().value;
            let mut replacements = Vec::new();
            for occurrence in region.tags() {
                let target_id = welded
                    .integrated()
                    .resolved()
                    .book()
                    .resolve(occurrence.input())
                    .expect("checked display tag");
                let site = region.site(occurrence.site());
                let insertion = site.occurrence.expect("checked insertion index");
                let target_name = if *target == DocumentTarget::Markdown {
                    "markdown"
                } else {
                    "latex"
                };
                replacements.push((
                    occurrence.site().decoded.clone().expect("insertion"),
                    Inline::Tag {
                        handle: occurrence.input().into(),
                        label: target_id.label().into(),
                        token: format!("u{index}-d{target_name}-{language}-o{insertion}"),
                        site,
                    },
                ));
            }
            for occurrence in region.math() {
                let (markdown, latex, pending) =
                    &equations[&(unit.label().as_str().into(), occurrence.name().into())];
                replacements.push((
                    occurrence.site().decoded.clone().expect("insertion"),
                    Inline::Math {
                        definition_site: unit.math()[occurrence.name()].site.clone(),
                        key: occurrence.name().into(),
                        markdown: markdown.clone(),
                        latex: latex.clone(),
                        pending: pending.clone(),
                        site: region.site(occurrence.site()),
                    },
                ));
            }
            for (value, local, citation) in region.spans() {
                let site = region.site(&local);
                replacements.push((
                    local.decoded.expect("span"),
                    if citation {
                        citation_run(
                            value,
                            site,
                            format!(
                                "u{index}-d{}-{language}-c{span}",
                                if *target == DocumentTarget::Markdown {
                                    "markdown"
                                } else {
                                    "latex"
                                }
                            ),
                            unit.label().as_str(),
                            span,
                            Some(*target),
                            welded,
                        )
                    } else {
                        Inline::Mention { text: value, site }
                    },
                ));
                span += 1;
            }
            replacements.sort_by_key(|(r, _)| r.start);
            let mut runs = Vec::new();
            let mut at = 0;
            for (range, run) in replacements {
                if at < range.start {
                    runs.push(Inline::Text(text[at..range.start].replace("@@", "@")));
                }
                runs.push(run);
                at = range.end;
            }
            if at < text.len() {
                runs.push(Inline::Text(text[at..].replace("@@", "@")));
            }
            normalize(&mut runs);
            regions.push(Block {
                pending_names: BTreeMap::new(),
                unit: unit.label().as_str().into(),
                depth: None,
                quotation: false,
                paragraph: false,
                equation: false,
                econ: None,
                site: region.site(&region.source().site),
                runs,
                displays: BTreeMap::new(),
            });
        }
        result.insert(
            *target,
            DisplayBody {
                source: body.source().value.clone(),
                layout: body.layout().clone(),
                regions,
                site: body.source().site.clone(),
            },
        );
    }
    result
}

// Consume actual target delimiters independently of the writer's Part sequence.
#[allow(clippy::too_many_lines)] // Keep the independent layout reader and its record reconstruction together.
fn recover_display_block(
    source: &str,
    block: &Block,
    body: &DisplayBody,
    records: &[EmissionRecord],
    target: DocumentTarget,
) -> Option<Vec<EmissionRecord>> {
    fn take(rest: &mut &str, token: &str) -> Option<()> {
        *rest = rest.strip_prefix(token)?;
        Some(())
    }
    fn region<'a>(rest: &mut &'a str, end: &str) -> Option<&'a str> {
        let at = rest.find(end)?;
        let text = &rest[..at];
        *rest = &rest[at..];
        Some(text)
    }
    use ftoo_manifest::{DisplayLayout, PictureCommand};
    let (caption, display) = source.split_once("\n\n").unwrap_or((source, ""));
    let caption_count = records.iter().position(|r| r.kind == EmissionKind::Body)?;
    let recovered = recover_block(caption, block, target)?;
    let mut result = recovered_records(block, &recovered, &records[..caption_count], target);
    if recovered != expected_prose_runs(block, target) && result == records[..caption_count] {
        result[0].value = caption.into();
    }
    if display.is_empty() {
        return Some(result);
    }
    let full = format!("{display}\n");
    let mut rest = full.as_str();
    let mut values = Vec::new();

    match &body.layout {
        DisplayLayout::Table {
            columns,
            rows,
            rules,
        } => {
            if target == DocumentTarget::Latex {
                take(
                    &mut rest,
                    &format!(
                        "\\begin{{center}}\n\\begin{{tabular}}{{{}}}\n",
                        columns.concat()
                    ),
                )?;
            }
            for (row, cells) in rows.iter().enumerate() {
                if target == DocumentTarget::Markdown {
                    take(&mut rest, "| ")?;
                } else if rules.contains(&row) {
                    take(&mut rest, "\\hline\n")?;
                }
                for (column, index) in cells.iter().enumerate() {
                    let delimiter = match (target, column + 1 == cells.len()) {
                        (DocumentTarget::Markdown, false) => " | ",
                        (DocumentTarget::Markdown, true) => " |\n",
                        (DocumentTarget::Latex, false) => " & ",
                        (DocumentTarget::Latex, true) => concat!(" ", r"\\", "\n"),
                    };
                    values.push((*index, region(&mut rest, delimiter)?));
                    take(&mut rest, delimiter)?;
                }
                if target == DocumentTarget::Markdown && row == 0 {
                    take(&mut rest, &format!("| {} |\n", columns.join(" | ")))?;
                }
            }
            if target == DocumentTarget::Latex {
                take(&mut rest, "\\end{tabular}\n\\end{center}\n")?;
            }
        }
        DisplayLayout::Quote { rows } => {
            for row in rows {
                if let Some(index) = row {
                    take(&mut rest, "> ")?;
                    values.push((*index, region(&mut rest, "\n")?));
                } else {
                    take(&mut rest, ">")?;
                }
                take(&mut rest, "\n")?;
            }
        }
        DisplayLayout::Picture {
            width,
            height,
            commands,
        } => {
            take(
                &mut rest,
                &format!(
                    "\\begingroup\\setlength{{\\unitlength}}{{1pt}}\n\\begin{{picture}}({width},{height})\n"
                ),
            )?;
            for command in commands {
                match command {
                    PictureCommand::Label {
                        x,
                        y,
                        region: index,
                    } => {
                        take(&mut rest, &format!("\\put({x},{y}){{"))?;
                        values.push((*index, region(&mut rest, "}\n")?));
                        take(&mut rest, "}\n")?;
                    }
                    PictureCommand::Line {
                        x,
                        y,
                        dx,
                        dy,
                        length,
                        vector,
                    } => {
                        let name = if *vector { "vector" } else { "line" };
                        take(
                            &mut rest,
                            &format!("\\put({x},{y}){{\\{name}({dx},{dy}){{{length}}}}}\n"),
                        )?;
                    }
                }
            }
            take(&mut rest, "\\end{picture}\n\\endgroup\n")?;
        }
    }
    let consumed = full.len() - rest.len();
    let mut repetitions = 1;
    while !rest.is_empty() {
        rest = rest.strip_prefix('\n').unwrap_or(rest);
        rest = rest.strip_prefix(&full[..consumed])?;
        repetitions += 1;
    }
    let mut body_record = records[caption_count].clone();
    let mut changed = false;
    let mut uses = Vec::new();
    for (index, source) in values {
        let region = &body.regions[index];
        let recovered = recover_block(source, region, target)?;
        let expected: Vec<_> = records[caption_count + 1..]
            .iter()
            .filter(|r| r.site.path == region.site.path)
            .cloned()
            .collect();
        let mut region_records = vec![EmissionRecord {
            kind: EmissionKind::Body,
            unit: block.unit.clone(),
            language: records[0].language.clone(),
            target,
            occurrence: None,
            value: region.runs.iter().map(canonical_run).collect(),
            site: region.site.clone(),
        }];
        region_records.extend(expected);
        let actual = recovered_records(region, &recovered, &region_records, target);
        changed |= recovered != expected_prose_runs(region, target);
        uses.extend(actual.into_iter().skip(1));
    }
    if changed {
        body_record.value = display.into();
    }
    for _ in 0..repetitions {
        result.push(body_record.clone());
        result.extend(uses.clone());
    }
    Some(result)
}

fn pending_records(
    plan: &DocumentPlan,
    block: &Block,
    target: DocumentTarget,
) -> Vec<EmissionRecord> {
    // Independently walk the caption/heading/prose and each target region.
    // Deduplicate by semantic identity, then order by the checked catalogue index.
    let mut collected = BTreeMap::new();
    let mut regions = vec![block.runs.as_slice()];
    if let Some(display) = block.displays.get(&target) {
        for region in &display.regions {
            regions.push(region.runs.as_slice());
        }
    }
    for runs in regions {
        for run in runs {
            if let Inline::Math {
                pending,
                site,
                definition_site,
                ..
            } = run
            {
                for entry in pending {
                    let (_, explanation) =
                        collected.entry(entry.definition()).or_insert_with(|| {
                            (
                                entry.order(),
                                super::PendingExplanation {
                                    definition: entry.definition(),
                                    body_macro: entry.body_macro().into(),
                                    body_unicode: entry.body_unicode().into(),
                                    reasons: entry.reasons().to_vec(),
                                    names: block.pending_names[&(target, entry.definition())]
                                        .clone(),
                                    occurrences: vec![],
                                },
                            )
                        });
                    explanation.occurrences.push(super::PendingOccurrence {
                        definition: SourceOrigin {
                            unit: Some(block.unit.clone()),
                            language: Some(plan.language.clone()),
                            target: None,
                            site: definition_site.clone(),
                        },
                        source: SourceOrigin {
                            unit: Some(block.unit.clone()),
                            language: Some(plan.language.clone()),
                            target: Some(target),
                            site: site.clone(),
                        },
                        notation: entry.origins().to_vec(),
                    });
                }
            }
        }
    }
    let mut entries: Vec<_> = collected.into_values().collect();
    entries.sort_by_key(|(order, _)| *order);
    entries
        .into_iter()
        .map(|(_, entry)| entry)
        .map(|entry| {
            let body = if target == DocumentTarget::Latex {
                &entry.body_macro
            } else {
                &entry.body_unicode
            };
            let value = format!(
                "{body}: {}",
                entry
                    .reasons
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            EmissionRecord {
                kind: EmissionKind::PendingExplanation(Box::new(entry)),
                unit: block.unit.clone(),
                language: plan.language.clone(),
                target,
                occurrence: None,
                value,
                site: block.site.clone(),
            }
        })
        .collect()
}

fn recover_pending(
    plan: &DocumentPlan,
    block: &Block,
    target: DocumentTarget,
    source: Option<&str>,
    expected: &[EmissionRecord],
) -> Vec<EmissionRecord> {
    let Some(source) = source else { return vec![] };
    let mut wanted = vec![];
    let mut prose = String::from("Pending notation: ");
    for (index, record) in expected.iter().enumerate() {
        let EmissionKind::PendingExplanation(entry) = &record.kind else {
            unreachable!("pending record")
        };
        flush_text(&mut wanted, &mut prose);
        wanted.push(Recovered::Math(if target == DocumentTarget::Latex {
            entry.body_macro.clone()
        } else {
            entry.body_unicode.clone()
        }));
        prose.push_str(": ");
        let mut names = entry.names.iter();
        for (reason, value) in entry.reasons.iter().enumerate() {
            if reason != 0 {
                prose.push_str("; ");
            }
            for part in value.text_parts() {
                match part {
                    ftoo_manifest::ReasonPart::Text(text) => prose.push_str(&text),
                    ftoo_manifest::ReasonPart::Object { .. } => {
                        let name = names.next().expect("every reason object resolved");
                        flush_text(&mut wanted, &mut prose);
                        wanted.push(Recovered::Tag {
                            handle: name.handle.clone(),
                            label: name.label.clone(),
                            token: (target == DocumentTarget::Latex).then(|| name.token.clone()),
                        });
                    }
                }
            }
        }
        prose.push_str(if index + 1 == expected.len() {
            "."
        } else {
            ". "
        });
    }
    flush_text(&mut wanted, &mut prose);
    let full = format!("Pending notation: {source}");
    let recovered = match target {
        DocumentTarget::Markdown => recover_markdown(
            &full,
            wanted.iter().filter_map(|run| match run {
                Recovered::Tag { handle, .. } => Some(handle.as_str()),
                _ => None,
            }),
        ),
        DocumentTarget::Latex => recover_latex(&full),
    };
    if recovered == Some(wanted) {
        return expected.to_vec();
    }
    let mut record = expected.first().cloned().unwrap_or_else(|| EmissionRecord {
        kind: EmissionKind::Paragraph,
        unit: block.unit.clone(),
        language: plan.language.clone(),
        target,
        occurrence: None,
        value: String::new(),
        site: block.site.clone(),
    });
    if let (Some(recovered), EmissionKind::PendingExplanation(entry)) =
        (recovered, &mut record.kind)
    {
        let tags: Vec<_> = recovered
            .into_iter()
            .filter_map(|run| match run {
                Recovered::Tag {
                    handle,
                    label,
                    token,
                } => Some((handle, label, token)),
                _ => None,
            })
            .collect();
        for (name, (handle, label, token)) in entry.names.iter_mut().zip(tags) {
            name.handle = handle;
            name.label = label;
            if let Some(token) = token {
                name.token = token;
            }
        }
    }
    record.value = full;
    vec![record]
}

fn citation_run(
    text: String,
    site: UnitSite,
    token: String,
    unit: &str,
    span: usize,
    target: Option<DocumentTarget>,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
) -> Inline {
    let mint = welded.integrated().mints()[&text].clone();
    let resolved = match mint.kind {
        ftoo_manifest::MintKind::RustIdentity => {
            let ftoo_manifest::LabelSite::RustIdentity { identity, .. } = &mint.site else {
                unreachable!("checked Rust mint retains its identity site")
            };
            ftoo_manifest::AddressTarget::Theory(*identity)
        }
        ftoo_manifest::MintKind::Markdown
        | ftoo_manifest::MintKind::UnitMetadata
        | ftoo_manifest::MintKind::OpenQuestion => {
            ftoo_manifest::AddressTarget::Documentary(text.clone())
        }
        ftoo_manifest::MintKind::PaperTex | ftoo_manifest::MintKind::UnitPaperMint => {
            unreachable!("strict unit integration refuses paper mints")
        }
    };
    Inline::Citation {
        identity: super::CitationIdentity {
            label: text.clone(),
            resolved: Some(resolved),
            handle: String::new(),
            token,
            position: ftoo_manifest::ReaderIdentityPosition::Citation {
                unit: unit.into(),
                span,
                target,
            },
            mint: Some(mint),
        },
        text,
        site,
    }
}
impl Block {
    fn map_citations(
        &mut self,
        book: &ftoo_manifest::AddressBook<'_>,
        document: &str,
        language: &str,
    ) -> Result<(), DocumentError> {
        let mut errors = vec![];
        let unit = self.unit.clone();
        map_citations_in_runs(&mut self.runs, book, document, language, &unit, &mut errors);
        for region in self
            .displays
            .values_mut()
            .flat_map(|body| body.regions.iter_mut())
        {
            map_citations_in_runs(
                &mut region.runs,
                book,
                document,
                language,
                &unit,
                &mut errors,
            );
        }
        AssemblyErrors::from_diagnostics(errors)
            .map_or(Ok(()), |errors| Err(DocumentError::Assembly(errors)))
    }
}
fn map_citations_in_runs(
    runs: &mut [Inline],
    book: &ftoo_manifest::AddressBook<'_>,
    document: &str,
    language: &str,
    unit: &str,
    errors: &mut Vec<AssemblyDiagnostic>,
) {
    for run in runs {
        if let Inline::Styled { children, .. } = run {
            map_citations_in_runs(children, book, document, language, unit, errors);
        } else if let Inline::Citation { identity, site, .. } = run {
            let target = match &identity.position {
                ftoo_manifest::ReaderIdentityPosition::Citation { target, .. } => *target,
                _ => unreachable!("citation position"),
            };
            match book.reader_identity(identity.resolved.clone().expect("checked citation target"))
            {
                Ok(handle) => identity.handle = handle,
                Err(fault) => collect_reader_error(
                    errors,
                    reader_failure(
                        fault,
                        document,
                        language,
                        Some(unit),
                        target,
                        identity.position.clone(),
                        citation_origins(identity, site, unit, language),
                    ),
                ),
            }
        }
    }
}

fn recover_handle(tail: &str) -> Option<(String, usize)> {
    if let Some(encoded) = tail.strip_prefix(r"\FtdocHandle{") {
        let end = encoded.find('}')?;
        let mut value = String::new();
        for scalar in encoded[..end].split(',') {
            let n = scalar.parse::<u32>().ok()?;
            if n.to_string() != scalar {
                return None;
            }
            value.push(char::from_u32(n)?);
        }
        if value.is_empty()
            || value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b":-/.".contains(&b))
        {
            return None;
        }
        let used = r"\FtdocHandle{".len() + end + 1;
        tail[used..].strip_prefix('}')?;
        Some((value, used + 1))
    } else {
        let end = tail.find('}')?;
        let value = &tail[..end];
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b":-/.".contains(&b))
        {
            return None;
        }
        Some((value.into(), end + 1))
    }
}
fn recovered_citation(
    block: &Block,
    index: usize,
    handle: &str,
    label: &str,
    token: Option<&str>,
) -> super::CitationIdentity {
    let mut flattened = Vec::new();
    flatten_runs(&block.runs, &mut flattened);
    let candidates = flattened
        .into_iter()
        .filter_map(|run| {
            if let Inline::Citation { identity, .. } = run {
                Some(identity)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let candidate = candidates.get(index).or_else(|| candidates.last()).copied();
    let mut identity = candidate
        .cloned()
        .unwrap_or_else(|| super::CitationIdentity {
            label: String::new(),
            resolved: None,
            handle: String::new(),
            token: String::new(),
            position: ftoo_manifest::ReaderIdentityPosition::Citation {
                unit: block.unit.clone(),
                span: index,
                target: None,
            },
            mint: None,
        });
    if identity.label != label {
        identity.resolved = None;
        identity.mint = None;
    }
    identity.label = label.into();
    identity.handle = handle.into();
    if let Some(token) = token {
        identity.token = token.into();
    }
    identity
}
pub(super) fn citation_origins(
    identity: &super::CitationIdentity,
    site: &UnitSite,
    unit: &str,
    language: &str,
) -> Vec<SourceOrigin> {
    let target = match &identity.position {
        ftoo_manifest::ReaderIdentityPosition::Citation { target, .. } => *target,
        _ => unreachable!("citation position"),
    };
    let mut origins = vec![SourceOrigin {
        unit: Some(unit.into()),
        language: Some(language.into()),
        target,
        site: site.clone(),
    }];
    if let Some(mint) = &identity.mint {
        let (unit, site) = match &mint.site {
            ftoo_manifest::LabelSite::Unit { unit, site } => (Some(unit.clone()), site.clone()),
            ftoo_manifest::LabelSite::Markdown { file, bytes } => (
                None,
                UnitSite {
                    file: file.clone(),
                    path: vec![],
                    anchor: ftoo_manifest::SourceAnchor::Bytes(bytes.start, bytes.end),
                    decoded: None,
                    occurrence: None,
                },
            ),
            ftoo_manifest::LabelSite::RustIdentity { file, .. }
            | ftoo_manifest::LabelSite::OpenQuestion { file, .. } => (
                None,
                UnitSite {
                    file: file.clone(),
                    path: vec![],
                    anchor: ftoo_manifest::SourceAnchor::Unavailable,
                    decoded: None,
                    occurrence: None,
                },
            ),
            ftoo_manifest::LabelSite::RustHistory { .. }
            | ftoo_manifest::LabelSite::UnitPaperMint { .. }
            | ftoo_manifest::LabelSite::PaperMintFile { .. }
            | ftoo_manifest::LabelSite::PaperTagRow { .. }
            | ftoo_manifest::LabelSite::MintContext => unreachable!("checked mint origin"),
        };
        origins.push(SourceOrigin {
            unit,
            language: None,
            target: None,
            site,
        });
    }
    origins
}

fn insertion_protection(text: &ftoo_manifest::CheckedText) -> Vec<std::ops::Range<usize>> {
    text.tag_occurrences()
        .iter()
        .map(ftoo_manifest::TagOccurrence::site)
        .chain(
            text.occurrences()
                .iter()
                .map(ftoo_manifest::MathOccurrence::site),
        )
        .chain(
            text.reference_occurrences()
                .iter()
                .map(ftoo_manifest::ReferenceOccurrence::site),
        )
        .filter_map(|site| site.decoded.clone())
        .collect()
}
fn check_reader_literals(
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
    errors: &mut Vec<AssemblyDiagnostic>,
) {
    let text = &unit.texts()[language];
    let spans = ftoo_manifest::assembly_text_spans(text);
    let protected = insertion_protection(text)
        .into_iter()
        .chain(
            spans
                .iter()
                .filter(|(_, _, citation)| *citation)
                .filter_map(|(_, site, _)| site.decoded.clone()),
        )
        .collect::<Vec<_>>();
    literal_region(
        unit,
        language,
        None,
        text.text(),
        text.site(),
        &spans,
        &protected,
        0,
        errors,
    );
    if let Some(bodies) = displays.bodies().get(unit.label().as_str()) {
        for ((target, body_language), body) in bodies {
            if body_language != language {
                continue;
            }
            for region in body.regions() {
                let spans = region.spans();
                let protected = region
                    .tags()
                    .iter()
                    .map(ftoo_manifest::TagOccurrence::site)
                    .chain(
                        region
                            .math()
                            .iter()
                            .map(ftoo_manifest::MathOccurrence::site),
                    )
                    .filter_map(|site| site.decoded.clone())
                    .chain(
                        spans
                            .iter()
                            .filter(|(_, _, citation)| *citation)
                            .filter_map(|(_, site, _)| site.decoded.clone()),
                    )
                    .collect::<Vec<_>>();
                literal_region(
                    unit,
                    language,
                    Some(*target),
                    &region.source().value,
                    &region.source().site,
                    &spans,
                    &protected,
                    region.bytes().start,
                    errors,
                );
            }
        }
    }
}
#[allow(clippy::too_many_arguments)] // Preserve the source region and its original decoded coordinate system.
fn literal_region(
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    target: Option<DocumentTarget>,
    text: &str,
    site: &UnitSite,
    spans: &[(String, UnitSite, bool)],
    protected: &[std::ops::Range<usize>],
    offset: usize,
    errors: &mut Vec<AssemblyDiagnostic>,
) {
    for bytes in ftoo_manifest::reader_label_literals(text) {
        if protected
            .iter()
            .any(|range| range.start <= bytes.start && bytes.end <= range.end)
        {
            continue;
        }
        let mut site = spans
            .iter()
            .find(|(_, site, citation)| {
                !citation
                    && site
                        .decoded
                        .as_ref()
                        .is_some_and(|range| range.start <= bytes.start && bytes.end <= range.end)
            })
            .map_or_else(|| site.clone(), |(_, site, _)| site.clone());
        if let Some(range) = &mut site.decoded {
            range.start += offset;
            range.end += offset;
        }
        errors.push(AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: Some(unit.document().into()),
            unit: Some(unit.label().as_str().into()),
            language: Some(language.into()),
            target,
            site: AssemblySite::Source(site),
            fault: AssemblyFault::ReaderLabelLiteral {
                value: text[bytes.clone()].into(),
                bytes: bytes.start + offset..bytes.end + offset,
            },
        });
    }
}

#[cfg(test)]
pub(super) fn fixture_citation_plan(welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>) -> DocumentPlan {
    let book = welded.integrated().resolved().book();
    let unit = &welded.integrated().resolved().corpus().units()[0];
    let mut block = Block {
        unit: unit.label().as_str().into(),
        depth: None,
        quotation: false,
        paragraph: false,
        equation: false,
        econ: None,
        site: super::input::site("fixture.toml"),
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        runs: runs(0, unit, "en", welded, &BTreeMap::new(), "", None).unwrap(),
    };
    block.map_citations(book, "fixture", "en").unwrap();
    DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![block],
        screen: None,
    }
}

#[cfg(test)]
mod spacing_tests {
    use super::*;

    #[test]
    fn identity_literals_require_exact_escaping() {
        for (raw, escaped) in [
            ("_", r"\_"),
            ("#", r"\#"),
            ("$", r"\$"),
            ("&", r"\&"),
            ("^", r"\textasciicircum{}"),
            ("~", r"\textasciitilde{}"),
        ] {
            assert_eq!(recover_identity(raw), None);
            assert_eq!(recover_identity(escaped), Some(raw.into()));
            assert!(exact_literal(escaped, raw));
            assert!(!exact_literal(raw, raw));
        }
        assert!(exact_literal("Coverage text.", "Coverage text."));
        for encoded in [
            r"Coverage\space{}text.",
            "Coverage  text.",
            "Coverage\ntext.",
            r"Cover\allowbreak{}age text.",
        ] {
            assert!(!exact_literal(encoded, "Coverage text."));
        }
    }

    #[test]
    fn identity_boundaries_decode_exactly_and_refuse_extra_or_missing_breaks() {
        for (encoded, decoded) in [
            (
                r"a:\allowbreak{}b-\allowbreak{}c/\allowbreak{}d.\allowbreak{}e",
                "a:b-c/d.e",
            ),
            ("abcdefghijklmnopqrstuvwx", "abcdefghijklmnopqrstuvwx"),
            (
                r"a\allowbreak{}b\allowbreak{}c\allowbreak{}d\allowbreak{}e\allowbreak{}f\allowbreak{}g\allowbreak{}h\allowbreak{}i\allowbreak{}j\allowbreak{}k\allowbreak{}l\allowbreak{}m\allowbreak{}n\allowbreak{}o\allowbreak{}p\allowbreak{}q\allowbreak{}r\allowbreak{}s\allowbreak{}t\allowbreak{}u\allowbreak{}v\allowbreak{}w\allowbreak{}x\allowbreak{}λ",
                "abcdefghijklmnopqrstuvwxλ",
            ),
        ] {
            assert_eq!(recover_identity(encoded), Some(decoded.into()));
            assert_eq!(
                recover_identity(&format!("{encoded}\\allowbreak{{}}")),
                None
            );
            if encoded.contains(r"\allowbreak{}") {
                assert_eq!(
                    recover_identity(&encoded.replacen(r"\allowbreak{}", "", 1)),
                    None
                );
            }
            assert_eq!(recover_identity(encoded), Some(decoded.into()));
        }
    }

    #[test]
    fn token_spaces_normalize_source_lines_and_observe_tex_line_states() {
        let site = UnitSite {
            file: "fixture.toml".into(),
            path: vec![],
            anchor: ftoo_manifest::SourceAnchor::WholeFile,
            decoded: None,
            occurrence: None,
        };
        let mut runs = vec![
            Inline::Text(" \n  before\t ".into()),
            Inline::Tag {
                handle: "c:1".into(),
                label: "def:fixture:one".into(),
                token: "u0-t0".into(),
                site,
            },
            Inline::Text("\n\t after  \n".into()),
        ];
        normalize(&mut runs);
        let mut emitted = String::new();
        super::super::latex::inline(&mut emitted, &runs);
        assert_eq!(
            emitted,
            "before \\FtdocTag{c:1}{u0-t0}% label def:fixture:one\n\\space{}after"
        );
        assert_eq!(
            recover_latex(&emitted),
            Some(vec![
                Recovered::Text("before ".into()),
                Recovered::Tag {
                    handle: "c:1".into(),
                    label: "def:fixture:one".into(),
                    token: Some("u0-t0".into())
                },
                Recovered::Text(" after".into())
            ])
        );
        for source in ["word next", "word\n  next", "  word\t \n  next"] {
            assert_eq!(
                recover_latex(source),
                Some(vec![Recovered::Text("word next".into())])
            );
        }
        assert_eq!(recover_latex("word\n\nnext"), None);
        assert_eq!(
            recover_latex(r"\space{}\space{}word"),
            Some(vec![Recovered::Text("  word".into())])
        );
    }
}
