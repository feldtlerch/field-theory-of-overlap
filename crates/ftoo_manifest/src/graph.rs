use std::{cmp::Ordering, error::Error, fmt};

use crate::{BudgetSpec, CapabilityId, ClaimAudit, Coverage, Maturity, NodeId, NodeKind, Status};

/// The blocker remains explicit until manifest-bound audit is implemented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Awaiting {
    AuditBinding,
    Capability(CapabilityId),
}

/// Descriptions register referents; they are not executable mathematics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodePayload {
    Quantity {
        description: String,
    },
    Claim {
        description: String,
        audit: ClaimAudit,
    },
}
impl NodePayload {
    #[must_use]
    pub const fn kind(&self) -> NodeKind {
        match self {
            Self::Quantity { .. } => NodeKind::Quantity,
            Self::Claim { .. } => NodeKind::Claim,
        }
    }
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::Quantity { description } | Self::Claim { description, .. } => description,
        }
    }
}

/// Authored fields only; derived axes cannot enter a draft (M011).
/// ```compile_fail,E0560
/// use ftoo_manifest::{NodeDraft, NodeId, NodePayload, Maturity, Evidence};
/// let node = NodeDraft {
///     id: NodeId::LangevinCoefficientOne,
///     maturity: Maturity::Proposed,
///     payload: NodePayload::Quantity { description: "coefficient".into() },
///     evidence: Evidence::Deferred,
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeDraft {
    pub id: NodeId,
    pub maturity: Maturity,
    pub payload: NodePayload,
}

/// Guards have no prose or string expression escape (M005).
/// ```compile_fail,E0308
/// use ftoo_manifest::Guard;
/// let guard: Guard = "always";
/// ```
/// ```compile_fail,E0599
/// use ftoo_manifest::Guard;
/// let guard = Guard::Conditional;
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Guard {
    Always,
}

/// Each relation declares its own endpoint, cycle, self-edge and symmetry policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelationContract {
    pub endpoints: &'static [(NodeKind, NodeKind)],
    pub acyclic: bool,
    pub allow_self: bool,
    pub symmetric: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RelationKind {
    DependsOn,
    CompatibleWith,
}
impl RelationKind {
    pub const ALL: [Self; 2] = [Self::DependsOn, Self::CompatibleWith];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DependsOn => "depends-on",
            Self::CompatibleWith => "compatible-with",
        }
    }
    #[must_use]
    pub const fn contract(self) -> RelationContract {
        use NodeKind::{Claim, Quantity};
        match self {
            Self::DependsOn => RelationContract {
                endpoints: &[(Quantity, Quantity), (Claim, Quantity), (Claim, Claim)],
                acyclic: true,
                allow_self: false,
                symmetric: false,
            },
            Self::CompatibleWith => RelationContract {
                endpoints: &[(Quantity, Quantity)],
                acyclic: false,
                allow_self: false,
                symmetric: true,
            },
        }
    }
}
impl Ord for RelationKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}
impl PartialOrd for RelationKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Directed record. For dependencies, source needs target; symmetry is explicit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub relation: RelationKind,
    pub source: NodeId,
    pub target: NodeId,
    pub guard: Guard,
}
impl Edge {
    #[must_use]
    pub const fn reverse(self) -> Self {
        Self {
            source: self.target,
            target: self.source,
            ..self
        }
    }
}

/// Vectors retain duplicate records until validation can diagnose them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestDraft {
    pub nodes: Vec<NodeDraft>,
    pub edges: Vec<Edge>,
    pub coverage: Coverage,
    pub budgets: Vec<BudgetSpec>,
}

/// Immutable structural admission, obtainable only through validation.
/// ```compile_fail,E0616
/// let mut graph = ftoo_manifest::seed().unwrap();
/// graph.nodes.clear();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedManifest {
    nodes: Vec<NodeDraft>,
    edges: Vec<Edge>,
    coverage: Coverage,
    budgets: Vec<BudgetSpec>,
}
impl ValidatedManifest {
    pub(crate) fn from_checked(draft: ManifestDraft) -> Self {
        Self {
            nodes: draft.nodes,
            edges: draft.edges,
            coverage: draft.coverage,
            budgets: draft.budgets,
        }
    }
    #[must_use]
    pub fn budgets(&self) -> &[BudgetSpec] {
        &self.budgets
    }
    #[must_use]
    pub fn nodes(&self) -> &[NodeDraft] {
        &self.nodes
    }
    #[must_use]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&NodeDraft> {
        self.nodes.iter().find(|node| node.id == id)
    }
    #[must_use]
    pub fn status(&self, id: NodeId) -> Option<Status> {
        self.node(id).map(Status::derive)
    }
    #[must_use]
    pub const fn coverage(&self) -> &Coverage {
        &self.coverage
    }

    /// Targets before sources, selecting the smallest semantic name ready next.
    /// This relation-local helper is not a complete audit schedule.
    ///
    /// # Errors
    /// Returns `DependencyOrderError` when the relation permits cycles.
    pub fn dependency_order(
        &self,
        relation: RelationKind,
    ) -> Result<Vec<NodeId>, DependencyOrderError> {
        if !relation.contract().acyclic {
            return Err(DependencyOrderError { relation });
        }
        let mut remaining: std::collections::BTreeSet<_> =
            self.nodes.iter().map(|node| node.id).collect();
        let mut ordered = Vec::new();
        while let Some(next) = remaining.iter().copied().find(|id| {
            !self.edges.iter().any(|edge| {
                edge.relation == relation && edge.source == *id && remaining.contains(&edge.target)
            })
        }) {
            remaining.remove(&next);
            ordered.push(next);
        }
        // Validation guarantees that no remaining node can be trapped in a cycle.
        debug_assert!(remaining.is_empty());
        Ok(ordered)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyOrderError {
    pub relation: RelationKind,
}
impl fmt::Display for DependencyOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} permits cycles and has no dependency order",
            self.relation.name()
        )
    }
}
impl Error for DependencyOrderError {}
