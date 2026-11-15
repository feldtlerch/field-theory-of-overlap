//! Reuse documentary admission with the paper's label-mint evidence.
use super::{Result, bytes};
use std::path::Path;

pub(super) fn check(root: &Path) -> Result<()> {
    crate::check_projection(root).map_err(|e| e.to_string())?;
    let projection = bytes(&root.join("generated/manifest.json"))?;
    let graph = ftoo_manifest::load_projection(&projection).map_err(|e| e.to_string())?;
    let book = graph.addressing().map_err(|e| e.to_string())?;
    let corpus = crate::load_unit_files(root).map_err(|e| e.to_string())?;
    let markdown = crate::load_markdown_files(root).map_err(|e| e.to_string())?;
    let joint =
        ftoo_manifest::collect_joint_mints(&corpus, &markdown).map_err(|e| e.to_string())?;
    let data = bytes(&root.join("documents/paper-build/label-mints.tsv"))?;
    let rows = crate::documents::registry::paper_mint_rows(&data).map_err(|e| e.to_string())?;
    let joint = joint.with_paper_mints(rows).map_err(|e| e.to_string())?;
    let source = crate::documents::registry::read(root).map_err(|e| e.to_string())?;
    let events = crate::documents::registry::load(&source).map_err(|e| e.to_string())?;
    book.with_documentary(&joint, events)
        .map_err(|e| e.to_string())?;
    Ok(())
}
