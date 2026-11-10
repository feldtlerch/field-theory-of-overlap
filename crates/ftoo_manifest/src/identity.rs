use std::cmp::Ordering;

/// The closed referent kinds; payloads determine kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Quantity,
    Claim,
}

impl NodeKind {
    pub const ALL: [Self; 2] = [Self::Quantity, Self::Claim];
}

/// Permanent identities, independent of Rust variant spelling.
///
/// M005: strings cannot introduce identities.
/// ```compile_fail,E0308
/// use ftoo_manifest::NodeId;
/// let id: NodeId = "def:langevin:coefficient-one";
/// ```
/// ```compile_fail,E0599
/// use ftoo_manifest::NodeId;
/// let id = NodeId::Unregistered;
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeId {
    LangevinCoefficientOne,
    LangevinCoefficientThree,
    LangevinCoefficientOneValue,
    LangevinCoefficientThreeValue,
}

impl NodeId {
    pub const ALL: [Self; 4] = [
        Self::LangevinCoefficientOne,
        Self::LangevinCoefficientThree,
        Self::LangevinCoefficientOneValue,
        Self::LangevinCoefficientThreeValue,
    ];

    #[must_use]
    pub const fn semantic_name(self) -> &'static str {
        match self {
            Self::LangevinCoefficientOne => "def:langevin:coefficient-one",
            Self::LangevinCoefficientThree => "def:langevin:coefficient-three",
            Self::LangevinCoefficientOneValue => "claim:langevin:coefficient-one-value",
            Self::LangevinCoefficientThreeValue => "claim:langevin:coefficient-three-value",
        }
    }

    #[must_use]
    pub const fn expected_kind(self) -> NodeKind {
        match self {
            Self::LangevinCoefficientOne | Self::LangevinCoefficientThree => NodeKind::Quantity,
            Self::LangevinCoefficientOneValue | Self::LangevinCoefficientThreeValue => {
                NodeKind::Claim
            }
        }
    }
}

impl Ord for NodeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.semantic_name().cmp(other.semantic_name())
    }
}
impl PartialOrd for NodeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
