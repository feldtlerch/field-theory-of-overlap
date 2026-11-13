use std::{collections::BTreeMap, error::Error, fmt};

use crate::tag_history::seed_history;
use crate::tag_validation::construct;
use crate::{AddressingErrors, HistoryEvent, NodeId, RetirementCause, ValidatedManifest};

/// Semantic domains share one exact handle namespace.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AddressTarget {
    Theory(NodeId),
    Documentary(String),
}
impl AddressTarget {
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Theory(id) => id.semantic_name(),
            Self::Documentary(label) => label,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)] // Keep complete source provenance in the public descriptor.
pub enum CommonOrigin {
    Theory(TagId),
    Documentary {
        admitted_at: crate::UnitSite,
        mint: crate::MintOrigin,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommonDescriptor {
    handle: String,
    target: AddressTarget,
    origin: CommonOrigin,
}
impl CommonDescriptor {
    #[must_use]
    pub fn handle(&self) -> &str {
        &self.handle
    }
    #[must_use]
    pub fn target(&self) -> &AddressTarget {
        &self.target
    }
    #[must_use]
    pub fn origin(&self) -> &CommonOrigin {
        &self.origin
    }
}
/// Permanent opaque handles. Their prefixes carry no type information.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TagId {
    LangevinCoefficientOne,
    LangevinCoefficientThree,
    LangevinCoefficientOneValue,
    LangevinCoefficientThreeValue,
}

impl TagId {
    pub const ALL: [Self; 4] = [
        Self::LangevinCoefficientOne,
        Self::LangevinCoefficientThree,
        Self::LangevinCoefficientOneValue,
        Self::LangevinCoefficientThreeValue,
    ];

    #[must_use]
    pub const fn descriptor(self) -> TagDescriptor {
        let (spelling, target) = match self {
            Self::LangevinCoefficientOne => ("c:1", NodeId::LangevinCoefficientOne),
            Self::LangevinCoefficientThree => ("c:3", NodeId::LangevinCoefficientThree),
            Self::LangevinCoefficientOneValue => ("v:1", NodeId::LangevinCoefficientOneValue),
            Self::LangevinCoefficientThreeValue => ("v:3", NodeId::LangevinCoefficientThreeValue),
        };
        TagDescriptor {
            tag: self,
            spelling,
            target,
        }
    }
}

/// An immutable permanent binding. Source revisions must preserve prior bindings.
/// M030's load-bearing privacy checks:
/// ```compile_fail,E0451
/// use ftoo_manifest::{TagDescriptor, TagId, NodeId};
/// let d = TagDescriptor { tag: TagId::LangevinCoefficientOne,
///     spelling: "other", target: NodeId::LangevinCoefficientThree };
/// ```
/// ```compile_fail,E0616
/// use ftoo_manifest::{TagId, NodeId};
/// let mut d = TagId::LangevinCoefficientOne.descriptor();
/// d.target = NodeId::LangevinCoefficientThree;
/// ```
/// ```compile_fail,E0616
/// let mut d = ftoo_manifest::TagId::LangevinCoefficientOne.descriptor();
/// d.spelling = "renamed";
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TagDescriptor {
    pub(super) tag: TagId,
    pub(super) spelling: &'static str,
    pub(super) target: NodeId,
}
impl TagDescriptor {
    #[must_use]
    pub const fn tag(self) -> TagId {
        self.tag
    }
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        self.spelling
    }
    #[must_use]
    pub const fn target(self) -> NodeId {
        self.target
    }
}

pub(super) fn valid_spelling(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutOfSliceCause {
    TargetAbsent,
    ClaimNotSelected,
    QuantityOutsideSupport,
}
impl fmt::Display for OutOfSliceCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TargetAbsent => "target-absent",
            Self::ClaimNotSelected => "claim-not-selected",
            Self::QuantityOutsideSupport => "quantity-outside-support",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetiredEntry {
    pub(super) tag: TagId,
    pub(super) cause: RetirementCause,
}
impl RetiredEntry {
    #[must_use]
    pub const fn tag(&self) -> TagId {
        self.tag
    }
    #[must_use]
    pub const fn cause(&self) -> &RetirementCause {
        &self.cause
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfSliceEntry {
    pub(super) tag: TagId,
    pub(super) cause: OutOfSliceCause,
}
impl OutOfSliceEntry {
    #[must_use]
    pub const fn tag(self) -> TagId {
        self.tag
    }
    #[must_use]
    pub const fn cause(self) -> OutOfSliceCause {
        self.cause
    }
}

/// A checked partition borrowing precisely the graph whose slice it addresses.
/// Addressability supplies no evidence and cannot promote base status.
/// ```compile_fail,E0616
/// let graph = ftoo_manifest::seed().unwrap();
/// let mut book = ftoo_manifest::addressing(&graph).unwrap();
/// book.live.clear();
/// ```
/// Rust reports this private-fields construction without an error code.
/// ```compile_fail
/// use ftoo_manifest::AddressBook;
/// let graph = ftoo_manifest::seed().unwrap();
/// let book = AddressBook { graph: &graph, descriptors: vec![], history: vec![],
///     live: vec![], retired: vec![], out_of_slice: vec![] };
/// ```
/// These secondary checks establish method absence, not field protection.
/// ```compile_fail,E0599
/// let graph = ftoo_manifest::seed().unwrap();
/// let mut book = ftoo_manifest::addressing(&graph).unwrap();
/// book.rebind("c:1", ftoo_manifest::NodeId::LangevinCoefficientThree);
/// ```
/// ```compile_fail,E0599
/// let graph = ftoo_manifest::seed().unwrap();
/// let mut book = ftoo_manifest::addressing(&graph).unwrap();
/// book.reactivate(ftoo_manifest::TagId::LangevinCoefficientOne);
/// ```
/// ```compile_fail,E0308
/// let evidence: ftoo_manifest::Evidence = ftoo_manifest::TagId::LangevinCoefficientOne;
/// ```
/// ```compile_fail,E0308
/// let graph = ftoo_manifest::seed().unwrap();
/// let book = ftoo_manifest::addressing(&graph).unwrap();
/// let evidence: ftoo_manifest::Evidence = book.resolve("v:1").unwrap();
/// ```
#[derive(Debug)]
pub struct AddressBook<'a> {
    pub(crate) documentary: Vec<crate::DocumentaryBinding>,
    pub(crate) documentary_history: Vec<crate::DocumentaryEventDraft>,
    pub(crate) joint_mints: Option<BTreeMap<String, crate::MintOrigin>>,
    pub(crate) common: BTreeMap<String, CommonDescriptor>,
    pub(super) graph: &'a ValidatedManifest,
    pub(super) descriptors: Vec<TagDescriptor>,
    pub(super) history: Vec<HistoryEvent>,
    pub(super) live: Vec<TagId>,
    pub(super) retired: Vec<RetiredEntry>,
    pub(super) out_of_slice: Vec<OutOfSliceEntry>,
}
impl AddressBook<'_> {
    /// The admitted immutable catalogue, including unavailable handles.
    #[must_use]
    pub fn descriptors(&self) -> &[TagDescriptor] {
        &self.descriptors
    }

    /// Select the sole live reader name for an exact semantic object.
    /// # Errors
    /// Retains every candidate and its exact resolution when no unique name exists.
    pub fn reader_identity(
        &self,
        target: impl Into<AddressTarget>,
    ) -> Result<String, ReaderIdentityFault> {
        let target = target.into();
        let candidates = self
            .common
            .values()
            .filter(|d| d.target == target)
            .map(|d| ReaderTagCandidate {
                handle: d.handle.clone(),
                outcome: self.resolve(&d.handle),
            })
            .collect();
        select_reader_identity(target, candidates)
    }
    pub fn common_descriptors(&self) -> impl Iterator<Item = &CommonDescriptor> {
        self.common.values()
    }
    pub(crate) fn reindex(&mut self) {
        self.common = self
            .descriptors
            .iter()
            .map(|d| {
                (
                    d.spelling.into(),
                    CommonDescriptor {
                        handle: d.spelling.into(),
                        target: AddressTarget::Theory(d.target),
                        origin: CommonOrigin::Theory(d.tag),
                    },
                )
            })
            .chain(self.documentary.iter().map(|d| {
                (
                    d.handle.clone(),
                    CommonDescriptor {
                        handle: d.handle.clone(),
                        target: AddressTarget::Documentary(d.label.clone()),
                        origin: CommonOrigin::Documentary {
                            admitted_at: d.admitted_at.clone(),
                            mint: d.origin.clone(),
                        },
                    },
                )
            }))
            .collect();
    }
    #[must_use]
    pub const fn graph(&self) -> &ValidatedManifest {
        self.graph
    }
    pub fn live(&self) -> impl Iterator<Item = TagId> + '_ {
        self.live.iter().copied()
    }
    pub fn retired(&self) -> impl Iterator<Item = &RetiredEntry> {
        self.retired.iter()
    }
    pub fn out_of_slice(&self) -> impl Iterator<Item = &OutOfSliceEntry> {
        self.out_of_slice.iter()
    }
    pub fn history(&self) -> impl Iterator<Item = &HistoryEvent> {
        self.history.iter()
    }

    /// Resolve exact UTF-8 spelling through the live store only.
    /// # Errors
    /// Returns the original input with its unknown, retired or out-of-slice refusal.
    /// # Panics
    /// Panics only if the checked partition is internally corrupted, which
    /// callers cannot arrange through this immutable API.
    #[allow(clippy::result_large_err)] // The exact refusal retains the complete retirement site.
    pub fn resolve(&self, input: &str) -> Result<AddressTarget, TagResolutionFault> {
        let Some(common) = self.common.get(input) else {
            return Err(TagResolutionFault::UnknownTag {
                input: input.into(),
            });
        };
        if let AddressTarget::Documentary(label) = &common.target {
            let binding = self
                .documentary
                .iter()
                .find(|d| d.handle == input)
                .expect("checked common index");
            return match &binding.state {
                crate::DocumentaryState::Live => Ok(common.target.clone()),
                crate::DocumentaryState::Retired(event) => {
                    Err(TagResolutionFault::RetiredDocumentaryTag {
                        input: input.into(),
                        label: label.clone(),
                        event: event.clone(),
                    })
                }
            };
        }
        let d = self
            .descriptors
            .iter()
            .find(|d| d.spelling == input)
            .expect("checked theory index");
        if self.live.contains(&d.tag) {
            return Ok(AddressTarget::Theory(d.target));
        }
        if let Some(entry) = self.retired.iter().find(|e| e.tag == d.tag) {
            return Err(TagResolutionFault::RetiredTag {
                input: input.into(),
                tag: d.tag,
                target: d.target,
                cause: entry.cause.clone(),
            });
        }
        // The checked partition has exactly one entry for every descriptor.
        let cause = self
            .out_of_slice
            .iter()
            .find(|e| e.tag == d.tag)
            .map_or_else(|| unreachable!("checked partition"), |e| e.cause);
        Err(TagResolutionFault::OutOfSliceTag {
            input: input.into(),
            tag: d.tag,
            target: d.target,
            cause,
        })
    }
}

/// Validate permanent authority and derive the slice's three stores.
/// # Errors
/// Returns sorted admission diagnostics; no partial address book escapes.
pub fn addressing(graph: &ValidatedManifest) -> Result<AddressBook<'_>, AddressingErrors> {
    construct(
        graph,
        TagId::ALL.map(TagId::descriptor).to_vec(),
        seed_history(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TagResolutionFault {
    RetiredDocumentaryTag {
        input: String,
        label: String,
        event: crate::DocumentaryRetirement,
    },
    UnknownTag {
        input: String,
    },
    RetiredTag {
        input: String,
        tag: TagId,
        target: NodeId,
        cause: RetirementCause,
    },
    OutOfSliceTag {
        input: String,
        tag: TagId,
        target: NodeId,
        cause: OutOfSliceCause,
    },
}
impl fmt::Display for TagResolutionFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RetiredDocumentaryTag {
                input,
                label,
                event,
            } => write!(
                f,
                "tag {input:?}: retired; label {label}; event {}; authority {}; reason {:?}",
                event.ordinal, event.authority_record, event.reason
            ),
            Self::UnknownTag { input } => write!(f, "tag {input:?}: unknown"),
            Self::RetiredTag {
                input,
                target,
                cause,
                ..
            } => {
                let event = cause.event();
                write!(
                    f,
                    "tag {input:?}: retired; label {}; event {}; authority {}; reason {:?}",
                    target.semantic_name(),
                    event.ordinal(),
                    event.authority_record(),
                    event.reason()
                )
            }
            Self::OutOfSliceTag {
                input,
                target,
                cause,
                ..
            } => write!(
                f,
                "tag {input:?}: out of slice; label {}; cause {cause}",
                target.semantic_name()
            ),
        }
    }
}
impl Error for TagResolutionFault {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderTagCandidate {
    pub handle: String,
    pub outcome: Result<AddressTarget, TagResolutionFault>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderIdentityReason {
    MissingTag,
    NoLiveTag,
    AmbiguousLiveTags,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderIdentityFault {
    pub target: AddressTarget,
    pub reason: ReaderIdentityReason,
    pub candidates: Vec<ReaderTagCandidate>,
}
impl fmt::Display for ReaderIdentityFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ReaderIdentityFault {}
pub(super) fn select_reader_identity(
    target: impl Into<AddressTarget>,
    candidates: Vec<ReaderTagCandidate>,
) -> Result<String, ReaderIdentityFault> {
    let target = target.into();
    let live: Vec<_> = candidates
        .iter()
        .filter(|candidate| candidate.outcome.as_ref() == Ok(&target))
        .collect();
    if let [candidate] = live.as_slice() {
        return Ok(candidate.handle.clone());
    }
    let reason = if candidates.is_empty() {
        ReaderIdentityReason::MissingTag
    } else if live.is_empty() {
        ReaderIdentityReason::NoLiveTag
    } else {
        ReaderIdentityReason::AmbiguousLiveTags
    };
    Err(ReaderIdentityFault {
        target,
        reason,
        candidates,
    })
}
