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
const PREPARE: &str = include_str!("../../../../tools/paper/prepare.py");
const TYPED_TEMPLATE: &str = r"    \exp_args:NNee \NewDocumentCommand #1
      { \prg_replicate:nn {#2} { m } }
      { \exp_not:V \l__ftoo_q_log_tl \exp_not:n {#3} }
";
const TYPED_REDECLARATIONS: &str = r"\cs_undefine:N \normGvec
\cs_undefine:N \normGcov
\cs_undefine:N \ipGvec
\cs_undefine:N \ipGcov
\cs_undefine:N \pairing

\NewDocumentCommand{\normGvec}{m}
  {
    \__ftoo_var_assert:nn {#1} {vector}
    \__ftoo_metric_spend:n {normGvec}
    \ensuremath{ \left\lVert #1 \right\rVert \sb{ G\sb{\star} } }
  }
\NewDocumentCommand{\normGcov}{m}
  {
    \__ftoo_var_assert:nn {#1} {covector}
    \__ftoo_metric_spend:n {normGcov}
    \ensuremath{ \left\lVert #1 \right\rVert \sb{ G\sb{\star}\sp{-1} } }
  }
\NewDocumentCommand{\ipGvec}{m m}
  {
    \__ftoo_var_assert:nn {#1} {vector}
    \__ftoo_var_assert:nn {#2} {vector}
    \__ftoo_metric_spend:n {ipGvec}
    \ensuremath{ \langle #1 , #2 \rangle \sb{ G\sb{\star} } }
  }
\NewDocumentCommand{\ipGcov}{m m}
  {
    \__ftoo_var_assert:nn {#1} {covector}
    \__ftoo_var_assert:nn {#2} {covector}
    \__ftoo_metric_spend:n {ipGcov}
    \ensuremath{ \langle #1 , #2 \rangle \sb{ G\sb{\star}\sp{-1} } }
  }
\NewDocumentCommand{\pairing}{m m}
  {
    \__ftoo_var_assert:nn {#1} {covector}
    \__ftoo_var_assert:nn {#2} {vector}
    \ensuremath{ \langle #1 , #2 \rangle }
  }";
// These are the quantity-store operations in the captured typed tools source.
// Every other inherited input has no operation on this store family.
const TYPED_STORE_SITES: &[&str] = &[
    "\\prop_get:NeNTF \\g__ftoo_qty_spend_prop { \\tl_to_str:n {##1} } \\l__ftoo_scratch_tl",
    "\\prop_new:N \\g__ftoo_qty_dim_prop",
    "\\prop_new:N \\g__ftoo_qty_var_prop",
    "\\prop_new:N \\g__ftoo_qty_spend_prop",
    "\\prop_new:N \\g__ftoo_qty_arity_prop",
    "\\prop_new:N \\g__ftoo_qty_body_prop",
    "\\prop_new:N \\g__ftoo_qty_bodyof_prop",
    "\\prop_get:NeNT \\g__ftoo_qty_body_prop { \\tl_to_str:n {#2} } \\l__ftoo_scratch_tl",
    "\\prop_if_in:NeF \\g__ftoo_qty_dim_prop { \\tl_to_str:N \\l__ftoo_q_alias_tl }",
    "\\prop_get:NeN \\g__ftoo_qty_bodyof_prop",
    "\\prop_gput:Nee \\g__ftoo_qty_body_prop   { \\tl_to_str:n {#2} } { \\tl_to_str:n {#1} }",
    "\\prop_gput:Nee \\g__ftoo_qty_bodyof_prop { \\tl_to_str:n {#1} } { \\tl_to_str:n {#2} }",
    "\\prop_gput:Nee \\g__ftoo_qty_dim_prop   { \\tl_to_str:n {#1} } { \\exp_not:V \\l__ftoo_q_dim_tl }",
    "\\prop_gput:NeV \\g__ftoo_qty_var_prop   { \\tl_to_str:n {#1} } \\l__ftoo_q_var_tl",
    "\\prop_gput:NeV \\g__ftoo_qty_spend_prop { \\tl_to_str:n {#1} } \\l__ftoo_q_spend_clist",
    "\\prop_gput:Nen \\g__ftoo_qty_arity_prop { \\tl_to_str:n {#1} } {#2}",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {#1} } {#2}",
    "\\prop_get:NeNTF \\g__ftoo_qty_var_prop",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\applyop} }    { vector }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\applycovop} } { covector }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\normGvec} }   { scalar }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\normGcov} }   { scalar }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\ipGvec} }     { scalar }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\ipGcov} }     { scalar }",
    "\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\pairing} }    { scalar }",
    "\\prop_get:NeNTF \\g__ftoo_qty_dim_prop",
    "\\prop_get:NeN \\g__ftoo_qty_arity_prop",
    "\\prop_get:NeNF \\g__ftoo_qty_var_prop",
    "\\prop_get:NeNTF \\g__ftoo_qty_dim_prop { \\tl_to_str:n {#1} } \\l__ftoo_scratch_tl",
];
pub(super) static OTHER_INPUTS: &[(&str, &str)] = &[
    (
        "compat.tex",
        include_str!("../../../../documents/paper-build/compat.tex"),
    ),
    (
        "main.tex",
        include_str!("../../../../documents/paper/main.tex"),
    ),
    (
        "ftoo_manifest.tex",
        include_str!("../../../../documents/paper/data/ftoo_manifest.tex"),
    ),
    (
        "macros_overlap.tex",
        include_str!("../../../../documents/paper/macros_overlap.tex"),
    ),
    (
        "reader-hooks.tex",
        include_str!("../../../../documents/paper-build/reader-hooks.tex"),
    ),
    (
        "ftoo_typed_tools.tex",
        include_str!("../../../../documents/paper/ftoo_typed_tools.tex"),
    ),
    (
        "econ_membrane.tex",
        include_str!("../../../../documents/paper/econ_membrane.tex"),
    ),
    (
        "front.tex",
        include_str!("../../../../documents/paper/parts/front.tex"),
    ),
    (
        "partA.tex",
        include_str!("../../../../documents/paper/parts/partA.tex"),
    ),
    (
        "partB.tex",
        include_str!("../../../../documents/paper/parts/partB.tex"),
    ),
    (
        "gateways.tex",
        include_str!("../../../../documents/paper/parts/gateways.tex"),
    ),
    (
        "partC.tex",
        include_str!("../../../../documents/paper/parts/partC.tex"),
    ),
    (
        "partD.tex",
        include_str!("../../../../documents/paper/parts/partD.tex"),
    ),
    (
        "partE.tex",
        include_str!("../../../../documents/paper/parts/partE.tex"),
    ),
    (
        "partF.tex",
        include_str!("../../../../documents/paper/parts/partF.tex"),
    ),
    (
        "partG.tex",
        include_str!("../../../../documents/paper/parts/partG.tex"),
    ),
    (
        "coda.tex",
        include_str!("../../../../documents/paper/parts/coda.tex"),
    ),
    (
        "appendices.tex",
        include_str!("../../../../documents/paper/parts/appendices.tex"),
    ),
    (
        "ftoo_index.tex",
        include_str!("../../../../documents/paper/data/ftoo_index.tex"),
    ),
    (
        "main_explainer.tex.in",
        include_str!("../../../../documents/paper/main_explainer.tex.in"),
    ),
    (
        "explainer_displayskips.tex",
        include_str!("../../../../documents/paper/explainer_displayskips.tex"),
    ),
    (
        "explainer_strings_english.tex",
        include_str!("../../../../documents/paper/explainer_strings_english.tex"),
    ),
    (
        "paper_xref.tex.in",
        include_str!("../../../../documents/paper/paper_xref.tex.in"),
    ),
    (
        "explainer.tex",
        include_str!("../../../../documents/paper/explainer/explainer.tex"),
    ),
];
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
/// Retained source bytes for the closed decoded single-symbol declarations.
/// # Panics
/// Panics only if the compiled source ceases to decode after catalogue admission.
#[must_use]
pub fn inherited_decoded_declarations() -> Vec<(DefinitionId, String)> {
    super::inherited::specs(
        SYMBOLS,
        HISTORICAL,
        &super::primitives::catalogue(),
        &base_catalogue(),
    )
    .expect("admitted source declarations decode")
    .1
}
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
    Accountability,
    Anchorprior,
    Attenuation,
    Bendfn,
    Biforce,
    Colourcoordinate,
    Colourgainfamily,
    Colourspan,
    Cyclicshift,
    Decayrate,
    Edgeload,
    Fielddrift,
    Frameloading,
    Frameoffsets,
    Heldweight,
    HingeAlpha,
    HingeBeta,
    Horizon,
    Idcov,
    Jaccouple,
    Jacgrowth,
    Leanbare,
    Moralvel,
    Nonreprfrac,
    Recoveryfact,
    Reports,
    Residuals,
    Rewardfield,
    Rewardpotential,
    Rootofunity,
    Will,
    WoodburyJay,
    Woodburyd,
}
impl DefinitionId {
    pub const ALL: [Self; 47] = [
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
        Self::Accountability,
        Self::Anchorprior,
        Self::Attenuation,
        Self::Bendfn,
        Self::Biforce,
        Self::Colourcoordinate,
        Self::Colourgainfamily,
        Self::Colourspan,
        Self::Cyclicshift,
        Self::Decayrate,
        Self::Edgeload,
        Self::Fielddrift,
        Self::Frameloading,
        Self::Frameoffsets,
        Self::Heldweight,
        Self::HingeAlpha,
        Self::HingeBeta,
        Self::Horizon,
        Self::Idcov,
        Self::Jaccouple,
        Self::Jacgrowth,
        Self::Leanbare,
        Self::Moralvel,
        Self::Nonreprfrac,
        Self::Recoveryfact,
        Self::Reports,
        Self::Residuals,
        Self::Rewardfield,
        Self::Rewardpotential,
        Self::Rootofunity,
        Self::Will,
        Self::WoodburyJay,
        Self::Woodburyd,
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
            Self::Accountability => "accountability",
            Self::Anchorprior => "anchorprior",
            Self::Attenuation => "attenuation",
            Self::Bendfn => "bendfn",
            Self::Biforce => "biforce",
            Self::Colourcoordinate => "colourcoordinate",
            Self::Colourgainfamily => "colourgainfamily",
            Self::Colourspan => "colourspan",
            Self::Cyclicshift => "cyclicshift",
            Self::Decayrate => "decayrate",
            Self::Edgeload => "edgeload",
            Self::Fielddrift => "fielddrift",
            Self::Frameloading => "frameloading",
            Self::Frameoffsets => "frameoffsets",
            Self::Heldweight => "heldweight",
            Self::HingeAlpha => "hingeA",
            Self::HingeBeta => "hingeB",
            Self::Horizon => "horizon",
            Self::Idcov => "idcov",
            Self::Jaccouple => "jaccouple",
            Self::Jacgrowth => "jacgrowth",
            Self::Leanbare => "leanbare",
            Self::Moralvel => "moralvel",
            Self::Nonreprfrac => "nonreprfrac",
            Self::Recoveryfact => "recoveryfact",
            Self::Reports => "reports",
            Self::Residuals => "residuals",
            Self::Rewardfield => "rewardfield",
            Self::Rewardpotential => "rewardpotential",
            Self::Rootofunity => "rootofunity",
            Self::Will => "will",
            Self::WoodburyJay => "woodburyJ",
            Self::Woodburyd => "woodburyd",
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
    pub(crate) fn command(&self) -> String {
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
#[allow(clippy::too_many_lines)] // The closed legacy catalogue keeps reviewed pairings together.
fn base_catalogue() -> Vec<DefinitionSpec> {
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
pub(super) fn try_catalogue() -> Result<Vec<DefinitionSpec>, NotationErrors> {
    let primitives = super::primitives::catalogue();
    let mut legacy = base_catalogue();
    let decoded = super::inherited::decode(
        SYMBOLS,
        DefinitionId::Comovfield.token(),
        &primitives,
        &legacy,
    );
    if let Ok((_, body)) = decoded {
        if body == literal("S") {
            let spec = legacy
                .iter_mut()
                .find(|spec| spec.id == DefinitionId::Comovfield)
                .expect("legacy pairing");
            spec.kind = DefinitionKind::Inherited { body };
        } else {
            return Err(source_fault(DefinitionId::Comovfield));
        }
    } else {
        return Err(source_fault(DefinitionId::Comovfield));
    }
    super::inherited::specs(SYMBOLS, HISTORICAL, &primitives, &legacy)
        .map(|(definitions, _)| definitions)
        .map_err(|(id, _)| source_fault(id))
}
fn source_fault(id: DefinitionId) -> NotationErrors {
    let index = DefinitionId::ALL
        .iter()
        .position(|entry| *entry == id)
        .unwrap_or(0);
    let historical = super::inherited::SYMBOLS
        .iter()
        .any(|(candidate, _, file)| {
            *candidate == id && matches!(file, super::inherited::SourceFile::Historical)
        });
    let (file, source) = if historical {
        (
            "documents/paper/data/ftoo_symbols_historical.tex",
            HISTORICAL,
        )
    } else {
        ("documents/paper/data/ftoo_symbols.tex", SYMBOLS)
    };
    NotationErrors::from_diagnostics(vec![diagnostic(
        RuleId::M078,
        NotationStage::Catalogue,
        inherited_site(index, id, file, source),
        NotationFault::SourceMismatch { definition: id },
    )])
    .expect("one source mismatch")
}
#[cfg(test)]
pub(super) fn catalogue() -> Vec<DefinitionSpec> {
    try_catalogue().expect("retained source declarations decode")
}
pub(super) fn site(index: usize, id: DefinitionId) -> NotationSite {
    registry_site(FILE, RegistryEntry::Definition(id), index)
}
fn inherited_site(index: usize, id: DefinitionId, file: &str, source: &str) -> NotationSite {
    NotationSite::Registry {
        file: file.into(),
        entry: RegistryEntry::Definition(id),
        index,
        path: vec![super::NotationPathPart::Field("source".into())],
        source_range: super::inherited::declaration_range(source, id.token()),
    }
}
fn registry_site(file: &str, entry: RegistryEntry, index: usize) -> NotationSite {
    NotationSite::Registry {
        file: file.into(),
        entry,
        index,
        path: vec![],
        source_range: None,
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
            } else {
                result.push_str(&" ".repeat(scalar.len_utf8()));
            }
            continue;
        }
        if scalar == '%' && !escaped {
            comment = true;
            result.push(' ');
            continue;
        }
        result.push(scalar);
        escaped = scalar == '\\' && !escaped;
    }
    result
}
fn skip_expandafter(mut source: &str) -> &str {
    while let Some(rest) = source.trim_start().strip_prefix("\\expandafter") {
        source = rest;
    }
    source
}
fn declaration_target(source: &str) -> Option<&str> {
    let source = skip_expandafter(source).trim_start_matches(char::is_whitespace);
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
fn declaration_positions(source: &str, name: &str) -> Vec<usize> {
    if !source.contains(name) {
        return Vec::new();
    }
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
    ]
    .into_iter()
    .flat_map(|command| {
        source
            .match_indices(command)
            .filter(|(at, _)| {
                let tail = source[*at + command.len()..].trim_start();
                declaration_target(tail) == Some(name)
            })
            .map(|(at, _)| at)
            .collect::<Vec<_>>()
    })
    .chain(source.match_indices("\\cs_").filter_map(|(at, _)| {
        let rest = &source[at..];
        let command = declared_name(rest)?;
        let (stem, signature) = command.split_once(':')?;
        if !["cs_set", "cs_gset", "cs_undefine", "cs_gundefine"]
            .iter()
            .any(|prefix| stem.starts_with(prefix))
        {
            return None;
        }
        let tail = rest[command.len() + 1..].trim_start();
        let target = match signature.as_bytes().first() {
            Some(b'N') => declaration_target(tail),
            Some(b'c') => tail
                .strip_prefix('{')?
                .split_once('}')
                .map(|(name, _)| name.trim()),
            _ => None,
        };
        (target == Some(name)).then_some(at)
    }))
    .collect()
}
fn declaration_sites(source: &str, name: &str) -> usize {
    declaration_positions(source, name).len()
}
fn quantity_property_put_keys(source: &str) -> Vec<&str> {
    source
        .match_indices("\\prop_")
        .filter_map(|(at, _)| {
            let rest = &source[at..];
            let command_end = rest.find(char::is_whitespace)?;
            let command = &rest[..command_end];
            if !command.starts_with("\\prop_gput:") && !command.starts_with("\\prop_put:") {
                return None;
            }
            let property = rest[command_end..]
                .trim_start()
                .strip_prefix("\\g__ftoo_qty_")?;
            let (_, key) = property.split_once("_prop")?;
            let (range, _) = super::inherited::group(key, 0)?;
            Some(&key[range])
        })
        .collect()
}
fn quantity_store_sites(source: &str) -> Vec<(usize, &str)> {
    source
        .match_indices("\\prop_")
        .filter_map(|(at, _)| {
            let rest = &source[at..];
            let command_end = 1 + rest[1..]
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b':'))
                .count();
            let argument = rest[command_end..].trim_start_matches(char::is_whitespace);
            (argument.starts_with("\\g__ftoo_qty_")
                || argument.starts_with("{g__ftoo_qty_")
                || argument.starts_with("\\csname g__ftoo_qty_"))
            .then(|| {
                let line_end = rest.find('\n').unwrap_or(rest.len());
                (at, &rest[..line_end])
            })
        })
        .collect()
}
fn dynamic_definition_sites(source: &str) -> Vec<(usize, &str)> {
    source
        .match_indices('\\')
        .filter_map(|(at, _)| {
            let rest = &source[at..];
            let command = declared_name(rest)?;
            let l3_definition = ["cs_set", "cs_gset", "cs_undefine", "cs_gundefine"]
                .iter()
                .any(|stem| command.starts_with(stem));
            let other_definition = [
                "NewDocumentCommand",
                "RenewDocumentCommand",
                "DeclareDocumentCommand",
                "ProvideDocumentCommand",
                "NewExpandableDocumentCommand",
                "RenewExpandableDocumentCommand",
                "DeclareExpandableDocumentCommand",
                "ProvideExpandableDocumentCommand",
                "DeclareRobustCommand",
                "NewCommandCopy",
                "RenewCommandCopy",
                "DeclareCommandCopy",
                "DeclareQuantity",
                "newcommand",
                "renewcommand",
                "providecommand",
                "def",
                "gdef",
                "edef",
                "xdef",
                "let",
            ]
            .contains(&command);
            let tail = skip_expandafter(&rest[command.len() + 1..]).trim_start();
            let dynamic_target = {
                let tail = tail.strip_prefix('*').unwrap_or(tail).trim_start();
                if tail.starts_with('{') {
                    super::inherited::group(tail, 0).is_some_and(|(range, _)| {
                        let target = tail[range].trim();
                        let name = target.strip_prefix('\\').unwrap_or(target);
                        name.is_empty()
                            || !name.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'@')
                            })
                    })
                } else if let Some(rest) = tail.strip_prefix("\\csname") {
                    csname_name(rest).is_none()
                } else {
                    tail.starts_with('#')
                }
            };
            let controlled_target = preceded_by_exp_args(source, at);
            ((l3_definition || other_definition) && (dynamic_target || controlled_target)).then(
                || {
                    let end = rest.find('\n').unwrap_or(rest.len());
                    (at, &rest[..end])
                },
            )
        })
        .collect()
}
fn preceded_by_exp_args(source: &str, at: usize) -> bool {
    source[..at]
        .trim_end()
        .rsplit_once('\\')
        .is_some_and(|(_, prior)| {
            prior.starts_with("exp_args:")
                && prior
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
        })
}
fn literal_registration_target(tail: &str) -> bool {
    let tail = skip_expandafter(tail).trim_start();
    if tail.starts_with('{') {
        let Some((range, _)) = super::inherited::group(tail, 0) else {
            return false;
        };
        let target = tail[range].trim();
        if let Some(name) = declared_name(target) {
            name != "csname" && target == format!("\\{name}")
        } else {
            !target.is_empty()
                && target
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        }
    } else {
        declared_name(tail).is_some_and(|name| name != "csname")
    }
}
fn nonliteral_registration_sites(source: &str) -> Vec<(usize, &str)> {
    ["\\RegisterVariance", "\\DeclareMetricPair"]
        .into_iter()
        .flat_map(|command| {
            source
                .match_indices(command)
                .filter_map(move |(at, _)| {
                    if declared_name(&source[at..]) != Some(&command[1..])
                        || source[..at]
                            .strip_suffix("\\string")
                            .is_some_and(|before| !before.ends_with('\\'))
                    {
                        return None;
                    }
                    let preceding = source[..at].rsplit('\n').next().unwrap_or("");
                    let verbatim = preceding.rsplit_once("\\verb").is_some_and(|(_, suffix)| {
                        let delimiter = suffix.strip_prefix('*').unwrap_or(suffix);
                        delimiter.len() == 1 && source[at + command.len()..].starts_with(delimiter)
                    });
                    if verbatim {
                        return None;
                    }
                    let tail = &source[at + command.len()..];
                    (!literal_registration_target(tail) || preceded_by_exp_args(source, at)).then(
                        || {
                            let end = source[at..].find('\n').map_or(source.len(), |end| at + end);
                            (at, &source[at..end])
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
fn quantity_site_definition(site: &str) -> DefinitionId {
    site.match_indices('\\')
        .filter_map(|(at, _)| declared_name(&site[at..]))
        .find_map(|name| {
            DefinitionId::ALL
                .into_iter()
                .find(|id| id.token() == name)
                .or_else(|| {
                    GREEK_DEPENDENCIES
                        .iter()
                        .find(|(symbol, _)| *symbol == name)
                        .map(|(_, id)| *id)
                })
        })
        .unwrap_or(DefinitionId::Will)
}
fn registered_variance_sites(source: &str, name: &str) -> usize {
    if !source.contains(name) {
        return 0;
    }
    let source = uncommented(source);
    let named = ["\\RegisterVariance", "\\DeclareMetricPair"]
        .into_iter()
        .map(|command| {
            source
                .match_indices(command)
                .filter(|(at, _)| {
                    declared_name(skip_expandafter(&source[*at + command.len()..])) == Some(name)
                })
                .count()
        })
        .sum::<usize>();
    let direct = quantity_property_put_keys(&source)
        .into_iter()
        .filter(|key| {
            key.match_indices('\\')
                .any(|(offset, _)| declared_name(&key[offset..]) == Some(name))
        })
        .count();
    named + direct
}
const GREEK_DEPENDENCIES: &[(&str, DefinitionId)] = &[
    ("alpha", DefinitionId::Jacgrowth),
    ("beta", DefinitionId::Biforce),
    ("omega", DefinitionId::Will),
    ("sigma", DefinitionId::Leanbare),
    ("gamma", DefinitionId::Decayrate),
    ("Omega", DefinitionId::Idcov),
    ("Xi", DefinitionId::Colourspan),
    ("chi", DefinitionId::Colourcoordinate),
    ("eta", DefinitionId::Attenuation),
    ("lambda", DefinitionId::Heldweight),
    ("mu", DefinitionId::Recoveryfact),
    ("phi", DefinitionId::Bendfn),
    ("tau", DefinitionId::Horizon),
    ("theta", DefinitionId::Frameoffsets),
    ("varepsilon", DefinitionId::Residuals),
    ("zeta", DefinitionId::Rootofunity),
];
fn other_counts(other_inputs: &[(&str, &str)], name: &str) -> (usize, usize) {
    use std::sync::OnceLock;
    static RETAINED: OnceLock<BTreeMap<&'static str, (usize, usize)>> = OnceLock::new();
    if std::ptr::eq(other_inputs, OTHER_INPUTS) {
        return *RETAINED
            .get_or_init(|| {
                DefinitionId::ALL
                    .into_iter()
                    .map(|id| {
                        let name = id.token();
                        (
                            name,
                            (
                                OTHER_INPUTS
                                    .iter()
                                    .map(|(_, source)| declaration_sites(source, name))
                                    .sum(),
                                OTHER_INPUTS
                                    .iter()
                                    .map(|(_, source)| registered_variance_sites(source, name))
                                    .sum(),
                            ),
                        )
                    })
                    .collect()
            })
            .get(name)
            .expect("closed name");
    }
    (
        other_inputs
            .iter()
            .map(|(_, source)| declaration_sites(source, name))
            .sum(),
        other_inputs
            .iter()
            .map(|(_, source)| registered_variance_sites(source, name))
            .sum(),
    )
}
#[allow(clippy::too_many_lines)] // One comparison keeps all admitted source declarations and bodies together.
pub(super) fn inherited_source(
    definitions: &[DefinitionSpec],
    sources: &InheritedSources<'_>,
) -> Result<(), NotationErrors> {
    inherited_source_with_inputs(definitions, sources, OTHER_INPUTS)
}
#[allow(clippy::too_many_lines)] // The closure, interfaces and legacy pairings share one comparison result.
pub(super) fn inherited_source_with_inputs(
    definitions: &[DefinitionSpec],
    sources: &InheritedSources<'_>,
    other_inputs: &[(&str, &str)],
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
    let expected_ids: BTreeSet<_> = expected
        .iter()
        .map(|(id, ..)| *id)
        .chain(super::inherited::SYMBOLS.iter().map(|(id, ..)| *id))
        .collect();
    let named_sources = [
        ("ftoo_symbols.tex", sources.symbols),
        ("ftoo_symbols_historical.tex", sources.historical),
        ("ftoo_registers.tex", sources.registers),
    ];
    let mut errors = vec![];
    for (path, source) in other_inputs.iter().copied().chain(named_sources) {
        if path == "ftoo_typed_tools.tex" && !source_matches(source, TYPED_TEMPLATE) {
            let id = DefinitionId::Will;
            let at = source
                .find("\\exp_args:NNee \\NewDocumentCommand #1")
                .unwrap_or(0);
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Registry {
                    file: path.into(),
                    entry: RegistryEntry::Definition(id),
                    index: definitions
                        .iter()
                        .position(|definition| definition.id == id)
                        .expect("complete definitions"),
                    path: vec![super::NotationPathPart::Field("source".into())],
                    source_range: Some(at..at + "\\exp_args:NNee".len()),
                },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
        if path == "ftoo_typed_tools.tex" && !source_matches(source, TYPED_REDECLARATIONS) {
            let id = DefinitionId::Will;
            let at = source.find("\\cs_undefine:N \\normGvec").unwrap_or(0);
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Registry {
                    file: path.into(),
                    entry: RegistryEntry::Definition(id),
                    index: definitions
                        .iter()
                        .position(|definition| definition.id == id)
                        .expect("complete definitions"),
                    path: vec![super::NotationPathPart::Field("source".into())],
                    source_range: Some(at..at + "\\cs_undefine:N".len()),
                },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
        let current_effects = uncommented(source);
        for (at, effect) in dynamic_definition_sites(&current_effects) {
            if path == "ftoo_typed_tools.tex" && effect.trim_end() == "\\NewDocumentCommand #1" {
                continue;
            }
            let id = quantity_site_definition(effect);
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Registry {
                    file: path.into(),
                    entry: RegistryEntry::Definition(id),
                    index: definitions
                        .iter()
                        .position(|definition| definition.id == id)
                        .expect("complete definitions"),
                    path: vec![super::NotationPathPart::Field("source".into())],
                    source_range: Some(at..at + 1),
                },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
        if path == "ftoo_typed_tools.tex"
            && dynamic_definition_sites(&current_effects)
                .into_iter()
                .filter(|(_, effect)| effect.trim_end() == "\\NewDocumentCommand #1")
                .count()
                != 1
        {
            let id = DefinitionId::Will;
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Catalogue { file: path.into() },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
        if other_inputs
            .iter()
            .any(|(other_path, _)| *other_path == path)
        {
            for (name, id) in definitions
                .iter()
                .filter(|definition| matches!(&definition.kind, DefinitionKind::Inherited { .. }))
                .map(|definition| (definition.id.token(), definition.id))
                .chain(GREEK_DEPENDENCIES.iter().copied())
            {
                let mut positions = declaration_positions(source, name);
                positions.sort_unstable();
                for at in positions {
                    errors.push(diagnostic(
                        RuleId::M078,
                        NotationStage::Catalogue,
                        NotationSite::Registry {
                            file: path.into(),
                            entry: RegistryEntry::Definition(id),
                            index: definitions
                                .iter()
                                .position(|definition| definition.id == id)
                                .expect("complete definitions"),
                            path: vec![super::NotationPathPart::Field("source".into())],
                            source_range: Some(at..at + 1),
                        },
                        NotationFault::SourceMismatch { definition: id },
                    ));
                }
            }
        }
        let mut retained: BTreeMap<&str, usize> = BTreeMap::new();
        if path == "ftoo_typed_tools.tex" {
            for &site in TYPED_STORE_SITES {
                *retained.entry(site).or_default() += 1;
            }
        }
        let mut mismatch = None;
        for (at, site) in quantity_store_sites(&current_effects) {
            let count = retained.entry(site.trim_end()).or_default();
            if *count == 0 {
                mismatch = Some((at, site));
                break;
            }
            *count -= 1;
        }
        if mismatch.is_none() && retained.values().any(|count| *count != 0) {
            mismatch = Some((source.find("\\prop_").unwrap_or(0), ""));
        }
        if let Some((at, site)) = mismatch {
            let id = quantity_site_definition(site);
            let index = definitions
                .iter()
                .position(|definition| definition.id == id)
                .expect("complete definitions");
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Registry {
                    file: path.into(),
                    entry: RegistryEntry::Definition(id),
                    index,
                    path: vec![super::NotationPathPart::Field("source".into())],
                    source_range: Some(at..at + "\\prop_".len()),
                },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
    }
    for (path, source) in other_inputs.iter().copied().chain(named_sources) {
        let current = uncommented(source);
        for (at, _) in nonliteral_registration_sites(&current) {
            let line_start = current[..at].rfind('\n').map_or(0, |index| index + 1);
            let line_end = current[at..]
                .find('\n')
                .map_or(current.len(), |end| at + end);
            let line = current[line_start..line_end].trim();
            if path == "ftoo_typed_tools.tex"
                && [
                    "\\NewDocumentCommand{\\RegisterVariance}{m m}",
                    "\\NewDocumentCommand{\\DeclareMetricPair}{m}",
                    "{ '#1'~needs~\\string\\DeclareMetricPair~first:~no~anchored~metric~is~declared. }",
                ]
                .contains(&line)
            {
                continue;
            }
            let id = DefinitionId::Will;
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                NotationSite::Registry {
                    file: path.into(),
                    entry: RegistryEntry::Definition(id),
                    index: definitions
                        .iter()
                        .position(|definition| definition.id == id)
                        .expect("complete definitions"),
                    path: vec![super::NotationPathPart::Field("source".into())],
                    source_range: Some(at..at + 1),
                },
                NotationFault::SourceMismatch { definition: id },
            ));
        }
    }
    for (path, source, markers) in [
        (
            "main.tex",
            other_inputs
                .iter()
                .find(|(path, _)| *path == "main.tex")
                .map(|(_, text)| *text),
            &[
                "\\input{data/ftoo_manifest.tex}",
                "\\input{data/ftoo_symbols.tex}",
                "\\input{data/ftoo_symbols_historical.tex}",
                "\\input{macros_overlap.tex}",
                "\\input{ftoo_typed_tools.tex}",
                "\\input{data/ftoo_registers.tex}",
                "\\input{econ_membrane.tex}",
                "\\subfile{parts/front}",
                "\\subfile{parts/partA}",
                "\\subfile{parts/partB}",
                "\\subfile{parts/gateways}",
                "\\subfile{parts/partC}",
                "\\subfile{parts/partD}",
                "\\subfile{parts/partE}",
                "\\subfile{parts/partF}",
                "\\subfile{parts/partG}",
                "\\subfile{parts/coda}",
                "\\subfile{parts/appendices}",
                "\\input{data/ftoo_index.tex}",
            ][..],
        ),
        (
            "main_explainer.tex.in",
            other_inputs
                .iter()
                .find(|(path, _)| *path == "main_explainer.tex.in")
                .map(|(_, text)| *text),
            &[
                "\\input{explainer_displayskips}",
                "\\input{data/ftoo_symbols.tex}",
                "\\input{data/ftoo_symbols_historical.tex}",
                "\\input{macros_overlap.tex}",
                "\\input{econ_membrane.tex}",
                "\\input{@LANG_STRINGS@}",
                "\\input{paper_xref}",
                "\\subfile{@SUBFILE@}",
            ][..],
        ),
    ] {
        if source.is_some_and(|source| !ordered_markers(source, markers)) {
            let id = DefinitionId::Will;
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
        let _ = path;
    }
    let compat = other_inputs.iter().find(|(path, _)| *path == "compat.tex");
    let hooks = other_inputs
        .iter()
        .find(|(path, _)| *path == "reader-hooks.tex");
    if compat.is_none()
        || hooks.is_none()
        || !source_matches(
            PREPARE,
            "shutil.copyfile(root/'compat.tex',work/'compat.tex')",
        )
        || !source_matches(
            PREPARE,
            "write(work/'main_explainer.tex', '\\\\input{compat.tex}\\n'+master)",
        )
        || !source_matches(
            PREPARE,
            "write(work/'main.tex', '\\\\input{compat.tex}\\n'+paper_master(read(work/'main.tex'), templates['version']))",
        )
        || !compat.is_some_and(|(_, source)| {
            source_matches(
                source,
                "\\AddToHook{file/macros_overlap.tex/after}{\\input{reader-hooks.tex}}",
            )
        })
    {
        let id = DefinitionId::Will;
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
    for (path, source) in other_inputs.iter().copied().chain(named_sources) {
        let expected_markers: &[&str] = match path {
            "compat.tex" => &["\\input{reader-hooks.tex}"],
            "reader-hooks.tex" => &["\\input{reader-lookup.tex}"],
            _ => &[],
        };
        if path != "main.tex"
            && path != "main_explainer.tex.in"
            && !ordered_markers(source, expected_markers)
        {
            let id = DefinitionId::Will;
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
    for (id, source, bytes, body) in expected {
        let actual = definitions
            .iter()
            .find(|d| d.id == id)
            .expect("complete definitions");
        let (other_sites, other_variance) = other_counts(other_inputs, id.token());
        let sites = other_sites
            + [sources.symbols, sources.historical, sources.registers]
                .into_iter()
                .map(|text| declaration_sites(text, id.token()))
                .sum::<usize>();
        let variance_sites = other_variance
            + [sources.symbols, sources.historical, sources.registers]
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
    for &(id, _, file) in &super::inherited::SYMBOLS {
        let source = match file {
            super::inherited::SourceFile::Symbols => sources.symbols,
            super::inherited::SourceFile::Historical => sources.historical,
        };
        let actual = definitions
            .iter()
            .find(|d| d.id == id)
            .expect("complete definitions");
        let decoded = super::inherited::decode(
            source,
            id.token(),
            &super::primitives::catalogue(),
            definitions,
        );
        let (other_sites, other_variance) = other_counts(other_inputs, id.token());
        let sites = other_sites
            + [sources.symbols, sources.historical, sources.registers]
                .into_iter()
                .map(|text| declaration_sites(text, id.token()))
                .sum::<usize>();
        let variance_sites = other_variance
            + [sources.symbols, sources.historical, sources.registers]
                .into_iter()
                .map(|text| registered_variance_sites(text, id.token()))
                .sum::<usize>();
        if !matches!(actual.kind, DefinitionKind::Inherited { .. })
            || sites != 1
            || variance_sites != 0
            || decoded
                .as_ref()
                .map_or(true, |(_, body)| actual.body() != Some(body))
            || actual.signature.arity != 0
        {
            let source_file = match file {
                super::inherited::SourceFile::Symbols => "documents/paper/data/ftoo_symbols.tex",
                super::inherited::SourceFile::Historical => {
                    "documents/paper/data/ftoo_symbols_historical.tex"
                }
            };
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                inherited_site(
                    definitions
                        .iter()
                        .position(|d| d.id == id)
                        .expect("complete definitions"),
                    id,
                    source_file,
                    source,
                ),
                NotationFault::SourceMismatch { definition: id },
            ));
        }
    }
    for &(name, owner) in GREEK_DEPENDENCIES {
        if [sources.symbols, sources.historical, sources.registers]
            .into_iter()
            .chain(other_inputs.iter().map(|(_, source)| *source))
            .any(|source| declaration_sites(source, name) != 0)
        {
            errors.push(diagnostic(
                RuleId::M078,
                NotationStage::Catalogue,
                site(
                    definitions
                        .iter()
                        .position(|d| d.id == owner)
                        .expect("complete definitions"),
                    owner,
                )
                .field("source"),
                NotationFault::SourceMismatch { definition: owner },
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
fn ordered_markers(source: &str, markers: &[&str]) -> bool {
    load_markers(source) == markers
}
fn load_markers(source: &str) -> Vec<String> {
    let source = uncommented(source);
    let mut markers = Vec::new();
    for (at, _) in source.match_indices('\\') {
        let rest = &source[at..];
        for command in ["input", "subfile", "include", "InputIfFileExists"] {
            let prefix = format!("\\{command}");
            if !rest.starts_with(&prefix)
                || rest
                    .as_bytes()
                    .get(prefix.len())
                    .is_some_and(u8::is_ascii_alphabetic)
            {
                continue;
            }
            let tail = rest[prefix.len()..].trim_start_matches(char::is_whitespace);
            if let Some(after_open) = tail.strip_prefix('{')
                && let Some(end) = after_open.find('}')
            {
                markers.push(format!("{prefix}{{{}}}", &after_open[..end]));
                continue;
            }
            markers.push(prefix);
        }
    }
    markers
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
