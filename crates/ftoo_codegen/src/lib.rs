//! Generate and check the complete manifest artifact against Rust authority.
//! Semantic comparison precedes registry admission and exact byte comparison.
mod artifact;
mod documents;
pub use documents::*;
mod notation;
mod support_bracket;
mod telemetry;
pub use notation::{
    AdapterComponent, AdapterOperation, BatchReason, InstalledPackageChange,
    MAX_NOTATION_OUTPUT_BYTES, NotationAdapterFault, NotationRenderError, PendingEntry,
    ProtocolReason, RenderableExpression, RenderableNotation, RenderedExpression, SupportCopy,
    SupportEntryKind, SupportIntegrityFault, SupportIoOperation, SupportMetadataReason,
    SupportReason, SweepRow, prepare_notation,
};
pub use support_bracket::SupportIdentity;
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
