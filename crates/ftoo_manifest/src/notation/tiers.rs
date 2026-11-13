use super::{DefinitionId, DefinitionKind, DefinitionSpec, catalogue::Dependencies};
use crate::{
    AddressBook, Awaiting, Blocker, Calibration, Evidence, Identification, Maturity, NodeId,
    NodeKind,
};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SymbolTier {
    Pending,
    Authorised,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PendingReason {
    BackingUnbound {
        object: NodeId,
    },
    BackingAbsent {
        object: NodeId,
    },
    FormulationProposed {
        object: NodeId,
    },
    ClaimDeferred {
        object: NodeId,
        blocker: Blocker,
    },
    ClaimRefuted {
        object: NodeId,
    },
    ClaimUndecided {
        object: NodeId,
        stage: super::EvidenceStage,
        exhaustion: super::EvidenceExhaustion,
    },
    ClaimAuditDeferred {
        object: NodeId,
        reason: crate::DeferredReason,
    },
}
fn blocker_text(blocker: Blocker) -> String {
    match blocker {
        Blocker::AuditRunRequired => "audit-run-required".into(),
        Blocker::Awaiting(Awaiting::AuditBinding) => "audit-binding".into(),
        Blocker::Awaiting(Awaiting::Capability(id)) => {
            let token = match id {
                crate::CapabilityId::ExactRationalLinearEquivalence => {
                    "exact-rational-linear-equivalence"
                }
                crate::CapabilityId::FormalTruncatedPowerSeries => "formal-truncated-power-series",
                crate::CapabilityId::SymbolicTrigonometry => "symbolic-trigonometry",
                crate::CapabilityId::TrigonometricIntegration => "trigonometric-integration",
            };
            format!("capability {token}")
        }
    }
}
impl PendingReason {
    #[must_use]
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.key()
            .cmp(&other.key())
            .then_with(|| match (self, other) {
                (
                    Self::ClaimUndecided {
                        stage: a,
                        exhaustion: x,
                        ..
                    },
                    Self::ClaimUndecided {
                        stage: b,
                        exhaustion: y,
                        ..
                    },
                ) => (a, x).cmp(&(b, y)),
                _ => std::cmp::Ordering::Equal,
            })
    }
    fn key(&self) -> (u8, &'static str, String) {
        match self {
            Self::BackingUnbound { object } => (0, object.semantic_name(), String::new()),
            Self::BackingAbsent { object } => (1, object.semantic_name(), String::new()),
            Self::FormulationProposed { object } => (2, object.semantic_name(), String::new()),
            Self::ClaimDeferred { object, blocker } => {
                (3, object.semantic_name(), blocker_text(*blocker))
            }
            Self::ClaimRefuted { object } => (4, object.semantic_name(), String::new()),
            Self::ClaimUndecided { object, .. } => (5, object.semantic_name(), String::new()),
            Self::ClaimAuditDeferred { object, .. } => {
                (6, object.semantic_name(), self.to_string())
            }
        }
    }
}
impl PendingReason {
    /// Fixed prose around the permanent object identity, shared by diagnostics and presentation.
    #[must_use]
    pub fn text_parts(&self) -> Vec<super::ReasonPart> {
        use super::{
            ReasonPart::{Object, Text},
            ReasonRole,
        };
        let (before, object, after) = match self {
            Self::BackingUnbound { object } => ("backing object ", *object, " is unbound".into()),
            Self::BackingAbsent { object } => ("backing object ", *object, " is absent".into()),
            Self::FormulationProposed { object } => {
                ("retained formulation ", *object, String::new())
            }
            Self::ClaimDeferred { object, blocker } => (
                "claim evidence ",
                *object,
                format!(" awaits {}", blocker_text(*blocker)),
            ),
            Self::ClaimRefuted { object } => ("claim evidence ", *object, " is refuted".into()),
            Self::ClaimUndecided {
                object,
                stage,
                exhaustion,
            } => (
                "claim evidence ",
                *object,
                format!(" is undecided: {}", exhaustion.description(*stage)),
            ),
            Self::ClaimAuditDeferred { object, reason } => {
                let mut parts = vec![
                    Text("claim evidence ".into()),
                    Object {
                        role: ReasonRole::Subject,
                        id: *object,
                    },
                    Text(" awaits ".into()),
                ];
                for part in super::deferred_parts(*reason) {
                    if let (Some(Text(before)), Text(after)) = (parts.last_mut(), &part) {
                        before.push_str(after);
                    } else {
                        parts.push(part);
                    }
                }
                return parts;
            }
        };
        vec![
            Text(before.into()),
            Object {
                role: ReasonRole::Subject,
                id: object,
            },
            Text(after),
        ]
    }
}
impl fmt::Display for PendingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in self.text_parts() {
            match part {
                super::ReasonPart::Text(text) => f.write_str(&text)?,
                super::ReasonPart::Object { id, .. } => f.write_str(id.semantic_name())?,
            }
        }
        Ok(())
    }
}

/// Authority is derived; authors cannot construct or mutate status.
/// ```compile_fail,E0616
/// fn erase(status: &mut ftoo_manifest::SymbolStatus) { status.reasons.clear(); }
/// ```
/// ```compile_fail,E0451
/// use ftoo_manifest::{SymbolStatus, SymbolTier};
/// let status = SymbolStatus { tier: SymbolTier::Authorised, reasons: vec![] };
/// ```
/// ```compile_fail,E0616
/// fn promote(status: &mut ftoo_manifest::SymbolStatus) {
///     status.tier = ftoo_manifest::SymbolTier::Authorised;
/// }
/// ```
/// ```compile_fail,E0599
/// let status = ftoo_manifest::SymbolStatus::default();
/// ```
/// ```compile_fail,E0277
/// let status: ftoo_manifest::SymbolStatus = true.into();
/// ```
/// ```compile_fail,E0277
/// let status: ftoo_manifest::SymbolStatus = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolStatus {
    tier: SymbolTier,
    reasons: Vec<PendingReason>,
}
impl SymbolStatus {
    #[must_use]
    pub const fn tier(&self) -> SymbolTier {
        self.tier
    }
    #[must_use]
    pub fn reasons(&self) -> &[PendingReason] {
        &self.reasons
    }
}
fn own(book: &AddressBook<'_>, object: NodeId, backing: Option<NodeId>) -> Vec<PendingReason> {
    if backing.is_none() {
        return vec![PendingReason::BackingUnbound { object }];
    }
    let Some(status) = book.graph().status(object) else {
        return vec![PendingReason::BackingAbsent { object }];
    };
    let mut reasons = vec![];
    match status.identification() {
        Identification::Identified => {}
    }
    match status.calibration() {
        Calibration::NotApplicable => {}
    }
    match status.maturity() {
        Maturity::Proposed => reasons.push(PendingReason::FormulationProposed { object }),
        Maturity::Retained => {}
    }
    match (object.expected_kind(), status.evidence()) {
        (NodeKind::Quantity, Evidence::NotAClaim) => {}
        (NodeKind::Claim, Evidence::Deferred) => {
            let blocker = status
                .blocker()
                .expect("validated claim has a base blocker");
            match blocker {
                Blocker::AuditRunRequired
                | Blocker::Awaiting(Awaiting::AuditBinding | Awaiting::Capability(_)) => {
                    reasons.push(PendingReason::ClaimDeferred { object, blocker });
                }
            }
        }
        (NodeKind::Quantity, Evidence::Deferred) | (NodeKind::Claim, Evidence::NotAClaim) => {
            unreachable!("validated payload status")
        }
    }
    reasons
}
fn collect(
    id: DefinitionId,
    book: &AddressBook<'_>,
    definitions: &[DefinitionSpec],
    edges: &Dependencies,
    reasons: &mut Vec<PendingReason>,
) {
    let spec = definitions
        .iter()
        .find(|d| d.id == id)
        .expect("complete definitions");
    match spec.kind {
        DefinitionKind::Canonical {
            object, backing, ..
        }
        | DefinitionKind::Alias {
            object, backing, ..
        } => reasons.extend(own(book, object, backing)),
        DefinitionKind::Constructor { .. } => {}
    }
    for (child, _) in &edges[&id] {
        collect(*child, book, definitions, edges, reasons);
    }
}
pub(super) fn derive(
    id: DefinitionId,
    book: &AddressBook<'_>,
    definitions: &[DefinitionSpec],
    edges: &Dependencies,
) -> SymbolStatus {
    let mut reasons = vec![];
    collect(id, book, definitions, edges, &mut reasons);
    reasons.sort_by(PendingReason::compare);
    reasons.dedup();
    SymbolStatus {
        tier: if reasons.is_empty() {
            SymbolTier::Authorised
        } else {
            SymbolTier::Pending
        },
        reasons,
    }
}
