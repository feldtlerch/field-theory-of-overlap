use std::collections::{BTreeMap, BTreeSet};

use crate::tag_validation::diagnostic;
use crate::{
    AddressingDiagnostic, AddressingFault as F, AddressingLocation as L, AddressingStage as S,
    NodeId, RuleId, TagDescriptor, TagId,
};

/// Closed authority acts: no rename, rebind, deletion or reactivation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryChange {
    AdmitIdentity {
        identity: NodeId,
        original_formulation: String,
    },
    AdmitTag {
        tag: TagId,
    },
    RetireTag {
        tag: TagId,
    },
    RetireIdentity {
        identity: NodeId,
    },
}

/// Immutable historical record; strings neither mint identities nor execute rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEvent {
    pub(super) ordinal: u32,
    pub(super) authority_record: String,
    pub(super) reason: String,
    pub(super) change: HistoryChange,
}
impl HistoryEvent {
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
    #[must_use]
    pub fn authority_record(&self) -> &str {
        &self.authority_record
    }
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
    #[must_use]
    pub const fn change(&self) -> &HistoryChange {
        &self.change
    }
}

/// The first effective terminal cause, retaining its immutable original event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetirementCause {
    TagRetirement(HistoryEvent),
    IdentityRetirement(HistoryEvent),
}
impl RetirementCause {
    #[must_use]
    pub const fn event(&self) -> &HistoryEvent {
        match self {
            Self::TagRetirement(e) | Self::IdentityRetirement(e) => e,
        }
    }
}

pub(super) fn seed_history() -> Vec<HistoryEvent> {
    use NodeId::{
        LangevinCoefficientOne, LangevinCoefficientOneValue, LangevinCoefficientThree,
        LangevinCoefficientThreeValue,
    };
    let mut events: Vec<_> = [
        (
            1,
            LangevinCoefficientOne,
            "The coefficient of k in the three-dimensional Langevin expansion.",
        ),
        (
            2,
            LangevinCoefficientThree,
            "The coefficient of k cubed in the same expansion.",
        ),
        (
            3,
            LangevinCoefficientOneValue,
            "The first coefficient is exactly 1/3.",
        ),
        (
            4,
            LangevinCoefficientThreeValue,
            "The third coefficient is exactly -1/45.",
        ),
    ]
    .into_iter()
    .map(|(ordinal, identity, text)| HistoryEvent {
        ordinal,
        authority_record: "proposal:manifest:kernel-bite".into(),
        reason: "Retrospective registration of the M1 seed identity.".into(),
        change: HistoryChange::AdmitIdentity {
            identity,
            original_formulation: text.into(),
        },
    })
    .collect();
    for (ordinal, tag) in (5..=8).zip(TagId::ALL) {
        events.push(HistoryEvent {
            ordinal,
            authority_record: "proposal:manifest:addressing-lifecycle-bite".into(),
            reason: format!(
                "The handle addresses the existing seed referent {}.",
                tag.descriptor().target().semantic_name()
            ),
            change: HistoryChange::AdmitTag { tag },
        });
    }
    events
}

#[derive(Default)]
pub(super) struct Lifecycle {
    identities: BTreeSet<NodeId>,
    tags: BTreeSet<TagId>,
    pub(super) retired_identities: BTreeMap<NodeId, HistoryEvent>,
    pub(super) retired_tags: BTreeMap<TagId, HistoryEvent>,
}
impl Lifecycle {
    pub(super) fn cause(&self, d: &TagDescriptor) -> Option<RetirementCause> {
        self.retired_tags
            .get(&d.tag())
            .cloned()
            .map(RetirementCause::TagRetirement)
            .or_else(|| {
                self.retired_identities
                    .get(&d.target())
                    .cloned()
                    .map(RetirementCause::IdentityRetirement)
            })
    }
}

fn label_shape(s: &str) -> bool {
    let parts: Vec<_> = s.split(':').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

fn entity(change: &HistoryChange, descriptors: &[TagDescriptor]) -> (Option<TagId>, NodeId) {
    match change {
        HistoryChange::AdmitIdentity { identity, .. }
        | HistoryChange::RetireIdentity { identity } => (None, *identity),
        HistoryChange::AdmitTag { tag } | HistoryChange::RetireTag { tag } => (
            Some(*tag),
            descriptors
                .iter()
                .find(|d| d.tag() == *tag)
                .map_or_else(|| tag.descriptor().target(), |d| d.target()),
        ),
    }
}

fn prerequisites(
    descriptors: &[TagDescriptor],
    ordered: &[(usize, &HistoryEvent)],
    diagnostics: &mut Vec<AddressingDiagnostic>,
) -> (bool, BTreeSet<NodeId>, BTreeSet<TagId>) {
    let mut blocked_nodes = BTreeSet::new();
    let mut blocked_tags = BTreeSet::new();
    let mut malformed_order = false;
    for (expected, (index, event)) in (1_u64..).zip(ordered) {
        let (tag, node) = entity(&event.change, descriptors);
        if u64::from(event.ordinal) != expected {
            malformed_order = true;
            diagnostics.push(diagnostic(
                RuleId::M032,
                S::HistoryAdmission,
                L::History(*index),
                tag,
                Some(node),
                F::InvalidHistoryOrder {
                    expected,
                    actual: event.ordinal,
                },
            ));
        }
        let mut faults = Vec::new();
        if event.reason.trim().is_empty() {
            faults.push(F::BlankHistoryReason);
        }
        if !label_shape(&event.authority_record) {
            faults.push(F::InvalidAuthorityRecord {
                value: event.authority_record.clone(),
            });
        }
        if let HistoryChange::AdmitIdentity {
            original_formulation,
            ..
        } = &event.change
            && original_formulation.trim().is_empty()
        {
            faults.push(F::BlankOriginalFormulation);
        }
        if !faults.is_empty() {
            if let Some(tag) = tag {
                blocked_tags.insert(tag);
            } else {
                blocked_nodes.insert(node);
            }
        }
        for fault in faults {
            diagnostics.push(diagnostic(
                RuleId::M033,
                S::HistoryAdmission,
                L::History(*index),
                tag,
                Some(node),
                fault,
            ));
        }
    }
    (malformed_order, blocked_nodes, blocked_tags)
}

pub(super) fn replay(
    descriptors: &[TagDescriptor],
    events: &[HistoryEvent],
    diagnostics: &mut Vec<AddressingDiagnostic>,
) -> Lifecycle {
    let mut ordered: Vec<_> = events.iter().enumerate().collect();
    ordered.sort_by_key(|(_, event)| event.ordinal);
    let (malformed_order, blocked_nodes, blocked_tags) =
        prerequisites(descriptors, &ordered, diagnostics);
    let mut state = Lifecycle::default();
    if malformed_order {
        return state;
    }
    let mut incomplete_nodes = blocked_nodes.clone();
    let mut incomplete_tags = blocked_tags.clone();
    for (index, event) in ordered {
        let (tag, node) = entity(&event.change, descriptors);
        if blocked_nodes.contains(&node) || tag.is_some_and(|t| blocked_tags.contains(&t)) {
            continue;
        }
        if let Some((rule, fault)) = apply_event(&mut state, event, node) {
            if rule == RuleId::M032 {
                if let Some(tag) = tag {
                    incomplete_tags.insert(tag);
                } else {
                    incomplete_nodes.insert(node);
                }
            }
            diagnostics.push(diagnostic(
                rule,
                S::HistoryAdmission,
                L::History(index),
                tag,
                Some(node),
                fault,
            ));
        }
    }
    for node in NodeId::ALL {
        if !state.identities.contains(&node) && !blocked_nodes.contains(&node) {
            incomplete_nodes.insert(node);
            diagnostics.push(diagnostic(
                RuleId::M032,
                S::HistoryAdmission,
                L::Identity(node),
                None,
                Some(node),
                F::MissingIdentityAdmission,
            ));
        }
    }
    for d in descriptors {
        if !state.tags.contains(&d.tag())
            && !blocked_tags.contains(&d.tag())
            && !blocked_nodes.contains(&d.target())
        {
            incomplete_tags.insert(d.tag());
            diagnostics.push(diagnostic(
                RuleId::M032,
                S::HistoryAdmission,
                L::Store(d.tag()),
                Some(d.tag()),
                Some(d.target()),
                F::MissingTagAdmission,
            ));
        }
    }
    for node in NodeId::ALL {
        if !incomplete_nodes.contains(&node)
            && !descriptors
                .iter()
                .any(|d| d.target() == node && incomplete_tags.contains(&d.tag()))
            && !descriptors
                .iter()
                .any(|d| d.target() == node && state.tags.contains(&d.tag()))
        {
            diagnostics.push(diagnostic(
                RuleId::M036,
                S::HistoryAdmission,
                L::Identity(node),
                None,
                Some(node),
                F::IdentityWithoutTag,
            ));
        }
    }
    state
}

fn apply_event(state: &mut Lifecycle, event: &HistoryEvent, node: NodeId) -> Option<(RuleId, F)> {
    use HistoryChange::{AdmitIdentity, AdmitTag, RetireIdentity, RetireTag};
    match &event.change {
        AdmitIdentity { .. } => {
            if state.identities.insert(node) {
                None
            } else {
                Some((RuleId::M032, F::DuplicateIdentityAdmission))
            }
        }
        AdmitTag { tag } => {
            // Uniqueness is a prerequisite for terminal-state checks.
            if !state.tags.insert(*tag) {
                Some((RuleId::M032, F::DuplicateTagAdmission))
            } else if !state.identities.contains(&node) {
                Some((RuleId::M032, F::TargetNotAdmitted))
            } else {
                state
                    .retired_identities
                    .get(&node)
                    .map(|e| (RuleId::M034, F::TargetRetired { ordinal: e.ordinal }))
            }
        }
        RetireTag { tag } => {
            if !state.tags.contains(tag) {
                Some((RuleId::M034, F::RetirementBeforeAdmission))
            } else if let Some(prior) = state
                .retired_tags
                .get(tag)
                .or_else(|| state.retired_identities.get(&node))
            {
                Some((
                    RuleId::M034,
                    F::AlreadyRetired {
                        ordinal: prior.ordinal,
                    },
                ))
            } else {
                state.retired_tags.insert(*tag, event.clone());
                None
            }
        }
        RetireIdentity { .. } => {
            if !state.identities.contains(&node) {
                Some((RuleId::M034, F::RetirementBeforeAdmission))
            } else if let Some(prior) = state.retired_identities.get(&node) {
                Some((
                    RuleId::M034,
                    F::AlreadyRetired {
                        ordinal: prior.ordinal,
                    },
                ))
            } else {
                state.retired_identities.insert(node, event.clone());
                None
            }
        }
    }
}
