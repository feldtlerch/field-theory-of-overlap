use crate::{Awaiting, ClaimAudit, NodeDraft, NodeKind, NodePayload};

/// Editorial formulation only, never mathematical certification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Maturity {
    Proposed,
    Retained,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Identification {
    Identified,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Calibration {
    NotApplicable,
}
/// Certification cannot be named before its checking consumer is admitted.
/// ```compile_fail,E0599
/// use ftoo_manifest::Evidence;
/// let evidence = Evidence::Certified;
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Evidence {
    NotAClaim,
    Deferred,
}

/// Four axes obtained only from a validated node. Only maturity is authored.
///
/// M011: derived fields cannot be authored or mutated.
/// ```compile_fail,E0451
/// use ftoo_manifest::{Status, Maturity, Identification, Calibration, Evidence};
/// let status = Status {
///     blocker: None,
///     maturity: Maturity::Proposed,
///     identification: Identification::Identified,
///     calibration: Calibration::NotApplicable,
///     evidence: Evidence::Deferred,
/// };
/// ```
/// ```compile_fail,E0616
/// use ftoo_manifest::{seed, NodeId, Evidence};
/// let mut status = seed().unwrap().status(NodeId::LangevinCoefficientOne).unwrap();
/// status.evidence = Evidence::Deferred;
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Status {
    blocker: Option<Blocker>,
    maturity: Maturity,
    identification: Identification,
    calibration: Calibration,
    evidence: Evidence,
}
impl Status {
    pub(crate) fn derive(node: &NodeDraft) -> Self {
        Self {
            blocker: match node.payload {
                NodePayload::Quantity { .. } => None,
                NodePayload::Claim {
                    audit: ClaimAudit::Awaiting(awaits),
                    ..
                } => Some(Blocker::Awaiting(awaits)),
                NodePayload::Claim {
                    audit: ClaimAudit::Bound { .. },
                    ..
                } => Some(Blocker::AuditRunRequired),
            },
            maturity: node.maturity,
            identification: Identification::Identified,
            calibration: Calibration::NotApplicable,
            evidence: match node.payload.kind() {
                NodeKind::Quantity => Evidence::NotAClaim,
                NodeKind::Claim => Evidence::Deferred,
            },
        }
    }
    #[must_use]
    pub const fn blocker(self) -> Option<Blocker> {
        self.blocker
    }
    #[must_use]
    pub const fn maturity(self) -> Maturity {
        self.maturity
    }
    #[must_use]
    pub const fn identification(self) -> Identification {
        self.identification
    }
    #[must_use]
    pub const fn calibration(self) -> Calibration {
        self.calibration
    }
    #[must_use]
    pub const fn evidence(self) -> Evidence {
        self.evidence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Blocker {
    Awaiting(Awaiting),
    AuditRunRequired,
}
