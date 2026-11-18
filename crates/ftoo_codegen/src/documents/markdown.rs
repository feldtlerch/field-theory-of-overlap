use super::{
    diagnostics::{AssemblyFault, DocumentError},
    plan::{DocumentPlan, Inline},
};
use ftoo_manifest::{AssemblySite, AssemblyStage, DocumentTarget, RuleId};
use std::fmt::Write;
use std::ops::Range;
pub(super) fn size(plan: &DocumentPlan) -> usize {
    if plan.screen.is_some() {
        return render(plan).len();
    }
    let mut size = format!(
        "<!-- generated document {} language {} -->\n{}\n",
        plan.document, plan.language, plan.coverage.declaration
    )
    .len()
    .saturating_add(inline_size(&plan.coverage.runs))
    .saturating_add(6);
    for runs in &plan.coverage.audit {
        size = size.saturating_add(inline_size(runs)).saturating_add(5);
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
        "<!-- generated document {} language {} -->\n{}<br>\n",
        plan.document, plan.language, plan.coverage.declaration
    );
    inline(&mut output, &plan.coverage.runs);
    for runs in &plan.coverage.audit {
        output.push_str("<br>\n");
        inline(&mut output, runs);
    }
    output.push_str("\n\n");
    if let Some(screen) = &plan.screen {
        let heading = &plan.blocks[0];
        writeln!(
            output,
            "<!-- unit {} document {} language {} -->",
            heading.unit, plan.document, plan.language
        )
        .expect("string write");
        output.push_str("# ");
        inline(&mut output, &heading.runs);
        output.push_str("\n\n");
        for (index, refrain) in screen.refrains.iter().enumerate() {
            writeln!(output, "{}\n", text(&screen.gaps[index].message)).expect("string write");
            let block = &plan.blocks[index + 1];
            writeln!(
                output,
                "<!-- unit {} document {} language {} -->",
                block.unit, plan.document, plan.language
            )
            .expect("string write");
            write!(
                output,
                "<p align=\"center\"><a id=\"{}\"></a><em>",
                refrain.fragment
            )
            .expect("string write");
            inline(&mut output, &block.runs);
            output.push_str("</em></p>\n<p align=\"right\">");
            output.push_str(&text(&refrain.handle));
            writeln!(output, "<!-- label {} --></p>\n", refrain.bound_label).expect("string write");
        }
        writeln!(
            output,
            "{}\n",
            text(&screen.gaps.last().expect("gap").message)
        )
        .expect("string write");
        output.pop();
        return output;
    }
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

pub(super) fn render_roles(plan: &DocumentPlan) -> Result<(String, String), DocumentError> {
    let annotated = render(plan);
    let reader = erase_annotations(&annotated).map_err(|at| {
        role_failure(
            plan,
            true,
            at..at + 1,
            AssemblyFault::MarkdownRoleMismatch { offset: at },
        )
    })?;
    Ok((annotated, reader))
}

pub(super) fn emit(
    plan: &DocumentPlan,
    known_labels: &[&str],
) -> Result<(String, String), DocumentError> {
    let roles = render_roles(plan)?;
    check_reader(plan, &roles.1, known_labels)?;
    Ok(roles)
}

pub(super) fn erase_annotations(source: &str) -> Result<String, usize> {
    let mut reader = String::with_capacity(source.len());
    let mut at = 0;
    while let Some(relative) = source[at..].find("<!--") {
        let start = at + relative;
        reader.push_str(&source[at..start]);
        let Some(end) = source[start..].find("-->").map(|offset| start + offset + 3) else {
            return Err(start);
        };
        let comment = &source[start..end];
        if ![
            "<!-- generated document ",
            "<!-- unit ",
            "<!-- label ",
            "<!-- cite ",
        ]
        .iter()
        .any(|prefix| comment.starts_with(prefix))
        {
            return Err(start);
        }
        let line = start == 0 || source.as_bytes()[start - 1] == b'\n';
        at = end;
        if line {
            if !source[at..].starts_with('\n') {
                return Err(start);
            }
            at += 1;
        }
    }
    reader.push_str(&source[at..]);
    Ok(reader)
}

fn role_failure(
    plan: &DocumentPlan,
    annotated: bool,
    bytes: Range<usize>,
    fault: AssemblyFault,
) -> DocumentError {
    super::failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Generated {
            path: format!(
                "generated/documents/{}/{}{}",
                plan.document,
                plan.language,
                if annotated { ".labels.md" } else { ".md" }
            ),
            bytes,
            origins: plan.origins(DocumentTarget::Markdown),
        },
        fault,
    )
}

#[allow(clippy::too_many_lines)] // Decoded text offsets and public anchors share one reader scan.
pub(super) fn check_reader(
    plan: &DocumentPlan,
    reader: &str,
    known_labels: &[&str],
) -> Result<(), DocumentError> {
    if let Some(at) = reader.find("<!--") {
        return Err(role_failure(
            plan,
            false,
            at..at + 1,
            AssemblyFault::MarkdownRoleMismatch { offset: at },
        ));
    }
    let mut decoded = String::with_capacity(reader.len());
    let mut offsets = Vec::with_capacity(reader.len());
    let mut at = 0;
    while at < reader.len() {
        let tail = &reader[at..];
        if let Some(rest) = tail.strip_prefix("&#")
            && let Some(end) = rest.find(';')
            && let Ok(number) = rest[..end].parse::<u32>()
            && let Some(value) = char::from_u32(number)
        {
            decoded.push(value);
            offsets.extend(std::iter::repeat_n(at, value.len_utf8()));
            at += end + 3;
        } else {
            let value = tail.chars().next().expect("nonempty tail");
            decoded.push(value);
            offsets.extend(std::iter::repeat_n(at, value.len_utf8()));
            at += value.len_utf8();
        }
    }
    let mut leak = ftoo_manifest::reader_label_literals(&decoded)
        .into_iter()
        .map(|range| {
            let start = offsets[range.start];
            let end = offsets.get(range.end).copied().unwrap_or(reader.len());
            (start..end, decoded[range].to_owned())
        })
        .next();
    for &label in known_labels {
        if let Some(start) = reader.find(label) {
            select_leak(&mut leak, start..start + label.len(), label);
        }
        if let Some(start) = decoded.find(label) {
            let end = start + label.len();
            let raw_start = offsets[start];
            let raw_end = offsets.get(end).copied().unwrap_or(reader.len());
            select_leak(&mut leak, raw_start..raw_end, label);
        }
    }
    if plan.screen.is_some() {
        for (index, (at, prefix)) in reader.match_indices("id=\"").enumerate() {
            let start = at + prefix.len();
            let tag_end = reader[start..].find('>').map(|offset| start + offset);
            let end = reader[start..].find('"').map(|offset| start + offset);
            let Some(end) = end.filter(|end| tag_end.is_none_or(|tag_end| *end < tag_end)) else {
                let end = tag_end.unwrap_or(reader.len());
                return Err(role_failure(
                    plan,
                    false,
                    start..end,
                    AssemblyFault::UnterminatedPublicAnchor {
                        fragment: reader[start..end].into(),
                    },
                ));
            };
            let fragment = &reader[start..end];
            let Some(handle) = super::plan::inverse_fragment(fragment) else {
                return Err(role_failure(
                    plan,
                    false,
                    start..end,
                    AssemblyFault::InvalidPublicAnchor {
                        fragment: fragment.into(),
                    },
                ));
            };
            if known_labels.iter().any(|label| handle.contains(label))
                || !ftoo_manifest::reader_label_literals(&handle).is_empty()
            {
                select_leak(&mut leak, start..end, &handle);
            } else if let Some(refrain) = plan
                .screen
                .as_ref()
                .and_then(|screen| screen.refrains.get(index))
                && handle != refrain.handle
            {
                return Err(role_failure(
                    plan,
                    false,
                    start..end,
                    AssemblyFault::PublicAnchorMismatch {
                        fragment: fragment.into(),
                        handle,
                        expected: refrain.handle.clone(),
                    },
                ));
            }
        }
    }
    if let Some((bytes, value)) = leak {
        return Err(role_failure(
            plan,
            false,
            bytes.clone(),
            AssemblyFault::ReaderLabelLiteral { value, bytes },
        ));
    }
    // Erasure produces the reader from the four generated comment forms, so
    // repeating erasure cannot independently check equality. Any comment
    // opener remaining in the reader is refused before publication.
    Ok(())
}

fn select_leak(leak: &mut Option<(Range<usize>, String)>, bytes: Range<usize>, value: &str) {
    if leak
        .as_ref()
        .is_none_or(|(known, _)| bytes.start < known.start)
    {
        *leak = Some((bytes, value.to_owned()));
    }
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
