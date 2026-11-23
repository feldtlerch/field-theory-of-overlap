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
    CheckedExpression, CheckedNotation, DefinitionId, NotationDiagnostic, NotationErrors,
    NotationFault, NotationOrigin, NotationSite, NotationStage, PendingReason, RenderOwner,
    RenderTarget, RuleId, SymbolLookupFault, SymbolUse,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolReason {
    InvalidUtf8,
    UnexpectedStatus,
    UnexpectedStderr,
    EmptyOutput,
    MissingMathml,
    InvalidVersionResponse,
    SetupFailure,
    BatchFrame { index: usize, reason: BatchReason },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchReason {
    TruncatedHeader,
    TruncatedPayload,
    WrongIndex { expected: u64, actual: u64 },
    UnknownStatus { value: u8 },
    TooManyRequests { maximum: usize, actual: usize },
    UnexpectedEof,
    ChangedAdapter { expected: Vec<u8>, actual: Vec<u8> },
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
    SupportChanged {
        member: String,
        expected: crate::SupportIdentity,
        actual: Option<crate::SupportIdentity>,
    },
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
#[cfg(test)]
fn typeset(
    support: &support::Support,
    target: RenderTarget,
    source: &str,
    site: NotationSite,
) -> Result<()> {
    typeset_with_mode(support, target, source, site, false)
}
fn typeset_with_mode(
    support: &support::Support,
    target: RenderTarget,
    source: &str,
    site: NotationSite,
    display: bool,
) -> Result<()> {
    let result = if display {
        katex::invoke_display(support, source)?
    } else {
        katex::invoke(support, Some(source))?
    };
    if let Err(message) = result {
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
    effective: Option<ftoo_audit::retention::RetainedNotation<'catalogue, 'book, 'graph>>,
    support: std::rc::Rc<support::Support>,
    declarations: String,
    rows: Vec<SweepRow>,
    pending: Vec<PendingEntry>,
}
impl<'catalogue, 'book, 'graph> RenderableNotation<'catalogue, 'book, 'graph> {
    /// Render a checked expanded source as `MathML` through the pinned strict adapter.
    /// # Errors
    /// Refuses unavailable or changed notation support and returns parser refusals unchanged.
    pub(crate) fn mathml_inline(
        &self,
        source: &str,
    ) -> std::result::Result<std::result::Result<String, String>, NotationAdapterFault> {
        katex::invoke_mathml(&self.support, source)
    }
    /// # Panics
    /// Panics only if an admitted inherited definition has no derived status, which callers cannot arrange.
    #[must_use]
    pub fn status_artifact(&self) -> Vec<u8> {
        let inherited: Vec<_> = self
            .catalogue
            .definitions()
            .iter()
            .filter(|definition| {
                matches!(
                    definition.spec().kind(),
                    ftoo_manifest::DefinitionKind::Inherited { .. }
                )
            })
            .map(|definition| {
                let status = self
                    .pending
                    .iter()
                    .find(|entry| entry.definition() == definition.spec().id())
                    .expect("inherited status");
                serde_json::json!({
                    "definition": definition.spec().id().token(),
                    "tier": "pending",
                    "reasons": status.reasons().iter().map(ToString::to_string).collect::<Vec<_>>()
                })
            })
            .collect();
        let dependents: Vec<_> = self
            .pending
            .iter()
            .filter(|entry| {
                matches!(
                    self.catalogue.definition(entry.definition()).spec().kind(),
                    ftoo_manifest::DefinitionKind::Canonical { .. }
                        | ftoo_manifest::DefinitionKind::Alias { .. }
                ) && entry
                    .reasons()
                    .iter()
                    .all(|reason| matches!(reason, PendingReason::InheritedUnbound { .. }))
            })
            .map(|entry| {
                serde_json::json!({
                    "definition": entry.definition().token(),
                    "tier": "pending",
                    "reasons": entry.reasons().iter().map(ToString::to_string).collect::<Vec<_>>()
                })
            })
            .collect();
        let mut bytes =
            serde_json::json!({"schema": 1, "inherited": inherited, "dependents": dependents})
                .to_string()
                .into_bytes();
        bytes.push(b'\n');
        bytes
    }
    #[must_use]
    pub fn declarations(&self) -> &str {
        &self.declarations
    }
    #[must_use]
    pub(crate) fn retained(&self) -> Option<&ftoo_audit::retention::RetainedNotation<'_, '_, '_>> {
        self.effective.as_ref()
    }
    #[must_use]
    pub fn sweep_rows(&self) -> &[SweepRow] {
        &self.rows
    }
    #[must_use]
    pub fn typesetting_calls(&self) -> usize {
        (self.rows.len() + self.pending.len()) * 2
            - self
                .pending
                .iter()
                .filter(|entry| {
                    matches!(
                        self.catalogue.definition(entry.definition()).spec().kind(),
                        ftoo_manifest::DefinitionKind::Inherited { .. }
                    )
                })
                .count()
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
    /// Bind a checked document expression, including its bounded plain text runs.
    /// # Errors
    /// Returns semantic admission faults for an invalid draft.
    pub fn check_document_expression<'a>(
        &'a self,
        draft: ftoo_manifest::ExprDraft,
    ) -> Result<RenderableExpression<'a, 'catalogue, 'book, 'graph>> {
        let expression = ftoo_manifest::check_document_expression(self.catalogue, draft)?;
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
        self.support.scope(
            &support::repository().join("tools/notation/check.cjs"),
            || self.render_inner(expression, target, false),
        )
    }
    fn render_display<'a>(
        &'a self,
        expression: &CheckedExpression<'catalogue, 'book, 'graph>,
        target: RenderTarget,
    ) -> Result<RenderedExpression<'a, 'catalogue, 'book, 'graph>> {
        self.support.scope(
            &support::repository().join("tools/notation/check.cjs"),
            || self.render_inner(expression, target, true),
        )
    }
    fn render_inner<'a>(
        &'a self,
        expression: &CheckedExpression<'catalogue, 'book, 'graph>,
        target: RenderTarget,
        display: bool,
    ) -> Result<RenderedExpression<'a, 'catalogue, 'book, 'graph>> {
        let render_expression = if display {
            render::display_expression
        } else {
            render::expression
        };
        let expanded = render_expression(
            self.effective.as_ref(),
            self.catalogue,
            expression.draft(),
            RenderTarget::Unicode,
            RenderOwner::Expression,
            "expression",
        )?;
        typeset_with_mode(
            &self.support,
            RenderTarget::Unicode,
            &expanded.source,
            expanded.site(RenderOwner::Expression, "expression"),
            display,
        )?;
        let source = match target {
            RenderTarget::Unicode => expanded.source,
            RenderTarget::Macro => {
                let result = render_expression(
                    self.effective.as_ref(),
                    self.catalogue,
                    expression.draft(),
                    target,
                    RenderOwner::Expression,
                    "expression",
                )?;
                let context = if expression.uses().iter().any(|usage| {
                    matches!(
                        self.catalogue.definition(usage.definition()).spec().kind(),
                        ftoo_manifest::DefinitionKind::Inherited { .. }
                    )
                }) {
                    render::temporary_context(self.effective.as_ref(), self.catalogue)?
                } else {
                    self.declarations.clone()
                };
                let combined = render::combine(
                    &context,
                    &result.source,
                    result.site(RenderOwner::Expression, "expression"),
                )?;
                typeset_with_mode(
                    &self.support,
                    target,
                    &combined,
                    rendered_site(
                        RenderOwner::Expression,
                        "expression",
                        &combined,
                        expanded.origins,
                    ),
                    display,
                )?;
                result.source
            }
        };
        Ok(RenderedExpression {
            notation: self,
            source,
            display,
            uses: expression.uses().to_vec(),
            pending: pending_uses(&self.pending, expression.uses()),
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
    /// Render a display spelling under the library's display layout.
    /// # Errors
    /// Refuses output limits, changed support, adapter failures and grammar rejection.
    pub fn render_display(
        &self,
        target: RenderTarget,
    ) -> Result<RenderedExpression<'a, 'catalogue, 'book, 'graph>> {
        self.notation.render_display(&self.expression, target)
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
    display: bool,
    uses: Vec<SymbolUse>,
    pending: Vec<PendingEntry>,
}
impl<'a, 'catalogue, 'book, 'graph> RenderedExpression<'a, 'catalogue, 'book, 'graph> {
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    #[must_use]
    pub const fn is_display(&self) -> bool {
        self.display
    }
    #[must_use]
    pub fn pending_entries(&self) -> &[PendingEntry] {
        &self.pending
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
    let _phase = crate::telemetry::Phase::new("notation.preparation");
    let location = std::env::var_os("FTOO_NOTATION_SUPPORT").map(PathBuf::from);
    let support = std::rc::Rc::new(support::resolve(location.as_deref())?);
    let records = render::declarations(None, catalogue)?;
    let (declarations, rows, pending) =
        sweep::prepared(None, catalogue, &support, &records, false)?;
    Ok(RenderableNotation {
        catalogue,
        effective: None,
        support,
        declarations,
        rows,
        pending,
    })
}

pub(crate) fn with_preparation<'c, 'b, 'g, T, E>(
    effective: Option<&ftoo_audit::retention::RetainedNotation<'c, 'b, 'g>>,
    catalogue: &'c CheckedNotation<'b, 'g>,
    run: impl FnOnce(&RenderableNotation<'_, '_, '_>) -> std::result::Result<T, E>,
) -> std::result::Result<T, E>
where
    E: From<NotationRenderError> + From<NotationAdapterFault>,
{
    let location = std::env::var_os("FTOO_NOTATION_SUPPORT").map(PathBuf::from);
    let support = std::rc::Rc::new(support::resolve(location.as_deref())?);
    let records = render::declarations(effective, catalogue)?;
    sweep::with_prepared(
        effective,
        catalogue,
        &support,
        &records,
        false,
        |(declarations, rows, pending)| {
            let notation = RenderableNotation {
                catalogue,
                effective: effective.cloned(),
                support: std::rc::Rc::clone(&support),
                declarations,
                rows,
                pending,
            };
            run(&notation)
        },
    )
}

/// Checked explanation for one pending symbol. Construction is private to preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingEntry {
    definition: DefinitionId,
    order: usize,
    macro_body: String,
    unicode_body: String,
    reasons: Vec<PendingReason>,
    origins: Vec<NotationOrigin>,
}
impl PendingEntry {
    #[cfg(test)]
    pub(crate) fn fixture(
        definition: DefinitionId,
        order: usize,
        body: &str,
        reasons: Vec<PendingReason>,
        origins: Vec<NotationOrigin>,
    ) -> Self {
        Self {
            definition,
            order,
            macro_body: body.into(),
            unicode_body: body.into(),
            reasons,
            origins,
        }
    }
    pub(crate) const fn order(&self) -> usize {
        self.order
    }
    #[must_use]
    pub fn body_macro(&self) -> &str {
        &self.macro_body
    }
    #[must_use]
    pub fn body_unicode(&self) -> &str {
        &self.unicode_body
    }
    #[must_use]
    pub const fn definition(&self) -> DefinitionId {
        self.definition
    }
    #[must_use]
    pub fn body(&self, target: RenderTarget) -> &str {
        match target {
            RenderTarget::Macro => &self.macro_body,
            RenderTarget::Unicode => &self.unicode_body,
        }
    }
    #[must_use]
    pub fn reasons(&self) -> &[PendingReason] {
        &self.reasons
    }
    #[must_use]
    pub fn origins(&self) -> &[NotationOrigin] {
        &self.origins
    }
}
fn pending_uses(prepared: &[PendingEntry], uses: &[SymbolUse]) -> Vec<PendingEntry> {
    prepared
        .iter()
        .filter_map(|entry| {
            let occurrence = uses
                .iter()
                .find(|used| used.definition() == entry.definition)?;
            if entry
                .reasons
                .iter()
                .all(|reason| matches!(reason, PendingReason::InheritedUnbound { .. }))
            {
                return None;
            }
            let mut result = entry.clone();
            result.origins = occurrence.origins().to_vec();
            Some(result)
        })
        .collect()
}
