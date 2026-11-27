//! Located source projection for one bounded corpus-slot comparison.

use crate::{documents::input, units::toml};
use ftoo_manifest::corpus::{
    AccountOwner, BlockKind, BlockStatus, LocatedPiece, SlotBlock, SlotRun, SourceDraft,
    check_transfer, import_source, located_inline,
};
use ftoo_manifest::{ShapeKind, SourceAnchor};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

#[cfg(test)]
const SOURCE: &str = "documents/paper/explainer/explainer.tex";
#[cfg(test)]
const SLOT: &str = "units/corpus/explainer-plain.toml";
#[cfg(test)]
const INTRO: &str = "units/statements/plain-intro.toml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    schema: u32,
    form: String,
    parent: String,
    candidate: String,
    source: String,
    #[serde(rename = "source-blob")]
    source_blob: String,
    first: usize,
    last: usize,
    unit: String,
    blocks: Vec<SelectedBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectedBlock {
    kind: String,
    first: usize,
    last: usize,
}

fn digest(bytes: &[u8]) -> String {
    crate::hex(&Sha256::digest(bytes))
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git {} refused: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim_end().to_owned())
        .map_err(|error| error.to_string())
}

fn captured(root: &Path, revision: &str, path: &str) -> Result<(Vec<u8>, String), String> {
    let blob = git(root, &["rev-parse", &format!("{revision}:{path}")])?;
    let bytes = fs::read(root.join(path)).map_err(|error| format!("{path}: {error}"))?;
    if blob_bytes(root, &blob)? != bytes {
        return Err(format!("{path}: captured input differs from {revision}"));
    }
    Ok((bytes, blob))
}

fn blob_bytes(root: &Path, blob: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "blob", blob])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("git cat-file blob {blob} refused"));
    }
    Ok(output.stdout)
}

fn line_start(bytes: &[u8], line: usize) -> Result<usize, String> {
    if line == 0 {
        return Err("source line must be positive".into());
    }
    let mut at = 0;
    for _ in 1..line {
        let Some(offset) = bytes[at..].iter().position(|byte| *byte == b'\n') else {
            return Err(format!("source has no line {line}"));
        };
        at += offset + 1;
    }
    Ok(at)
}

fn selection(request: &Assignment, bytes: &[u8], before: bool) -> Result<(usize, usize), String> {
    let result_form = request.blocks.len() == 1
        && request.blocks[0].kind == "theorem-container"
        && request.blocks[0].first == request.first
        && request.blocks[0].last == request.last;
    if request.schema != 4
        || request.form != "coarse-slot"
        || request.first == 0
        || request.first > request.last
        || (!result_form && request.blocks.len() < 2)
        || (!result_form && request.blocks[0].kind != "heading")
        || request.blocks[0].first != request.first
        || request
            .blocks
            .last()
            .is_none_or(|block| block.last != request.last)
        || (!result_form && !request.blocks.iter().any(|block| block.kind == "paragraph"))
        || request.blocks.iter().enumerate().any(|(index, block)| {
            block.first == 0
                || block.first > block.last
                || (index > 0
                    && (request.blocks[index - 1].last.checked_add(1) != Some(block.first)
                        || !matches!(
                            block.kind.as_str(),
                            "paragraph" | "existing-native-inclusion"
                        )))
        })
    {
        return Err("unsupported corpus-slot assignment".into());
    }
    let first = line_start(bytes, request.first)?;
    let end = if before {
        line_start(
            bytes,
            request.last.checked_add(1).ok_or("source line overflows")?,
        )
        .unwrap_or(bytes.len())
    } else {
        let after = line_start(
            bytes,
            request
                .first
                .checked_add(1)
                .ok_or("source line overflows")?,
        )
        .unwrap_or(bytes.len());
        let marker = format!("% generated: {}\n", request.unit);
        if bytes.get(first..after) != Some(marker.as_bytes()) {
            return Err("candidate marker differs from the selected slot".into());
        }
        if bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| *line == marker.trim_end().as_bytes())
            .count()
            != 1
        {
            return Err("candidate slot marker is not unique".into());
        }
        after
    };
    if end - first > 65_536 {
        return Err("selected source exceeds correspondence byte bound".into());
    }
    Ok((first, end))
}

fn span(anchor: &SourceAnchor) -> Result<(usize, usize), String> {
    match anchor {
        SourceAnchor::Bytes(start, end) => Ok((*start, *end)),
        _ => Err("native field has no byte span".into()),
    }
}

fn field<'a>(node: &'a toml::Node, name: &str) -> Result<&'a toml::Node, String> {
    node.fields
        .iter()
        .find(|(key, _, _)| key == name)
        .map(|(_, _, value)| value)
        .ok_or_else(|| format!("missing native field {name}"))
}

fn plain_field(node: &toml::Node, source: &str) -> Result<(usize, usize, String), String> {
    let value = toml::string(node).map_err(|error| error.to_string())?.value;
    let (start, end) = span(&node.site.anchor)?;
    let raw = source
        .get(start..end)
        .ok_or("native string range is invalid")?;
    if !raw.starts_with('"')
        || !raw.ends_with('"')
        || raw[1..raw.len() - 1].contains('\\')
        || raw[1..raw.len() - 1] != value
    {
        return Err("escaped or unsupported native string".into());
    }
    Ok((start + 1, end - 1, value))
}

fn origin(
    path: &str,
    blob: &str,
    bytes: &[u8],
    start: usize,
    end: usize,
    field: Option<&str>,
) -> Value {
    json!({"path":path,"blob":blob,"sha256":digest(bytes),"range":[start,end],"field":field})
}

fn control(kind: &str, start: usize, end: usize) -> Value {
    json!({"kind":kind,"range":[start,end]})
}

fn check_token_spelling(tokens: &[Value], source: &str) -> Result<(), String> {
    for token in tokens {
        let range = token["source_range"]
            .as_array()
            .ok_or("token range is absent")?;
        if range.len() != 2 {
            return Err("token range has wrong arity".into());
        }
        let start = usize::try_from(range[0].as_u64().ok_or("token start is invalid")?)
            .map_err(|error| error.to_string())?;
        let end = usize::try_from(range[1].as_u64().ok_or("token end is invalid")?)
            .map_err(|error| error.to_string())?;
        if source.get(start..end) != token["source_spelling"].as_str() {
            return Err("located token range does not reproduce its source spelling".into());
        }
    }
    Ok(())
}

fn tokenize(pieces: &[LocatedPiece], source: &str) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut characters = Vec::new();
    let mut controls = Vec::new();
    for piece in pieces {
        if let Some(kind) = piece.control {
            if kind == "unsupported-style" {
                return Err("unsupported inline style in correspondence".into());
            }
            controls.push(control(kind, piece.start, piece.end));
        }
        for ch in piece.semantic.chars() {
            characters.push((ch, piece.start, piece.end));
        }
    }
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut separator: Option<(usize, usize)> = None;
    let mut previous_end = pieces.first().map_or(0, |piece| piece.start);
    while index < characters.len() {
        let (ch, start, end) = characters[index];
        if ch.is_ascii_whitespace() {
            separator = Some(separator.map_or((start, end), |(begin, _)| (begin, end)));
            index += 1;
            continue;
        }
        let begin = index;
        let kind = if ch.is_ascii_alphabetic() {
            index += 1;
            while index < characters.len()
                && characters[index - 1].2 == characters[index].1
                && (characters[index].0.is_ascii_alphabetic()
                    || matches!(characters[index].0, '\'' | '’')
                        && characters
                            .get(index + 1)
                            .is_some_and(|next| next.0.is_ascii_alphabetic()))
            {
                index += 1;
            }
            "word"
        } else if ch.is_ascii_digit() {
            index += 1;
            while index < characters.len() && characters[index].0.is_ascii_digit() {
                index += 1;
            }
            "number"
        } else if ".,;:!?()/–—-“. ”‘’".contains(ch) {
            index += 1;
            "mark"
        } else {
            return Err(format!(
                "unsupported located character U+{:04X}",
                u32::from(ch)
            ));
        };
        let last = characters[index - 1].2;
        let spelling = source
            .get(start..last)
            .ok_or("located token is outside its source")?;
        let semantic: String = characters[begin..index]
            .iter()
            .map(|(value, _, _)| *value)
            .collect::<String>()
            .replace('\'', "’");
        tokens.push(json!({
            "kind":kind,"source_range":[start,last],"source_spelling":spelling,
            "semantic":semantic,"extracted":semantic,
            "separator_before":separator.is_some(),
            "separator_range":separator.map(|_| [previous_end,start]),"tag":null
        }));
        previous_end = last;
        separator = None;
    }
    if tokens.is_empty() {
        return Err("selected child has no located tokens".into());
    }
    if tokens.len() > 16_384 {
        return Err("selected child exceeds correspondence token bound".into());
    }
    check_token_spelling(&tokens, source)?;
    Ok((tokens, controls))
}

fn plain_pieces(text: &str, base: usize) -> Vec<LocatedPiece> {
    text.char_indices()
        .map(|(at, ch)| LocatedPiece {
            start: base + at,
            end: base + at + ch.len_utf8(),
            semantic: ch.to_string(),
            control: None,
        })
        .collect()
}

fn native_child(
    root: &Path,
    revision: &str,
    unit: &str,
    document: &str,
    language: &str,
    index: usize,
    before: bool,
) -> Result<Value, String> {
    let checked = crate::load_unit_files(root).map_err(|error| error.to_string())?;
    let owner = checked
        .units()
        .iter()
        .find(|entry| entry.label().as_str() == unit && entry.document() == document)
        .ok_or_else(|| format!("native child {unit} has no checked owner"))?;
    let checked_text = owner
        .texts()
        .get(language)
        .ok_or_else(|| format!("native child {unit} has no {language} text"))?;
    if !owner.math().is_empty() {
        return Err(format!(
            "native child {unit} is outside the plain paragraph profile"
        ));
    }
    let path = checked_text.site().file.as_str();
    let (bytes, blob) = captured(root, revision, path)?;
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let tree = toml::parse(path, text).map_err(|error| error.to_string())?;
    toml::numeric(&tree, text).map_err(|error| error.to_string())?;
    let root_fields = toml::fixed(&tree, &["schema", "unit"]).map_err(|error| error.to_string())?;
    if toml::raw(root_fields[0], text) != "1" {
        return Err("native child schema changed".into());
    }
    let unit_fields = toml::fixed(root_fields[1], &["id", "document", "math", "texts"])
        .map_err(|error| error.to_string())?;
    let id = toml::string(unit_fields[0])
        .map_err(|error| error.to_string())?
        .value;
    if id != unit {
        return Err("native child identity changed".into());
    }
    let document = toml::string(unit_fields[1])
        .map_err(|error| error.to_string())?
        .value;
    if document != owner.document() {
        return Err("native child document changed".into());
    }
    if !toml::typed(unit_fields[2], ShapeKind::Table)
        .map_err(|error| error.to_string())?
        .fields
        .is_empty()
    {
        return Err("native child mathematics changed".into());
    }
    let native_text = field(unit_fields[3], language)?;
    let (start, end, value) = plain_field(native_text, text)?;
    if value != checked_text.text() {
        return Err("native child decoded text changed".into());
    }
    let base = if before { start } else { 0 };
    let (tokens, controls) = tokenize(
        &plain_pieces(&value, base),
        if before { text } else { &value },
    )?;
    Ok(json!({"kind":"existing-native-inclusion","index":index,
        "origin":origin(path,&blob,&bytes,start,end,Some(format!("unit.texts.{language}").as_str())),
        "tokens":tokens,"controls":controls}))
}

fn source_accounting(source: &SourceDraft, start: usize, end: usize) -> Result<Vec<Value>, String> {
    let mut accounting = Vec::new();
    let mut cursor = start;
    for piece in source
        .accounting
        .iter()
        .filter(|piece| piece.start_byte >= start && piece.start_byte < end)
    {
        if piece.start_byte != cursor || piece.end_byte > end {
            return Err("selected source accounting has a gap or overlap".into());
        }
        let kind = match piece.owner {
            AccountOwner::Content(_) | AccountOwner::ExistingNative(_) => "block",
            AccountOwner::Event(index) => {
                if source.events[index].kind == "lexical-trivia" {
                    "separator"
                } else {
                    "event"
                }
            }
            AccountOwner::Refusal(_) => {
                return Err("selected source has a refused byte range".into());
            }
        };
        accounting.push(json!({"kind":kind,"range":[piece.start_byte,piece.end_byte]}));
        cursor = piece.end_byte;
    }
    if cursor != end {
        return Err("selected source accounting ends early".into());
    }
    Ok(accounting)
}

struct BeforeHeading {
    pieces: Vec<LocatedPiece>,
    controls: Vec<Value>,
    label_controls: Vec<Value>,
}

fn before_heading(
    source: &SourceDraft,
    request: &Assignment,
    text: &str,
    start: usize,
    end: usize,
) -> Result<BeforeHeading, String> {
    let prefix = "\\section{";
    if !text[start..end].starts_with(prefix) {
        return Err("heading syntax changed".into());
    }
    let title_start = start + prefix.len();
    let close = text[title_start..end]
        .find('}')
        .ok_or("heading close is missing")?
        + title_start;
    let pieces = located_inline(&request.source, text, title_start, close)?;
    let controls = vec![
        control("section-open", start, title_start),
        control("section-close", close, close + 1),
    ];
    let targets: Vec<_> = source
        .metadata_targets
        .iter()
        .filter(|target| target.site.start_byte >= start && target.site.end_byte <= end)
        .collect();
    let [target] = targets.as_slice() else {
        return Err("heading has no unique checked label".into());
    };
    let label_start = target.site.start_byte;
    let label = format!("\\label{{{}}}", request.unit);
    if target.exact_key != request.unit
        || target.selector != request.unit
        || target.source_command != "\\label"
        || text.get(label_start..target.site.end_byte) != Some(label.as_str())
    {
        return Err("heading label changed".into());
    }
    let label_controls = vec![
        control("label-open", label_start, label_start + 7),
        control(
            "label-value",
            label_start + 7,
            label_start + 7 + request.unit.len(),
        ),
        control(
            "label-close",
            label_start + 7 + request.unit.len(),
            target.site.end_byte,
        ),
    ];
    Ok(BeforeHeading {
        pieces,
        controls,
        label_controls,
    })
}

fn before_blocks(
    root: &Path,
    revision: &str,
    bytes: &[u8],
    blob: &str,
    request: &Assignment,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    captured(root, revision, "generated/documents/inclusions.tsv")?;
    let inclusions = input::import_inclusions(root).map_err(|error| error.to_string())?;
    let markers = inclusions
        .iter()
        .filter(|row| row.source == request.source)
        .map(|row| (row.unit.clone(), row.role.clone()))
        .collect::<Vec<_>>();
    let source = import_source(&request.source, bytes, digest(bytes), &markers)
        .map_err(|error| error.to_string())?;
    let (selected_start, selected_end) = selection(request, bytes, true)?;
    let selected: Vec<_> = source
        .blocks
        .iter()
        .filter(|block| {
            block.site.start_byte >= selected_start && block.site.start_byte < selected_end
        })
        .collect();
    if selected.len() != request.blocks.len() {
        return Err("imported block count differs from assignment".into());
    }
    let mut blocks = Vec::new();
    for (index, (selected, draft)) in request.blocks.iter().zip(selected).enumerate() {
        if draft.status != BlockStatus::Decoded
            || draft.site.start_line != selected.first
            || draft.site.end_line > selected.last
            || selected.kind
                != serde_json::to_value(&draft.kind)
                    .map_err(|error| error.to_string())?
                    .as_str()
                    .ok_or("imported block kind is not a string")?
        {
            return Err(format!(
                "source block {index} differs from its selected lines or kind"
            ));
        }
        if let BlockKind::ExistingNativeInclusion = draft.kind {
            let ftoo_manifest::corpus::BlockFields::NativeInclusion { unit, role, .. } =
                &draft.fields
            else {
                return Err(format!("source child {index} has no native marker"));
            };
            if role != "paragraph" {
                return Err(format!("source child {index} is not a paragraph use"));
            }
            blocks.push(native_child(
                root,
                revision,
                unit,
                "explainer-pilot",
                "en",
                index,
                true,
            )?);
            continue;
        }
        let start = draft.site.start_byte;
        let end = if index == 0 {
            line_start(
                bytes,
                selected
                    .last
                    .checked_add(1)
                    .ok_or("source line overflows")?,
            )
            .unwrap_or(bytes.len())
        } else {
            draft.site.end_byte
        };
        let BeforeHeading {
            pieces,
            mut controls,
            label_controls,
        } = if index == 0 {
            before_heading(&source, request, text, start, end)?
        } else {
            BeforeHeading {
                pieces: located_inline(&request.source, text, start, end)?,
                controls: Vec::new(),
                label_controls: Vec::new(),
            }
        };
        let (tokens, inline_controls) = tokenize(&pieces, text)?;
        controls.extend(inline_controls);
        controls.extend(label_controls);
        blocks.push(json!({"kind":selected.kind,"index":index,
            "origin":origin(&request.source,blob,bytes,start,end,None),
            "tokens":tokens,"controls":controls}));
    }
    Ok((
        blocks,
        source_accounting(&source, selected_start, selected_end)?,
    ))
}

fn native_text(node: &toml::Node, text: &str, buffer: &mut String) -> Result<(), String> {
    let kind = toml::string(field(node, "kind")?)
        .map_err(|error| error.to_string())?
        .value;
    match kind.as_str() {
        "text" => buffer.push_str(&plain_field(field(node, "decoded")?, text)?.2),
        "space" => {
            let value = toml::string(field(node, "space")?)
                .map_err(|error| error.to_string())?
                .value;
            if value != "en-space" {
                return Err("unsupported native spacing".into());
            }
            buffer.push(' ');
        }
        "style" => {
            let children = field(node, "children")?;
            for child in &children.elements {
                native_text(child, text, buffer)?;
            }
        }
        _ => return Err("unsupported native run".into()),
    }
    Ok(())
}

fn append_native_runs(runs: &[SlotRun], out: &mut String) {
    for run in runs {
        match run {
            SlotRun::Text(text) => out.push_str(text),
            SlotRun::EnSpace => out.push(' '),
            SlotRun::Style { children, .. } => append_native_runs(children, out),
        }
    }
}

fn after_blocks(
    root: &Path,
    revision: &str,
    request: &Assignment,
    parent_bytes: &[u8],
) -> Result<Vec<Value>, String> {
    captured(root, revision, "generated/documents/inclusions.tsv")?;
    let inclusions = input::import_inclusions(root).map_err(|error| error.to_string())?;
    let selected: Vec<_> = inclusions
        .iter()
        .filter(|row| {
            row.source == request.source && row.unit == request.unit && row.role == "corpus-slot"
        })
        .collect();
    if selected.len() != 1 || selected[0].line != request.first {
        return Err("candidate slot has no unique declared physical occurrence".into());
    }
    let units = crate::load_unit_files(root).map_err(|error| error.to_string())?;
    let (slots, _) =
        crate::documents::corpus::load(root, &units).map_err(|error| error.to_string())?;
    let checked = slots
        .get(&request.unit)
        .ok_or("candidate slot has no checked native owner")?;
    if checked.document() != "explainer-pilot" || checked.language() != "en" {
        return Err("candidate slot document or language differs".into());
    }
    let slot_path = checked.source();
    let (bytes, blob) = captured(root, revision, slot_path)?;
    let child_markers = checked
        .blocks()
        .iter()
        .filter_map(|block| match block {
            SlotBlock::Use { unit, role } => Some((unit.clone(), role.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    let inherited = import_source(
        &request.source,
        parent_bytes,
        digest(parent_bytes),
        &child_markers,
    )
    .map_err(|error| error.to_string())?;
    check_transfer(
        checked,
        &inherited,
        &request.source,
        parent_bytes,
        request.first,
        request.last,
    )
    .map_err(|error| format!("parent interval transfer refuses: {error}"))?;
    if checked.blocks().len() != request.blocks.len() {
        return Err("native slot identity or order changed".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let tree = toml::parse(slot_path, text).map_err(|error| error.to_string())?;
    let nodes = &field(field(&tree, "unit")?, "blocks")?.elements;
    let mut blocks = Vec::new();
    for (index, (block, node)) in checked.blocks().iter().zip(nodes).enumerate() {
        if let SlotBlock::Use { unit, role } = block {
            if role != "paragraph" {
                return Err(format!("native child {index} has a nonparagraph role"));
            }
            blocks.push(native_child(
                root,
                revision,
                unit,
                checked.document(),
                checked.language(),
                index,
                false,
            )?);
            continue;
        }
        let (runs, field_name) = match block {
            SlotBlock::Heading { title, .. } if index == 0 => (title, "title"),
            SlotBlock::Paragraph { runs } if index >= 1 => (runs, "runs"),
            _ => return Err(format!("native block {index} changed kind")),
        };
        let run_node = field(node, field_name)?;
        let mut value = String::new();
        for child in &run_node.elements {
            native_text(child, text, &mut value)?;
        }
        let mut semantic = String::new();
        append_native_runs(runs, &mut semantic);
        if value != semantic {
            return Err(format!(
                "native block {index} locator differs from its parsed runs"
            ));
        }
        let (start, end) = span(&run_node.site.anchor)?;
        let (tokens, controls) = tokenize(&plain_pieces(&value, 0), &value)?;
        blocks.push(json!({"kind":request.blocks[index].kind,"index":index,
            "origin":origin(slot_path,&blob,&bytes,start,end,Some(format!("unit.blocks[{index}].{field_name}").as_str())),
            "tokens":tokens,"controls":controls}));
    }
    Ok(blocks)
}

#[allow(clippy::too_many_lines)] // Each captured side has a distinct source owner.
fn result_blocks(
    root: &Path,
    revision: &str,
    request: &Assignment,
    parent_bytes: &[u8],
    parent_blob: &str,
    before: bool,
) -> Result<(Vec<Value>, Vec<Value>, Value), String> {
    use ftoo_manifest::corpus::{BlockFields, ResultHeader, ResultPartKind};
    let text = std::str::from_utf8(parent_bytes).map_err(|error| error.to_string())?;
    let inclusions = input::import_inclusions(root).map_err(|error| error.to_string())?;
    let markers = inclusions
        .iter()
        .filter(|row| row.source == request.source)
        .map(|row| (row.unit.clone(), row.role.clone()))
        .collect::<Vec<_>>();
    let source = import_source(
        &request.source,
        parent_bytes,
        digest(parent_bytes),
        &markers,
    )
    .map_err(|error| error.to_string())?;
    let (first, end) = selection(request, parent_bytes, true)?;
    let selected: Vec<_> = source
        .blocks
        .iter()
        .filter(|block| block.site.start_byte >= first && block.site.start_byte < end)
        .collect();
    let [draft] = selected.as_slice() else {
        return Err("selected result is not one imported outer block".into());
    };
    if draft.status != BlockStatus::Decoded
        || draft.kind != BlockKind::TheoremContainer
        || draft.site.start_line != request.first
        || draft.site.end_line > request.last
    {
        return Err("selected result differs from its imported interval".into());
    }
    let BlockFields::Result {
        result_kind,
        header: ResultHeader::Registered { key, .. },
        grade,
        parts,
    } = &draft.fields
    else {
        return Err("selected result lacks a registered header".into());
    };
    let [child] = draft.children.as_slice() else {
        return Err("selected result requires one admitted plain child".into());
    };
    if child.kind != BlockKind::Paragraph || child.status != BlockStatus::Decoded {
        return Err("selected result child is not admitted plain prose".into());
    }
    let gap = parts
        .iter()
        .any(|part| part.kind == ResultPartKind::GradeGap);
    if before {
        let metadata = json!({"kind":result_kind.name(),"key":key,
            "grade":grade.as_ref().map(|value| value.code.name()),"grade_gap":gap});
        let start = child.site.start_byte;
        let stop = child.site.end_byte;
        let pieces = located_inline(&request.source, text, start, stop)?;
        let (tokens, controls) = tokenize(&pieces, text)?;
        let block = json!({"kind":"paragraph","index":0,
            "origin":origin(&request.source,parent_blob,parent_bytes,start,stop,None),
            "tokens":tokens,"controls":controls});
        return Ok((
            vec![block],
            source_accounting(&source, first, end)?,
            metadata,
        ));
    }
    let units = crate::load_unit_files(root).map_err(|error| error.to_string())?;
    let (slots, _) =
        crate::documents::corpus::load(root, &units).map_err(|error| error.to_string())?;
    let checked = slots
        .get(&request.unit)
        .ok_or("candidate result has no checked native owner")?;
    if checked.document() != "paper-pilot" || checked.language() != "en" {
        return Err("candidate result document or language differs".into());
    }
    check_transfer(
        checked,
        &source,
        &request.source,
        parent_bytes,
        request.first,
        request.last,
    )
    .map_err(|error| format!("parent result transfer refuses: {error}"))?;
    let [
        SlotBlock::Result {
            kind,
            key: native_key,
            grade: native_grade,
            grade_gap,
            children,
            ..
        },
    ] = checked.blocks()
    else {
        return Err("native result block differs".into());
    };
    if kind != result_kind
        || native_key != key
        || *native_grade != grade.as_ref().map(|value| value.code)
        || *grade_gap != gap
        || children.len() != 1
    {
        return Err("native result header or child count differs".into());
    }
    let metadata = json!({"kind":kind.name(),"key":native_key,
        "grade":native_grade.as_ref().map(|value| value.name()),"grade_gap":grade_gap});
    let slot_path = checked.source();
    let (bytes, blob) = captured(root, revision, slot_path)?;
    let native = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let tree = toml::parse(slot_path, native).map_err(|error| error.to_string())?;
    let [node] = field(field(&tree, "unit")?, "blocks")?.elements.as_slice() else {
        return Err("native result has more than one block".into());
    };
    let [child_node] = field(node, "children")?.elements.as_slice() else {
        return Err("native result has more than one child".into());
    };
    let run_node = field(child_node, "runs")?;
    let mut value = String::new();
    for run in &run_node.elements {
        native_text(run, native, &mut value)?;
    }
    let mut expected = String::new();
    append_native_runs(&children[0], &mut expected);
    if value != expected {
        return Err("native result child locator differs from checked runs".into());
    }
    let (start, stop) = span(&run_node.site.anchor)?;
    let (tokens, controls) = tokenize(&plain_pieces(&value, 0), &value)?;
    let block = json!({"kind":"paragraph","index":0,
        "origin":origin(slot_path,&blob,&bytes,start,stop,Some("unit.blocks[0].children[0].runs")),
        "tokens":tokens,"controls":controls});
    let (marker_start, marker_end) = selection(
        request,
        &fs::read(root.join(&request.source)).map_err(|error| error.to_string())?,
        false,
    )?;
    Ok((
        vec![block],
        vec![json!({"kind":"block","range":[marker_start,marker_end]})],
        metadata,
    ))
}

/// Check that a selected native child retains the same authored bytes.
fn unchanged_native_children(root: &Path, request: &Assignment) -> Result<(), String> {
    let units = crate::load_unit_files(root).map_err(|error| error.to_string())?;
    let (slots, _) =
        crate::documents::corpus::load(root, &units).map_err(|error| error.to_string())?;
    let slot = slots
        .get(&request.unit)
        .ok_or("candidate slot has no checked owner")?;
    for child in slot.blocks() {
        if let SlotBlock::Use { unit, .. } = child {
            let owner = units
                .units()
                .iter()
                .find(|entry| entry.label().as_str() == unit && entry.document() == slot.document())
                .ok_or("native child has no checked owner")?;
            let path = &owner.texts()[slot.language()].site().file;
            let parent = git(root, &["rev-parse", &format!("{}:{path}", request.parent)])?;
            let candidate = git(
                root,
                &["rev-parse", &format!("{}:{path}", request.candidate)],
            )?;
            if parent != candidate {
                return Err(format!(
                    "native child {unit} source changed across transfer"
                ));
            }
        }
    }
    Ok(())
}

/// Export one captured side of a bounded ordered slot.
/// # Errors
/// Refuses changed source blobs, malformed assignments, unsupported syntax and ambiguous output paths.
#[allow(clippy::too_many_lines)] // Both admitted slot forms share one capture boundary.
pub fn export_correspondence(
    root: &Path,
    assignment: &Path,
    side: &str,
    output: &Path,
) -> Result<(), String> {
    if !matches!(side, "before" | "after") {
        return Err("correspondence side must be before or after".into());
    }
    let input = fs::read(assignment).map_err(|error| error.to_string())?;
    if input.len() > 1_048_576 {
        return Err("assignment exceeds byte bound".into());
    }
    let request: Assignment = serde_json::from_slice(&input).map_err(|error| error.to_string())?;
    let source_list = fs::read_to_string(root.join("documents/paper-build/sources.tsv"))
        .map_err(|error| error.to_string())?;
    let result_form = request.blocks.len() == 1 && request.blocks[0].kind == "theorem-container";
    if !(request.source.starts_with("documents/paper/explainer/")
        || result_form && request.source.starts_with("documents/paper/parts/"))
        || !source_list.lines().any(|path| path == request.source)
    {
        return Err("assignment source is outside the declared explainer sources".into());
    }
    for revision in [&request.parent, &request.candidate] {
        if revision.len() != 40
            || !revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err("assignment revision is not a full lowercase commit id".into());
        }
    }
    let revision = if side == "before" {
        &request.parent
    } else {
        &request.candidate
    };
    if git(root, &["rev-parse", "HEAD"])? != *revision {
        return Err("captured checkout revision differs from assignment side".into());
    }
    let parent_blob = git(
        root,
        &[
            "rev-parse",
            &format!("{}:{}", request.parent, request.source),
        ],
    )?;
    if parent_blob != request.source_blob {
        return Err("parent source blob differs from assignment".into());
    }
    let (bytes, blob) = captured(root, revision, &request.source)?;
    let (first, end) = selection(&request, &bytes, side == "before")?;
    let (blocks, accounting, result_metadata) = if result_form {
        let parent_bytes = blob_bytes(root, &parent_blob)?;
        let (blocks, accounting, metadata) = result_blocks(
            root,
            revision,
            &request,
            &parent_bytes,
            &parent_blob,
            side == "before",
        )?;
        (blocks, accounting, Some(metadata))
    } else if side == "before" {
        let (blocks, accounting) = before_blocks(root, revision, &bytes, &blob, &request)?;
        (blocks, accounting, None)
    } else {
        unchanged_native_children(root, &request)?;
        let (blocks, accounting) = (
            after_blocks(root, revision, &request, &blob_bytes(root, &parent_blob)?)?,
            vec![json!({"kind":"block","range":[first,end]})],
        );
        (blocks, accounting, None)
    };
    let selection = json!({"source":request.source,"first":request.first,"last":request.last,"unit":request.unit});
    let mut result = json!({"schema":1,"side":side,"revision":revision,"selection":selection,
        "source-blob":blob,"source-sha256":digest(&bytes),
        "accounting":accounting,"blocks":blocks});
    if let Some(metadata) = result_metadata {
        result["result"] = metadata;
    }
    if fs::read(root.join(&request.source)).map_err(|error| error.to_string())? != bytes {
        return Err("source changed during correspondence export".into());
    }
    if side == "after" {
        let units = crate::load_unit_files(root).map_err(|error| error.to_string())?;
        let (slots, _) =
            crate::documents::corpus::load(root, &units).map_err(|error| error.to_string())?;
        let slot = slots
            .get(&request.unit)
            .ok_or("candidate slot has no checked owner")?;
        captured(root, revision, slot.source())?;
    }
    if fs::read(assignment).map_err(|error| error.to_string())? != input {
        return Err("assignment changed during correspondence export".into());
    }
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(output)
    };
    let parent = output
        .parent()
        .ok_or("output has no parent")?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let output = parent.join(output.file_name().ok_or("output has no name")?);
    if output.starts_with(root) || root.starts_with(&output) {
        return Err("output overlaps captured repository".into());
    }
    fs::create_dir(&output).map_err(|error| error.to_string())?;
    let mut encoded = serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    fs::write(output.join("correspondence.json"), encoded).map_err(|error| error.to_string())
}

/// Read one captured candidate selection for checked screen recovery.
/// # Errors
/// Refuses an unsupported request, changed candidate revision or missing source marker.
pub(crate) fn recovery_selection(
    root: &Path,
    assignment: &Path,
) -> Result<(String, usize, usize, String), String> {
    let input = fs::read(assignment).map_err(|error| error.to_string())?;
    if input.len() > 1_048_576 {
        return Err("assignment exceeds byte bound".into());
    }
    let request: Assignment = serde_json::from_slice(&input).map_err(|error| error.to_string())?;
    let source_list = fs::read_to_string(root.join("documents/paper-build/sources.tsv"))
        .map_err(|error| error.to_string())?;
    let result_form = request.blocks.len() == 1 && request.blocks[0].kind == "theorem-container";
    if !(request.source.starts_with("documents/paper/explainer/")
        || result_form && request.source.starts_with("documents/paper/parts/"))
        || !source_list.lines().any(|path| path == request.source)
    {
        return Err("assignment source is outside the declared explainer sources".into());
    }
    if git(root, &["rev-parse", "HEAD"])? != request.candidate {
        return Err("captured checkout revision differs from assignment candidate".into());
    }
    let bytes = fs::read(root.join(&request.source)).map_err(|error| error.to_string())?;
    selection(&request, &bytes, false)?;
    Ok((request.source, request.first, request.last, request.unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn repo() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ftoo-correspondence-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("documents/paper/explainer")).unwrap();
        fs::create_dir_all(root.join("documents/paper/parts")).unwrap();
        fs::create_dir_all(root.join("documents/paper-build")).unwrap();
        fs::create_dir_all(root.join("units/statements")).unwrap();
        fs::create_dir_all(root.join("units/corpus")).unwrap();
        fs::create_dir_all(root.join("units/scopes")).unwrap();
        fs::create_dir_all(root.join("generated/documents")).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (from, to) in [
            ("units/fixtures/explainer-prefix-source.tex", SOURCE),
            (INTRO, INTRO),
            ("units/fixtures/explainer-plain.toml", SLOT),
            (
                "units/fixtures/explainer-prefix-scope.toml",
                "units/scopes/explainer.toml",
            ),
            ("units/documents.toml", "units/documents.toml"),
        ] {
            fs::copy(source.join(from), root.join(to)).unwrap();
        }
        fs::write(
            root.join("documents/paper/parts/coda.tex"),
            "% generated: sec:fixture:paper\n",
        )
        .unwrap();
        fs::write(
            root.join("documents/paper-build/sources.tsv"),
            format!("{SOURCE}\ndocuments/paper/parts/coda.tex\n"),
        )
        .unwrap();
        fs::write(root.join("units/scopes/coda.toml"),
            "schema = 1\n\n[scope]\ndocument = \"paper-pilot\"\nsource = \"documents/paper/parts/coda.tex\"\n\n[[scope.occurrences]]\nunit = \"sec:fixture:paper\"\nrole = \"paragraph\"\n").unwrap();
        fs::write(root.join("generated/documents/inclusions.tsv"),
            format!("{SOURCE}\tsec:reading:plain-intro\tgenerated/documents/explainer-pilot/sec-reading-plain-intro.tex\tparagraph\t47\n"),
        ).unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.name", "Test"],
            vec!["config", "user.email", "test@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .status()
                .unwrap();
            assert!(status.success());
        }
        root
    }

    fn commit(root: &Path) -> String {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["add", "."])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["commit", "-qm", "fixture"])
                .status()
                .unwrap()
                .success()
        );
        git(root, &["rev-parse", "HEAD"]).unwrap()
    }

    fn assignment(root: &Path, parent: &str, candidate: &str) -> PathBuf {
        let value = json!({
            "schema":4,"form":"coarse-slot","parent":parent,"candidate":candidate,
            "source":SOURCE,"source-blob":git(root,&["rev-parse",&format!("{parent}:{SOURCE}")]).unwrap(),
            "first":44,"last":98,"unit":"sec:explainer:plain",
            "blocks":[
                {"kind":"heading","first":44,"last":46},
                {"kind":"existing-native-inclusion","first":47,"last":48},
                {"kind":"paragraph","first":49,"last":55},
                {"kind":"paragraph","first":56,"last":68},
                {"kind":"paragraph","first":69,"last":84},
                {"kind":"paragraph","first":85,"last":98}
            ]
        });
        let path = root.join("assignment.json");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        path
    }

    fn candidate_source(root: &Path) {
        let path = root.join(SOURCE);
        let bytes = fs::read(&path).unwrap();
        let start = line_start(&bytes, 44).unwrap();
        let end = line_start(&bytes, 99).unwrap();
        let mut result = bytes[..start].to_vec();
        result.extend_from_slice(b"% generated: sec:explainer:plain\n");
        result.extend_from_slice(&bytes[end..]);
        fs::write(path, result).unwrap();
        fs::write(root.join("generated/documents/inclusions.tsv"),
            format!("{SOURCE}\tsec:explainer:plain\tgenerated/documents/explainer-pilot/sec-explainer-plain.tex\tcorpus-slot\t44\n"),
        ).unwrap();
        let scope = root.join("units/scopes/explainer.toml");
        let original = fs::read_to_string(&scope).unwrap();
        fs::write(
            scope,
            original.replace(
                "unit = \"sec:reading:plain-intro\"\nrole = \"paragraph\"",
                "unit = \"sec:explainer:plain\"\nrole = \"corpus-slot\"",
            ),
        )
        .unwrap();
    }

    fn assert_before_locations(old: &Value, source: &str) {
        assert_eq!(old["schema"], 1);
        assert_eq!(old["blocks"].as_array().unwrap().len(), 6);
        let mut cursor = line_start(source.as_bytes(), 44).unwrap();
        for piece in old["accounting"].as_array().unwrap() {
            let range = piece["range"].as_array().unwrap();
            assert_eq!(range[0].as_u64().unwrap(), u64::try_from(cursor).unwrap());
            cursor = usize::try_from(range[1].as_u64().unwrap()).unwrap();
        }
        assert_eq!(cursor, line_start(source.as_bytes(), 99).unwrap());
        assert!(
            old["accounting"]
                .as_array()
                .unwrap()
                .iter()
                .any(|piece| piece["kind"] == "event")
        );
        for block in old["blocks"].as_array().unwrap() {
            if block["index"] == 1 {
                continue;
            }
            for token in block["tokens"].as_array().unwrap() {
                let range = token["source_range"].as_array().unwrap();
                let start = usize::try_from(range[0].as_u64().unwrap()).unwrap();
                let end = usize::try_from(range[1].as_u64().unwrap()).unwrap();
                assert_eq!(
                    &source[start..end],
                    token["source_spelling"].as_str().unwrap()
                );
            }
        }
        assert!(old.to_string().contains("en-space"));
        assert!(old.to_string().contains("\"source_spelling\":\"---\""));
        let start_word = old["blocks"][2]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|token| token["semantic"] == "Start")
            .unwrap();
        assert_eq!(start_word["separator_before"], true);
        let separator = start_word["separator_range"].as_array().unwrap();
        let a = usize::try_from(separator[0].as_u64().unwrap()).unwrap();
        let b = usize::try_from(separator[1].as_u64().unwrap()).unwrap();
        assert_eq!(&source[a..b], "}\\enspace ");
        let mut wrong_range = old["blocks"][0]["tokens"].as_array().unwrap().clone();
        wrong_range[0]["source_range"][0] = json!(0);
        assert!(
            check_token_spelling(&wrong_range, source)
                .unwrap_err()
                .contains("does not reproduce")
        );
    }

    #[test]
    fn real_prefix_exports_both_captured_sides_and_refuses_changed_origins() {
        let root = repo();
        let parent = commit(&root);
        candidate_source(&root);
        let candidate = commit(&root);
        let request = assignment(&root, &parent, &candidate);
        let before_output = root.parent().unwrap().join(format!(
            "{}-before",
            root.file_name().unwrap().to_string_lossy()
        ));
        let after_output = root.parent().unwrap().join(format!(
            "{}-after",
            root.file_name().unwrap().to_string_lossy()
        ));
        assert!(
            export_correspondence(&root, &request, "before", &before_output)
                .unwrap_err()
                .contains("revision")
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["checkout", "-q", &parent])
                .status()
                .unwrap()
                .success()
        );
        export_correspondence(&root, &request, "before", &before_output).unwrap();
        let old: Value =
            serde_json::from_slice(&fs::read(before_output.join("correspondence.json")).unwrap())
                .unwrap();
        let old_source = fs::read_to_string(root.join(SOURCE)).unwrap();
        assert_before_locations(&old, &old_source);
        let mut wrong_blob: Value = serde_json::from_slice(&fs::read(&request).unwrap()).unwrap();
        wrong_blob["source-blob"] = json!("0".repeat(40));
        let wrong_request = root.join("wrong-blob.json");
        fs::write(&wrong_request, serde_json::to_vec(&wrong_blob).unwrap()).unwrap();
        assert!(
            export_correspondence(&root, &wrong_request, "before", &root.join("bad-blob"))
                .unwrap_err()
                .contains("parent source blob differs")
        );
        fs::write(root.join(SOURCE), b"candidate text").unwrap();
        assert!(
            export_correspondence(&root, &request, "before", &root.join("bad-before"))
                .unwrap_err()
                .contains("captured input differs")
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["checkout", "-q", "-f", &candidate])
                .status()
                .unwrap()
                .success()
        );
        export_correspondence(&root, &request, "after", &after_output).unwrap();
        let new: Value =
            serde_json::from_slice(&fs::read(after_output.join("correspondence.json")).unwrap())
                .unwrap();
        assert_eq!(new["blocks"].as_array().unwrap().len(), 6);
        assert_eq!(new["accounting"].as_array().unwrap().len(), 1);
        assert!(
            new["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|block| block["tokens"].as_array().unwrap())
                .any(|token| {
                    token["source_spelling"] == "—"
                        && token["source_range"][1].as_u64().unwrap()
                            - token["source_range"][0].as_u64().unwrap()
                            == 3
                })
        );
        let path = root.join(SLOT);
        let native = fs::read_to_string(&path).unwrap();
        fs::write(&path, native.replacen("Ties.", "Tie\\u0073.", 1)).unwrap();
        let escaped = commit(&root);
        let escaped_request = assignment(&root, &parent, &escaped);
        assert!(
            export_correspondence(&root, &escaped_request, "after", &root.join("bad-after"))
                .unwrap_err()
                .contains("escaped or unsupported native string")
        );
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(before_output).unwrap();
        fs::remove_dir_all(after_output).unwrap();
    }

    #[test]
    fn after_export_refuses_an_unaccounted_parent_event() {
        let root = repo();
        let path = root.join(SOURCE);
        let source = fs::read_to_string(&path).unwrap();
        let changed = source.replacen(
            "\\label{sec:explainer:plain}",
            "\\label{sec:explainer:plain}\\vspace{1pt}",
            1,
        );
        assert_ne!(changed, source);
        fs::write(&path, changed).unwrap();
        let parent = commit(&root);
        candidate_source(&root);
        let candidate = commit(&root);
        let request = assignment(&root, &parent, &candidate);
        let output = root.parent().unwrap().join(format!(
            "{}-event",
            root.file_name().unwrap().to_string_lossy()
        ));
        let error = export_correspondence(&root, &request, "after", &output).unwrap_err();
        assert!(
            error.contains("parent interval transfer refuses: SourceEvent"),
            "{error}"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
