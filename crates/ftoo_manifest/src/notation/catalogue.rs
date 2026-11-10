use super::{
    AliasTargetReason, Callee, DefinitionShapeReason, ExprDraft, NotationDiagnostic,
    NotationErrors, NotationFault, NotationSite, NotationStage, PrimitiveId, PrimitiveSpec,
    RegistryEntry, Signature, ast, diagnostics::finish,
};
use crate::{NodeId, NodeKind, RuleId};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const FILE: &str = "crates/ftoo_manifest/src/notation/catalogue.rs";
pub(super) const PRIMITIVES: &str = "crates/ftoo_manifest/src/notation/primitives.rs";

/// Closed authored definitions, distinct from permanent theory identities.
/// ```compile_fail,E0599
/// let id = ftoo_manifest::DefinitionId::Unregistered;
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DefinitionId {
    CoefficientOne,
    CoefficientThree,
    CoefficientOneValue,
    CoefficientThreeValue,
    FirstCoefficient,
    Dimension,
    Product,
}
impl DefinitionId {
    pub const ALL: [Self; 7] = [
        Self::CoefficientOne,
        Self::CoefficientThree,
        Self::CoefficientOneValue,
        Self::CoefficientThreeValue,
        Self::FirstCoefficient,
        Self::Dimension,
        Self::Product,
    ];
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::CoefficientOne => "coefficient-one",
            Self::CoefficientThree => "coefficient-three",
            Self::CoefficientOneValue => "coefficient-one-value",
            Self::CoefficientThreeValue => "coefficient-three-value",
            Self::FirstCoefficient => "first-coefficient",
            Self::Dimension => "dimension",
            Self::Product => "product",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefinitionKind {
    Canonical {
        object: NodeId,
        backing: Option<NodeId>,
        body: ExprDraft,
    },
    Alias {
        object: NodeId,
        backing: Option<NodeId>,
        target: DefinitionId,
    },
    Constructor {
        body: ExprDraft,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionCategory {
    Quantity,
    Object,
    Alias,
    Constructor,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionSpec {
    pub(super) id: DefinitionId,
    pub(super) token: &'static str,
    pub(super) stem: &'static str,
    pub(super) signature: Signature,
    pub(super) kind: DefinitionKind,
}
impl DefinitionSpec {
    #[must_use]
    pub const fn id(&self) -> DefinitionId {
        self.id
    }
    #[must_use]
    pub const fn token(&self) -> &'static str {
        self.token
    }
    #[must_use]
    pub const fn stem(&self) -> &'static str {
        self.stem
    }
    #[must_use]
    pub const fn signature(&self) -> Signature {
        self.signature
    }
    #[must_use]
    pub const fn kind(&self) -> &DefinitionKind {
        &self.kind
    }
    #[must_use]
    pub const fn category(&self) -> DefinitionCategory {
        match self.kind {
            DefinitionKind::Canonical { object, .. } => match object.expected_kind() {
                NodeKind::Quantity => DefinitionCategory::Quantity,
                NodeKind::Claim => DefinitionCategory::Object,
            },
            DefinitionKind::Alias { .. } => DefinitionCategory::Alias,
            DefinitionKind::Constructor { .. } => DefinitionCategory::Constructor,
        }
    }
    pub(super) fn command(&self) -> String {
        let category = match self.category() {
            DefinitionCategory::Quantity => 'Q',
            DefinitionCategory::Object => 'O',
            DefinitionCategory::Alias => 'A',
            DefinitionCategory::Constructor => 'N',
        };
        format!("\\Ftoo{category}{}", self.stem)
    }
    pub(super) fn body(&self) -> Option<&ExprDraft> {
        match &self.kind {
            DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => {
                Some(body)
            }
            DefinitionKind::Alias { .. } => None,
        }
    }
}

pub(super) fn literal(value: &str) -> ExprDraft {
    ExprDraft::Literal {
        value: value.into(),
    }
}
pub(super) fn primitive(id: PrimitiveId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Primitive(id),
        arguments,
    }
}
pub(super) fn call(id: DefinitionId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Definition(id),
        arguments,
    }
}
pub(super) fn catalogue() -> Vec<DefinitionSpec> {
    use DefinitionId::{
        CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue, Dimension,
        FirstCoefficient, Product,
    };
    use NodeId::{
        LangevinCoefficientOne, LangevinCoefficientOneValue, LangevinCoefficientThree,
        LangevinCoefficientThreeValue,
    };
    let canonical = |id: DefinitionId, stem, object, body| DefinitionSpec {
        id,
        token: id.token(),
        stem,
        signature: Signature { arity: 0 },
        kind: DefinitionKind::Canonical {
            object,
            backing: Some(object),
            body,
        },
    };
    vec![
        canonical(
            CoefficientOne,
            "CoefficientOne",
            LangevinCoefficientOne,
            primitive(PrimitiveId::Subscript, vec![literal("c"), literal("1")]),
        ),
        canonical(
            CoefficientThree,
            "CoefficientThree",
            LangevinCoefficientThree,
            primitive(PrimitiveId::Subscript, vec![literal("c"), literal("3")]),
        ),
        canonical(
            CoefficientOneValue,
            "CoefficientOneValue",
            LangevinCoefficientOneValue,
            primitive(
                PrimitiveId::Equal,
                vec![
                    call(CoefficientOne, vec![]),
                    primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
                ],
            ),
        ),
        canonical(
            CoefficientThreeValue,
            "CoefficientThreeValue",
            LangevinCoefficientThreeValue,
            primitive(
                PrimitiveId::Equal,
                vec![
                    call(CoefficientThree, vec![]),
                    primitive(
                        PrimitiveId::Negate,
                        vec![primitive(
                            PrimitiveId::Fraction,
                            vec![literal("1"), literal("45")],
                        )],
                    ),
                ],
            ),
        ),
        DefinitionSpec {
            id: FirstCoefficient,
            token: FirstCoefficient.token(),
            stem: "FirstCoefficient",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Alias {
                object: LangevinCoefficientOne,
                backing: Some(LangevinCoefficientOne),
                target: CoefficientOne,
            },
        },
        DefinitionSpec {
            id: Dimension,
            token: Dimension.token(),
            stem: "Dimension",
            signature: Signature { arity: 1 },
            kind: DefinitionKind::Constructor {
                body: primitive(PrimitiveId::Dimension, vec![ExprDraft::Slot { index: 0 }]),
            },
        },
        DefinitionSpec {
            id: Product,
            token: Product.token(),
            stem: "Product",
            signature: Signature { arity: 2 },
            kind: DefinitionKind::Constructor {
                body: primitive(
                    PrimitiveId::Multiply,
                    vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 1 }],
                ),
            },
        },
    ]
}
pub(super) fn site(index: usize, id: DefinitionId) -> NotationSite {
    registry_site(FILE, RegistryEntry::Definition(id), index)
}
fn registry_site(file: &str, entry: RegistryEntry, index: usize) -> NotationSite {
    NotationSite::Registry {
        file: file.into(),
        entry,
        index,
        path: vec![],
    }
}
fn valid_stem(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    let mut i = 0;
    if bytes.is_empty() {
        return false;
    }
    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            return false;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_lowercase() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    true
}
fn diagnostic(
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) -> NotationDiagnostic {
    NotationDiagnostic {
        rule,
        stage,
        site,
        fault,
    }
}

fn inventory(
    file: &str,
    expected: &[RegistryEntry],
    actual: &[(RegistryEntry, &str)],
) -> Vec<NotationDiagnostic> {
    let mut errors = vec![];
    for entry in expected {
        let positions: Vec<_> = actual
            .iter()
            .enumerate()
            .filter(|(_, (id, _))| id == entry)
            .map(|(i, _)| i)
            .collect();
        if positions.len() != 1 {
            errors.push(diagnostic(
                RuleId::M047,
                NotationStage::Catalogue,
                registry_site(
                    file,
                    *entry,
                    positions.first().copied().unwrap_or(actual.len()),
                )
                .field("id"),
                NotationFault::CatalogMultiplicity {
                    entry: *entry,
                    expected: 1,
                    actual: positions.len(),
                    positions,
                },
            ));
        }
    }
    let mut tokens = BTreeMap::<&str, Vec<usize>>::new();
    for (i, (_, token)) in actual.iter().enumerate() {
        tokens.entry(token).or_default().push(i);
    }
    for (token, positions) in tokens {
        if positions.len() > 1 {
            errors.push(diagnostic(
                RuleId::M047,
                NotationStage::Catalogue,
                registry_site(file, actual[positions[0]].0, positions[0]).field("token"),
                NotationFault::DuplicateToken {
                    token: token.into(),
                    entries: positions.iter().map(|i| actual[*i].0).collect(),
                    positions,
                },
            ));
        }
    }
    errors
}

fn shape(
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
) -> Result<(), NotationErrors> {
    let mut errors = inventory(
        PRIMITIVES,
        &PrimitiveId::ALL.map(RegistryEntry::Primitive),
        &primitives
            .iter()
            .map(|p| (RegistryEntry::Primitive(p.id), p.token))
            .collect::<Vec<_>>(),
    );
    errors.extend(inventory(
        FILE,
        &DefinitionId::ALL.map(RegistryEntry::Definition),
        &definitions
            .iter()
            .map(|d| (RegistryEntry::Definition(d.id), d.token))
            .collect::<Vec<_>>(),
    ));
    for id in PrimitiveId::ALL.into_iter().filter(|_| {
        PrimitiveId::ALL
            .into_iter()
            .all(|id| primitives.iter().filter(|p| p.id == id).count() == 1)
    }) {
        let positions: Vec<_> = primitives
            .iter()
            .enumerate()
            .filter(|(_, p)| p.operation == id.operation())
            .map(|(i, _)| i)
            .collect();
        if positions.len() != 1 {
            errors.push(diagnostic(
                RuleId::M047,
                NotationStage::Catalogue,
                registry_site(
                    PRIMITIVES,
                    RegistryEntry::Primitive(id),
                    positions.first().copied().unwrap_or(primitives.len()),
                )
                .field("operation"),
                NotationFault::CatalogMultiplicity {
                    entry: RegistryEntry::Primitive(id),
                    expected: 1,
                    actual: positions.len(),
                    positions,
                },
            ));
        }
    }
    for (i, p) in primitives.iter().enumerate() {
        if p.signature.arity != p.operation.arity() {
            errors.push(diagnostic(
                RuleId::M047,
                NotationStage::Catalogue,
                registry_site(PRIMITIVES, RegistryEntry::Primitive(p.id), i)
                    .field("signature")
                    .field("arity"),
                NotationFault::PrimitiveSignature {
                    primitive: p.id,
                    expected: p.operation.arity(),
                    actual: p.signature.arity,
                },
            ));
        }
    }
    definition_shapes(definitions, &mut errors);
    finish(errors)
}

fn definition_shapes(definitions: &[DefinitionSpec], errors: &mut Vec<NotationDiagnostic>) {
    let mut names = BTreeMap::<String, Vec<(usize, DefinitionId)>>::new();
    for (i, d) in definitions.iter().enumerate() {
        if valid_stem(d.stem) {
            names.entry(d.command()).or_default().push((i, d.id));
        } else {
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::Catalogue,
                site(i, d.id).field("stem"),
                NotationFault::InvalidStem {
                    value: d.stem.into(),
                },
            ));
        }
        let (reason, binding) = match d.kind {
            DefinitionKind::Canonical {
                object, backing, ..
            } => (
                (d.signature.arity != 0).then_some(DefinitionShapeReason::CanonicalMustBeNullary),
                Some((object, backing)),
            ),
            DefinitionKind::Alias {
                object, backing, ..
            } => (
                (d.signature.arity != 0).then_some(DefinitionShapeReason::AliasMustBeNullary),
                Some((object, backing)),
            ),
            DefinitionKind::Constructor { .. } => (
                (!((1..=9).contains(&d.signature.arity)))
                    .then_some(DefinitionShapeReason::ConstructorArity),
                None,
            ),
        };
        if let Some(reason) = reason {
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::Catalogue,
                site(i, d.id).field("signature").field("arity"),
                NotationFault::DefinitionShape {
                    definition: d.id,
                    reason,
                },
            ));
        }
        if let Some((object, Some(backing))) = binding
            && object != backing
        {
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::Catalogue,
                site(i, d.id).field("backing"),
                NotationFault::DefinitionShape {
                    definition: d.id,
                    reason: DefinitionShapeReason::BackingObjectMismatch {
                        expected: object,
                        actual: backing,
                    },
                },
            ));
        }
    }
    for (command, entries) in names {
        if entries.len() > 1 {
            let mut ids: Vec<_> = entries.iter().map(|(_, id)| *id).collect();
            ids.sort_by_key(|id| id.token());
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::Catalogue,
                site(entries[0].0, entries[0].1).field("stem"),
                NotationFault::CommandCollision {
                    command,
                    definitions: ids,
                },
            ));
        }
    }
}

fn references(definitions: &[DefinitionSpec]) -> Result<(), NotationErrors> {
    let mut errors = vec![];
    for object in NodeId::ALL {
        let owners: Vec<_> = definitions.iter().enumerate().filter(|(_, d)| matches!(d.kind, DefinitionKind::Canonical { object: id, .. } if id == object)).collect();
        if owners.len() != 1 {
            let location = owners.first().map_or_else(
                || NotationSite::Catalogue { file: FILE.into() },
                |(i, d)| site(*i, d.id).field("object"),
            );
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::References,
                location,
                NotationFault::CanonicalMultiplicity {
                    object,
                    expected: 1,
                    actual: owners.len(),
                    definitions: owners.iter().map(|(_, d)| d.id).collect(),
                    positions: owners.iter().map(|(i, _)| *i).collect(),
                },
            ));
        }
    }
    for (i, d) in definitions.iter().enumerate() {
        if let DefinitionKind::Alias { object, target, .. } = d.kind {
            let mut current = target;
            let mut seen = BTreeSet::new();
            while seen.insert(current) {
                let target_spec = definitions
                    .iter()
                    .find(|d| d.id == current)
                    .expect("complete definitions");
                let reason = match target_spec.kind {
                    DefinitionKind::Constructor { .. } => {
                        Some(AliasTargetReason::ConstructorTarget)
                    }
                    DefinitionKind::Canonical { object: actual, .. }
                    | DefinitionKind::Alias { object: actual, .. }
                        if actual != object =>
                    {
                        Some(AliasTargetReason::ObjectMismatch {
                            expected: object,
                            actual,
                        })
                    }
                    _ => None,
                };
                if let Some(reason) = reason {
                    errors.push(diagnostic(
                        RuleId::M048,
                        NotationStage::References,
                        site(i, d.id).field("target"),
                        NotationFault::AliasTarget {
                            alias: d.id,
                            target,
                            reason,
                        },
                    ));
                    break;
                }
                if let DefinitionKind::Alias { target, .. } = target_spec.kind {
                    current = target;
                } else {
                    break;
                }
            }
        }
    }
    finish(errors)
}

pub(super) type Dependencies = BTreeMap<DefinitionId, Vec<(DefinitionId, NotationSite)>>;
pub(super) fn dependencies(definitions: &[DefinitionSpec]) -> Dependencies {
    definitions
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut edges = match &d.kind {
                DefinitionKind::Alias { target, .. } => {
                    vec![(*target, site(i, d.id).field("target"))]
                }
                DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => {
                    ast::calls(body, site(i, d.id).field("body"))
                }
            };
            edges.sort_by_key(|(id, _)| id.token());
            (d.id, edges)
        })
        .collect()
}
fn reachable(start: DefinitionId, edges: &Dependencies) -> BTreeSet<DefinitionId> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        if seen.insert(id) {
            stack.extend(edges[&id].iter().map(|(id, _)| *id));
        }
    }
    seen
}
fn cycle_path(
    current: DefinitionId,
    start: DefinitionId,
    component: &BTreeSet<DefinitionId>,
    edges: &Dependencies,
    chain: &mut Vec<DefinitionId>,
    sites: &mut Vec<NotationSite>,
) -> bool {
    for (next, site) in &edges[&current] {
        if *next == start {
            chain.push(start);
            sites.push(site.clone());
            return true;
        }
        if component.contains(next) && !chain.contains(next) {
            chain.push(*next);
            sites.push(site.clone());
            if cycle_path(*next, start, component, edges, chain, sites) {
                return true;
            }
            chain.pop();
            sites.pop();
        }
    }
    false
}
fn cycles(edges: &Dependencies) -> Result<(), NotationErrors> {
    let mut remaining: Vec<_> = DefinitionId::ALL.to_vec();
    remaining.sort_by_key(|id| id.token());
    let mut handled = BTreeSet::<DefinitionId>::new();
    let mut errors = vec![];
    for start in remaining {
        if handled.contains(&start) {
            continue;
        }
        let component: BTreeSet<_> = reachable(start, edges)
            .into_iter()
            .filter(|id| reachable(*id, edges).contains(&start))
            .collect();
        handled.extend(component.iter().copied());
        let mut chain = vec![start];
        let mut sites = vec![];
        if cycle_path(start, start, &component, edges, &mut chain, &mut sites) {
            errors.push(diagnostic(
                RuleId::M048,
                NotationStage::Dependencies,
                sites[0].clone(),
                NotationFault::DefinitionCycle { chain, sites },
            ));
        }
    }
    finish(errors)
}
pub(super) fn admit(
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
) -> Result<Dependencies, NotationErrors> {
    shape(primitives, definitions)?;
    references(definitions)?;
    let mut errors = vec![];
    for (i, d) in definitions.iter().enumerate() {
        if let Some(body) = d.body() {
            errors.extend(ast::validate(
                body,
                d.signature.arity,
                &site(i, d.id).field("body"),
                primitives,
                definitions,
            ));
        }
    }
    finish(errors)?;
    let edges = dependencies(definitions);
    cycles(&edges)?;
    let mut errors = vec![];
    for (i, d) in definitions.iter().enumerate() {
        let alias_body;
        let (body, field) = match &d.kind {
            DefinitionKind::Alias { target, .. } => {
                alias_body = call(*target, vec![]);
                (&alias_body, "target")
            }
            DefinitionKind::Canonical { body, .. } | DefinitionKind::Constructor { body } => {
                (body, "body")
            }
        };
        if let Err(error) = ast::expand(
            body,
            site(i, d.id).field(field),
            definitions,
            d.signature.arity,
        ) {
            errors.push(*error);
        }
    }
    finish(errors)?;
    Ok(edges)
}
