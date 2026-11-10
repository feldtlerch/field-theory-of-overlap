//! Generate and check the complete manifest artifact against Rust authority.
//! Semantic comparison precedes registry admission and exact byte comparison.
mod artifact;
mod labels;
mod projection;
mod units;
mod weld;
pub use labels::{
    LabelCheckSummary, LabelEntryFault, LabelFileError, LabelIoOperation, MarkdownEntryKind,
    check_label_files, load_markdown_files,
};
pub use units::{
    EntryKind, UnitCheckSummary, UnitInputFault, UnitIoOperation, UnitLoadError, UnitSource,
    check_unit_files, load_unit_files, load_unit_sources,
};
pub use weld::{WeldCheckSummary, WeldFileError, check_weld_files};
#[cfg(test)]
mod tests;
pub use artifact::{
    ByteGateFault, IoOperation, ProjectionCheckError, check_projection, generate_projection,
};
