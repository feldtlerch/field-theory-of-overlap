//! Closed semantic notation, checked before any rendering exists.
//! Syntax admission proves neither quantity typing nor mathematical truth.
mod ast;
mod catalogue;
mod diagnostics;
mod primitives;
#[cfg(test)]
mod tests;
mod tiers;

use crate::{AddressBook, NodeId, NodeKind};
pub use ast::{
    ExprDraft, MAX_AST_NODES, MAX_DEPTH, MAX_EXPANSION_NODES, MAX_LITERAL_BYTES, MathSort,
    Signature,
};
pub use catalogue::{DefinitionCategory, DefinitionId, DefinitionKind, DefinitionSpec};
pub use diagnostics::{
    AliasTargetReason, DefinitionShapeReason, LiteralReason, NotationDiagnostic, NotationErrors,
    NotationFault, NotationOrigin, NotationPathPart, NotationResource, NotationSite, NotationStage,
    RegistryEntry, SymbolLookupFault,
};
pub use primitives::{PrimitiveId, PrimitiveOperation, PrimitiveSpec};
pub use tiers::{PendingReason, SymbolStatus, SymbolTier};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Callee {
    Primitive(PrimitiveId),
    Definition(DefinitionId),
}

/// Checked entries cannot be assembled from authored statuses.
/// ```compile_fail,E0451
/// use ftoo_manifest::{CheckedDefinition, DefinitionSpec, NotationSite};
/// fn forge(spec: DefinitionSpec, site: NotationSite) -> CheckedDefinition {
///     CheckedDefinition { spec, command: "fake".into(), status: None, site }
/// }
/// ```
#[derive(Debug)]
pub struct CheckedDefinition {
    spec: DefinitionSpec,
    command: String,
    status: Option<SymbolStatus>,
    site: NotationSite,
}
impl CheckedDefinition {
    #[must_use]
    pub const fn spec(&self) -> &DefinitionSpec {
        &self.spec
    }
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    #[must_use]
    pub const fn status(&self) -> Option<&SymbolStatus> {
        self.status.as_ref()
    }
    #[must_use]
    pub const fn site(&self) -> &NotationSite {
        &self.site
    }
}
/// An immutable catalogue borrowing the exact supplied book.
/// ```compile_fail,E0451
/// use ftoo_manifest::{CheckedNotation, AddressBook};
/// fn forge<'a, 'g>(book: &'a AddressBook<'g>) -> CheckedNotation<'a, 'g> {
///     CheckedNotation { book, primitives: vec![], definitions: vec![] }
/// }
/// ```
/// ```compile_fail,E0277
/// let n: ftoo_manifest::CheckedNotation<'_, '_> = ().into();
/// ```
/// ```compile_fail,E0616
/// fn alter(n: &mut ftoo_manifest::CheckedNotation<'_, '_>) { n.definitions.clear(); }
/// ```
/// ```compile_fail,E0599
/// let n = ftoo_manifest::CheckedNotation::default();
/// ```
/// ```compile_fail,E0277
/// let n: ftoo_manifest::CheckedNotation<'_, '_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0597
/// let graph = ftoo_manifest::seed().unwrap();
/// let notation = {
///     let book = ftoo_manifest::addressing(&graph).unwrap();
///     ftoo_manifest::notation(&book).unwrap()
/// };
/// println!("{:?}", notation);
/// ```
#[derive(Debug)]
pub struct CheckedNotation<'book, 'graph> {
    book: &'book AddressBook<'graph>,
    primitives: Vec<PrimitiveSpec>,
    definitions: Vec<CheckedDefinition>,
}
impl CheckedNotation<'_, '_> {
    #[must_use]
    pub fn primitives(&self) -> &[PrimitiveSpec] {
        &self.primitives
    }
    #[must_use]
    pub fn definitions(&self) -> &[CheckedDefinition] {
        &self.definitions
    }
    /// # Panics
    /// Panics only if internal completeness is corrupted, which callers cannot arrange.
    #[must_use]
    pub fn definition(&self, id: DefinitionId) -> &CheckedDefinition {
        self.definitions
            .iter()
            .find(|d| d.spec.id == id)
            .expect("complete checked catalogue")
    }
    /// Select the unique canonical without implying tag liveness.
    /// # Errors
    /// Refuses an object absent from this book's graph.
    /// # Panics
    /// Panics only if internal canonical coverage is corrupted, which callers cannot arrange.
    pub fn object_symbol(&self, object: NodeId) -> Result<&CheckedDefinition, SymbolLookupFault> {
        if self.book.graph().node(object).is_none() {
            return Err(SymbolLookupFault::AbsentObject { object });
        }
        Ok(self.definitions.iter().find(|d| matches!(d.spec.kind, DefinitionKind::Canonical { object: id, .. } if id == object)).expect("complete canonical coverage"))
    }
    /// # Errors
    /// Refuses absent objects before refusing a present nonquantity.
    /// ```compile_fail,E0308
    /// fn misuse(n: &ftoo_manifest::CheckedNotation<'_, '_>) {
    ///     n.quantity_symbol(ftoo_manifest::DefinitionId::Dimension);
    /// }
    /// ```
    pub fn quantity_symbol(&self, object: NodeId) -> Result<&CheckedDefinition, SymbolLookupFault> {
        let Some(node) = self.book.graph().node(object) else {
            return Err(SymbolLookupFault::AbsentObject { object });
        };
        match node.payload.kind() {
            NodeKind::Quantity => self.object_symbol(object),
            actual @ NodeKind::Claim => Err(SymbolLookupFault::NotQuantity { object, actual }),
        }
    }
}

/// Validate the compiled authority and derive status solely from this book.
/// # Errors
/// Returns complete, ordered diagnostics without a partially admitted catalogue.
pub fn notation<'book, 'graph>(
    book: &'book AddressBook<'graph>,
) -> Result<CheckedNotation<'book, 'graph>, NotationErrors> {
    construct(book, primitives::catalogue(), catalogue::catalogue())
}
fn construct<'book, 'graph>(
    book: &'book AddressBook<'graph>,
    primitives: Vec<PrimitiveSpec>,
    definitions: Vec<DefinitionSpec>,
) -> Result<CheckedNotation<'book, 'graph>, NotationErrors> {
    let edges = catalogue::admit(&primitives, &definitions)?;
    let statuses: Vec<_> = definitions
        .iter()
        .map(|spec| {
            if matches!(spec.kind, DefinitionKind::Constructor { .. }) {
                None
            } else {
                Some(tiers::derive(spec.id, book, &definitions, &edges))
            }
        })
        .collect();
    let checked = definitions
        .into_iter()
        .zip(statuses)
        .enumerate()
        .map(|(i, (spec, status))| CheckedDefinition {
            status,
            command: spec.command(),
            site: catalogue::site(i, spec.id),
            spec,
        })
        .collect();
    Ok(CheckedNotation {
        book,
        primitives,
        definitions: checked,
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolUse {
    definition: DefinitionId,
    origins: Vec<NotationOrigin>,
    status: SymbolStatus,
}
impl SymbolUse {
    #[must_use]
    pub const fn definition(&self) -> DefinitionId {
        self.definition
    }
    #[must_use]
    pub fn origins(&self) -> &[NotationOrigin] {
        &self.origins
    }
    #[must_use]
    pub const fn status(&self) -> &SymbolStatus {
        &self.status
    }
}
/// A closed expression with bounded substitution and immutable provenance.
/// ```compile_fail,E0597
/// let graph = ftoo_manifest::seed().unwrap();
/// let book = ftoo_manifest::addressing(&graph).unwrap();
/// let catalogue = ftoo_manifest::notation(&book).unwrap();
/// let draft = {
///     let expression = ftoo_manifest::check_expression(&catalogue,
///         ftoo_manifest::ExprDraft::Literal { value: "x".into() }).unwrap();
///     expression.draft()
/// };
/// println!("{:?}", draft);
/// ```
/// ```compile_fail,E0451
/// use ftoo_manifest::{CheckedExpression, CheckedNotation, ExprDraft};
/// fn forge<'c, 'b, 'g>(catalogue: &'c CheckedNotation<'b, 'g>) -> CheckedExpression<'c, 'b, 'g> {
///     CheckedExpression { catalogue, draft: ExprDraft::Slot { index: 0 }, uses: vec![] }
/// }
/// ```
/// ```compile_fail,E0599
/// let e = ftoo_manifest::CheckedExpression::default();
/// ```
/// ```compile_fail,E0277
/// let e: ftoo_manifest::CheckedExpression<'_, '_, '_> = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail,E0277
/// let e: ftoo_manifest::CheckedExpression<'_, '_, '_> = ftoo_manifest::ExprDraft::Slot { index: 0 }.into();
/// ```
/// ```compile_fail,E0597
/// let n = {
///     let graph = ftoo_manifest::seed().unwrap();
///     let book = ftoo_manifest::addressing(&graph).unwrap();
///     ftoo_manifest::notation(&book).unwrap()
/// };
/// println!("{:?}", n);
/// ```
/// ```compile_fail,E0616
/// fn mutate(e: &mut ftoo_manifest::CheckedExpression<'_, '_, '_>) {
///     e.draft = ftoo_manifest::ExprDraft::Slot { index: 0 };
/// }
/// ```
/// ```compile_fail,E0597
/// let graph = ftoo_manifest::seed().unwrap();
/// let book = ftoo_manifest::addressing(&graph).unwrap();
/// let expression = {
///     let n = ftoo_manifest::notation(&book).unwrap();
///     ftoo_manifest::check_expression(&n, ftoo_manifest::ExprDraft::Literal { value: "x".into() }).unwrap()
/// };
/// println!("{:?}", expression);
/// ```
#[derive(Debug)]
pub struct CheckedExpression<'catalogue, 'book, 'graph> {
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
    draft: ExprDraft,
    uses: Vec<SymbolUse>,
}
impl<'catalogue, 'book, 'graph> CheckedExpression<'catalogue, 'book, 'graph> {
    #[must_use]
    pub const fn catalogue(&self) -> &'catalogue CheckedNotation<'book, 'graph> {
        self.catalogue
    }
    #[must_use]
    pub const fn draft(&self) -> &ExprDraft {
        &self.draft
    }
    #[must_use]
    pub fn uses(&self) -> &[SymbolUse] {
        &self.uses
    }
}
/// Check closed syntax and every transitive substituted occurrence.
/// # Errors
/// Returns AST or expansion diagnostics; never renderer readiness.
/// # Panics
/// Panics only if checked symbol status is internally corrupted, which callers cannot arrange.
pub fn check_expression<'catalogue, 'book, 'graph>(
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
    draft: ExprDraft,
) -> Result<CheckedExpression<'catalogue, 'book, 'graph>, NotationErrors> {
    let definitions: Vec<_> = catalogue
        .definitions
        .iter()
        .map(|d| d.spec.clone())
        .collect();
    let site = NotationSite::Expression { path: vec![] };
    diagnostics::finish(ast::validate(
        &draft,
        0,
        &site,
        &catalogue.primitives,
        &definitions,
    ))?;
    let uses = ast::expand(&draft, site, &definitions, 0)
        .map_err(|e| NotationErrors::from_diagnostics(vec![*e]).expect("one diagnostic"))?;
    let uses = uses
        .into_iter()
        .map(|(id, origins)| SymbolUse {
            definition: id,
            origins,
            status: catalogue
                .definition(id)
                .status()
                .expect("symbol status")
                .clone(),
        })
        .collect();
    Ok(CheckedExpression {
        catalogue,
        draft,
        uses,
    })
}
