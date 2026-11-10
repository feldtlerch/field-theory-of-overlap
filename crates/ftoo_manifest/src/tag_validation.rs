use std::{collections::BTreeSet, error::Error, fmt};

use crate::tag_history::{Lifecycle, replay};
use crate::tags::valid_spelling;
use crate::{
    AddressBook, HistoryEvent, NodeId, NodeKind, OutOfSliceCause, OutOfSliceEntry, RelationKind,
    RetiredEntry, RetirementCause, RuleId, TagDescriptor, TagId, ValidatedManifest,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressingStage {
    CatalogAdmission,
    HistoryAdmission,
    StoreConstruction,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AddressingLocation {
    Descriptor(usize),
    History(usize),
    Identity(NodeId),
    Store(TagId),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreState {
    Live,
    Retired(RetirementCause),
    OutOfSlice(OutOfSliceCause),
}

/// IDs and original positions are retained in the enclosing diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AddressingFault {
    InvalidTagSpelling {
        spelling: String,
    },
    DuplicateTagSpelling {
        spelling: String,
        positions: Vec<usize>,
    },
    MissingIdentityAdmission,
    MissingTagAdmission,
    DuplicateIdentityAdmission,
    DuplicateTagAdmission,
    InvalidHistoryOrder {
        expected: u64,
        actual: u32,
    },
    TargetNotAdmitted,
    BlankHistoryReason,
    InvalidAuthorityRecord {
        value: String,
    },
    BlankOriginalFormulation,
    RetirementBeforeAdmission,
    AlreadyRetired {
        ordinal: u32,
    },
    TargetRetired {
        ordinal: u32,
    },
    RetiredIdentityPresent {
        ordinal: u32,
    },
    StorePartitionMismatch {
        expected: usize,
        actual: usize,
    },
    StoreStateMismatch {
        expected: StoreState,
        actual: StoreState,
    },
    IdentityWithoutTag,
}
impl AddressingFault {
    const fn rank(&self) -> u8 {
        match self {
            Self::InvalidTagSpelling { .. } => 0,
            Self::DuplicateTagSpelling { .. } => 1,
            Self::MissingIdentityAdmission => 2,
            Self::MissingTagAdmission => 3,
            Self::DuplicateIdentityAdmission => 4,
            Self::DuplicateTagAdmission => 5,
            Self::InvalidHistoryOrder { .. } => 6,
            Self::TargetNotAdmitted => 7,
            Self::BlankHistoryReason => 8,
            Self::InvalidAuthorityRecord { .. } => 9,
            Self::BlankOriginalFormulation => 10,
            Self::RetirementBeforeAdmission => 11,
            Self::AlreadyRetired { .. } => 12,
            Self::TargetRetired { .. } => 13,
            Self::RetiredIdentityPresent { .. } => 14,
            Self::StorePartitionMismatch { .. } => 15,
            Self::StoreStateMismatch { .. } => 16,
            Self::IdentityWithoutTag => 17,
        }
    }
}
impl fmt::Display for AddressingFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressingDiagnostic {
    pub rule: RuleId,
    pub stage: AddressingStage,
    pub location: AddressingLocation,
    pub tag: Option<TagId>,
    pub identity: Option<NodeId>,
    pub fault: AddressingFault,
}
impl fmt::Display for AddressingDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} at {:?}: {}",
            self.rule, self.stage, self.location, self.fault
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressingErrors {
    diagnostics: Vec<AddressingDiagnostic>,
}
impl AddressingErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[AddressingDiagnostic] {
        &self.diagnostics
    }
}
impl fmt::Display for AddressingErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl Error for AddressingErrors {}

pub(super) fn diagnostic(
    rule: RuleId,
    stage: AddressingStage,
    location: AddressingLocation,
    tag: Option<TagId>,
    identity: Option<NodeId>,
    fault: AddressingFault,
) -> AddressingDiagnostic {
    AddressingDiagnostic {
        rule,
        stage,
        location,
        tag,
        identity,
        fault,
    }
}

fn errors(
    mut diagnostics: Vec<AddressingDiagnostic>,
    descriptors: &[TagDescriptor],
) -> AddressingErrors {
    let spelling = |tag: Option<TagId>| {
        tag.and_then(|t| {
            descriptors
                .iter()
                .find(|d| d.tag() == t)
                .map(|d| d.spelling())
        })
    };
    diagnostics.sort_by(|a, b| {
        (
            a.rule,
            spelling(a.tag),
            a.identity,
            &a.location,
            a.fault.rank(),
        )
            .cmp(&(
                b.rule,
                spelling(b.tag),
                b.identity,
                &b.location,
                b.fault.rank(),
            ))
    });
    AddressingErrors { diagnostics }
}

fn catalog(descriptors: &[TagDescriptor], diagnostics: &mut Vec<AddressingDiagnostic>) {
    for (index, d) in descriptors.iter().enumerate() {
        let mut faults = Vec::new();
        if !valid_spelling(d.spelling()) {
            faults.push(AddressingFault::InvalidTagSpelling {
                spelling: d.spelling().into(),
            });
        }
        let positions: Vec<_> = descriptors
            .iter()
            .enumerate()
            .filter(|(_, other)| other.spelling() == d.spelling())
            .map(|(i, _)| i)
            .collect();
        if positions.len() > 1 {
            faults.push(AddressingFault::DuplicateTagSpelling {
                spelling: d.spelling().into(),
                positions,
            });
        }
        for fault in faults {
            diagnostics.push(diagnostic(
                RuleId::M031,
                AddressingStage::CatalogAdmission,
                AddressingLocation::Descriptor(index),
                Some(d.tag()),
                Some(d.target()),
                fault,
            ));
        }
    }
}

fn support(graph: &ValidatedManifest) -> BTreeSet<NodeId> {
    let mut reached = BTreeSet::new();
    let mut pending = graph.coverage().slice.clone();
    while let Some(id) = pending.pop() {
        if reached.insert(id) {
            pending.extend(
                graph
                    .edges()
                    .iter()
                    .filter(|e| e.relation == RelationKind::DependsOn && e.source == id)
                    .map(|e| e.target),
            );
        }
    }
    reached
}

pub(super) type Candidate = Vec<(TagId, StoreState)>;

fn derive(
    graph: &ValidatedManifest,
    descriptors: &[TagDescriptor],
    lifecycle: &Lifecycle,
) -> Candidate {
    let support = support(graph);
    descriptors
        .iter()
        .map(|d| {
            let state = if let Some(cause) = lifecycle.cause(d) {
                StoreState::Retired(cause)
            } else if graph.node(d.target()).is_none() {
                StoreState::OutOfSlice(OutOfSliceCause::TargetAbsent)
            } else {
                match d.target().expected_kind() {
                    NodeKind::Claim if !graph.coverage().slice.contains(&d.target()) => {
                        StoreState::OutOfSlice(OutOfSliceCause::ClaimNotSelected)
                    }
                    NodeKind::Quantity if !support.contains(&d.target()) => {
                        StoreState::OutOfSlice(OutOfSliceCause::QuantityOutsideSupport)
                    }
                    NodeKind::Claim | NodeKind::Quantity => StoreState::Live,
                }
            };
            (d.tag(), state)
        })
        .collect()
}

// Independent checker: walk prerequisites backwards to a selected root rather
// than using the forward support set used to fill the stores. Graph validation
// has already established acyclicity of DependsOn.
fn has_selected_root(graph: &ValidatedManifest, id: NodeId) -> bool {
    graph.coverage().slice.contains(&id)
        || graph.edges().iter().any(|e| {
            e.relation == RelationKind::DependsOn
                && e.target == id
                && has_selected_root(graph, e.source)
        })
}

pub(super) fn check_partition(
    graph: &ValidatedManifest,
    descriptors: &[TagDescriptor],
    lifecycle: &Lifecycle,
    candidate: &Candidate,
    diagnostics: &mut Vec<AddressingDiagnostic>,
) {
    for d in descriptors {
        let entries: Vec<_> = candidate
            .iter()
            .filter(|(tag, _)| *tag == d.tag())
            .collect();
        let fault = if entries.len() == 1 {
            let expected = match lifecycle.cause(d) {
                Some(cause) => StoreState::Retired(cause),
                None => match graph.node(d.target()) {
                    None => StoreState::OutOfSlice(OutOfSliceCause::TargetAbsent),
                    Some(node) => match node.payload.kind() {
                        NodeKind::Claim => {
                            if graph.coverage().slice.contains(&node.id) {
                                StoreState::Live
                            } else {
                                StoreState::OutOfSlice(OutOfSliceCause::ClaimNotSelected)
                            }
                        }
                        NodeKind::Quantity => {
                            if has_selected_root(graph, node.id) {
                                StoreState::Live
                            } else {
                                StoreState::OutOfSlice(OutOfSliceCause::QuantityOutsideSupport)
                            }
                        }
                    },
                },
            };
            (entries[0].1 != expected).then(|| AddressingFault::StoreStateMismatch {
                expected,
                actual: entries[0].1.clone(),
            })
        } else {
            Some(AddressingFault::StorePartitionMismatch {
                expected: 1,
                actual: entries.len(),
            })
        };
        if let Some(fault) = fault {
            diagnostics.push(diagnostic(
                RuleId::M035,
                AddressingStage::StoreConstruction,
                AddressingLocation::Store(d.tag()),
                Some(d.tag()),
                Some(d.target()),
                fault,
            ));
        }
    }
}

pub(super) fn construct(
    graph: &ValidatedManifest,
    descriptors: Vec<TagDescriptor>,
    history: Vec<HistoryEvent>,
) -> Result<AddressBook<'_>, AddressingErrors> {
    construct_candidate(graph, descriptors, history, None)
}

pub(super) fn construct_candidate(
    graph: &ValidatedManifest,
    mut descriptors: Vec<TagDescriptor>,
    mut history: Vec<HistoryEvent>,
    candidate: Option<Candidate>,
) -> Result<AddressBook<'_>, AddressingErrors> {
    let mut diagnostics = Vec::new();
    catalog(&descriptors, &mut diagnostics);
    let lifecycle = replay(&descriptors, &history, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(errors(diagnostics, &descriptors));
    }
    for (id, event) in &lifecycle.retired_identities {
        if graph.node(*id).is_some() {
            diagnostics.push(diagnostic(
                RuleId::M034,
                AddressingStage::StoreConstruction,
                AddressingLocation::Identity(*id),
                None,
                Some(*id),
                AddressingFault::RetiredIdentityPresent {
                    ordinal: event.ordinal(),
                },
            ));
        }
    }
    let candidate = candidate.unwrap_or_else(|| derive(graph, &descriptors, &lifecycle));
    check_partition(
        graph,
        &descriptors,
        &lifecycle,
        &candidate,
        &mut diagnostics,
    );
    if !diagnostics.is_empty() {
        return Err(errors(diagnostics, &descriptors));
    }
    descriptors.sort_by_key(|d| d.spelling());
    history.sort_by_key(HistoryEvent::ordinal);
    let mut book = AddressBook {
        graph,
        descriptors,
        history,
        live: Vec::new(),
        retired: Vec::new(),
        out_of_slice: Vec::new(),
    };
    for d in &book.descriptors {
        for (_, state) in candidate.iter().filter(|(tag, _)| *tag == d.tag()) {
            match state {
                StoreState::Live => book.live.push(d.tag()),
                StoreState::Retired(cause) => book.retired.push(RetiredEntry {
                    tag: d.tag(),
                    cause: cause.clone(),
                }),
                StoreState::OutOfSlice(cause) => book.out_of_slice.push(OutOfSliceEntry {
                    tag: d.tag(),
                    cause: *cause,
                }),
            }
        }
    }
    Ok(book)
}
