//! Two derived forms, admitted only with the actual grammar and complete sweep.
// Large typed evidence retains original bytes and paths without lossy boxing seams.
#![allow(clippy::result_large_err)]
mod katex;
mod render;
mod support;
mod sweep;
#[cfg(test)]
mod tests;

use ftoo_manifest::{
    CheckedExpression, CheckedNotation, NotationDiagnostic, NotationErrors, NotationFault,
    NotationOrigin, NotationSite, NotationStage, RenderOwner, RenderTarget, RuleId,
    SymbolLookupFault, SymbolUse,
};
use std::{fmt, io, path::PathBuf};
pub use support::{
    InstalledPackageChange, SupportCopy, SupportEntryKind, SupportIntegrityFault,
    SupportIoOperation, SupportMetadataReason,
};
pub use sweep::SweepRow;

pub const MAX_NOTATION_OUTPUT_BYTES: usize = 262_144;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterComponent {
    Runtime,
    KaTeX,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterOperation {
    ReadRuntimePin,
    Spawn,
    WriteInput,
    WaitOutput,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolReason {
    InvalidUtf8,
    UnexpectedStatus,
    UnexpectedStderr,
    EmptyOutput,
    MissingMathml,
    InvalidVersionResponse,
    SetupFailure,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportReason {
    Missing,
    Relative,
    RepositoryOverlap,
    EscapingEntry {
        component: AdapterComponent,
        resolved: PathBuf,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotationAdapterFault {
    SupportConfiguration {
        value: Option<PathBuf>,
        reason: SupportReason,
    },
    Integrity(SupportIntegrityFault),
    Unavailable {
        component: AdapterComponent,
        kind: io::ErrorKind,
    },
    VersionMismatch {
        component: AdapterComponent,
        expected: String,
        actual: String,
    },
    Io {
        operation: AdapterOperation,
        kind: io::ErrorKind,
    },
    Protocol {
        status: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        reason: ProtocolReason,
    },
}
impl fmt::Display for NotationAdapterFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotationRenderError {
    Admission(NotationErrors),
    Adapter(NotationAdapterFault),
    Lookup(SymbolLookupFault),
}
impl fmt::Display for NotationRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(e) => e.fmt(f),
            Self::Adapter(_) | Self::Lookup(_) => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for NotationRenderError {}
impl From<NotationAdapterFault> for NotationRenderError {
    fn from(e: NotationAdapterFault) -> Self {
        Self::Adapter(e)
    }
}
impl From<NotationErrors> for NotationRenderError {
    fn from(e: NotationErrors) -> Self {
        Self::Admission(e)
    }
}
type Result<T> = std::result::Result<T, NotationRenderError>;
fn admission(
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) -> NotationRenderError {
    NotationRenderError::Admission(
        NotationErrors::from_diagnostics(vec![NotationDiagnostic {
            rule,
            stage,
            site,
            fault,
        }])
        .expect("one diagnostic"),
    )
}
fn rendered_site(
    owner: RenderOwner,
    case: &str,
    source: &str,
    origins: Vec<NotationOrigin>,
) -> NotationSite {
    NotationSite::Rendered {
        owner,
        case: case.into(),
        bytes: 0..source.len(),
        origins,
    }
}
fn typeset(
    support: &support::Support,
    target: RenderTarget,
    source: &str,
    site: NotationSite,
) -> Result<()> {
    if let Err(message) = katex::invoke(support, Some(source))? {
        return Err(admission(
            RuleId::M051,
            NotationStage::Typesetting,
            site,
            NotationFault::KaTeXRejected {
                target,
                source: source.into(),
                message,
            },
        ));
    }
    Ok(())
}

/// The only declaration/export surface; construction requires every sweep row.
/// ```compile_fail,E0616
/// fn alter(n: &mut ftoo_codegen::RenderableNotation<'_, '_, '_>) { n.declarations.clear(); }
/// ```
/// ```compile_fail,E0599
/// let n = ftoo_codegen::RenderableNotation::default();
/// ```
/// ```compile_fail,E0277
/// let n: ftoo_codegen::RenderableNotation<'_, '_, '_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0451
/// fn forge(n: ftoo_codegen::RenderableNotation<'_, '_, '_>) {
///     let _ = ftoo_codegen::RenderableNotation { declarations: String::new(), ..n };
/// }
/// ```
/// ```compile_fail,E0515
/// fn escape(n: &ftoo_manifest::CheckedNotation<'_, '_>) -> ftoo_codegen::RenderedExpression<'static, 'static, 'static, 'static> {
///     let prepared = ftoo_codegen::prepare_notation(n).unwrap();
///     let e = prepared.check_expression(ftoo_manifest::ExprDraft::Literal { value: "x".into() }).unwrap();
///     e.render(ftoo_manifest::RenderTarget::Unicode).unwrap()
/// }
/// ```
#[derive(Debug)]
pub struct RenderableNotation<'catalogue, 'book, 'graph> {
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
    support: support::Support,
    declarations: String,
    rows: Vec<SweepRow>,
}
impl<'catalogue, 'book, 'graph> RenderableNotation<'catalogue, 'book, 'graph> {
    #[must_use]
    pub fn declarations(&self) -> &str {
        &self.declarations
    }
    #[must_use]
    pub fn sweep_rows(&self) -> &[SweepRow] {
        &self.rows
    }
    #[must_use]
    pub fn typesetting_calls(&self) -> usize {
        self.rows.len() * 2
    }
    #[must_use]
    pub const fn catalogue(&self) -> &'catalogue CheckedNotation<'book, 'graph> {
        self.catalogue
    }
    /// Bind a closed expression to this exact renderable catalogue.
    /// # Errors
    /// Returns the unchanged semantic admission faults for an invalid draft.
    pub fn check_expression<'a>(
        &'a self,
        draft: ftoo_manifest::ExprDraft,
    ) -> Result<RenderableExpression<'a, 'catalogue, 'book, 'graph>> {
        let expression = ftoo_manifest::check_expression(self.catalogue, draft)?;
        Ok(RenderableExpression {
            notation: self,
            expression,
        })
    }
    fn render<'a>(
        &'a self,
        expression: &CheckedExpression<'catalogue, 'book, 'graph>,
        target: RenderTarget,
    ) -> Result<RenderedExpression<'a, 'catalogue, 'book, 'graph>> {
        let expanded = render::expression(
            self.catalogue,
            expression.draft(),
            RenderTarget::Unicode,
            RenderOwner::Expression,
            "expression",
        )?;
        typeset(
            &self.support,
            RenderTarget::Unicode,
            &expanded.source,
            expanded.site(RenderOwner::Expression, "expression"),
        )?;
        let source = match target {
            RenderTarget::Unicode => expanded.source,
            RenderTarget::Macro => {
                let result = render::expression(
                    self.catalogue,
                    expression.draft(),
                    target,
                    RenderOwner::Expression,
                    "expression",
                )?;
                let combined = render::combine(
                    &self.declarations,
                    &result.source,
                    result.site(RenderOwner::Expression, "expression"),
                )?;
                typeset(
                    &self.support,
                    target,
                    &combined,
                    rendered_site(
                        RenderOwner::Expression,
                        "expression",
                        &combined,
                        expanded.origins,
                    ),
                )?;
                result.source
            }
        };
        Ok(RenderedExpression {
            notation: self,
            source,
            uses: expression.uses().to_vec(),
        })
    }
}

/// An admitted expression that selects its own renderable catalogue; callers
/// cannot pass it to another catalogue or substitute a cached status.
/// ```compile_fail,E0616
/// fn change(e: &mut ftoo_codegen::RenderableExpression<'_, '_, '_, '_>) { e.expression = todo!(); }
/// ```
#[derive(Debug)]
pub struct RenderableExpression<'a, 'catalogue, 'book, 'graph> {
    notation: &'a RenderableNotation<'catalogue, 'book, 'graph>,
    expression: CheckedExpression<'catalogue, 'book, 'graph>,
}
impl<'a, 'catalogue, 'book, 'graph> RenderableExpression<'a, 'catalogue, 'book, 'graph> {
    /// # Errors
    /// Refuses output limits, changed support, adapter failures and grammar rejection.
    pub fn render(
        &self,
        target: RenderTarget,
    ) -> Result<RenderedExpression<'a, 'catalogue, 'book, 'graph>> {
        self.notation.render(&self.expression, target)
    }
}

/// Returned bytes have passed the pinned grammar under retained integrity.
/// ```compile_fail,E0616
/// fn alter(e: &mut ftoo_codegen::RenderedExpression<'_, '_, '_, '_>) { e.source.clear(); }
/// ```
/// ```compile_fail,E0277
/// let e: ftoo_codegen::RenderedExpression<'_, '_, '_, '_> = String::new().into();
/// ```
/// ```compile_fail,E0599
/// let e = ftoo_codegen::RenderedExpression::default();
/// ```
/// ```compile_fail,E0277
/// let e: ftoo_codegen::RenderedExpression<'_, '_, '_, '_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0624
/// fn cross(a: &ftoo_codegen::RenderableNotation<'_, '_, '_>, b: &ftoo_manifest::CheckedExpression<'_, '_, '_>) {
///     a.render(b, ftoo_manifest::RenderTarget::Unicode).unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct RenderedExpression<'a, 'catalogue, 'book, 'graph> {
    notation: &'a RenderableNotation<'catalogue, 'book, 'graph>,
    source: String,
    uses: Vec<SymbolUse>,
}
impl<'a, 'catalogue, 'book, 'graph> RenderedExpression<'a, 'catalogue, 'book, 'graph> {
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    #[must_use]
    pub fn uses(&self) -> &[SymbolUse] {
        &self.uses
    }
    #[must_use]
    pub const fn notation(&self) -> &'a RenderableNotation<'catalogue, 'book, 'graph> {
        self.notation
    }
}

/// Admit declarations, collisions and the entire generated invocation sweep.
/// # Errors
/// Missing support requires separate preparation; this function never installs,
/// fetches, repairs, caches external integrity or skips a missing prerequisite.
pub fn prepare_notation<'catalogue, 'book, 'graph>(
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
) -> Result<RenderableNotation<'catalogue, 'book, 'graph>> {
    let location = std::env::var_os("FTOO_NOTATION_SUPPORT").map(PathBuf::from);
    let support = support::resolve(location.as_deref())?;
    if let Err(message) = katex::invoke(&support, None)? {
        return Err(NotationAdapterFault::Protocol {
            status: Some(2),
            stdout: message.into_bytes(),
            stderr: vec![],
            reason: ProtocolReason::UnexpectedStatus,
        }
        .into());
    }
    let records = render::declarations(catalogue)?;
    sweep::inventory(catalogue, &records)?;
    sweep::namespace(&support, &records, "")?;
    let declarations = records
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let rows = sweep::run(catalogue, &support, &declarations)?;
    Ok(RenderableNotation {
        catalogue,
        support,
        declarations,
        rows,
    })
}
