use super::{
    AliasTargetReason, Callee, DefinitionShapeReason, ExprDraft, NotationDiagnostic,
    NotationErrors, NotationFault, NotationSite, NotationStage, PrimitiveId, PrimitiveSpec,
    RegistryEntry, Signature, ast, diagnostics::finish,
};
use crate::{NodeId, NodeKind, RuleId};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const FILE: &str = "crates/ftoo_manifest/src/notation/catalogue.rs";
pub(super) const PRIMITIVES: &str = "crates/ftoo_manifest/src/notation/primitives.rs";
const SYMBOLS: &str = include_str!("../../../../documents/paper/data/ftoo_symbols.tex");
const HISTORICAL: &str =
    include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex");
const REGISTERS: &str = include_str!("../../../../documents/paper/data/ftoo_registers.tex");
pub const INHERITED_SOURCE_FILES: [(&str, &str); 3] = [
    ("ftoo_symbols.tex", SYMBOLS),
    ("ftoo_symbols_historical.tex", HISTORICAL),
    ("ftoo_registers.tex", REGISTERS),
];
pub const INHERITED_SOURCE_DECLARATIONS: [(DefinitionId, &str); 7] = [
    (
        DefinitionId::Comovfield,
        "\\NewDocumentCommand\n  { \\comovfield }\n  {  }\n  {\n    \\ensuremath{S}\n  }\n",
    ),
    (
        DefinitionId::Readout,
        "\\NewDocumentCommand\n  { \\readout }\n  {  }\n  {\n    \\ensuremath{\\symcal{M}}\n  }\n",
    ),
    (
        DefinitionId::Willvirtue,
        "\\NewDocumentCommand\n  { \\willvirtue }\n  {  }\n  {\n    \\ensuremath{\\omega\\sp{\\mathrm{vir}}}\n  }\n",
    ),
    (
        DefinitionId::Gensym,
        "\\DeclareQuantity{\\gensym}{0}{ dim={\\Time[-1]}, variance=operator, spends={} }\n  { A\\sb{\\mathrm S} }\n",
    ),
    (
        DefinitionId::Alignrate,
        "\\NewDocumentCommand\n  { \\alignrate }\n  {  }\n  {\n    \\ensuremath{\\dot{\\sigma}}\n  }\n",
    ),
    (
        DefinitionId::Willnet,
        "\\NewDocumentCommand\n  { \\willnet }\n  {  }\n  {\n    \\ensuremath{\\omega\\sb{\\mathrm{net}}}\n  }\n",
    ),
    (
        DefinitionId::Willdecay,
        "\\NewDocumentCommand\n  { \\willdecay }\n  {  }\n  {\n    \\ensuremath{\\gamma\\sigma}\n  }\n",
    ),
];
pub const INHERITED_VARIANCE_DECLARATION: &str = "\\RegisterVariance{\\comovfield}{operator}\n";
pub(super) struct InheritedSources<'a> {
    pub symbols: &'a str,
    pub historical: &'a str,
    pub registers: &'a str,
}
const SOURCES: InheritedSources<'static> = InheritedSources {
    symbols: SYMBOLS,
    historical: HISTORICAL,
    registers: REGISTERS,
};

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
    Comovfield,
    Readout,
    Willvirtue,
    Gensym,
    Alignrate,
    Willnet,
    Willdecay,
}
impl DefinitionId {
    pub const ALL: [Self; 14] = [
        Self::CoefficientOne,
        Self::CoefficientThree,
        Self::CoefficientOneValue,
        Self::CoefficientThreeValue,
        Self::FirstCoefficient,
        Self::Dimension,
        Self::Product,
        Self::Comovfield,
        Self::Readout,
        Self::Willvirtue,
        Self::Gensym,
        Self::Alignrate,
        Self::Willnet,
        Self::Willdecay,
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
            Self::Comovfield => "comovfield",
            Self::Readout => "readout",
            Self::Willvirtue => "willvirtue",
            Self::Gensym => "gensym",
            Self::Alignrate => "alignrate",
            Self::Willnet => "willnet",
            Self::Willdecay => "willdecay",
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
    Inherited {
        body: ExprDraft,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionCategory {
    Quantity,
    Object,
    Alias,
    Constructor,
    Inherited,
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
            DefinitionKind::Inherited { .. } => DefinitionCategory::Inherited,
        }
    }
    pub(super) fn command(&self) -> String {
        let category = match self.category() {
            DefinitionCategory::Quantity => 'Q',
            DefinitionCategory::Object => 'O',
            DefinitionCategory::Alias => 'A',
            DefinitionCategory::Constructor => 'N',
            DefinitionCategory::Inherited => 'I',
        };
        if matches!(self.kind, DefinitionKind::Inherited { .. }) {
            format!("\\{}", self.token)
        } else {
            format!("\\Ftoo{category}{}", self.stem)
        }
    }
    pub(super) fn body(&self) -> Option<&ExprDraft> {
        match &self.kind {
            DefinitionKind::Canonical { body, .. }
            | DefinitionKind::Constructor { body }
            | DefinitionKind::Inherited { body } => Some(body),
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
#[allow(clippy::too_many_lines)] // The closed catalogue keeps source identities and bodies together.
pub(super) fn catalogue() -> Vec<DefinitionSpec> {
    use DefinitionId::{
        Alignrate, CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue,
        Comovfield, Dimension, FirstCoefficient, Gensym, Product, Readout, Willdecay, Willnet,
        Willvirtue,
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
        DefinitionSpec {
            id: Comovfield,
            token: Comovfield.token(),
            stem: "Comovfield",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited { body: literal("S") },
        },
        DefinitionSpec {
            id: Readout,
            token: Readout.token(),
            stem: "Readout",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(PrimitiveId::Calligraphic, vec![literal("M")]),
            },
        },
        DefinitionSpec {
            id: Willvirtue,
            token: Willvirtue.token(),
            stem: "Willvirtue",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Superscript,
                    vec![
                        primitive(PrimitiveId::Omega, vec![]),
                        primitive(PrimitiveId::UprightVir, vec![]),
                    ],
                ),
            },
        },
        DefinitionSpec {
            id: Gensym,
            token: Gensym.token(),
            stem: "Gensym",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Subscript,
                    vec![literal("A"), primitive(PrimitiveId::UprightS, vec![])],
                ),
            },
        },
        DefinitionSpec {
            id: Alignrate,
            token: Alignrate.token(),
            stem: "Alignrate",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Dot,
                    vec![primitive(PrimitiveId::Sigma, vec![])],
                ),
            },
        },
        DefinitionSpec {
            id: Willnet,
            token: Willnet.token(),
            stem: "Willnet",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Subscript,
                    vec![
                        primitive(PrimitiveId::Omega, vec![]),
                        primitive(PrimitiveId::UprightNet, vec![]),
                    ],
                ),
            },
        },
        DefinitionSpec {
            id: Willdecay,
            token: Willdecay.token(),
            stem: "Willdecay",
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::TightJoin,
                    vec![
                        primitive(PrimitiveId::Gamma, vec![]),
                        primitive(PrimitiveId::Sigma, vec![]),
                    ],
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
            DefinitionKind::Inherited { .. } => (
                (d.signature.arity != 0).then_some(DefinitionShapeReason::InheritedMustBeNullary),
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
        if let DefinitionKind::Inherited { body } = &d.kind {
            for (target, use_site) in ast::calls(body, site(i, d.id).field("body")) {
                if definitions.iter().any(|candidate| {
                    candidate.id == target
                        && !matches!(candidate.kind, DefinitionKind::Inherited { .. })
                }) {
                    errors.push(diagnostic(
                        RuleId::M048,
                        NotationStage::Catalogue,
                        use_site,
                        NotationFault::DefinitionShape {
                            definition: d.id,
                            reason: DefinitionShapeReason::InheritedDependency { target },
                        },
                    ));
                }
            }
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
                    DefinitionKind::Inherited { .. } => Some(AliasTargetReason::InheritedTarget),
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
                DefinitionKind::Canonical { body, .. }
                | DefinitionKind::Constructor { body }
                | DefinitionKind::Inherited { body } => {
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
pub(super) fn cycles(edges: &Dependencies) -> Result<(), NotationErrors> {
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
fn fully_expanded(
    expr: &ExprDraft,
    definitions: &[DefinitionSpec],
    slots: &[ExprDraft],
    remaining: &mut usize,
) -> Option<ExprDraft> {
    if *remaining == 0 {
        return None;
    }
    *remaining -= 1;
    match expr {
        ExprDraft::Literal { .. } | ExprDraft::Text { .. } => Some(expr.clone()),
        ExprDraft::Slot { index } => slots.get(usize::from(*index)).cloned(),
        ExprDraft::Call { callee, arguments } => {
            let arguments = arguments
                .iter()
                .map(|argument| fully_expanded(argument, definitions, slots, remaining))
                .collect::<Option<Vec<_>>>()?;
            match callee {
                Callee::Primitive(_) => Some(ExprDraft::Call {
                    callee: *callee,
                    arguments,
                }),
                Callee::Definition(id) => {
                    let definition = definitions.iter().find(|d| d.id == *id)?;
                    match &definition.kind {
                        DefinitionKind::Alias { target, .. } => {
                            fully_expanded(&call(*target, vec![]), definitions, &[], remaining)
                        }
                        DefinitionKind::Canonical { body, .. }
                        | DefinitionKind::Constructor { body }
                        | DefinitionKind::Inherited { body } => {
                            fully_expanded(body, definitions, &arguments, remaining)
                        }
                    }
                }
            }
        }
    }
}
fn expanded_body_conflicts(definitions: &[DefinitionSpec]) -> Result<(), NotationErrors> {
    let mut errors = vec![];
    for (i, inherited) in definitions.iter().enumerate() {
        let DefinitionKind::Inherited { body } = &inherited.kind else {
            continue;
        };
        let mut remaining = ast::MAX_EXPANSION_NODES;
        let expanded = fully_expanded(body, definitions, &[], &mut remaining)
            .expect("validated inherited body expands within limits");
        for canonical in definitions
            .iter()
            .filter(|d| matches!(d.kind, DefinitionKind::Canonical { .. }))
        {
            let mut remaining = ast::MAX_EXPANSION_NODES;
            let other = fully_expanded(
                canonical.body().expect("canonical body exists"),
                definitions,
                &[],
                &mut remaining,
            )
            .expect("validated canonical body expands within limits");
            if expanded == other {
                errors.push(diagnostic(
                    RuleId::M048,
                    NotationStage::Catalogue,
                    site(i, inherited.id).field("body"),
                    NotationFault::DefinitionShape {
                        definition: inherited.id,
                        reason: DefinitionShapeReason::InheritedBodyConflict {
                            canonical: canonical.id,
                        },
                    },
                ));
            }
        }
    }
    finish(errors)
}
pub(super) fn source_matches(source: &str, bytes: &str) -> bool {
    source.matches(bytes).count() == 1
}
fn declared_name(source: &str) -> Option<&str> {
    let source = source.trim_start_matches(char::is_whitespace);
    let source = source
        .strip_prefix('*')
        .unwrap_or(source)
        .trim_start_matches(char::is_whitespace);
    let source = source
        .strip_prefix('{')
        .unwrap_or(source)
        .trim_start_matches(char::is_whitespace);
    let name = source.strip_prefix('\\')?;
    let len = name
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b':'))
        .count();
    (len > 0).then(|| &name[..len])
}
fn csname_name(source: &str) -> Option<&str> {
    let source = source.trim_start_matches(char::is_whitespace);
    let len = source
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b':'))
        .count();
    let (name, rest) = source.split_at(len);
    (len > 0
        && rest
            .trim_start_matches(char::is_whitespace)
            .starts_with("\\endcsname"))
    .then_some(name)
}
fn uncommented(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut escaped = false;
    let mut comment = false;
    for scalar in source.chars() {
        if comment {
            if scalar == '\n' {
                comment = false;
                result.push(scalar);
            }
            continue;
        }
        if scalar == '%' && !escaped {
            comment = true;
            continue;
        }
        result.push(scalar);
        escaped = scalar == '\\' && !escaped;
    }
    result
}
fn declaration_target(source: &str) -> Option<&str> {
    let source = source.trim_start_matches(char::is_whitespace);
    let source = source
        .strip_prefix('*')
        .unwrap_or(source)
        .trim_start_matches(char::is_whitespace);
    let source = source
        .strip_prefix('{')
        .unwrap_or(source)
        .trim_start_matches(char::is_whitespace);
    if let Some(source) = source.strip_prefix("\\csname") {
        csname_name(source)
    } else {
        declared_name(source)
    }
}
fn declaration_sites(source: &str, name: &str) -> usize {
    let source = uncommented(source);
    [
        "\\NewDocumentCommand",
        "\\RenewDocumentCommand",
        "\\DeclareDocumentCommand",
        "\\ProvideDocumentCommand",
        "\\NewExpandableDocumentCommand",
        "\\RenewExpandableDocumentCommand",
        "\\DeclareExpandableDocumentCommand",
        "\\ProvideExpandableDocumentCommand",
        "\\DeclareRobustCommand",
        "\\NewCommandCopy",
        "\\RenewCommandCopy",
        "\\DeclareCommandCopy",
        "\\DeclareQuantity",
        "\\newcommand",
        "\\renewcommand",
        "\\providecommand",
        "\\def",
        "\\gdef",
        "\\edef",
        "\\xdef",
        "\\let",
        "\\cs_set:Npn",
    ]
    .into_iter()
    .map(|command| {
        source
            .match_indices(command)
            .filter(|(at, _)| declaration_target(&source[*at + command.len()..]) == Some(name))
            .count()
    })
    .sum()
}
fn registered_variance_sites(source: &str, name: &str) -> usize {
    let source = uncommented(source);
    source
        .match_indices("\\RegisterVariance")
        .filter(|(at, _)| declared_name(&source[*at + "\\RegisterVariance".len()..]) == Some(name))
        .count()
}
#[allow(clippy::too_many_lines)] // One comparison keeps all admitted source declarations and bodies together.
pub(super) fn inherited_source(
    definitions: &[DefinitionSpec],
    sources: &InheritedSources<'_>,
) -> Result<(), NotationErrors> {
    let expected = [
        (
            DefinitionId::Comovfield,
            sources.symbols,
            INHERITED_SOURCE_DECLARATIONS[0].1,
            literal("S"),
        ),
        (
            DefinitionId::Readout,
            sources.historical,
            INHERITED_SOURCE_DECLARATIONS[1].1,
            primitive(PrimitiveId::Calligraphic, vec![literal("M")]),
        ),
        (
            DefinitionId::Willvirtue,
            sources.historical,
            INHERITED_SOURCE_DECLARATIONS[2].1,
            primitive(
                PrimitiveId::Superscript,
                vec![
                    primitive(PrimitiveId::Omega, vec![]),
                    primitive(PrimitiveId::UprightVir, vec![]),
                ],
            ),
        ),
        (
            DefinitionId::Gensym,
            sources.registers,
            INHERITED_SOURCE_DECLARATIONS[3].1,
            primitive(
                PrimitiveId::Subscript,
                vec![literal("A"), primitive(PrimitiveId::UprightS, vec![])],
            ),
        ),
        (
            DefinitionId::Alignrate,
            sources.historical,
            INHERITED_SOURCE_DECLARATIONS[4].1,
            primitive(
                PrimitiveId::Dot,
                vec![primitive(PrimitiveId::Sigma, vec![])],
            ),
        ),
        (
            DefinitionId::Willnet,
            sources.historical,
            INHERITED_SOURCE_DECLARATIONS[5].1,
            primitive(
                PrimitiveId::Subscript,
                vec![
                    primitive(PrimitiveId::Omega, vec![]),
                    primitive(PrimitiveId::UprightNet, vec![]),
                ],
            ),
        ),
        (
            DefinitionId::Willdecay,
            sources.historical,
            INHERITED_SOURCE_DECLARATIONS[6].1,
            primitive(
                PrimitiveId::TightJoin,
                vec![
                    primitive(PrimitiveId::Gamma, vec![]),
                    primitive(PrimitiveId::Sigma, vec![]),
                ],
            ),
        ),
    ];
    let expected_ids: BTreeSet<_> = expected.iter().map(|(id, ..)| *id).collect();
    let mut errors = vec![];
    for (id, source, bytes, body) in expected {
        let actual = definitions
            .iter()
            .find(|d| d.id == id)
            .expect("complete definitions");
        let sites = [sources.symbols, sources.historical, sources.registers]
            .into_iter()
            .map(|text| declaration_sites(text, id.token()))
            .sum::<usize>();
        let variance_sites = [sources.symbols, sources.historical, sources.registers]
            .into_iter()
            .map(|text| registered_variance_sites(text, id.token()))
            .sum::<usize>();
        if !matches!(actual.kind, DefinitionKind::Inherited { .. })
            || sites != 1
            || variance_sites != usize::from(id == DefinitionId::Comovfield)
            || !source_matches(source, bytes)
            || (id == DefinitionId::Comovfield
                && !source_matches(sources.registers, INHERITED_VARIANCE_DECLARATION))
            || actual.body() != Some(&body)
            || actual.signature.arity != 0
        {
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                site(
                    definitions
                        .iter()
                        .position(|d| d.id == id)
                        .expect("complete definitions"),
                    id,
                )
                .field("source"),
                NotationFault::SourceMismatch { definition: id },
            ));
        }
    }
    for (i, definition) in definitions.iter().enumerate() {
        if matches!(definition.kind, DefinitionKind::Inherited { .. })
            && !expected_ids.contains(&definition.id)
        {
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                site(i, definition.id).field("source"),
                NotationFault::SourceMismatch {
                    definition: definition.id,
                },
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
    inherited_source(definitions, &SOURCES)?;
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
                false,
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
            DefinitionKind::Canonical { body, .. }
            | DefinitionKind::Constructor { body }
            | DefinitionKind::Inherited { body } => (body, "body"),
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
    expanded_body_conflicts(definitions)?;
    Ok(edges)
}
