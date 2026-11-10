use crate::NodeId;

/// Explicit completeness assertions, not exemptions from reference checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosedWorld {
    pub referents: bool,
    pub relations: bool,
    pub audit: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseGate {
    ExplicitPartial,
    Complete,
}

/// The slice names claims reached by manifest-bound audit, not registered claims.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Coverage {
    pub slice: Vec<NodeId>,
    pub closed_world: ClosedWorld,
    pub release_gate: ReleaseGate,
}
impl Coverage {
    pub(crate) fn has_partial_assertions(&self) -> bool {
        !self.closed_world.referents
            && !self.closed_world.relations
            && self.release_gate == ReleaseGate::ExplicitPartial
    }
}
