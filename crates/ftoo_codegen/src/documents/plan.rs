use super::diagnostics::{
    AssemblyDiagnostic, AssemblyErrors, AssemblyFault, DocumentError, EmissionKind, EmissionRecord,
};
use crate::RenderableNotation;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, CheckedSkeletons, DocumentTarget, MathUseSite, RenderTarget,
    RuleId, SourceOrigin, UnitSite, WeldedUnits,
};
use std::collections::BTreeMap;
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
pub(super) enum Inline {
    Text(String),
    Tag {
        handle: String,
        label: String,
        token: String,
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
    Mention {
        text: String,
        site: UnitSite,
    },
}

#[derive(Debug, PartialEq)]
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
    Mention(String),
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
fn inline_handle(run: &Inline) -> Option<&str> {
    match run {
        Inline::Tag { handle, .. }
        | Inline::Citation {
            identity: super::CitationIdentity { handle, .. },
            ..
        } => Some(handle),
        _ => None,
    }
}
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
        if tail.starts_with("<!-- label ") || tail.starts_with("<!-- cite ") {
            let citation = tail.starts_with("<!-- cite ");
            let comment_prefix = if citation { 10 } else { 11 };
            let tail = &tail[comment_prefix..];
            let end = tail.find(" -->")?;
            let label = &tail[..end];
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
                    token: None,
                }
            });
            at += comment_prefix + end + 4;
        } else if let Some(tail) = tail.strip_prefix("<code>") {
            flush_text(&mut output, &mut text);
            let end = tail.find("</code>")?;
            let value = decode_code(&tail[..end])?;
            at += 6 + end + 7;
            output.push(Recovered::Mention(value));
        } else if let Some(tail) = tail.strip_prefix('$') {
            flush_text(&mut output, &mut text);
            let end = tail.find('$')?;
            output.push(Recovered::Math(tail[..end].into()));
            at += end + 2;
        } else if tail.starts_with('&') {
            let (c, n) = entity(tail, false)?;
            text.push(c);
            at += n;
        } else {
            let c = tail.chars().next()?;
            if matches!(c, '<' | '>' | '`' | '\n' | '\r') {
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
    matches!(run,Recovered::Text(text)|Recovered::Mention(text) if !ftoo_manifest::reader_label_literals(text).is_empty())
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
    let mut output = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    let mut line = TexLine::NewLine;
    while at < body.len() {
        let tail = &body[at..];
        if let Some(tail) = tail.strip_prefix(r"\FtdocTag{") {
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
                text.push(c);
                line = TexLine::MidLine;
            }
            at += c.len_utf8();
        }
    }
    flush_text(&mut output, &mut text);
    (!output.iter().any(literal_recovered)).then_some(output)
}
fn expected_runs(block: &Block, target: DocumentTarget) -> Vec<Recovered> {
    block
        .runs
        .iter()
        .filter_map(|run| {
            Some(match run {
                Inline::Text(text) if text.is_empty() => return None,
                Inline::Text(text) => Recovered::Text(text.clone()),
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
                Inline::Mention { text, .. } => Recovered::Mention(text.clone()),
            })
        })
        .collect()
}
#[allow(clippy::too_many_lines)] // Compare one complete candidate inventory, including captions and bodies.
pub(super) fn check_emission(
    plan: &DocumentPlan,
    source: &str,
    target: DocumentTarget,
) -> Result<(), DocumentError> {
    check_coverage(plan, source, target)?;
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
            if actual == records && recovered != expected_runs(block, target) {
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
                    "md"
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
            "md"
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
    pub site: UnitSite,
    pub runs: Vec<Inline>,
}
impl Block {
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
        self.runs.iter().chain(
            self.displays
                .get(&target)
                .into_iter()
                .flat_map(|body| body.regions.iter())
                .flat_map(|region| region.runs.iter()),
        )
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
        format!(
            "{}\\FtdocContext{{{}}}{{{}}}\n\\begin{{document}}\n",
            super::latex::PREAMBLE,
            plan.document,
            plan.language
        )
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
        "\n"
    };
    let (declaration, rest) = rest.split_once(separator).unwrap_or(("", rest));
    let (slice, audit_rest) = rest
        .split_once(if plan.coverage.audit.is_empty() {
            "\n\n"
        } else {
            separator
        })
        .unwrap_or((rest, ""));
    check_audit_paragraphs(plan, target, audit_rest, separator)?;
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
        recover_markdown(slice, block.runs.iter().filter_map(inline_handle))
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
) -> Result<(), DocumentError> {
    for runs in &plan.coverage.audit {
        let (source, next) = rest.split_once(separator).unwrap_or((rest, ""));
        rest = next;
        let mut block = plan.coverage.block();
        block.runs.clone_from(runs);
        let expected = expected_runs(&block, target);
        let actual = if target == DocumentTarget::Latex {
            recover_latex(source)
        } else {
            recover_markdown(source, runs.iter().filter_map(inline_handle))
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
                0..source.len(),
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
}
impl DocumentPlan {
    pub fn records(&self, target: DocumentTarget) -> Vec<EmissionRecord> {
        let mut records = Vec::new();
        for block in &self.blocks {
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
                value: block.runs.iter().map(canonical_run).collect(),
                site: block.site.clone(),
            });
            for run in &block.runs {
                let (kind, value, site) = match run {
                    Inline::Text(_) => continue,
                    Inline::Tag { label, site, .. } => (EmissionKind::Tag, label, site),
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
                    };
                    records.extend(region_records(region, &plan, target));
                }
            }
            records.extend(pending_records(self, block, target));
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
pub(super) fn build(
    skeletons: &CheckedSkeletons<'_>,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    notation: &RenderableNotation<'_, '_, '_>,
    metadata: &BTreeMap<String, UnitSite>,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
) -> Result<Vec<DocumentPlan>, DocumentError> {
    let corpus = skeletons.corpus();
    let book = welded.integrated().resolved().book();
    let equations = equations(corpus, notation, displays)?;
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
                let runs = runs(index, unit, language, welded, &equations);
                let bodies = display_plans(index, unit, language, welded, &equations, displays);
                let mut block = Block {
                    pending_names: BTreeMap::new(),
                    displays: bodies,
                    unit: position.unit().into(),
                    depth: position.depth(),
                    site: metadata[position.unit()].clone(),
                    runs,
                };
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
                });
            }
        }
    }
    if let Some(errors) = AssemblyErrors::from_diagnostics(errors) {
        return Err(DocumentError::Assembly(errors));
    }
    Ok(plans)
}
type Equations = BTreeMap<(String, String), (String, String, Vec<crate::PendingEntry>)>;
fn equations(
    corpus: &ftoo_manifest::LanguageCheckedUnits,
    notation: &RenderableNotation<'_, '_, '_>,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
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
            let parsed = ftoo_manifest::parse_assembly_math(notation.catalogue(), &source.value);
            let outcome = match parsed {
                Ok(parsed) => {
                    let ranges = parsed.ranges().to_vec();
                    notation
                        .check_expression(parsed.into_draft())
                        .and_then(|expr| {
                            let unicode = expr.render(RenderTarget::Unicode)?;
                            let macro_form = expr.render(RenderTarget::Macro)?;
                            Ok((
                                unicode.source().to_owned(),
                                macro_form.source().to_owned(),
                                unicode.pending_entries().to_vec(),
                            ))
                        })
                        .map_err(|cause| {
                            (
                                notation_bytes(&cause, &ranges, source.value.len()),
                                AssemblyFault::MathNotation { cause },
                            )
                        })
                }
                Err((bytes, fault)) => Err((bytes, fault.into())),
            };
            match outcome {
                Ok(pair) => {
                    equations.insert((unit.label().as_str().to_owned(), key.clone()), pair);
                }
                Err((bytes, fault)) => errors.push(AssemblyDiagnostic {
                    rule: RuleId::M053,
                    stage: AssemblyStage::Math,
                    document: Some(unit.document().into()),
                    unit: Some(unit.label().as_str().into()),
                    language: None,
                    target: None,
                    site: AssemblySite::Math {
                        definition: source.site.clone(),
                        key: key.clone(),
                        bytes,
                        uses,
                    },
                    fault,
                }),
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
fn runs(
    index: usize,
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
    equations: &Equations,
) -> Vec<Inline> {
    let book = welded.integrated().resolved().book();
    let text = &unit.texts()[language];
    let mut replacements = Vec::new();
    for occurrence in text.tag_occurrences() {
        let range = occurrence
            .site()
            .decoded
            .clone()
            .expect("admitted occurrence");
        let target = book
            .resolve(occurrence.input())
            .expect("same resolved book");
        replacements.push((
            range,
            Inline::Tag {
                handle: occurrence.input().into(),
                label: target.label().into(),
                token: format!(
                    "u{index}-t{}",
                    occurrence.site().occurrence.expect("insertion index")
                ),
                site: occurrence.site().clone(),
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
    spans(
        index,
        unit.label().as_str(),
        language,
        text,
        welded,
        &mut replacements,
    );
    replacements.sort_by_key(|(r, _)| r.start);
    let mut runs = Vec::new();
    let mut at = 0;
    for (range, run) in replacements {
        if at < range.start {
            runs.push(Inline::Text(
                text.text()[at..range.start].replace("@@", "@"),
            ));
        }
        runs.push(run);
        at = range.end;
    }
    if at < text.text().len() {
        runs.push(Inline::Text(text.text()[at..].replace("@@", "@")));
    }
    normalize(&mut runs);
    runs
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
        if let Inline::Text(text) = run {
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
    if target == DocumentTarget::Markdown {
        let heading = block
            .depth
            .map_or_else(String::new, |d| format!("{} ", "#".repeat(d)));
        body.strip_prefix(&heading)
            .and_then(|body| recover_markdown(body, block.runs.iter().filter_map(inline_handle)))
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
        framed.and_then(recover_latex)
    }
}

fn canonical_run(run: &Inline) -> String {
    match run {
        Inline::Text(text) => text.clone(),
        Inline::Tag { handle, .. } => format!("@tag{{{handle}}}"),
        Inline::Math { key, .. } => format!("@math{{{key}}}"),
        Inline::Citation { text, .. } => format!("`{text}`"),
        Inline::Mention { text, .. } => format!("``{text}``"),
    }
}
fn recovered_tag_site(
    block: &Block,
    run: &Recovered,
    records: &[EmissionRecord],
) -> Option<UnitSite> {
    let tag_sites: Vec<_> = if let Recovered::Tag { handle, .. } = run {
        block
            .runs
            .iter()
            .filter_map(|run| match run {
                Inline::Tag {
                    handle: expected,
                    site,
                    ..
                } if expected == handle => Some(site),
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
        let sites = block
            .runs
            .iter()
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
fn recovered_records(
    block: &Block,
    recovered: &[Recovered],
    expected: &[EmissionRecord],
    target: DocumentTarget,
) -> Vec<EmissionRecord> {
    use std::fmt::Write;
    let mut records = vec![expected[0].clone()];
    let mut body = String::new();
    for run in recovered {
        let (kind, value) = match run {
            Recovered::Text(text) => {
                body.push_str(text);
                continue;
            }
            Recovered::Tag { handle, label, .. } => {
                write!(body, "@tag{{{handle}}}").expect("string write");
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
            Recovered::Mention(text) => {
                write!(body, "``{text}``").expect("string write");
                (EmissionKind::Mention, text.clone())
            }
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
    block
        .runs
        .iter()
        .filter_map(|run| {
            let (kind, value, site) = match run {
                Inline::Text(_) => return None,
                Inline::Tag { label, site, .. } => (EmissionKind::Tag, label, site),
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
    if recovered != expected_runs(block, target) && result == records[..caption_count] {
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
        changed |= recovered != expected_runs(region, target);
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
        ftoo_manifest::MintKind::Markdown | ftoo_manifest::MintKind::UnitMetadata => {
            ftoo_manifest::AddressTarget::Documentary(text.clone())
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
        for run in self.runs.iter_mut().chain(
            self.displays
                .values_mut()
                .flat_map(|body| body.regions.iter_mut())
                .flat_map(|region| region.runs.iter_mut()),
        ) {
            if let Inline::Citation { identity, site, .. } = run {
                let target = match &identity.position {
                    ftoo_manifest::ReaderIdentityPosition::Citation { target, .. } => *target,
                    _ => unreachable!("citation position"),
                };
                match book
                    .reader_identity(identity.resolved.clone().expect("checked citation target"))
                {
                    Ok(handle) => identity.handle = handle,
                    Err(fault) => collect_reader_error(
                        &mut errors,
                        reader_failure(
                            fault,
                            document,
                            language,
                            Some(&unit),
                            target,
                            identity.position.clone(),
                            citation_origins(identity, site, &unit, language),
                        ),
                    ),
                }
            }
        }
        AssemblyErrors::from_diagnostics(errors)
            .map_or(Ok(()), |errors| Err(DocumentError::Assembly(errors)))
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
    let candidates = block
        .runs
        .iter()
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
            ftoo_manifest::LabelSite::RustIdentity { file, .. } => (
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

fn check_reader_literals(
    unit: &ftoo_manifest::CheckedUnit,
    language: &str,
    displays: &ftoo_manifest::CheckedDisplays<'_>,
    errors: &mut Vec<AssemblyDiagnostic>,
) {
    let text = &unit.texts()[language];
    let spans = ftoo_manifest::assembly_text_spans(text);
    let protected = text
        .tag_occurrences()
        .iter()
        .map(ftoo_manifest::TagOccurrence::site)
        .chain(
            text.occurrences()
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
        site: super::input::site("fixture.toml"),
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        runs: runs(0, unit, "en", welded, &BTreeMap::new()),
    };
    block.map_citations(book, "fixture", "en").unwrap();
    DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![block],
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
