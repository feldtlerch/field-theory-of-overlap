//! Diagnostic whole-source import without admission to document generation.

use crate::documents::input;
use ftoo_manifest::corpus::{
    BlockStatus, CorpusDraft, SourceDraft, ensure_unchanged, import_source,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
mod correspondence;
mod slot;
pub use correspondence::export_correspondence;
pub use slot::parse_slot;
#[cfg(test)]
mod tests;

fn digest(bytes: &[u8]) -> String {
    crate::hex(&Sha256::digest(bytes))
}

#[allow(clippy::case_sensitive_file_extension_comparisons)] // The declared source list uses exact lowercase TeX names.
fn body_path(path: &str) -> bool {
    path == "documents/paper/explainer/explainer.tex"
        || (path.starts_with("documents/paper/parts/") && path.ends_with(".tex"))
}

fn serialize(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn nested_count(block: &ftoo_manifest::corpus::BlockDraft) -> usize {
    block.children.len() + block.children.iter().map(nested_count).sum::<usize>()
}

#[derive(Default)]
struct FaultCounts {
    occurrences: usize,
    blocks: BTreeSet<(String, usize)>,
    solo: BTreeSet<(String, usize)>,
    examples: Vec<Value>,
}

fn all_fault_rows(sources: &[SourceDraft], source_bytes: &BTreeMap<String, Vec<u8>>) -> Value {
    let mut rows: BTreeMap<String, FaultCounts> = BTreeMap::new();
    for source in sources {
        let text = String::from_utf8_lossy(&source_bytes[&source.path]);
        let mut by_block: BTreeMap<usize, BTreeSet<&str>> = BTreeMap::new();
        for refusal in &source.refusals {
            by_block
                .entry(refusal.outer_block_index)
                .or_default()
                .insert(&refusal.code);
        }
        for refusal in &source.refusals {
            let entry = rows.entry(refusal.code.clone()).or_default();
            entry.occurrences += 1;
            let identity = (source.path.clone(), refusal.outer_block_index);
            entry.blocks.insert(identity.clone());
            if by_block
                .get(&refusal.outer_block_index)
                .is_some_and(|codes| codes.len() == 1)
            {
                entry.solo.insert(identity);
            }
            if entry.examples.len() < 3 {
                let line = text
                    .lines()
                    .nth(refusal.site.start_line.saturating_sub(1))
                    .unwrap_or("");
                entry
                    .examples
                    .push(json!({"path":source.path,"line":refusal.site.start_line,"source":line}));
            }
        }
    }
    json!(rows.into_iter().map(|(kind, counts)| {
        (kind, json!({"occurrences":counts.occurrences,"blocks":counts.blocks.len(),"sole_kind_blocks":counts.solo.len(),"examples":counts.examples}))
    }).collect::<BTreeMap<_, _>>())
}

fn residue(corpus: &CorpusDraft, source_bytes: &BTreeMap<String, Vec<u8>>) -> Value {
    let mut files = Vec::new();
    let mut kinds_total: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    let mut refusals_total: BTreeMap<String, (usize, Vec<Value>)> = BTreeMap::new();
    let mut decoded_total = 0usize;
    let mut unresolved_total = 0usize;
    let mut accounted_total = 0usize;
    for source in &corpus.sources {
        let mut kinds: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        let mut decoded = 0usize;
        let mut unresolved = 0usize;
        let mut native = 0usize;
        let mut nested = 0usize;
        for block in &source.blocks {
            let kind = serde_json::to_value(&block.kind)
                .expect("block kind serializes")
                .as_str()
                .expect("block kind string")
                .to_owned();
            let complete = block.status == BlockStatus::Decoded;
            let counts = kinds.entry(kind.clone()).or_default();
            let total = kinds_total.entry(kind).or_default();
            let lines = block.site.end_line.saturating_sub(block.site.start_line) + 1;
            counts.2 += lines;
            total.2 += lines;
            nested += nested_count(block);
            if matches!(
                block.kind,
                ftoo_manifest::corpus::BlockKind::ExistingNativeInclusion
            ) {
                native += 1;
                counts.0 += 1;
                total.0 += 1;
            } else if complete {
                decoded += 1;
                counts.0 += 1;
                total.0 += 1;
            } else {
                unresolved += 1;
                counts.1 += 1;
                total.1 += 1;
            }
        }
        let text = String::from_utf8_lossy(&source_bytes[&source.path]);
        let mut refusal_kinds: BTreeMap<String, usize> = BTreeMap::new();
        for refusal in source.refusals.iter().filter(|refusal| refusal.is_primary) {
            *refusal_kinds.entry(refusal.code.clone()).or_default() += 1;
            let entry = refusals_total.entry(refusal.code.clone()).or_default();
            entry.0 += 1;
            if entry.1.len() < 3 {
                let line = text
                    .lines()
                    .nth(refusal.site.start_line.saturating_sub(1))
                    .unwrap_or("");
                entry
                    .1
                    .push(json!({"path":source.path,"line":refusal.site.start_line,"source":line}));
            }
        }
        let accounted: usize = source
            .accounting
            .iter()
            .map(|piece| piece.end_byte - piece.start_byte)
            .sum();
        accounted_total += accounted;
        decoded_total += decoded;
        unresolved_total += unresolved;
        let kind_rows: BTreeMap<_, _> = kinds.into_iter().map(|(kind, (fully_decoded, refused, source_line_incidence))|
            (kind, json!({"fully_decoded":fully_decoded,"refused":refused,"source_line_incidence":source_line_incidence}))).collect();
        files.push(json!({
            "path":source.path,"byte_length":source.byte_length,"accounted_bytes":accounted,
            "fully_decoded_outer":decoded,"remaining_outer":unresolved,"existing_native_inclusions":native,
            "nested_occurrences":nested,"unresolved_mathematics":source.math.len(),"kinds":kind_rows,
            "refusal_kinds":refusal_kinds,"first_fault_kinds":refusal_kinds.keys().collect::<Vec<_>>(),
            "all_faults":all_fault_rows(std::slice::from_ref(source), source_bytes)
        }));
    }
    let kind_rows: BTreeMap<_, _> = kinds_total.into_iter().map(|(kind, (fully_decoded, refused, source_line_incidence))|
        (kind, json!({"fully_decoded":fully_decoded,"refused":refused,"source_line_incidence":source_line_incidence}))).collect();
    json!({
        "schema":1,"source_list_digest":corpus.source_list_digest,"source_count":corpus.sources.len(),
        "fully_decoded_outer":decoded_total,"remaining_outer":unresolved_total,
        "kinds":kind_rows,"refusal_kinds":refusals_total,"all_faults":all_fault_rows(&corpus.sources, source_bytes),
        "accounted_bytes":accounted_total,"source_bytes":source_bytes.values().map(Vec::len).sum::<usize>(),
        "transfer_ready":false,"files":files
    })
}

/// Read the declared body files, validate source ownership and write diagnostic drafts.
/// # Errors
/// Refuses missing or changed inputs, invalid composition, accounting and unsafe output paths.
/// # Panics
/// Panics only if an internally constructed block kind fails its serde shape, which input bytes cannot arrange.
pub fn dry_run(root: &Path, output: &Path) -> Result<Value, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(output)
    };
    if output.exists() {
        return Err("output directory already exists".into());
    }
    let parent = output
        .parent()
        .ok_or("output directory has no parent")?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let name = output.file_name().ok_or("output directory has no name")?;
    if name == "." || name == ".." {
        return Err("output directory has no usable name".into());
    }
    let output = parent.join(name);
    if output.starts_with(&root) || root.starts_with(&output) {
        return Err("output overlaps the source tree".into());
    }
    let source_list_path = root.join("documents/paper-build/sources.tsv");
    let source_list = fs::read(&source_list_path).map_err(|error| error.to_string())?;
    let list = std::str::from_utf8(&source_list).map_err(|error| error.to_string())?;
    let source_order: Vec<_> = list
        .lines()
        .filter(|path| body_path(path))
        .map(str::to_owned)
        .collect();
    if source_order.len() != 12 || source_order.iter().collect::<BTreeSet<_>>().len() != 12 {
        return Err("declared body corpus must contain twelve distinct sources".into());
    }
    let inclusions = input::import_inclusions(&root).map_err(|error| error.to_string())?;
    let mut sources = Vec::new();
    let mut source_bytes = BTreeMap::new();
    for path in &source_order {
        let bytes = fs::read(root.join(path)).map_err(|error| format!("{path}: {error}"))?;
        let markers: Vec<_> = inclusions
            .iter()
            .filter(|row| &row.source == path)
            .map(|row| (row.unit.clone(), row.role.clone()))
            .collect();
        let draft: SourceDraft = import_source(path, &bytes, digest(&bytes), &markers)
            .map_err(|error| error.to_string())?;
        source_bytes.insert(path.clone(), bytes);
        sources.push(draft);
    }
    if fs::read(&source_list_path).map_err(|error| error.to_string())? != source_list {
        return Err("declared source list changed during import".into());
    }
    for (path, bytes) in &source_bytes {
        let current = fs::read(root.join(path)).map_err(|error| error.to_string())?;
        ensure_unchanged(path, bytes, &current).map_err(|error| error.to_string())?;
    }
    let corpus = CorpusDraft {
        schema: 1,
        source_order,
        source_list_digest: digest(&source_list),
        sources,
        transfer_ready: false,
    };
    let report = residue(&corpus, &source_bytes);
    fs::create_dir(&output).map_err(|error| error.to_string())?;
    serialize(&output.join("draft.json"), &corpus)?;
    serialize(&output.join("residue.json"), &report)?;
    Ok(report)
}
