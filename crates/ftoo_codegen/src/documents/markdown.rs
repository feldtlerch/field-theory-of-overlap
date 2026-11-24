use super::{
    diagnostics::{
        AssemblyDiagnostic, AssemblyErrors, AssemblyFault, DocumentError, MarkdownMathReason,
        MarkdownStyleReason, StyleBoundarySide, StyleNeighbor,
    },
    plan::{DocumentPlan, Inline, StyleKind},
};
use ftoo_manifest::{AssemblySite, AssemblyStage, DisplayLayout, DocumentTarget, RuleId, UnitSite};
use std::fmt::Write;
use std::ops::Range;
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
pub(super) fn code(source: &str) -> String {
    let mut output = String::from("<code>");
    output.push_str(&text(source));
    output.push_str("</code>");
    output
}

fn math_failure(
    plan: &DocumentPlan,
    unit: Option<&str>,
    site: &UnitSite,
    definition_site: &UnitSite,
    key: &str,
    context: &str,
    reason: MarkdownMathReason,
) -> DocumentError {
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: Some(plan.document.clone()),
            unit: unit.map(str::to_owned),
            language: Some(plan.language.clone()),
            target: Some(DocumentTarget::Markdown),
            site: AssemblySite::Source(site.clone()),
            fault: AssemblyFault::UnsupportedMarkdownMath {
                context: context.into(),
                reason,
                key: key.into(),
                definition_site: definition_site.clone(),
            },
        }])
        .expect("one diagnostic"),
    )
}
fn style_failure(
    plan: &DocumentPlan,
    unit: Option<&str>,
    site: &UnitSite,
    kind: &str,
    context: &str,
    reason: MarkdownStyleReason,
) -> DocumentError {
    DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Emission,
            document: Some(plan.document.clone()),
            unit: unit.map(str::to_owned),
            language: Some(plan.language.clone()),
            target: Some(DocumentTarget::Markdown),
            site: AssemblySite::Source(site.clone()),
            fault: AssemblyFault::UnsupportedMarkdownStyle {
                kind: kind.into(),
                context: context.into(),
                reason,
            },
        }])
        .expect("one diagnostic"),
    )
}

#[allow(clippy::too_many_arguments)] // A refusal retains both source sites and its exact context.
fn check_math_body(
    plan: &DocumentPlan,
    unit: Option<&str>,
    site: &UnitSite,
    definition_site: &UnitSite,
    key: &str,
    context: &str,
    admitted: bool,
    body: &str,
    adjacent: Option<MarkdownMathReason>,
) -> Result<(), DocumentError> {
    let reason = if !admitted {
        Some(MarkdownMathReason::UnobservedContext)
    } else if adjacent.is_some() {
        adjacent
    } else {
        body.char_indices().find_map(|(offset, character)| {
            matches!(
                character,
                '`' | '$' | '|' | '<' | '&' | '*' | ':' | '\n' | '\r'
            )
            .then_some(MarkdownMathReason::ForbiddenBodyCharacter { character, offset })
        })
    };
    if let Some(reason) = reason {
        return Err(math_failure(
            plan,
            unit,
            site,
            definition_site,
            key,
            context,
            reason,
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // One ordered pass retains the first source fault across run kinds.
fn check_runs(
    plan: &DocumentPlan,
    unit: Option<&str>,
    runs: &[Inline],
    context: &str,
    admitted: bool,
) -> Result<(), DocumentError> {
    let mut adjacent = None;
    for (index, run) in runs.iter().enumerate() {
        match run {
            Inline::Text(text) if !text.is_empty() => adjacent = None,
            Inline::Text(_) => {}
            Inline::Tag { .. }
            | Inline::PaperTag { .. }
            | Inline::Citation { .. }
            | Inline::See { .. }
            | Inline::BibliographyCitation { .. } => {
                adjacent = Some(MarkdownMathReason::DirectTagOrCitationAdjacency);
            }
            Inline::Mention { .. } => {
                adjacent = Some(MarkdownMathReason::DirectCodeMentionAdjacency);
            }
            Inline::Styled {
                kind,
                children,
                site,
                ..
            } => {
                if !matches!(*kind, StyleKind::Emphasis | StyleKind::Strong) {
                    return Err(style_failure(
                        plan,
                        unit,
                        site,
                        kind.as_str(),
                        context,
                        MarkdownStyleReason::UnsupportedKind,
                    ));
                }
                if context != "ordinary prose" {
                    return Err(style_failure(
                        plan,
                        unit,
                        site,
                        kind.as_str(),
                        context,
                        MarkdownStyleReason::UnobservedContext,
                    ));
                }
                let previous = match adjacent.as_ref() {
                    Some(MarkdownMathReason::DirectMathAdjacency) => {
                        Some(StyleNeighbor::Mathematics)
                    }
                    Some(MarkdownMathReason::DirectTagOrCitationAdjacency) => {
                        Some(StyleNeighbor::TagOrCitation)
                    }
                    _ => None,
                };
                if let Some(neighbor) = previous.or_else(|| next_style_neighbor(&runs[index + 1..]))
                {
                    return Err(style_failure(
                        plan,
                        unit,
                        site,
                        kind.as_str(),
                        context,
                        MarkdownStyleReason::DirectAdjacency { neighbor },
                    ));
                }
                let mut emitted = String::new();
                inline(&mut emitted, children);
                let left = runs[..index]
                    .iter()
                    .rev()
                    .find_map(|run| edge_char(run, false));
                let right = runs[index + 1..]
                    .iter()
                    .find_map(|run| edge_char(run, true));
                let side = if emitted.chars().next().is_some_and(punctuation_edge)
                    && left.is_some_and(|character| !safe_outer_edge(character))
                {
                    Some(StyleBoundarySide::Start)
                } else if emitted.chars().last().is_some_and(punctuation_edge)
                    && right.is_some_and(|character| !safe_outer_edge(character))
                {
                    Some(StyleBoundarySide::End)
                } else {
                    None
                };
                if let Some(side) = side {
                    return Err(style_failure(
                        plan,
                        unit,
                        site,
                        kind.as_str(),
                        context,
                        MarkdownStyleReason::PunctuationEdge { side },
                    ));
                }
                check_runs(plan, unit, children, "styled span", false)?;
                adjacent = None;
            }
            Inline::Math {
                definition_site,
                key,
                markdown,
                pending,
                site,
                ..
            } => {
                check_math_body(
                    plan,
                    unit,
                    site,
                    definition_site,
                    key,
                    context,
                    admitted,
                    markdown,
                    adjacent.clone(),
                )?;
                for entry in pending {
                    check_math_body(
                        plan,
                        unit,
                        site,
                        definition_site,
                        key,
                        "pending prose",
                        true,
                        entry.body_unicode(),
                        None,
                    )?;
                }
                adjacent = Some(MarkdownMathReason::DirectMathAdjacency);
            }
        }
    }
    Ok(())
}

fn next_style_neighbor(runs: &[Inline]) -> Option<StyleNeighbor> {
    for run in runs {
        match run {
            Inline::Text(text) if text.is_empty() => {}
            Inline::Math { .. } => return Some(StyleNeighbor::Mathematics),
            Inline::Styled { .. } => return Some(StyleNeighbor::Style),
            Inline::Tag { .. }
            | Inline::PaperTag { .. }
            | Inline::Citation { .. }
            | Inline::See { .. }
            | Inline::BibliographyCitation { .. } => {
                return Some(StyleNeighbor::TagOrCitation);
            }
            _ => return None,
        }
    }
    None
}

fn edge_char(run: &Inline, first: bool) -> Option<std::primitive::char> {
    let mut emitted = String::new();
    inline(&mut emitted, std::slice::from_ref(run));
    if first {
        emitted.chars().next()
    } else {
        emitted.chars().last()
    }
}

fn punctuation_edge(value: char) -> bool {
    !value.is_alphanumeric() && !value.is_whitespace()
}

fn safe_outer_edge(value: char) -> bool {
    value.is_ascii_whitespace() || value.is_ascii_punctuation()
}

pub(super) fn check_math_admission(plan: &DocumentPlan) -> Result<(), DocumentError> {
    check_runs(plan, None, &plan.coverage.runs, "coverage prose", true)?;
    for runs in &plan.coverage.audit {
        check_runs(plan, None, runs, "audit prose", true)?;
    }
    for (index, block) in plan.blocks.iter().enumerate() {
        let (context, admitted) = if plan.screen.is_some() {
            if index == 0 {
                ("screen heading", false)
            } else if block.paragraph {
                ("ordinary prose", true)
            } else {
                ("refrain HTML block", false)
            }
        } else {
            match block.depth {
                None => ("ordinary prose", true),
                Some(4) => ("level-four heading", true),
                Some(_) => ("unobserved heading", false),
            }
        };
        check_runs(plan, Some(&block.unit), &block.runs, context, admitted)?;
        if let Some(body) = block.displays.get(&DocumentTarget::Markdown) {
            for (region_index, region) in body.regions.iter().enumerate() {
                let (context, admitted) = match &body.layout {
                    DisplayLayout::Table { rows, .. }
                        if rows
                            .first()
                            .is_some_and(|header| header.contains(&region_index)) =>
                    {
                        ("table header cell", false)
                    }
                    DisplayLayout::Table { .. } => ("table body cell", true),
                    DisplayLayout::Quote { .. } => ("blockquote", false),
                    DisplayLayout::Picture { .. } => ("picture", false),
                };
                check_runs(plan, Some(&block.unit), &region.runs, context, admitted)?;
            }
        }
    }
    Ok(())
}
pub(super) fn render(plan: &DocumentPlan) -> String {
    render_with_paper_spans(plan).0
}

#[allow(clippy::too_many_lines)] // One traversal binds emitted paper tags to their exact reader spans.
fn render_with_paper_spans(plan: &DocumentPlan) -> (String, Vec<(Range<usize>, String)>) {
    let mut paper_spans = Vec::new();
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
        for (index, block) in plan.blocks.iter().enumerate().skip(1) {
            if !screen.gaps[index - 1].message.is_empty() {
                writeln!(output, "{}\n", text(&screen.gaps[index - 1].message))
                    .expect("string write");
            }
            writeln!(
                output,
                "<!-- unit {} document {} language {} -->",
                block.unit, plan.document, plan.language
            )
            .expect("string write");
            if block.paragraph {
                for run in &block.runs {
                    if let Inline::PaperTag { value, .. } = run {
                        let start = output.len();
                        inline(&mut output, std::slice::from_ref(run));
                        paper_spans.push((
                            start..start + text(&value.spelling).len(),
                            value.key.clone(),
                        ));
                    } else {
                        inline(&mut output, std::slice::from_ref(run));
                    }
                }
                output.push_str("\n\n");
                continue;
            }
            let refrain = &screen.refrains[super::plan::refrain_index(&plan.blocks, index)
                .expect("non-paragraph screen position")];
            if refrain.quotation.is_some() {
                output.push_str("<p align=\"center\"><em>");
            } else {
                write!(
                    output,
                    "<p align=\"center\"><a id=\"{}\"></a><em>",
                    refrain.fragment
                )
                .expect("string write");
            }
            inline(&mut output, &block.runs);
            output.push_str("</em></p>\n<p align=\"right\">");
            if let Some(quotation) = &refrain.quotation {
                write!(
                    output,
                    "<a href=\"{}\">{}</a>",
                    quotation.href,
                    text(&refrain.handle)
                )
                .expect("string write");
                writeln!(output, "<!-- cite {} --></p>\n", refrain.bound_label)
                    .expect("string write");
            } else {
                output.push_str(&text(&refrain.handle));
                writeln!(output, "<!-- label {} --></p>\n", refrain.bound_label)
                    .expect("string write");
            }
        }
        writeln!(
            output,
            "{}\n",
            text(&screen.gaps.last().expect("gap").message)
        )
        .expect("string write");
        for section in super::plan::end_matter(plan) {
            match section {
                super::plan::EndMatter::Unavailable(entries) => {
                    output.push_str("## References to unavailable content\n\n");
                    for entry in entries {
                        writeln!(
                            output,
                            "<a id=\"{}\"></a>{} — {}. {}\n",
                            entry.fragment,
                            text(&entry.locator),
                            text(&entry.title),
                            text(entry.message)
                        )
                        .expect("string write");
                    }
                }
                super::plan::EndMatter::References(entries) => {
                    output.push_str("## References\n\n");
                    for entry in entries {
                        writeln!(
                            output,
                            "<a id=\"{}\"></a>{}<!-- bibentry {} -->\n",
                            entry.anchor,
                            text(&super::bibliography::screen_text(entry)),
                            entry.key
                        )
                        .expect("string write");
                    }
                }
            }
        }
        output.pop();
        return (output, paper_spans);
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
    (output, paper_spans)
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
    check_math_admission(plan)?;
    let roles = render_roles(plan)?;
    let origins = plan.origins(DocumentTarget::Markdown);
    super::check_rendered_bytes(
        &format!(
            "generated/documents/{}/{}.labels.md",
            plan.document, plan.language
        ),
        roles.0.as_bytes(),
        &origins,
    )?;
    super::check_rendered_bytes(
        &format!("generated/documents/{}/{}.md", plan.document, plan.language),
        roles.1.as_bytes(),
        &origins,
    )?;
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
            "<!-- see ",
            "<!-- bibcite ",
            "<!-- bibentry ",
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
    let (annotated, planned_spans) = render_with_paper_spans(plan);
    let allowed: Vec<_> = planned_spans
        .into_iter()
        .filter_map(|(range, key)| {
            let spelling = text(&key);
            (annotated[range.clone()] == spelling).then(|| {
                let start = erase_annotations(&annotated[..range.start])
                    .expect("generated annotation prefix")
                    .len();
                (start..start + spelling.len(), key)
            })
        })
        .filter(|(range, key)| reader.get(range.clone()) == Some(text(key).as_str()))
        .collect();
    let permitted = |range: &Range<usize>, label: &str| {
        allowed
            .iter()
            .any(|(span, key)| span == range && key == label)
    };
    let mut leak = None;
    for range in ftoo_manifest::reader_label_literals(&decoded) {
        let start = offsets[range.start];
        let end = offsets.get(range.end).copied().unwrap_or(reader.len());
        let bytes = start..end;
        if !permitted(&bytes, &decoded[range.clone()]) {
            select_leak(&mut leak, bytes, &decoded[range]);
        }
    }
    for &label in known_labels {
        for (start, _) in reader.match_indices(label) {
            let bytes = start..start + label.len();
            if !permitted(&bytes, label) {
                select_leak(&mut leak, bytes, label);
            }
        }
        for (start, _) in decoded.match_indices(label) {
            let end = start + label.len();
            let raw_start = offsets[start];
            let raw_end = offsets.get(end).copied().unwrap_or(reader.len());
            let bytes = raw_start..raw_end;
            if !permitted(&bytes, label) {
                select_leak(&mut leak, bytes, label);
            }
        }
    }
    // A checked paper handle in this plan makes a literal label in an attribute
    // a leak even when the attribute's address is malformed.
    if let Some((bytes, value)) = leak.as_ref().filter(|_| !allowed.is_empty()) {
        return Err(role_failure(
            plan,
            false,
            bytes.clone(),
            AssemblyFault::ReaderLabelLiteral {
                value: value.clone(),
                bytes: bytes.clone(),
            },
        ));
    }
    if plan.screen.is_some() {
        let bibliography = super::bibliography::entries(plan);
        let mut refrain_index = 0;
        for (at, prefix) in reader.match_indices("id=\"") {
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
            if bibliography.iter().any(|entry| entry.anchor == fragment) {
                if reader.matches(&format!("id=\"{fragment}\"")).count() != 1 {
                    return Err(role_failure(
                        plan,
                        false,
                        start..end,
                        AssemblyFault::InvalidPublicAnchor {
                            fragment: fragment.into(),
                        },
                    ));
                }
                continue;
            }
            if plan
                .screen
                .as_ref()
                .is_some_and(|screen| screen.index.iter().any(|entry| entry.fragment == fragment))
            {
                continue;
            }
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
            } else if let Some(refrain) = plan.screen.as_ref().and_then(|screen| {
                screen
                    .refrains
                    .iter()
                    .filter(|refrain| refrain.quotation.is_none())
                    .nth(refrain_index)
            }) && handle != refrain.handle
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
            refrain_index += 1;
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
    check_quotation_links(plan, reader, known_labels)?;
    // Erasure produces the reader from the generated comment forms, so
    // repeating erasure cannot independently check equality. Any comment
    // opener remaining in the reader is refused before publication.
    Ok(())
}

#[allow(clippy::too_many_lines)] // One link check binds its syntax, printed handle and planned destination.
fn check_quotation_links(
    plan: &DocumentPlan,
    reader: &str,
    known_labels: &[&str],
) -> Result<(), DocumentError> {
    let quotations: Vec<_> = plan
        .screen
        .iter()
        .flat_map(|screen| &screen.refrains)
        .filter_map(|refrain| refrain.quotation.as_ref().map(|quote| (refrain, quote)))
        .collect();
    let bibliography = super::bibliography::entries(plan);
    let mut quotation_index = 0;
    for (at, prefix) in reader.match_indices("href=\"") {
        let start = at + prefix.len();
        let tag_end = reader[start..].find('>').map(|offset| start + offset);
        let end = reader[start..].find('"').map(|offset| start + offset);
        let Some(end) = end.filter(|end| tag_end.is_none_or(|tag_end| *end < tag_end)) else {
            return Err(role_failure(
                plan,
                false,
                start..tag_end.unwrap_or(reader.len()),
                AssemblyFault::InvalidQuotationHref {
                    href: reader[start..tag_end.unwrap_or(reader.len())].into(),
                },
            ));
        };
        let href = &reader[start..end];
        if let Some(anchor) = href.strip_prefix('#')
            && bibliography.iter().any(|entry| entry.anchor == anchor)
        {
            continue;
        }
        if let Some(after) = reader[end..].strip_prefix("\">→ ")
            && let Some((locator, _)) = after.split_once("</a>")
        {
            let known = plan.blocks.iter().any(|block| {
                let mut runs = Vec::new();
                super::plan::flatten_runs(&block.runs, &mut runs);
                runs.iter().any(|run| matches!(run, Inline::See { locator: number, owner, language, fragment, .. } if locator == number && href == super::plan::markdown_document_href(owner, language, fragment)))
            });
            if !known {
                return Err(role_failure(
                    plan,
                    false,
                    start..end,
                    AssemblyFault::ReferenceDestination {
                        label: String::new(),
                        reason: "reader link differs from checked destination".into(),
                    },
                ));
            }
            continue;
        }
        let Some((_, _, fragment, decoded_handle)) = super::plan::parse_public_href(href) else {
            return Err(role_failure(
                plan,
                false,
                start..end,
                AssemblyFault::InvalidQuotationHref { href: href.into() },
            ));
        };
        let fragment_start = start + href.find('#').expect("parsed href") + 1;
        let fragment_range = fragment_start..end;
        if known_labels
            .iter()
            .any(|label| decoded_handle.contains(label))
            || !ftoo_manifest::reader_label_literals(&decoded_handle).is_empty()
        {
            return Err(role_failure(
                plan,
                false,
                fragment_range.clone(),
                AssemblyFault::ReaderLabelLiteral {
                    value: decoded_handle,
                    bytes: fragment_range,
                },
            ));
        }
        let Some((refrain, quotation)) = quotations.get(quotation_index) else {
            return Err(role_failure(
                plan,
                false,
                start..end,
                AssemblyFault::InvalidQuotationHref { href: href.into() },
            ));
        };
        quotation_index += 1;
        if !reader[..at].ends_with("<a ") || !reader[end..].starts_with("\">") {
            return Err(role_failure(
                plan,
                false,
                start..end,
                AssemblyFault::InvalidQuotationHref { href: href.into() },
            ));
        }
        let text_start = end + 2;
        let Some(close) = reader[text_start..]
            .find("</a>")
            .map(|offset| text_start + offset)
        else {
            return Err(role_failure(
                plan,
                false,
                text_start..reader.len(),
                AssemblyFault::QuotationHandleMismatch {
                    fragment: fragment.into(),
                    handle: reader[text_start..].into(),
                },
            ));
        };
        let printed = &reader[text_start..close];
        if printed != text(&decoded_handle) || decoded_handle != refrain.handle {
            return Err(role_failure(
                plan,
                false,
                text_start..close,
                AssemblyFault::QuotationHandleMismatch {
                    fragment: fragment.into(),
                    handle: printed.into(),
                },
            ));
        }
        if href != quotation.href {
            return Err(role_failure(
                plan,
                false,
                start..end,
                AssemblyFault::QuotationDestinationMismatch {
                    href: href.into(),
                    expected: quotation.href.clone(),
                },
            ));
        }
    }
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
            Inline::PaperTag { value, .. } => {
                write!(
                    output,
                    "{}<!-- label {} -->",
                    text(&value.spelling),
                    value.target
                )
                .expect("string write");
            }
            Inline::See {
                label,
                locator,
                owner,
                language,
                fragment,
                ..
            } => {
                let href = super::plan::markdown_document_href(owner, language, fragment);
                write!(
                    output,
                    "<a href=\"{href}\">→ {locator}</a><!-- see {label} -->"
                )
                .expect("string write");
            }
            Inline::Math { markdown, .. } => {
                write!(output, "$`{markdown}`$").expect("string write");
            }
            Inline::Citation {
                text: label,
                identity,
                ..
            } => {
                write!(output, "{}<!-- cite {label} -->", text(&identity.handle))
                    .expect("string write");
            }
            Inline::BibliographyCitation { kind, entries, .. } => {
                if *kind == ftoo_manifest::BibliographyCitationKind::Parenthetical {
                    output.push('(');
                }
                for (index, entry) in entries.iter().enumerate() {
                    if index != 0 {
                        output.push_str("; ");
                    }
                    let label = super::bibliography::screen_citation_piece(*kind, entry);
                    write!(output, "<a href=\"#{}\">{}</a>", entry.anchor, text(&label))
                        .expect("string write");
                }
                if *kind == ftoo_manifest::BibliographyCitationKind::Parenthetical {
                    output.push(')');
                }
                write!(
                    output,
                    "<!-- bibcite {} -->",
                    entries
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .expect("string write");
            }
            Inline::Mention { text, .. } => output.push_str(&code(text)),
            Inline::Styled { kind, children, .. } => {
                let delimiter = match *kind {
                    StyleKind::Emphasis => "*",
                    StyleKind::Strong => "**",
                    StyleKind::Smallcaps | StyleKind::Regular => {
                        unreachable!("Markdown style admission precedes writing")
                    }
                };
                output.push_str(delimiter);
                inline(output, children);
                output.push_str(delimiter);
            }
        }
    }
}
