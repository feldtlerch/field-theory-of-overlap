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
\input{native-paper-support.tex}
\input{notation.tex}
\input{addressing.tex}
";
pub(super) const BIBLATEX_OPTIONS: &str = "backend=bibtex,style=authoryear,maxcitenames=2,mincitenames=1,uniquename=full,uniquelist=true,doi=true,isbn=false,url=false";
pub(super) const BIBLATEX_EXECUTE_OPTIONS: &str = "ibidtracker=false,idemtracker=false";
pub(super) fn text(source: &str) -> String {
    escaped(source, false)
}
pub(super) fn prose_text(source: &str) -> String {
    escaped(source, true)
}
fn escaped(source: &str, prose: bool) -> String {
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
            '’' if prose => output.push('\''),
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
pub(super) fn start_plan(plan: &DocumentPlan) -> String {
    let bibliography = super::bibliography::entries(plan);
    if bibliography.is_empty() {
        return start(&plan.document, &plan.language);
    }
    let mut source = String::from("\\begin{filecontents*}{references.bib}\n");
    for entry in &bibliography {
        writeln!(source, "{}", entry.raw).expect("string write");
    }
    source.push_str("\\end{filecontents*}\n");
    append(&mut source, Piece::Raw(RawTex(PREAMBLE)));
    writeln!(source, "\\usepackage[{BIBLATEX_OPTIONS}]{{biblatex}}").expect("string write");
    writeln!(
        source,
        "\\ExecuteBibliographyOptions{{{BIBLATEX_EXECUTE_OPTIONS}}}"
    )
    .expect("string write");
    source.push_str("\\addbibresource{references.bib}\n");
    writeln!(
        source,
        "\\FtdocContext{{{}}}{{{}}}\n\\begin{{document}}",
        plan.document, plan.language
    )
    .expect("string write");
    source
}
pub(super) fn render(plan: &DocumentPlan) -> String {
    let bibliography = super::bibliography::entries(plan);
    let mut output = start_plan(plan);
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
        if block.equation {
            let (_, source) = block.equation_pair().expect("checked equation body");
            writeln!(output, "\\[\n{source}.\n\\]\n").expect("string write");
            continue;
        }
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
        prose_inline(&mut output, &block.runs);
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
    if !bibliography.is_empty() {
        output.push_str("\\printbibliography\n");
    }
    output.push_str("\\end{document}\n");
    output
}

pub(super) fn inline(output: &mut String, runs: &[Inline]) {
    emit_inline(output, runs, false);
}
pub(super) fn prose_inline(output: &mut String, runs: &[Inline]) {
    emit_inline(output, runs, true);
}
fn emit_inline(output: &mut String, runs: &[Inline], prose: bool) {
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
                output.push_str(&escaped(value, prose));
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
            Inline::PaperTag { value, .. } => {
                write!(output, "\\ftag{{{}}}", value.key).expect("string write");
            }
            Inline::See { label, .. } => {
                write!(output, "\\see{{{label}}}").expect("string write");
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
            Inline::BibliographyCitation { kind, entries, .. } => {
                let command = match kind {
                    ftoo_manifest::BibliographyCitationKind::Parenthetical => "parencite",
                    ftoo_manifest::BibliographyCitationKind::Textual => "textcite",
                };
                write!(
                    output,
                    "\\{command}{{{}}}",
                    entries
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .expect("string write");
            }
            Inline::Mention { text: value, .. } => {
                write!(output, "\\texttt{{{}}}", text(value)).expect("string write");
            }
            Inline::Styled { kind, children, .. } => {
                output.push_str(match *kind {
                    super::plan::StyleKind::Emphasis => "\\emph{",
                    super::plan::StyleKind::Strong => "\\textbf{",
                    super::plan::StyleKind::Smallcaps => "{\\scshape ",
                    super::plan::StyleKind::Regular => "{\\normalfont ",
                });
                emit_inline(output, children, prose);
                output.push('}');
            }
        }
    }
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
