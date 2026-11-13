use super::plan::{DocumentPlan, Inline};
use std::fmt::Write;
#[derive(Clone, Copy)]
pub(super) struct RawTex(pub &'static str);
#[derive(Clone, Copy)]
pub(super) struct Insertion<'a>(pub &'a str);
#[derive(Clone, Copy)]
pub(super) enum Piece<'a> {
    Raw(RawTex),
    Insert(Insertion<'a>),
}
pub(super) fn append(output: &mut String, piece: Piece<'_>) {
    output.push_str(match piece {
        Piece::Raw(RawTex(s)) | Piece::Insert(Insertion(s)) => s,
    });
}
pub(super) const PREAMBLE: &str = r"\documentclass{article}
\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{fontspec}
\tracinglostchars=3
\hfuzz=0pt
\vfuzz=0pt
\setmainfont{lmroman10}[Extension=.otf,UprightFont=*-regular,BoldFont=*-bold,ItalicFont=*-italic,BoldItalicFont=*-bolditalic]
\setmonofont{lmmono10}[Extension=.otf,UprightFont=*-regular,ItalicFont=*-italic,BoldFont=lmmonolt10-bold,BoldItalicFont=lmmonolt10-boldoblique]
\input{notation.tex}
\input{addressing.tex}
";
pub(super) fn size(plan: &DocumentPlan) -> usize {
    let mut size = start(&plan.document, &plan.language).len();
    size = size
        .saturating_add(text(&plan.coverage.declaration).len())
        .saturating_add(2)
        .saturating_add(inline_size(&plan.coverage.runs))
        .saturating_add(2);
    for runs in &plan.coverage.audit {
        size = size.saturating_add(inline_size(runs)).saturating_add(2);
    }
    for block in &plan.blocks {
        size = size.saturating_add(
            format!(
                "% unit {} document {} language {}\n",
                block.unit, plan.document, plan.language
            )
            .len(),
        );
        if let Some(depth) = block.depth {
            size = size.saturating_add(
                match depth {
                    1 => r"\section*{}",
                    2 => r"\subsection*{}",
                    3 => r"\subsubsection*{}",
                    4 => r"\paragraph*{}",
                    5 => r"\subparagraph*{}",
                    _ => r"{\noindent\textbf{}\par}",
                }
                .len(),
            );
        }
        size = size.saturating_add(inline_size(&block.runs));
        size = size.saturating_add(2);
        if let Some(body) = block.displays.get(&ftoo_manifest::DocumentTarget::Latex) {
            size = size
                .saturating_add(super::display::size(
                    body,
                    ftoo_manifest::DocumentTarget::Latex,
                ))
                .saturating_add(1);
        }
        size = size.saturating_add(
            super::pending::paragraph(block, &plan.language, ftoo_manifest::DocumentTarget::Latex)
                .len(),
        );
    }
    size.saturating_add("\\end{document}\n".len())
}
fn text_size(source: &str) -> usize {
    source
        .chars()
        .map(|c| match c {
            '\\' => r"\textbackslash{}".len(),
            '{' | '}' | '%' | '#' | '$' | '&' | '_' => 2,
            '^' => r"\textasciicircum{}".len(),
            '~' => r"\textasciitilde{}".len(),
            _ => c.len_utf8(),
        })
        .fold(0, usize::saturating_add)
}
pub(super) fn text(source: &str) -> String {
    let mut output = String::new();
    for c in source.chars() {
        match c {
            '\\' => output.push_str(r"\textbackslash{}"),
            '{' => output.push_str(r"\{"),
            '}' => output.push_str(r"\}"),
            '%' => output.push_str(r"\%"),
            '#' => output.push_str(r"\#"),
            '$' => output.push_str(r"\$"),
            '&' => output.push_str(r"\&"),
            '_' => output.push_str(r"\_"),
            '^' => output.push_str(r"\textasciicircum{}"),
            '~' => output.push_str(r"\textasciitilde{}"),
            _ => output.push(c),
        }
    }
    output
}
pub(super) fn start(document: &str, language: &str) -> String {
    let mut output = String::new();
    append(&mut output, Piece::Raw(RawTex(PREAMBLE)));
    let context = format!("\\FtdocContext{{{document}}}{{{language}}}\n\\begin{{document}}\n");
    append(&mut output, Piece::Insert(Insertion(&context)));
    output
}
pub(super) fn render(plan: &DocumentPlan) -> String {
    let mut output = start(&plan.document, &plan.language);
    writeln!(output, "{}\n", text(&plan.coverage.declaration)).expect("string write");
    inline(&mut output, &plan.coverage.runs);
    for runs in &plan.coverage.audit {
        output.push_str("\n\n");
        inline(&mut output, runs);
    }
    output.push_str("\n\n");
    for block in &plan.blocks {
        writeln!(
            output,
            "% unit {} document {} language {}",
            block.unit, plan.document, plan.language
        )
        .expect("string write");
        if let Some(depth) = block.depth {
            output.push_str(match depth {
                1 => r"\section*{",
                2 => r"\subsection*{",
                3 => r"\subsubsection*{",
                4 => r"\paragraph*{",
                5 => r"\subparagraph*{",
                _ => r"{\noindent\textbf{",
            });
        }
        inline(&mut output, &block.runs);
        if let Some(depth) = block.depth {
            output.push_str(if depth == 6 { "}\\par}" } else { "}" });
        }
        output.push_str("\n\n");
        if let Some(body) = block.displays.get(&ftoo_manifest::DocumentTarget::Latex) {
            output.push_str(&super::display::render(
                body,
                ftoo_manifest::DocumentTarget::Latex,
            ));
            output.push('\n');
        }
        output.push_str(&super::pending::paragraph(
            block,
            &plan.language,
            ftoo_manifest::DocumentTarget::Latex,
        ));
    }
    output.push_str("\\end{document}\n");
    output
}

pub(super) fn inline(output: &mut String, runs: &[Inline]) {
    let mut commented = false;
    for run in runs {
        if matches!(run, Inline::Text(value) if value.is_empty()) {
            continue;
        }
        let previous_comment = std::mem::replace(&mut commented, false);
        match run {
            Inline::Text(value) => {
                let value = if previous_comment && let Some(rest) = value.strip_prefix(' ') {
                    output.push_str(r"\space{}");
                    rest
                } else {
                    value
                };
                output.push_str(&text(value));
            }
            Inline::Tag {
                handle,
                label,
                token,
                ..
            } => {
                let handle = handle_argument(handle);
                writeln!(output, "\\FtdocTag{{{handle}}}{{{token}}}% label {label}")
                    .expect("string write");
                commented = true;
            }
            Inline::Math { latex, .. } => {
                write!(output, "\\({latex}\\)").expect("string write");
            }
            Inline::Citation {
                text: label,
                identity,
                ..
            } => {
                writeln!(
                    output,
                    "\\FtdocTag{{{}}}{{{}}}% cite {label}",
                    handle_argument(&identity.handle),
                    identity.token
                )
                .expect("string write");
                commented = true;
            }
            Inline::Mention { text: value, .. } => {
                write!(output, "\\texttt{{{}}}", text(value)).expect("string write");
            }
        }
    }
}

pub(super) fn inline_size(runs: &[Inline]) -> usize {
    let mut size: usize = 0;
    let mut commented = false;
    for run in runs {
        if matches!(run, Inline::Text(value) if value.is_empty()) {
            continue;
        }
        let previous_comment = std::mem::replace(&mut commented, false);
        let count = match run {
            Inline::Text(value) => {
                if previous_comment && let Some(rest) = value.strip_prefix(' ') {
                    text_size(rest).saturating_add(r"\space{}".len())
                } else {
                    text_size(value)
                }
            }
            Inline::Tag {
                handle,
                label,
                token,
                ..
            } => {
                commented = true;
                let handle = handle_argument(handle);
                format!("\\FtdocTag{{{handle}}}{{{token}}}% label {label}\n").len()
            }
            Inline::Math { latex, .. } => latex.len().saturating_add(4),
            Inline::Citation { text, identity, .. } => {
                commented = true;
                handle_argument(&identity.handle)
                    .len()
                    .saturating_add(identity.token.len())
                    .saturating_add(format!("\\FtdocTag{{}}{{}}% cite {text}\n").len())
            }
            Inline::Mention { text, .. } => text_size(text).saturating_add(r"\texttt{}".len()),
        };
        size = size.saturating_add(count);
    }
    size
}

pub(super) fn handle_argument(handle: &str) -> String {
    if handle
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b":-/.".contains(&b))
    {
        return handle.into();
    }
    let scalars = handle
        .chars()
        .map(|c| u32::from(c).to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("\\FtdocHandle{{{scalars}}}")
}
