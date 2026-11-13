use super::plan::{DocumentPlan, Inline};
use std::fmt::Write;
pub(super) fn size(plan: &DocumentPlan) -> usize {
    let mut size = format!(
        "<!-- generated document {} language {} -->\n{}\n",
        plan.document, plan.language, plan.coverage.declaration
    )
    .len()
    .saturating_add(inline_size(&plan.coverage.runs))
    .saturating_add(2);
    for runs in &plan.coverage.audit {
        size = size.saturating_add(inline_size(runs)).saturating_add(1);
    }
    for block in &plan.blocks {
        size = size.saturating_add(
            format!(
                "<!-- unit {} document {} language {} -->\n",
                block.unit, plan.document, plan.language
            )
            .len(),
        );
        size = size.saturating_add(block.depth.map_or(0, |d| d + 1));
        size = size.saturating_add(inline_size(&block.runs));
        size = size.saturating_add(2);
        if let Some(body) = block.displays.get(&ftoo_manifest::DocumentTarget::Markdown) {
            size = size
                .saturating_add(super::display::size(
                    body,
                    ftoo_manifest::DocumentTarget::Markdown,
                ))
                .saturating_add(1);
        }
        size = size.saturating_add(
            super::pending::paragraph(
                block,
                &plan.language,
                ftoo_manifest::DocumentTarget::Markdown,
            )
            .len(),
        );
    }
    size.saturating_sub(1)
}
fn text_size(source: &str) -> usize {
    source
        .chars()
        .map(|c| {
            if c.is_ascii_punctuation() && !matches!(c, ',' | '.' | ':' | ';' | '?' | '!') {
                if u32::from(c) < 100 { 5 } else { 6 }
            } else {
                c.len_utf8()
            }
        })
        .fold(0, usize::saturating_add)
}
fn code_size(source: &str) -> usize {
    source
        .chars()
        .map(|c| match c {
            '&' | '`' => 5,
            '<' | '>' => 4,
            '|' => 6,
            _ => c.len_utf8(),
        })
        .fold(13, usize::saturating_add)
}
pub(super) fn text(source: &str) -> String {
    let mut output = String::new();
    for c in source.chars() {
        if c.is_ascii_punctuation() && !matches!(c, ',' | '.' | ':' | ';' | '?' | '!') {
            write!(output, "&#{};", u32::from(c)).expect("string write");
        } else {
            output.push(c);
        }
    }
    output
}
fn code(source: &str) -> String {
    let mut output = String::from("<code>");
    for c in source.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '|' => output.push_str("&#124;"),
            '`' => output.push_str("&#96;"),
            _ => output.push(c),
        }
    }
    output.push_str("</code>");
    output
}
pub(super) fn render(plan: &DocumentPlan) -> String {
    let mut output = format!(
        "<!-- generated document {} language {} -->\n{}\n",
        plan.document, plan.language, plan.coverage.declaration
    );
    inline(&mut output, &plan.coverage.runs);
    for runs in &plan.coverage.audit {
        output.push('\n');
        inline(&mut output, runs);
    }
    output.push_str("\n\n");
    for block in &plan.blocks {
        writeln!(
            output,
            "<!-- unit {} document {} language {} -->",
            block.unit, plan.document, plan.language
        )
        .expect("string write");
        if let Some(depth) = block.depth {
            write!(output, "{} ", "#".repeat(depth)).expect("string write");
        }
        inline(&mut output, &block.runs);
        output.push_str("\n\n");
        if let Some(body) = block.displays.get(&ftoo_manifest::DocumentTarget::Markdown) {
            output.push_str(&super::display::render(
                body,
                ftoo_manifest::DocumentTarget::Markdown,
            ));
            output.push('\n');
        }
        output.push_str(&super::pending::paragraph(
            block,
            &plan.language,
            ftoo_manifest::DocumentTarget::Markdown,
        ));
    }
    output.pop();
    output
}

pub(super) fn inline(output: &mut String, runs: &[Inline]) {
    for run in runs {
        match run {
            Inline::Text(value) => output.push_str(&text(value)),
            Inline::Tag { handle, label, .. } => {
                write!(output, "{}<!-- label {label} -->", text(handle)).expect("string write");
            }
            Inline::Math { markdown, .. } => {
                write!(output, "${markdown}$").expect("string write");
            }
            Inline::Citation {
                text: label,
                identity,
                ..
            } => {
                write!(output, "{}<!-- cite {label} -->", text(&identity.handle))
                    .expect("string write");
            }
            Inline::Mention { text, .. } => output.push_str(&code(text)),
        }
    }
}

pub(super) fn inline_size(runs: &[Inline]) -> usize {
    let mut size: usize = 0;
    for run in runs {
        let count = match run {
            Inline::Text(value) => text_size(value),
            Inline::Tag { handle, label, .. } => {
                text_size(handle).saturating_add(format!("<!-- label {label} -->").len())
            }
            Inline::Math { markdown, .. } => markdown.len().saturating_add(2),
            Inline::Citation { text, identity, .. } => {
                text_size(&identity.handle).saturating_add(format!("<!-- cite {text} -->").len())
            }
            Inline::Mention { text, .. } => code_size(text),
        };
        size = size.saturating_add(count);
    }
    size
}
