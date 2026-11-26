use super::Signature;

/// Closed semantic operations. No operation accepts authored renderer text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveOperation {
    Alpha,
    Beta,
    Fraction,
    Subscript,
    Superscript,
    Add,
    Equal,
    Negate,
    Multiply,
    Juxtapose,
    Parentheses,
    Dimension,
    Omega,
    Calligraphic,
    UprightVir,
    UprightS,
    Dot,
    Sigma,
    Gamma,
    Times,
    Subtract,
    TightJoin,
    Underbrace,
    UprightNet,
    CapitalOmega,
    CapitalXi,
    Chi,
    Eta,
    Lambda,
    Mu,
    Phi,
    Tau,
    Theta,
    VariantEpsilon,
    Zeta,
    Nabla,
    LessEqual,
}
impl PrimitiveOperation {
    /// Operations whose nullary spelling is an independent mathematical symbol.
    #[must_use]
    pub const fn is_symbol(self) -> bool {
        matches!(
            self,
            Self::Alpha
                | Self::Beta
                | Self::Omega
                | Self::Sigma
                | Self::Gamma
                | Self::CapitalOmega
                | Self::CapitalXi
                | Self::Chi
                | Self::Eta
                | Self::Lambda
                | Self::Mu
                | Self::Phi
                | Self::Tau
                | Self::Theta
                | Self::VariantEpsilon
                | Self::Zeta
                | Self::Nabla
        )
    }
    /// Closed host control tokens used by the renderer and the future shared
    /// mathematics lexer. Grouping punctuation is supplied by the typed writer.
    #[must_use]
    pub const fn host_tokens(self) -> &'static [&'static str] {
        match self {
            Self::Alpha => &["\\alpha"],
            Self::Beta => &["\\beta"],
            Self::Omega => &["\\omega"],
            Self::Calligraphic => &["\\symcal"],
            Self::UprightVir | Self::UprightS | Self::UprightNet => &["\\mathrm"],
            Self::Dot => &["\\dot"],
            Self::Sigma => &["\\sigma"],
            Self::Gamma => &["\\gamma"],
            Self::Times => &["\\times"],
            Self::Underbrace => &["\\underbrace"],
            Self::CapitalOmega => &["\\Omega"],
            Self::CapitalXi => &["\\Xi"],
            Self::Chi => &["\\chi"],
            Self::Eta => &["\\eta"],
            Self::Lambda => &["\\lambda"],
            Self::Mu => &["\\mu"],
            Self::Phi => &["\\phi"],
            Self::Tau => &["\\tau"],
            Self::Theta => &["\\theta"],
            Self::VariantEpsilon => &["\\varepsilon"],
            Self::Zeta => &["\\zeta"],
            Self::Nabla => &["\\nabla"],
            Self::LessEqual => &["\\le"],
            Self::Fraction => &["\\frac"],
            Self::Multiply => &["\\cdot"],
            Self::Juxtapose => &["\\,"],
            Self::Subtract
            | Self::TightJoin
            | Self::Subscript
            | Self::Superscript
            | Self::Add
            | Self::Equal
            | Self::Negate
            | Self::Parentheses
            | Self::Dimension => &[],
        }
    }
    /// Preferred Unicode spelling where the operation has a single scalar.
    #[must_use]
    pub const fn unicode_scalar(self) -> Option<&'static str> {
        match self {
            Self::Alpha => Some("α"),
            Self::Beta => Some("β"),
            Self::Omega => Some("ω"),
            Self::Negate => Some("−"),
            Self::Multiply => Some("⋅"),
            Self::Sigma => Some("σ"),
            Self::Gamma => Some("γ"),
            Self::Times => Some("×"),
            Self::CapitalOmega => Some("Ω"),
            Self::CapitalXi => Some("Ξ"),
            Self::Chi => Some("χ"),
            Self::Eta => Some("η"),
            Self::Lambda => Some("λ"),
            Self::Mu => Some("μ"),
            Self::Phi => Some("ϕ"),
            Self::Tau => Some("τ"),
            Self::Theta => Some("θ"),
            Self::VariantEpsilon => Some("ε"),
            Self::Zeta => Some("ζ"),
            Self::Nabla => Some("∇"),
            Self::LessEqual => Some("≤"),
            Self::Fraction
            | Self::Calligraphic
            | Self::UprightVir
            | Self::UprightS
            | Self::Subscript
            | Self::Superscript
            | Self::Add
            | Self::Equal
            | Self::Juxtapose
            | Self::Parentheses
            | Self::Dimension
            | Self::Dot
            | Self::Subtract
            | Self::TightJoin
            | Self::Underbrace
            | Self::UprightNet => None,
        }
    }
    #[must_use]
    pub const fn arity(self) -> u8 {
        match self {
            Self::Alpha
            | Self::Beta
            | Self::Omega
            | Self::UprightVir
            | Self::UprightS
            | Self::Sigma
            | Self::Gamma
            | Self::UprightNet
            | Self::CapitalOmega
            | Self::CapitalXi
            | Self::Chi
            | Self::Eta
            | Self::Lambda
            | Self::Mu
            | Self::Phi
            | Self::Tau
            | Self::Theta
            | Self::VariantEpsilon
            | Self::Zeta
            | Self::Nabla => 0,
            Self::Calligraphic | Self::Negate | Self::Parentheses | Self::Dimension | Self::Dot => {
                1
            }
            Self::Fraction
            | Self::Subscript
            | Self::Superscript
            | Self::Add
            | Self::Equal
            | Self::Multiply
            | Self::Juxtapose
            | Self::Times
            | Self::Subtract
            | Self::TightJoin
            | Self::Underbrace
            | Self::LessEqual => 2,
        }
    }
}

/// Strings cannot extend the primitive vocabulary.
/// ```compile_fail,E0599
/// let id = ftoo_manifest::PrimitiveId::Raw;
/// ```
/// ```compile_fail,E0308
/// let id: ftoo_manifest::PrimitiveId = "alpha";
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrimitiveId {
    Alpha,
    Beta,
    Fraction,
    Subscript,
    Superscript,
    Add,
    Equal,
    Negate,
    Multiply,
    Juxtapose,
    Parentheses,
    Dimension,
    Omega,
    Calligraphic,
    UprightVir,
    UprightS,
    Dot,
    Sigma,
    Gamma,
    Times,
    Subtract,
    TightJoin,
    Underbrace,
    UprightNet,
    CapitalOmega,
    CapitalXi,
    Chi,
    Eta,
    Lambda,
    Mu,
    Phi,
    Tau,
    Theta,
    VariantEpsilon,
    Zeta,
    Nabla,
    LessEqual,
}
impl PrimitiveId {
    pub const ALL: [Self; 37] = [
        Self::Alpha,
        Self::Beta,
        Self::Fraction,
        Self::Subscript,
        Self::Superscript,
        Self::Add,
        Self::Equal,
        Self::Negate,
        Self::Multiply,
        Self::Juxtapose,
        Self::Parentheses,
        Self::Dimension,
        Self::Omega,
        Self::Calligraphic,
        Self::UprightVir,
        Self::UprightS,
        Self::Dot,
        Self::Sigma,
        Self::Gamma,
        Self::Times,
        Self::Subtract,
        Self::TightJoin,
        Self::Underbrace,
        Self::UprightNet,
        Self::CapitalOmega,
        Self::CapitalXi,
        Self::Chi,
        Self::Eta,
        Self::Lambda,
        Self::Mu,
        Self::Phi,
        Self::Tau,
        Self::Theta,
        Self::VariantEpsilon,
        Self::Zeta,
        Self::Nabla,
        Self::LessEqual,
    ];
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Fraction => "fraction",
            Self::Subscript => "subscript",
            Self::Superscript => "superscript",
            Self::Add => "add",
            Self::Equal => "equal",
            Self::Negate => "negate",
            Self::Multiply => "multiply",
            Self::Juxtapose => "juxtapose",
            Self::Parentheses => "parentheses",
            Self::Dimension => "dimension",
            Self::Omega => "omega",
            Self::Calligraphic => "calligraphic",
            Self::UprightVir => "upright-vir",
            Self::UprightS => "upright-s",
            Self::Dot => "dot",
            Self::Sigma => "sigma",
            Self::Gamma => "gamma",
            Self::Times => "times",
            Self::Subtract => "subtract",
            Self::TightJoin => "tight-join",
            Self::Underbrace => "underbrace",
            Self::UprightNet => "upright-net",
            Self::CapitalOmega => "capital-omega",
            Self::CapitalXi => "capital-xi",
            Self::Chi => "chi",
            Self::Eta => "eta",
            Self::Lambda => "lambda",
            Self::Mu => "mu",
            Self::Phi => "phi",
            Self::Tau => "tau",
            Self::Theta => "theta",
            Self::VariantEpsilon => "variant-epsilon",
            Self::Zeta => "zeta",
            Self::Nabla => "nabla",
            Self::LessEqual => "less-equal",
        }
    }
    #[must_use]
    pub const fn operation(self) -> PrimitiveOperation {
        match self {
            Self::Alpha => PrimitiveOperation::Alpha,
            Self::Beta => PrimitiveOperation::Beta,
            Self::Fraction => PrimitiveOperation::Fraction,
            Self::Subscript => PrimitiveOperation::Subscript,
            Self::Superscript => PrimitiveOperation::Superscript,
            Self::Add => PrimitiveOperation::Add,
            Self::Equal => PrimitiveOperation::Equal,
            Self::Negate => PrimitiveOperation::Negate,
            Self::Multiply => PrimitiveOperation::Multiply,
            Self::Juxtapose => PrimitiveOperation::Juxtapose,
            Self::Parentheses => PrimitiveOperation::Parentheses,
            Self::Dimension => PrimitiveOperation::Dimension,
            Self::Omega => PrimitiveOperation::Omega,
            Self::Calligraphic => PrimitiveOperation::Calligraphic,
            Self::UprightVir => PrimitiveOperation::UprightVir,
            Self::UprightS => PrimitiveOperation::UprightS,
            Self::Dot => PrimitiveOperation::Dot,
            Self::Sigma => PrimitiveOperation::Sigma,
            Self::Gamma => PrimitiveOperation::Gamma,
            Self::Times => PrimitiveOperation::Times,
            Self::Subtract => PrimitiveOperation::Subtract,
            Self::TightJoin => PrimitiveOperation::TightJoin,
            Self::Underbrace => PrimitiveOperation::Underbrace,
            Self::UprightNet => PrimitiveOperation::UprightNet,
            Self::CapitalOmega => PrimitiveOperation::CapitalOmega,
            Self::CapitalXi => PrimitiveOperation::CapitalXi,
            Self::Chi => PrimitiveOperation::Chi,
            Self::Eta => PrimitiveOperation::Eta,
            Self::Lambda => PrimitiveOperation::Lambda,
            Self::Mu => PrimitiveOperation::Mu,
            Self::Phi => PrimitiveOperation::Phi,
            Self::Tau => PrimitiveOperation::Tau,
            Self::Theta => PrimitiveOperation::Theta,
            Self::VariantEpsilon => PrimitiveOperation::VariantEpsilon,
            Self::Zeta => PrimitiveOperation::Zeta,
            Self::Nabla => PrimitiveOperation::Nabla,
            Self::LessEqual => PrimitiveOperation::LessEqual,
        }
    }
}

/// Descriptors are immutable reads from the checked catalogue.
/// ```compile_fail,E0616
/// fn change(p: &mut ftoo_manifest::PrimitiveSpec) { p.signature.arity = 3; }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimitiveSpec {
    pub(super) id: PrimitiveId,
    pub(super) token: &'static str,
    pub(super) signature: Signature,
    pub(super) operation: PrimitiveOperation,
}
impl PrimitiveSpec {
    #[must_use]
    pub const fn id(&self) -> PrimitiveId {
        self.id
    }
    #[must_use]
    pub const fn token(&self) -> &'static str {
        self.token
    }
    #[must_use]
    pub const fn signature(&self) -> Signature {
        self.signature
    }
    #[must_use]
    pub const fn operation(&self) -> PrimitiveOperation {
        self.operation
    }
}

pub(super) fn catalogue() -> Vec<PrimitiveSpec> {
    PrimitiveId::ALL
        .into_iter()
        .map(|id| PrimitiveSpec {
            id,
            token: id.token(),
            signature: Signature {
                arity: id.operation().arity(),
            },
            operation: id.operation(),
        })
        .collect()
}
