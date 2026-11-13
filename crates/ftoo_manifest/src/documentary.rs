//! Documentary authority: ordered admission, terminal retirement and checked stores.
#[cfg(test)]
mod tests;
use crate::{
    AddressBook, AddressTarget, JointMints, LocatedString, MintKind, MintOrigin, RuleId, TagId,
    UnitSite,
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentaryRetirement {
    pub ordinal: u32,
    pub authority_record: String,
    pub reason: String,
    pub site: UnitSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentaryState {
    Live,
    Retired(DocumentaryRetirement),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentaryStateChange {
    pub handle: String,
    pub expected: DocumentaryState,
    pub actual: DocumentaryState,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandleOrigin {
    Theory { tag: TagId },
    Documentary { index: usize, site: UnitSite },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryResource {
    Bytes,
    Events,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataReason {
    Blank,
    InvalidLabel,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RegistryStage {
    Source,
    Catalogue,
    History,
    Targets,
    Stores,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryFault {
    Limit {
        resource: RegistryResource,
        limit: usize,
        actual: usize,
    },
    InvalidHandle {
        handle: String,
    },
    LabelShapedHandle {
        handle: String,
    },
    HandleCollision {
        handle: String,
        origins: Vec<HandleOrigin>,
    },
    EventOrdinal {
        index: usize,
        expected: u32,
        actual: String,
    },
    EventMetadata {
        field: String,
        value: String,
        reason: MetadataReason,
    },
    RetireBeforeAdmission {
        handle: String,
    },
    RetireTwice {
        handle: String,
        first: DocumentaryRetirement,
    },
    UnknownTarget {
        label: String,
    },
    TargetDomain {
        label: String,
        actual: MintOrigin,
    },
    ConcurrentLiveTarget {
        label: String,
        live_handle: String,
        live_admitted_at: UnitSite,
        attempted_handle: String,
    },
    AuthorityRecord {
        label: String,
        actual: Option<MintOrigin>,
    },
    DocumentaryPartition {
        missing: Vec<String>,
        repeated: Vec<(String, usize)>,
        unexpected: Vec<String>,
        changed: Vec<DocumentaryStateChange>,
    },
}
impl fmt::Display for RegistryFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryDiagnostic {
    pub rule: RuleId,
    pub stage: RegistryStage,
    pub site: UnitSite,
    pub fault: RegistryFault,
}
impl fmt::Display for RegistryDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} at {:?}: {}",
            self.rule, self.stage, self.site, self.fault
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryErrors(Vec<RegistryDiagnostic>);
impl RegistryErrors {
    #[must_use]
    pub fn diagnostics(&self) -> &[RegistryDiagnostic] {
        &self.0
    }
    #[must_use]
    pub fn from_diagnostics(diagnostics: Vec<RegistryDiagnostic>) -> Option<Self> {
        (!diagnostics.is_empty()).then_some(Self(diagnostics))
    }
}
impl fmt::Display for RegistryErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in &self.0 {
            writeln!(f, "{error}")?;
        }
        Ok(())
    }
}
impl std::error::Error for RegistryErrors {}

/// Untrusted event fields; only admission can attach them to an address book.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentaryEventDraft {
    pub site: UnitSite,
    pub ordinal: LocatedString,
    pub handle: LocatedString,
    pub label: Option<LocatedString>,
    pub authority_record: LocatedString,
    pub reason: LocatedString,
}
/// Immutable documentary binding and its complete event provenance.
/// ```compile_fail,E0616
/// fn change(v: &mut ftoo_manifest::DocumentaryBinding) { v.handle.clear(); }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentaryBinding {
    pub(crate) handle: String,
    pub(crate) label: String,
    pub(crate) admitted_at: UnitSite,
    pub(crate) origin: MintOrigin,
    pub(crate) state: DocumentaryState,
}
impl DocumentaryBinding {
    #[must_use]
    pub fn handle(&self) -> &str {
        &self.handle
    }
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub fn admitted_at(&self) -> &UnitSite {
        &self.admitted_at
    }
    #[must_use]
    pub fn origin(&self) -> &MintOrigin {
        &self.origin
    }
    #[must_use]
    pub fn state(&self) -> &DocumentaryState {
        &self.state
    }
}
fn error(stage: RegistryStage, site: &UnitSite, fault: RegistryFault) -> RegistryDiagnostic {
    let rule = if matches!(
        fault,
        RegistryFault::InvalidHandle { .. } | RegistryFault::HandleCollision { .. }
    ) {
        RuleId::M031
    } else {
        RuleId::M060
    };
    RegistryDiagnostic {
        rule,
        stage,
        site: site.clone(),
        fault,
    }
}
fn phase(errors: Vec<RegistryDiagnostic>) -> Result<(), RegistryErrors> {
    RegistryErrors::from_diagnostics(errors).map_or(Ok(()), Err)
}
fn retirement(event: &DocumentaryEventDraft) -> DocumentaryRetirement {
    DocumentaryRetirement {
        ordinal: event.ordinal.value.parse().unwrap_or_default(),
        authority_record: event.authority_record.value.clone(),
        reason: event.reason.value.clone(),
        site: event.site.clone(),
    }
}
fn catalogue(
    book: &AddressBook<'_>,
    events: &[DocumentaryEventDraft],
) -> Result<(), RegistryErrors> {
    let mut errors = vec![];
    let mut handles: BTreeMap<&str, Vec<HandleOrigin>> = BTreeMap::new();
    for descriptor in book.descriptors() {
        handles
            .entry(descriptor.spelling())
            .or_default()
            .push(HandleOrigin::Theory {
                tag: descriptor.tag(),
            });
    }
    let mut ordered: Vec<_> = events.iter().enumerate().collect();
    ordered.sort_by(|(i, a), (j, b)| (&a.handle.value, i).cmp(&(&b.handle.value, j)));
    for (index, event) in &ordered {
        let handle = &event.handle.value;
        if !crate::tags::valid_spelling(handle) {
            errors.push(error(
                RegistryStage::Catalogue,
                &event.handle.site,
                RegistryFault::InvalidHandle {
                    handle: handle.clone(),
                },
            ));
        } else if crate::units::label_name(handle) {
            errors.push(error(
                RegistryStage::Catalogue,
                &event.handle.site,
                RegistryFault::LabelShapedHandle {
                    handle: handle.clone(),
                },
            ));
        }
        if event.label.is_some() {
            handles
                .entry(handle)
                .or_default()
                .push(HandleOrigin::Documentary {
                    index: *index,
                    site: event.handle.site.clone(),
                });
        }
    }
    for (handle, origins) in handles {
        if origins.len() > 1 {
            let site = origins
                .iter()
                .find_map(|origin| match origin {
                    HandleOrigin::Documentary { site, .. } => Some(site),
                    HandleOrigin::Theory { .. } => None,
                })
                .expect("documentary collision");
            errors.push(error(
                RegistryStage::Catalogue,
                site,
                RegistryFault::HandleCollision {
                    handle: handle.into(),
                    origins: origins.clone(),
                },
            ));
        }
    }
    errors.sort_by(|a, b| {
        let handle = |fault: &RegistryFault| match fault {
            RegistryFault::InvalidHandle { handle }
            | RegistryFault::LabelShapedHandle { handle }
            | RegistryFault::HandleCollision { handle, .. } => handle.clone(),
            _ => unreachable!("catalogue fault"),
        };
        handle(&a.fault)
            .cmp(&handle(&b.fault))
            .then_with(|| a.site.path.cmp(&b.site.path))
    });
    phase(errors)
}
// Event index and logical field path precede the fault-table tie breaker.
fn fault_order(fault: &RegistryFault) -> u8 {
    match fault {
        RegistryFault::Limit { .. } => 0,
        RegistryFault::InvalidHandle { .. } => 1,
        RegistryFault::LabelShapedHandle { .. } => 2,
        RegistryFault::HandleCollision { .. } => 3,
        RegistryFault::EventOrdinal { .. } => 4,
        RegistryFault::EventMetadata { .. } => 5,
        RegistryFault::RetireBeforeAdmission { .. } => 6,
        RegistryFault::RetireTwice { .. } => 7,
        RegistryFault::UnknownTarget { .. } => 8,
        RegistryFault::TargetDomain { .. } => 9,
        RegistryFault::ConcurrentLiveTarget { .. } => 10,
        RegistryFault::AuthorityRecord { .. } => 11,
        RegistryFault::DocumentaryPartition { .. } => 12,
    }
}
fn history(events: &[DocumentaryEventDraft]) -> Result<(), RegistryErrors> {
    let mut errors = vec![];
    let mut states = BTreeMap::<&str, Option<DocumentaryRetirement>>::new();
    for (index, event) in events.iter().enumerate() {
        let expected = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if event.ordinal.value != expected.to_string() {
            errors.push(error(
                RegistryStage::History,
                &event.ordinal.site,
                RegistryFault::EventOrdinal {
                    index,
                    expected,
                    actual: event.ordinal.value.clone(),
                },
            ));
        }
        for (field, value, label) in [
            ("authority-record", &event.authority_record, true),
            ("reason", &event.reason, false),
        ]
        .into_iter()
        .chain(event.label.as_ref().map(|value| ("label", value, true)))
        {
            let reason = if value.value.trim().is_empty() {
                Some(MetadataReason::Blank)
            } else if label && !crate::units::label_name(&value.value) {
                Some(MetadataReason::InvalidLabel)
            } else {
                None
            };
            if let Some(reason) = reason {
                errors.push(error(
                    RegistryStage::History,
                    &value.site,
                    RegistryFault::EventMetadata {
                        field: field.into(),
                        value: value.value.clone(),
                        reason,
                    },
                ));
            }
        }
        if event.label.is_some() {
            states.insert(&event.handle.value, None);
        } else {
            match states.get_mut(event.handle.value.as_str()) {
                None => errors.push(error(
                    RegistryStage::History,
                    &event.handle.site,
                    RegistryFault::RetireBeforeAdmission {
                        handle: event.handle.value.clone(),
                    },
                )),
                Some(Some(first)) => errors.push(error(
                    RegistryStage::History,
                    &event.handle.site,
                    RegistryFault::RetireTwice {
                        handle: event.handle.value.clone(),
                        first: first.clone(),
                    },
                )),
                Some(state) => *state = Some(retirement(event)),
            }
        }
    }
    errors.sort_by(|a, b| {
        a.site
            .path
            .cmp(&b.site.path)
            .then_with(|| fault_order(&a.fault).cmp(&fault_order(&b.fault)))
    });
    phase(errors)
}
fn targets(
    events: &[DocumentaryEventDraft],
    mints: &JointMints<'_, '_>,
) -> Result<Vec<DocumentaryBinding>, RegistryErrors> {
    let mut errors = vec![];
    let mut bindings = BTreeMap::<String, DocumentaryBinding>::new();
    let mut live = BTreeMap::<String, String>::new();
    for event in events {
        let authority = mints.mints().get(&event.authority_record.value);
        if authority.is_none_or(|m| m.kind != MintKind::Markdown) {
            errors.push(error(
                RegistryStage::Targets,
                &event.authority_record.site,
                RegistryFault::AuthorityRecord {
                    label: event.authority_record.value.clone(),
                    actual: authority.cloned(),
                },
            ));
        }
        if let Some(label) = &event.label {
            let origin = match mints.mints().get(&label.value) {
                None => {
                    errors.push(error(
                        RegistryStage::Targets,
                        &label.site,
                        RegistryFault::UnknownTarget {
                            label: label.value.clone(),
                        },
                    ));
                    continue;
                }
                Some(origin) if origin.kind == MintKind::RustIdentity => {
                    errors.push(error(
                        RegistryStage::Targets,
                        &label.site,
                        RegistryFault::TargetDomain {
                            label: label.value.clone(),
                            actual: origin.clone(),
                        },
                    ));
                    continue;
                }
                Some(origin) => origin,
            };
            if let Some(handle) = live.get(&label.value) {
                errors.push(error(
                    RegistryStage::Targets,
                    &label.site,
                    RegistryFault::ConcurrentLiveTarget {
                        label: label.value.clone(),
                        live_handle: handle.clone(),
                        live_admitted_at: bindings[handle].admitted_at.clone(),
                        attempted_handle: event.handle.value.clone(),
                    },
                ));
            } else {
                live.insert(label.value.clone(), event.handle.value.clone());
            }
            bindings.insert(
                event.handle.value.clone(),
                DocumentaryBinding {
                    handle: event.handle.value.clone(),
                    label: label.value.clone(),
                    admitted_at: label.site.clone(),
                    origin: origin.clone(),
                    state: DocumentaryState::Live,
                },
            );
        } else if let Some(binding) = bindings.get_mut(&event.handle.value) {
            if live.get(&binding.label) == Some(&binding.handle) {
                live.remove(&binding.label);
            }
            binding.state = DocumentaryState::Retired(retirement(event));
        }
    }
    errors.sort_by(|a, b| {
        a.site
            .path
            .cmp(&b.site.path)
            .then_with(|| fault_order(&a.fault).cmp(&fault_order(&b.fault)))
    });
    phase(errors)?;
    Ok(bindings.into_values().collect())
}
fn partition(
    events: &[DocumentaryEventDraft],
    actual: &[DocumentaryBinding],
) -> Result<(), RegistryErrors> {
    let mut expected = BTreeMap::new();
    for event in events {
        expected.insert(
            event.handle.value.clone(),
            if event.label.is_some() {
                DocumentaryState::Live
            } else {
                DocumentaryState::Retired(retirement(event))
            },
        );
    }
    let mut seen = BTreeMap::<String, Vec<&DocumentaryState>>::new();
    for binding in actual {
        seen.entry(binding.handle.clone())
            .or_default()
            .push(&binding.state);
    }
    let missing = expected
        .keys()
        .filter(|h| !seen.contains_key(*h))
        .cloned()
        .collect::<Vec<_>>();
    let unexpected = seen
        .keys()
        .filter(|h| !expected.contains_key(*h))
        .cloned()
        .collect::<Vec<_>>();
    let repeated = seen
        .iter()
        .filter(|(_, s)| s.len() > 1)
        .map(|(h, s)| (h.clone(), s.len()))
        .collect::<Vec<_>>();
    let changed = seen
        .iter()
        .flat_map(|(h, states)| {
            states.iter().filter_map(|state| {
                expected
                    .get(h)
                    .filter(|e| *e != *state)
                    .map(|e| DocumentaryStateChange {
                        handle: h.clone(),
                        expected: e.clone(),
                        actual: (*state).clone(),
                    })
            })
        })
        .collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() && repeated.is_empty() && changed.is_empty() {
        return Ok(());
    }
    let site = UnitSite {
        file: events
            .first()
            .map_or("registry/documentary-tags.toml", |e| e.site.file.as_str())
            .into(),
        path: vec![],
        anchor: crate::SourceAnchor::WholeFile,
        decoded: None,
        occurrence: None,
    };
    Err(RegistryErrors(vec![error(
        RegistryStage::Stores,
        &site,
        RegistryFault::DocumentaryPartition {
            missing,
            repeated,
            unexpected,
            changed,
        },
    )]))
}
impl AddressBook<'_> {
    /// Admit documentary events against the checked joint mint inventory.
    /// # Errors
    /// Returns complete earliest-phase faults without publishing a partial book.
    pub fn with_documentary(
        mut self,
        mints: &JointMints<'_, '_>,
        events: Vec<DocumentaryEventDraft>,
    ) -> Result<Self, RegistryErrors> {
        if events.len() > 4096 {
            return Err(RegistryErrors(vec![error(
                RegistryStage::Source,
                &events[4096].handle.site,
                RegistryFault::Limit {
                    resource: RegistryResource::Events,
                    limit: 4096,
                    actual: 4097,
                },
            )]));
        }
        catalogue(&self, &events)?;
        history(&events)?;
        let bindings = targets(&events, mints)?;
        partition(&events, &bindings)?;
        self.documentary = bindings;
        self.documentary_history = events;
        self.joint_mints = Some(mints.mints().clone());
        self.reindex();
        Ok(self)
    }
    pub fn documentary(&self) -> impl Iterator<Item = &DocumentaryBinding> {
        self.documentary.iter()
    }
    #[must_use]
    pub fn documentary_history(&self) -> &[DocumentaryEventDraft] {
        &self.documentary_history
    }
}

impl From<crate::NodeId> for AddressTarget {
    fn from(value: crate::NodeId) -> Self {
        Self::Theory(value)
    }
}
