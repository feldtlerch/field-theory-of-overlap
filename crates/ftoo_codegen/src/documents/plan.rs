use super::diagnostics::{
    AssemblyDiagnostic, AssemblyErrors, AssemblyFault, DocumentError, EmissionKind, EmissionRecord,
};
use crate::RenderableNotation;
use ftoo_manifest::{
    AssemblySite, AssemblyStage, CheckedSkeletons, DocumentTarget, MathUseSite, RenderTarget,
    RuleId, SourceOrigin, UnitSite, WeldedUnits,
};
use std::collections::BTreeMap;

pub(super) enum Inline {
    Text(String),
    Tag {
        handle: String,
        label: String,
        token: String,
        site: UnitSite,
    },
    Math {
        key: String,
        markdown: String,
        latex: String,
        site: UnitSite,
    },
    Citation {
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
    Citation(String),
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
fn recover_markdown(body: &str, block: &Block) -> Option<Vec<Recovered>> {
    let mut output = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    while at < body.len() {
        let tail = &body[at..];
        if let Some(tail) = tail.strip_prefix("<!-- label ") {
            let end = tail.find(" -->")?;
            let label = &tail[..end];
            let handle = block
                .runs
                .iter()
                .filter_map(|run| match run {
                    Inline::Tag { handle, .. } if text.ends_with(handle) => Some(handle),
                    _ => None,
                })
                .max_by_key(|handle| handle.len())?;
            let prefix = text.strip_suffix(handle)?.to_owned();
            text = prefix;
            flush_text(&mut output, &mut text);
            output.push(Recovered::Tag {
                handle: handle.clone(),
                label: label.into(),
                token: None,
            });
            at += 11 + end + 4;
        } else if let Some(tail) = tail.strip_prefix("<code>") {
            flush_text(&mut output, &mut text);
            let end = tail.find("</code>")?;
            let value = decode_code(&tail[..end])?;
            at += 6 + end + 7;
            if let Some(tail) = body[at..].strip_prefix("<!-- cite ") {
                let end = tail.find(" -->")?;
                if tail[..end] != value {
                    return None;
                }
                at += 10 + end + 4;
                output.push(Recovered::Citation(value));
            } else {
                output.push(Recovered::Mention(value));
            }
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
    Some(output)
}
fn tex_literal(tail: &str) -> Option<(char, usize)> {
    for (spelling, c) in [
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
fn recover_latex(body: &str) -> Option<Vec<Recovered>> {
    let mut output = Vec::new();
    let mut text = String::new();
    let mut at = 0;
    while at < body.len() {
        let tail = &body[at..];
        if let Some(tail) = tail.strip_prefix(r"\FtdocTag{") {
            flush_text(&mut output, &mut text);
            let end = tail.find('}')?;
            let handle = &tail[..end];
            let rest = tail[end + 1..].strip_prefix('{')?;
            let token_end = rest.find('}')?;
            let comment = rest[token_end + 1..].strip_prefix("% label ")?;
            let comment_end = comment.find('\n')?;
            output.push(Recovered::Tag {
                handle: handle.into(),
                label: comment[..comment_end].into(),
                token: Some(rest[..token_end].into()),
            });
            at += 10 + end + 1 + 1 + token_end + 1 + 8 + comment_end + 1;
        } else if let Some(tail) = tail.strip_prefix(r"\(") {
            flush_text(&mut output, &mut text);
            let end = tail.find(r"\)")?;
            output.push(Recovered::Math(tail[..end].into()));
            at += 2 + end + 2;
        } else if let Some(tail) = tail.strip_prefix(r"\texttt{") {
            flush_text(&mut output, &mut text);
            let mut value = String::new();
            let mut end = 0;
            while !tail[end..].starts_with('}') {
                if tail[end..].starts_with('\\') {
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
            if let Some(tail) = body[at..].strip_prefix("% cite ") {
                let end = tail.find('\n')?;
                if tail[..end] != value {
                    return None;
                }
                at += 7 + end + 1;
                output.push(Recovered::Citation(value));
            } else {
                output.push(Recovered::Mention(value));
            }
        } else if tail.starts_with('\\') {
            let (c, n) = tex_literal(tail)?;
            text.push(c);
            at += n;
        } else {
            let c = tail.chars().next()?;
            if matches!(c, '{' | '}' | '%' | '\r' | '\n') {
                return None;
            }
            text.push(c);
            at += c.len_utf8();
        }
    }
    flush_text(&mut output, &mut text);
    Some(output)
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
                Inline::Citation { text, .. } => Recovered::Citation(text.clone()),
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
    let prefix = if target == DocumentTarget::Markdown {
        format!(
            "<!-- generated document {} language {} -->\n{}\n{}\n\n",
            plan.document, plan.language, plan.coverage[0], plan.coverage[1]
        )
    } else {
        format!(
            "{}\\FtdocContext{{{}}}{{{}}}\n\\begin{{document}}\n{}\n\n{}\n\n",
            super::latex::PREAMBLE,
            plan.document,
            plan.language,
            super::latex::text(&plan.coverage[0]),
            super::latex::text(&plan.coverage[1])
        )
    };
    source.starts_with(&prefix)
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
    pub displays: BTreeMap<DocumentTarget, DisplayBody>,
    pub unit: String,
    pub depth: Option<usize>,
    pub site: UnitSite,
    pub runs: Vec<Inline>,
}
impl Block {
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
pub(super) struct DocumentPlan {
    pub document: String,
    pub language: String,
    pub coverage: [String; 2],
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
                    Inline::Citation { text, site } => (EmissionKind::Citation, text, site),
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
        }
        records
    }
    pub fn origins(&self, target: DocumentTarget) -> Vec<SourceOrigin> {
        self.records(target)
            .into_iter()
            .map(|record| SourceOrigin {
                unit: Some(record.unit),
                language: Some(record.language),
                target: Some(target),
                site: record.site,
            })
            .collect()
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
        "Coverage declaration: {gate}. Closed world: referents={}; relations={}; audit={}. Evidence context: unaudited base graph.",
        closed.referents, closed.relations, closed.audit
    );
    let mut slice: Vec<_> = coverage.slice.iter().map(|id| id.semantic_name()).collect();
    slice.sort_unstable();
    let second = format!(
        "Declared slice: {}.",
        if slice.is_empty() {
            "(empty)".into()
        } else {
            slice.join(", ")
        }
    );
    let mut plans = Vec::new();
    for (document, positions) in skeletons.documents() {
        for language in &corpus.documents()[document] {
            let mut blocks = Vec::new();
            for position in positions {
                let (index, unit) = corpus
                    .units()
                    .iter()
                    .enumerate()
                    .find(|(_, u)| u.label().as_str() == position.unit())
                    .expect("checked skeleton unit");
                let runs = runs(index, unit, language, welded, &equations);
                let bodies = display_plans(index, unit, language, welded, &equations, displays);
                blocks.push(Block {
                    displays: bodies,
                    unit: position.unit().into(),
                    depth: position.depth(),
                    site: metadata[position.unit()].clone(),
                    runs,
                });
            }
            plans.push(DocumentPlan {
                document: document.clone(),
                language: language.clone(),
                coverage: [first.clone(), second.clone()],
                blocks,
            });
        }
    }
    Ok(plans)
}
type Equations = BTreeMap<(String, String), (String, String)>;
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
                            Ok((
                                expr.render(RenderTarget::Unicode)?.source().to_owned(),
                                expr.render(RenderTarget::Macro)?.source().to_owned(),
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
                label: target.semantic_name().into(),
                token: format!(
                    "u{index}-t{}",
                    occurrence.site().occurrence.expect("insertion index")
                ),
                site: occurrence.site().clone(),
            },
        ));
    }
    for occurrence in text.occurrences() {
        let (markdown, latex) =
            &equations[&(unit.label().as_str().into(), occurrence.name().into())];
        replacements.push((
            occurrence
                .site()
                .decoded
                .clone()
                .expect("admitted occurrence"),
            Inline::Math {
                key: occurrence.name().into(),
                markdown: markdown.clone(),
                latex: latex.clone(),
                site: occurrence.site().clone(),
            },
        ));
    }
    spans(
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
    for (value, site, citation) in ftoo_manifest::assembly_text_spans(text) {
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
                Inline::Citation { text: value, site }
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
            .and_then(|body| recover_markdown(body, block))
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
            Recovered::Citation(text) => {
                write!(body, "`{text}`").expect("string write");
                (EmissionKind::Citation, text.clone())
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
        let site = recovered_tag_site(block, run, &records).unwrap_or_else(|| {
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
                Inline::Citation { text, site } => (EmissionKind::Citation, text, site),
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
                        label: target_id.semantic_name().into(),
                        token: format!("u{index}-d{target_name}-{language}-o{insertion}"),
                        site,
                    },
                ));
            }
            for occurrence in region.math() {
                let (markdown, latex) =
                    &equations[&(unit.label().as_str().into(), occurrence.name().into())];
                replacements.push((
                    occurrence.site().decoded.clone().expect("insertion"),
                    Inline::Math {
                        key: occurrence.name().into(),
                        markdown: markdown.clone(),
                        latex: latex.clone(),
                        site: region.site(occurrence.site()),
                    },
                ));
            }
            for (value, local, citation) in region.spans() {
                let site = region.site(&local);
                replacements.push((
                    local.decoded.expect("span"),
                    if citation {
                        Inline::Citation { text: value, site }
                    } else {
                        Inline::Mention { text: value, site }
                    },
                ));
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
