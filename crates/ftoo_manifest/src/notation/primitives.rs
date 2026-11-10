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
}
impl PrimitiveOperation {
    /// Closed host control tokens used by the renderer and the future shared
    /// mathematics lexer. Grouping punctuation is supplied by the typed writer.
    #[must_use]
    pub const fn host_tokens(self) -> &'static [&'static str] {
        match self {
            Self::Alpha => &["\\alpha"],
            Self::Beta => &["\\beta"],
            Self::Fraction => &["\\frac"],
            Self::Multiply => &["\\cdot"],
            Self::Juxtapose => &["\\,"],
            Self::Subscript
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
            Self::Negate => Some("−"),
            Self::Multiply => Some("⋅"),
            Self::Fraction
            | Self::Subscript
            | Self::Superscript
            | Self::Add
            | Self::Equal
            | Self::Juxtapose
            | Self::Parentheses
            | Self::Dimension => None,
        }
    }
    #[must_use]
    pub const fn arity(self) -> u8 {
        match self {
            Self::Alpha | Self::Beta => 0,
            Self::Negate | Self::Parentheses | Self::Dimension => 1,
            Self::Fraction
            | Self::Subscript
            | Self::Superscript
            | Self::Add
            | Self::Equal
            | Self::Multiply
            | Self::Juxtapose => 2,
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
}
impl PrimitiveId {
    pub const ALL: [Self; 12] = [
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
