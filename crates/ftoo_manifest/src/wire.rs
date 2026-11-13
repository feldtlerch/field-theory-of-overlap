//! Complete projection admission and its sole canonical writer.
mod syntax;
#[cfg(test)]
mod tests;

use crate::{
    AddressBook, AddressingDiagnostic, AddressingErrors, HistoryChange, HistoryEvent,
    OutOfSliceCause, OutOfSliceEntry, RetiredEntry, RetirementCause, StoreState, TagDescriptor,
    TagId,
};
use crate::{
    AuditRoutes, Awaiting, Blocker, BudgetId, BudgetProfile, BudgetSpec, CAPABILITIES,
    CapabilityId, ClaimAudit, ClosedWorld, Coverage, ENGINE_PIN, Edge, EngineAvailability,
    Evidence, Guard, ManifestDraft, Maturity, NodeDraft, NodeId, NodeKind, NodePayload, ProducerId,
    RegionContract, RelationKind, ReleaseGate, ResolvedLimits, RuleId, Status, ValidatedManifest,
    ValidationErrors, validate,
};
use std::{error::Error, fmt};
use syntax::Value;

const TAG_FIELDS: &[&str] = &["tag", "target"];
const HISTORY_FIELDS: &[&str] = &["ordinal", "authority-record", "reason", "change"];
const ADMIT_IDENTITY_FIELDS: &[&str] = &["kind", "identity", "original-formulation"];
const IDENTITY_CHANGE_FIELDS: &[&str] = &["kind", "identity"];
const TAG_CHANGE_FIELDS: &[&str] = &["kind", "tag"];
const STORE_FIELDS: &[&str] = &["live", "retired", "out-of-slice"];
const LIVE_FIELDS: &[&str] = &["tag"];
const UNAVAILABLE_FIELDS: &[&str] = &["tag", "cause"];
const RETIREMENT_FIELDS: &[&str] = &["kind", "event"];
const TAGS: &[(TagId, &str)] = &[
    (TagId::LangevinCoefficientOne, "c:1"),
    (TagId::LangevinCoefficientThree, "c:3"),
    (TagId::LangevinCoefficientOneValue, "v:1"),
    (TagId::LangevinCoefficientThreeValue, "v:3"),
];
const CHANGES: &[(u8, &str)] = &[
    (0, "admit-identity"),
    (1, "admit-tag"),
    (2, "retire-tag"),
    (3, "retire-identity"),
];
const RETIREMENTS: &[(bool, &str)] = &[(true, "tag-retirement"), (false, "identity-retirement")];
const EXCLUSIONS: &[(OutOfSliceCause, &str)] = &[
    (OutOfSliceCause::TargetAbsent, "target-absent"),
    (OutOfSliceCause::ClaimNotSelected, "claim-not-selected"),
    (
        OutOfSliceCause::QuantityOutsideSupport,
        "quantity-outside-support",
    ),
];

fn tag_record(value: &Value, path: &str) -> Result<(TagId, NodeId), WireError> {
    let fields = object(value, path, TAG_FIELDS)?;
    Ok((
        token(get(fields, "tag"), &format!("{path}.tag"), TAGS)?,
        identity(get(fields, "target"), &format!("{path}.target"))?,
    ))
}
fn history_record(value: &Value, path: &str) -> Result<HistoryEvent, WireError> {
    let fields = object(value, path, HISTORY_FIELDS)?;
    let ordinal = u32::try_from(number(
        get(fields, "ordinal"),
        &format!("{path}.ordinal"),
        32,
    )?)
    .expect("checked u32");
    let authority_record = string(
        get(fields, "authority-record"),
        &format!("{path}.authority-record"),
    )?
    .into();
    let reason = string(get(fields, "reason"), &format!("{path}.reason"))?.into();
    let value = get(fields, "change");
    let path = format!("{path}.change");
    let kind = token(
        &s(discriminator(value, &path)?),
        &format!("{path}.kind"),
        CHANGES,
    )?;
    let fields = object(
        value,
        &path,
        match kind {
            0 => ADMIT_IDENTITY_FIELDS,
            1 | 2 => TAG_CHANGE_FIELDS,
            _ => IDENTITY_CHANGE_FIELDS,
        },
    )?;
    let change = match kind {
        0 => HistoryChange::AdmitIdentity {
            identity: identity(get(fields, "identity"), &format!("{path}.identity"))?,
            original_formulation: string(
                get(fields, "original-formulation"),
                &format!("{path}.original-formulation"),
            )?
            .into(),
        },
        1 => HistoryChange::AdmitTag {
            tag: token(get(fields, "tag"), &format!("{path}.tag"), TAGS)?,
        },
        2 => HistoryChange::RetireTag {
            tag: token(get(fields, "tag"), &format!("{path}.tag"), TAGS)?,
        },
        _ => HistoryChange::RetireIdentity {
            identity: identity(get(fields, "identity"), &format!("{path}.identity"))?,
        },
    };
    Ok(HistoryEvent {
        ordinal,
        authority_record,
        reason,
        change,
    })
}
enum StoreAssertion {
    Live,
    Retired { tag_retirement: bool, event: u32 },
    OutOfSlice(OutOfSliceCause),
}
struct StoreRecord {
    tag: TagId,
    state: StoreAssertion,
    path: String,
    position: usize,
}
fn stores_record(value: &Value) -> Result<Vec<StoreRecord>, WireError> {
    let fields = object(value, "$.stores", STORE_FIELDS)?;
    let mut records = Vec::new();
    for name in STORE_FIELDS {
        let path = format!("$.stores.{name}");
        for (position, value) in array(get(fields, name), &path)?.iter().enumerate() {
            let path = format!("{path}[{position}]");
            let fields = object(
                value,
                &path,
                if *name == "live" {
                    LIVE_FIELDS
                } else {
                    UNAVAILABLE_FIELDS
                },
            )?;
            let tag = token(get(fields, "tag"), &format!("{path}.tag"), TAGS)?;
            let cause_path = format!("{path}.cause");
            let state = match *name {
                "live" => StoreAssertion::Live,
                "retired" => {
                    let cause = object(get(fields, "cause"), &cause_path, RETIREMENT_FIELDS)?;
                    StoreAssertion::Retired {
                        tag_retirement: token(
                            get(cause, "kind"),
                            &format!("{cause_path}.kind"),
                            RETIREMENTS,
                        )?,
                        event: u32::try_from(number(
                            get(cause, "event"),
                            &format!("{cause_path}.event"),
                            32,
                        )?)
                        .expect("checked u32"),
                    }
                }
                _ => StoreAssertion::OutOfSlice(token(
                    get(fields, "cause"),
                    &cause_path,
                    EXCLUSIONS,
                )?),
            };
            records.push(StoreRecord {
                tag,
                state,
                path,
                position,
            });
        }
    }
    Ok(records)
}
fn addressing_error(errors: &AddressingErrors) -> ProjectionError {
    ProjectionError::Addressing(errors.diagnostics().to_vec())
}
fn check_tags(tags: &[(TagId, NodeId)]) -> Result<(), ProjectionError> {
    let mut errors = Vec::new();
    for tag in TagId::ALL {
        let positions: Vec<_> = tags
            .iter()
            .enumerate()
            .filter(|(_, (id, _))| *id == tag)
            .map(|(i, _)| i)
            .collect();
        if positions.len() != 1 {
            errors.push(assertion(
                "$.tags",
                positions.first().copied(),
                ProjectionFault::CatalogMultiplicity {
                    tag,
                    expected: 1,
                    actual: positions.len(),
                    positions: positions.clone(),
                },
            ));
        }
        for position in positions {
            if tags[position].1 != tag.descriptor().target() {
                errors.push(assertion(
                    &format!("$.tags[{position}].target"),
                    Some(position),
                    ProjectionFault::TagBindingMismatch {
                        tag,
                        expected: tag.descriptor().target(),
                        actual: tags[position].1,
                    },
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(ProjectionError::Assertion(errors));
    }
    Ok(())
}
fn store_candidate(
    stores: &[StoreRecord],
    history: &[HistoryEvent],
) -> Result<crate::tag_validation::Candidate, Vec<ProjectionDiagnostic>> {
    let mut errors = Vec::new();
    let mut candidate = Vec::new();
    for record in stores {
        let state = match record.state {
            StoreAssertion::Live => StoreState::Live,
            StoreAssertion::OutOfSlice(cause) => StoreState::OutOfSlice(cause),
            StoreAssertion::Retired {
                tag_retirement,
                event,
            } => {
                let found = history.iter().find(|e| e.ordinal() == event);
                let valid = found.is_some_and(|e| match e.change() {
                    HistoryChange::RetireTag { tag } => tag_retirement && *tag == record.tag,
                    HistoryChange::RetireIdentity { identity } => {
                        !tag_retirement && *identity == record.tag.descriptor().target()
                    }
                    _ => false,
                });
                if !valid {
                    errors.push(assertion(
                        &format!("{}.cause.event", record.path),
                        Some(record.position),
                        ProjectionFault::StoreEventMismatch {
                            tag: record.tag,
                            kind: spelling(tag_retirement, RETIREMENTS).into(),
                            ordinal: event,
                            actual: found.cloned().map(Box::new),
                        },
                    ));
                    continue;
                }
                let event = found.expect("validated event reference").clone();
                StoreState::Retired(if tag_retirement {
                    RetirementCause::TagRetirement(event)
                } else {
                    RetirementCause::IdentityRetirement(event)
                })
            }
        };
        candidate.push((record.tag, state));
    }
    if !errors.is_empty() {
        errors.sort_by_key(|e| {
            let ProjectionFault::StoreEventMismatch { tag, .. } = &e.fault else {
                unreachable!()
            };
            (spelling(*tag, TAGS), e.path.clone())
        });
        return Err(errors);
    }
    Ok(candidate)
}
fn admit_addressing<'a>(
    graph: &'a ValidatedManifest,
    tags: &[(TagId, NodeId)],
    history: &[HistoryEvent],
    stores: &[StoreRecord],
    authority: &[HistoryEvent],
) -> Result<AddressBook<'a>, ProjectionError> {
    check_tags(tags)?;
    let descriptors = TagId::ALL.map(TagId::descriptor).to_vec();
    // Derivation validates lifecycle prerequisites before event references. The
    // resulting stores alone may escape; asserted assignments never fill a book.
    let mut diagnostics =
        match crate::tag_validation::construct(graph, descriptors.clone(), history.to_vec()) {
            Ok(_) => Vec::new(),
            Err(errors) => {
                // History admission suppresses dependent partition checks. A present
                // retired identity is instead a store-construction diagnostic and
                // must accumulate with independently false partition assertions.
                if errors
                    .diagnostics()
                    .iter()
                    .any(|d| d.stage != crate::AddressingStage::StoreConstruction)
                {
                    return Err(addressing_error(&errors));
                }
                errors.diagnostics().to_vec()
            }
        };
    let candidate = match store_candidate(stores, history) {
        Ok(candidate) => candidate,
        Err(errors) => {
            return Err(if diagnostics.is_empty() {
                ProjectionError::Assertion(errors)
            } else {
                // Construction diagnostics take precedence; the event-reference
                // fault is withheld because conversion did not complete.
                ProjectionError::Addressing(diagnostics)
            });
        }
    };
    let lifecycle = crate::tag_history::replay(&descriptors, history, &mut diagnostics);
    crate::tag_validation::check_partition(
        graph,
        &descriptors,
        &lifecycle,
        &candidate,
        &mut diagnostics,
    );
    if !diagnostics.is_empty() {
        return Err(ProjectionError::Addressing(diagnostics));
    }
    let book = crate::tag_validation::construct(graph, descriptors, history.to_vec())
        .map_err(|e| addressing_error(&e))?;
    compare_history(history, authority)?;
    Ok(book)
}
fn compare_history(
    history: &[HistoryEvent],
    authority: &[HistoryEvent],
) -> Result<(), ProjectionError> {
    let mut ordinals: Vec<_> = history
        .iter()
        .chain(authority)
        .map(HistoryEvent::ordinal)
        .collect();
    ordinals.sort_unstable();
    ordinals.dedup();
    let mut errors = Vec::new();
    for ordinal in ordinals {
        let actual = history
            .iter()
            .enumerate()
            .find(|(_, e)| e.ordinal() == ordinal);
        let expected = authority.iter().find(|e| e.ordinal() == ordinal);
        let position = actual.map(|(i, _)| i);
        let path = position.map_or_else(|| "$.history".into(), |i| format!("$.history[{i}]"));
        let mut mismatch =
            |field: &str, expected: Option<ProjectionValue>, actual: Option<ProjectionValue>| {
                if expected != actual {
                    errors.push(assertion(
                        &format!("{path}{field}"),
                        position,
                        ProjectionFault::HistoryMismatch {
                            expected: expected.map(Box::new),
                            actual: actual.map(Box::new),
                        },
                    ));
                }
            };
        match (expected, actual) {
            (Some(e), Some((_, a))) => {
                mismatch(
                    ".authority-record",
                    Some(ProjectionValue::Text(e.authority_record().into())),
                    Some(ProjectionValue::Text(a.authority_record().into())),
                );
                compare_history_change(e, a, &mut mismatch);
                mismatch(
                    ".reason",
                    Some(ProjectionValue::Text(e.reason().into())),
                    Some(ProjectionValue::Text(a.reason().into())),
                );
            }
            (e, a) => mismatch(
                "",
                e.cloned().map(ProjectionValue::History),
                a.map(|(_, e)| ProjectionValue::History(e.clone())),
            ),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ProjectionError::Assertion(errors))
    }
}
fn compare_history_change(
    e: &HistoryEvent,
    a: &HistoryEvent,
    mismatch: &mut impl FnMut(&str, Option<ProjectionValue>, Option<ProjectionValue>),
) {
    match (e.change(), a.change()) {
        (
            HistoryChange::AdmitIdentity {
                identity: ei,
                original_formulation: ef,
            },
            HistoryChange::AdmitIdentity {
                identity: ai,
                original_formulation: af,
            },
        ) => {
            mismatch(
                ".change.identity",
                Some(ProjectionValue::Identity(*ei)),
                Some(ProjectionValue::Identity(*ai)),
            );
            mismatch(
                ".change.original-formulation",
                Some(ProjectionValue::Text(ef.clone())),
                Some(ProjectionValue::Text(af.clone())),
            );
        }
        (
            HistoryChange::RetireIdentity { identity: e },
            HistoryChange::RetireIdentity { identity: a },
        ) => mismatch(
            ".change.identity",
            Some(ProjectionValue::Identity(*e)),
            Some(ProjectionValue::Identity(*a)),
        ),
        (HistoryChange::AdmitTag { tag: e }, HistoryChange::AdmitTag { tag: a })
        | (HistoryChange::RetireTag { tag: e }, HistoryChange::RetireTag { tag: a }) => mismatch(
            ".change.tag",
            Some(ProjectionValue::Tag(*e)),
            Some(ProjectionValue::Tag(*a)),
        ),
        _ => mismatch(
            ".change",
            Some(ProjectionValue::History(e.clone())),
            Some(ProjectionValue::History(a.clone())),
        ),
    }
}
fn history_value(event: &HistoryEvent) -> Value {
    let change = match event.change() {
        HistoryChange::AdmitIdentity {
            identity,
            original_formulation,
        } => ordered(
            ADMIT_IDENTITY_FIELDS,
            [
                ("kind", s(spelling(0, CHANGES))),
                ("identity", s(identity.semantic_name())),
                ("original-formulation", s(original_formulation)),
            ],
        ),
        HistoryChange::AdmitTag { tag } | HistoryChange::RetireTag { tag } => ordered(
            TAG_CHANGE_FIELDS,
            [
                (
                    "kind",
                    s(spelling(
                        if matches!(event.change(), HistoryChange::AdmitTag { .. }) {
                            1
                        } else {
                            2
                        },
                        CHANGES,
                    )),
                ),
                ("tag", s(spelling(*tag, TAGS))),
            ],
        ),
        HistoryChange::RetireIdentity { identity } => ordered(
            IDENTITY_CHANGE_FIELDS,
            [
                ("kind", s(spelling(3, CHANGES))),
                ("identity", s(identity.semantic_name())),
            ],
        ),
    };
    ordered(
        HISTORY_FIELDS,
        [
            ("ordinal", n(u64::from(event.ordinal()))),
            ("authority-record", s(event.authority_record())),
            ("reason", s(event.reason())),
            ("change", change),
        ],
    )
}
fn stores_value(book: &AddressBook<'_>) -> Value {
    ordered(
        STORE_FIELDS,
        [
            (
                "live",
                arr(book.live(), |tag| {
                    ordered(LIVE_FIELDS, [("tag", s(spelling(tag, TAGS)))])
                }),
            ),
            (
                "retired",
                arr(book.retired(), |entry| {
                    ordered(
                        UNAVAILABLE_FIELDS,
                        [
                            ("tag", s(spelling(entry.tag(), TAGS))),
                            (
                                "cause",
                                ordered(
                                    RETIREMENT_FIELDS,
                                    [
                                        (
                                            "kind",
                                            s(spelling(
                                                matches!(
                                                    entry.cause(),
                                                    RetirementCause::TagRetirement(_)
                                                ),
                                                RETIREMENTS,
                                            )),
                                        ),
                                        ("event", n(u64::from(entry.cause().event().ordinal()))),
                                    ],
                                ),
                            ),
                        ],
                    )
                }),
            ),
            (
                "out-of-slice",
                arr(book.out_of_slice(), |entry| {
                    ordered(
                        UNAVAILABLE_FIELDS,
                        [
                            ("tag", s(spelling(entry.tag(), TAGS))),
                            ("cause", s(spelling(entry.cause(), EXCLUSIONS))),
                        ],
                    )
                }),
            ),
        ],
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireLocation {
    ByteOffset(usize),
    LineColumn {
        line: usize,
        column: usize,
        path: String,
    },
    Path(String),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireFault {
    InvalidUtf8,
    Bom,
    Syntax,
    DuplicateKey {
        key: String,
    },
    TrailingData,
    DepthLimit {
        maximum: usize,
    },
    NonIntegerNumber {
        token: String,
    },
    SchemaValue {
        expected: String,
        actual: String,
    },
    UnknownField {
        name: String,
        expected: Vec<String>,
    },
    MissingField {
        name: String,
    },
    WrongType {
        expected: String,
        actual: String,
    },
    Null {
        expected: String,
    },
    UnknownToken {
        expected: Vec<String>,
        actual: String,
    },
    IntegerRange {
        expected: String,
        actual: String,
    },
    StatusMismatch {
        node: NodeId,
        expected: String,
        actual: String,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireError {
    pub rule: RuleId,
    pub location: WireLocation,
    pub fault: WireFault,
}
impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at {:?}: {:?}",
            self.rule, self.location, self.fault
        )
    }
}
impl Error for WireError {}

/// Typed semantic difference values; edges retain their full directed tuple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionValue {
    Text(String),
    Integer(u64),
    Boolean(bool),
    Node(NodeDraft),
    Edge(Edge),
    Identity(NodeId),
    Identities(Vec<NodeId>),
    Capabilities(Vec<CapabilityId>),
    Budget(BudgetSpec),
    Tag(TagId),
    History(HistoryEvent),
    Store(StoreState),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDifference {
    pub path: String,
    pub position: Option<usize>,
    pub expected: Option<ProjectionValue>,
    pub actual: Option<ProjectionValue>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionFault {
    CatalogMultiplicity {
        tag: TagId,
        expected: usize,
        actual: usize,
        positions: Vec<usize>,
    },
    TagBindingMismatch {
        tag: TagId,
        expected: NodeId,
        actual: NodeId,
    },
    HistoryMismatch {
        expected: Option<Box<ProjectionValue>>,
        actual: Option<Box<ProjectionValue>>,
    },
    StoreEventMismatch {
        tag: TagId,
        kind: String,
        ordinal: u32,
        actual: Option<Box<HistoryEvent>>,
    },
    ContractMultiplicity {
        record: String,
        expected: usize,
        actual: usize,
        positions: Vec<usize>,
    },
    ContractMismatch {
        expected: Box<ProjectionValue>,
        actual: Box<ProjectionValue>,
    },
    BudgetLimitsMismatch {
        expected: u64,
        actual: u64,
    },
    PinMismatch {
        expected: String,
        actual: String,
    },
    SourceMismatch {
        differences: Vec<SourceDifference>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionDiagnostic {
    pub rule: RuleId,
    pub path: String,
    pub position: Option<usize>,
    pub fault: ProjectionFault,
}
impl fmt::Display for ProjectionDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {}: {:?}", self.rule, self.path, self.fault)
    }
}
impl Error for ProjectionDiagnostic {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    Wire(WireError),
    Graph(ValidationErrors),
    Addressing(Vec<AddressingDiagnostic>),
    Assertion(Vec<ProjectionDiagnostic>),
}
impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wire(e) => e.fmt(f),
            Self::Graph(e) => e.fmt(f),
            Self::Addressing(errors) => {
                for error in errors {
                    writeln!(f, "{error}")?;
                }
                Ok(())
            }
            Self::Assertion(errors) => {
                for error in errors {
                    writeln!(f, "{error}")?;
                }
                Ok(())
            }
        }
    }
}
impl Error for ProjectionError {}
impl From<WireError> for ProjectionError {
    fn from(e: WireError) -> Self {
        Self::Wire(e)
    }
}
impl From<ProjectionDiagnostic> for ProjectionError {
    fn from(e: ProjectionDiagnostic) -> Self {
        Self::Assertion(vec![e])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerExport {
    id: ProducerId,
    claim: NodeId,
    dependencies: Vec<NodeId>,
    capabilities: Vec<CapabilityId>,
    budget: BudgetId,
    revision: u32,
    target: u64,
    region: RegionContract,
}
impl ProducerExport {
    #[must_use]
    pub const fn id(&self) -> ProducerId {
        self.id
    }
    #[must_use]
    pub const fn claim(&self) -> NodeId {
        self.claim
    }
    #[must_use]
    pub fn dependencies(&self) -> &[NodeId] {
        &self.dependencies
    }
    #[must_use]
    pub fn capabilities(&self) -> &[CapabilityId] {
        &self.capabilities
    }
    #[must_use]
    pub const fn budget(&self) -> BudgetId {
        self.budget
    }
    #[must_use]
    pub const fn revision(&self) -> u32 {
        self.revision
    }
    #[must_use]
    pub const fn target(&self) -> u64 {
        self.target
    }
    #[must_use]
    pub const fn region(&self) -> RegionContract {
        self.region
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityExport {
    id: CapabilityId,
    availability: EngineAvailability,
    adapter_implemented: bool,
    evidence: String,
}
impl CapabilityExport {
    #[must_use]
    pub const fn id(&self) -> CapabilityId {
        self.id
    }
    #[must_use]
    pub const fn availability(&self) -> EngineAvailability {
        self.availability
    }
    #[must_use]
    pub const fn adapter_implemented(&self) -> bool {
        self.adapter_implemented
    }
    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetExport {
    spec: BudgetSpec,
    limits: ResolvedLimits,
}
impl BudgetExport {
    #[must_use]
    pub const fn spec(&self) -> BudgetSpec {
        self.spec
    }
    #[must_use]
    pub const fn limits(&self) -> ResolvedLimits {
        self.limits
    }
}

/// Checked projection bytes. M012 establishes base status only, never audited evidence.
/// ```compile_fail,E0451
/// let checked = ftoo_manifest::LoadedProjection {
///     manifest: ftoo_manifest::seed().unwrap(), pin: String::new(),
///     budgets: vec![], producers: vec![], capabilities: vec![],
///     tags: vec![], history: vec![], live: vec![], retired: vec![], out_of_slice: vec![],
/// };
/// ```
/// ```compile_fail,E0616
/// let graph = ftoo_manifest::seed().unwrap();
/// let bytes = ftoo_manifest::projection_bytes(&graph).unwrap();
/// let mut checked = ftoo_manifest::load_projection(&bytes).unwrap();
/// checked.producers.clear();
/// ```
/// ```compile_fail,E0616
/// let bytes = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
/// let mut checked = ftoo_manifest::load_projection(&bytes).unwrap();
/// checked.history.clear();
/// ```
/// ```compile_fail,E0616
/// let bytes = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
/// let mut checked = ftoo_manifest::load_projection(&bytes).unwrap();
/// checked.live.clear();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedProjection {
    manifest: ValidatedManifest,
    pin: String,
    budgets: Vec<BudgetExport>,
    producers: Vec<ProducerExport>,
    capabilities: Vec<CapabilityExport>,
    tags: Vec<TagDescriptor>,
    history: Vec<HistoryEvent>,
    live: Vec<TagId>,
    retired: Vec<RetiredEntry>,
    out_of_slice: Vec<OutOfSliceEntry>,
}
impl LoadedProjection {
    #[must_use]
    pub fn tags(&self) -> &[TagDescriptor] {
        &self.tags
    }
    #[must_use]
    pub fn history(&self) -> &[HistoryEvent] {
        &self.history
    }
    #[must_use]
    pub fn live(&self) -> &[TagId] {
        &self.live
    }
    #[must_use]
    pub fn retired(&self) -> &[RetiredEntry] {
        &self.retired
    }
    #[must_use]
    pub fn out_of_slice(&self) -> &[OutOfSliceEntry] {
        &self.out_of_slice
    }
    /// Derive a checked address book borrowing this exact graph.
    /// # Errors
    /// Returns lifecycle or partition diagnostics if the retained authority is invalid.
    pub fn addressing(&self) -> Result<AddressBook<'_>, AddressingErrors> {
        crate::tag_validation::construct(&self.manifest, self.tags.clone(), self.history.clone())
    }
    #[must_use]
    pub const fn manifest(&self) -> &ValidatedManifest {
        &self.manifest
    }
    #[must_use]
    pub fn engine_pin(&self) -> &str {
        &self.pin
    }
    #[must_use]
    pub fn budgets(&self) -> &[BudgetExport] {
        &self.budgets
    }
    #[must_use]
    pub fn producers(&self) -> &[ProducerExport] {
        &self.producers
    }
    #[must_use]
    pub fn capabilities(&self) -> &[CapabilityExport] {
        &self.capabilities
    }
}

const ROOT: &[&str] = &[
    "schema",
    "engine-pin",
    "nodes",
    "edges",
    "coverage",
    "budgets",
    "producers",
    "capabilities",
    "tags",
    "history",
    "stores",
];
const QUANTITY_FIELDS: &[&str] = &["id", "kind", "description", "status"];
const CLAIM_FIELDS: &[&str] = &["id", "kind", "description", "status", "audit"];
const QUANTITY_STATUS_FIELDS: &[&str] = &["maturity", "identification", "calibration", "evidence"];
const CLAIM_STATUS_FIELDS: &[&str] = &[
    "maturity",
    "identification",
    "calibration",
    "evidence",
    "blocker",
];
const BOUND_FIELDS: &[&str] = &["kind", "producer", "budget", "routes"];
const AWAITING_FIELDS: &[&str] = &["kind", "awaits"];
const CAPABILITY_AWAIT_FIELDS: &[&str] = &["kind", "capability"];
const KIND_FIELDS: &[&str] = &["kind"];
const EDGE_FIELDS: &[&str] = &["relation", "source", "target", "guard"];
const COVERAGE_FIELDS: &[&str] = &["slice", "closed-world", "release-gate"];
const CLOSED_FIELDS: &[&str] = &["referents", "relations", "audit"];
const BUDGET_FIELDS: &[&str] = &["id", "producer", "profile", "pin", "limits"];
const PRODUCER_FIELDS: &[&str] = &[
    "id",
    "claim",
    "dependencies",
    "capabilities",
    "budget",
    "revision",
    "target",
    "region",
];
const CAPABILITY_FIELDS: &[&str] = &["id", "availability", "adapter-implemented", "evidence"];
const PRODUCERS: &[(ProducerId, &str)] = &[
    (
        ProducerId::LangevinCoefficientOneValue,
        "langevin-coefficient-one-value",
    ),
    (
        ProducerId::LangevinCoefficientThreeValue,
        "langevin-coefficient-three-value",
    ),
];
const BUDGETS: &[(BudgetId, &str)] = &[
    (BudgetId::CoefficientOneV1, "coefficient-one-v1"),
    (BudgetId::CoefficientThreeV1, "coefficient-three-v1"),
];
const CAPS: &[(CapabilityId, &str)] = &[
    (
        CapabilityId::ExactRationalLinearEquivalence,
        "exact-rational-linear-equivalence",
    ),
    (
        CapabilityId::FormalTruncatedPowerSeries,
        "formal-truncated-power-series",
    ),
    (CapabilityId::SymbolicTrigonometry, "symbolic-trigonometry"),
    (
        CapabilityId::TrigonometricIntegration,
        "trigonometric-integration",
    ),
];
const MATURITY: &[(Maturity, &str)] = &[
    (Maturity::Proposed, "proposed"),
    (Maturity::Retained, "retained"),
];
const AVAILABILITY: &[(EngineAvailability, &str)] = &[
    (EngineAvailability::Present, "present"),
    (
        EngineAvailability::AbsentWithEvidence,
        "absent-with-evidence",
    ),
    (EngineAvailability::NotEstablished, "not-established"),
];

fn spelling<T: PartialEq + Copy>(value: T, table: &[(T, &'static str)]) -> &'static str {
    table
        .iter()
        .find(|(id, _)| *id == value)
        .map_or("", |(_, text)| *text)
}
fn shape(path: &str, fault: WireFault) -> WireError {
    WireError {
        rule: RuleId::M004,
        location: WireLocation::Path(path.into()),
        fault,
    }
}
fn wrong(value: &Value, path: &str, expected: &str) -> WireError {
    shape(
        path,
        if matches!(value, Value::Null) {
            WireFault::Null {
                expected: expected.into(),
            }
        } else {
            WireFault::WrongType {
                expected: expected.into(),
                actual: value.kind().into(),
            }
        },
    )
}
fn object<'a>(
    value: &'a Value,
    path: &str,
    names: &[&str],
) -> Result<&'a [(String, Value)], WireError> {
    let Value::Object(fields) = value else {
        return Err(wrong(value, path, "object"));
    };
    for (name, _) in fields {
        if !names.contains(&name.as_str()) {
            return Err(shape(
                path,
                WireFault::UnknownField {
                    name: name.clone(),
                    expected: names.iter().map(ToString::to_string).collect(),
                },
            ));
        }
    }
    for name in names {
        if !fields.iter().any(|(key, _)| key == name) {
            return Err(shape(
                path,
                WireFault::MissingField {
                    name: (*name).into(),
                },
            ));
        }
    }
    Ok(fields)
}
fn get<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    &fields
        .iter()
        .find(|(key, _)| key == name)
        .expect("exact fields were checked")
        .1
}
fn string<'a>(value: &'a Value, path: &str) -> Result<&'a str, WireError> {
    if let Value::String(s) = value {
        Ok(s)
    } else {
        Err(wrong(value, path, "string"))
    }
}
fn array<'a>(value: &'a Value, path: &str) -> Result<&'a [Value], WireError> {
    if let Value::Array(a) = value {
        Ok(a)
    } else {
        Err(wrong(value, path, "array"))
    }
}
fn boolean(value: &Value, path: &str) -> Result<bool, WireError> {
    if let Value::Bool(b) = value {
        Ok(*b)
    } else {
        Err(wrong(value, path, "boolean"))
    }
}
fn number(value: &Value, path: &str, bits: u32) -> Result<u64, WireError> {
    let Value::Integer(s) = value else {
        return Err(wrong(value, path, "integer"));
    };
    if !s.starts_with('-')
        && let Ok(n) = s.parse::<u64>()
        && (bits == 64 || u32::try_from(n).is_ok())
    {
        return Ok(n);
    }
    Err(shape(
        path,
        WireFault::IntegerRange {
            expected: format!("u{bits}"),
            actual: s.clone(),
        },
    ))
}
fn token<T: Copy>(value: &Value, path: &str, table: &[(T, &str)]) -> Result<T, WireError> {
    let s = string(value, path)?;
    table
        .iter()
        .find(|(_, text)| *text == s)
        .map(|(id, _)| *id)
        .ok_or_else(|| {
            shape(
                path,
                WireFault::UnknownToken {
                    expected: table.iter().map(|(_, s)| (*s).into()).collect(),
                    actual: s.into(),
                },
            )
        })
}
fn identity(value: &Value, path: &str) -> Result<NodeId, WireError> {
    token(value, path, &NodeId::ALL.map(|id| (id, id.semantic_name())))
}
fn discriminator<'a>(value: &'a Value, path: &str) -> Result<&'a str, WireError> {
    let Value::Object(fields) = value else {
        return Err(wrong(value, path, "object"));
    };
    let Some((_, kind)) = fields.iter().find(|(key, _)| key == "kind") else {
        return Err(shape(
            path,
            WireFault::MissingField {
                name: "kind".into(),
            },
        ));
    };
    string(kind, &format!("{path}.kind"))
}
fn singleton(value: &Value, path: &str, name: &str) -> Result<(), WireError> {
    let fields = object(value, path, KIND_FIELDS)?;
    token(get(fields, "kind"), &format!("{path}.kind"), &[((), name)])
}
fn awaits(value: &Value, path: &str) -> Result<Awaiting, WireError> {
    match discriminator(value, path)? {
        "audit-binding" => {
            singleton(value, path, "audit-binding")?;
            Ok(Awaiting::AuditBinding)
        }
        "capability" => {
            let fields = object(value, path, CAPABILITY_AWAIT_FIELDS)?;
            Ok(Awaiting::Capability(token(
                get(fields, "capability"),
                &format!("{path}.capability"),
                CAPS,
            )?))
        }
        actual => Err(shape(
            &format!("{path}.kind"),
            WireFault::UnknownToken {
                expected: vec!["audit-binding".into(), "capability".into()],
                actual: actual.into(),
            },
        )),
    }
}
fn audit(value: &Value, path: &str) -> Result<ClaimAudit, WireError> {
    match discriminator(value, path)? {
        "awaiting" => {
            let fields = object(value, path, AWAITING_FIELDS)?;
            Ok(ClaimAudit::Awaiting(awaits(
                get(fields, "awaits"),
                &format!("{path}.awaits"),
            )?))
        }
        "bound" => {
            let fields = object(value, path, BOUND_FIELDS)?;
            singleton(
                get(fields, "routes"),
                &format!("{path}.routes"),
                "unconditional",
            )?;
            Ok(ClaimAudit::Bound {
                producer: token(
                    get(fields, "producer"),
                    &format!("{path}.producer"),
                    PRODUCERS,
                )?,
                budget: token(get(fields, "budget"), &format!("{path}.budget"), BUDGETS)?,
                routes: AuditRoutes::Unconditional,
            })
        }
        actual => Err(shape(
            &format!("{path}.kind"),
            WireFault::UnknownToken {
                expected: vec!["bound".into(), "awaiting".into()],
                actual: actual.into(),
            },
        )),
    }
}
fn blocker(value: &Value, path: &str) -> Result<Blocker, WireError> {
    match discriminator(value, path)? {
        "audit-run-required" => {
            singleton(value, path, "audit-run-required")?;
            Ok(Blocker::AuditRunRequired)
        }
        "awaiting" => {
            let fields = object(value, path, AWAITING_FIELDS)?;
            Ok(Blocker::Awaiting(awaits(
                get(fields, "awaits"),
                &format!("{path}.awaits"),
            )?))
        }
        actual => Err(shape(
            &format!("{path}.kind"),
            WireFault::UnknownToken {
                expected: vec!["audit-run-required".into(), "awaiting".into()],
                actual: actual.into(),
            },
        )),
    }
}

struct NodeAssertion {
    evidence: Evidence,
    blocker: Option<Blocker>,
    path: String,
}
fn node(value: &Value, path: &str) -> Result<(NodeDraft, NodeAssertion), WireError> {
    let kind = discriminator(value, path)?;
    let claim = match kind {
        "claim" => true,
        "quantity" => false,
        actual => {
            return Err(shape(
                &format!("{path}.kind"),
                WireFault::UnknownToken {
                    expected: vec!["quantity".into(), "claim".into()],
                    actual: actual.into(),
                },
            ));
        }
    };
    let names: &[&str] = if claim { CLAIM_FIELDS } else { QUANTITY_FIELDS };
    let fields = object(value, path, names)?;
    let id = identity(get(fields, "id"), &format!("{path}.id"))?;
    let description = string(get(fields, "description"), &format!("{path}.description"))?.into();
    let payload = if claim {
        NodePayload::Claim {
            description,
            audit: audit(get(fields, "audit"), &format!("{path}.audit"))?,
        }
    } else {
        NodePayload::Quantity { description }
    };
    let status_path = format!("{path}.status");
    let names: &[&str] = if claim {
        CLAIM_STATUS_FIELDS
    } else {
        QUANTITY_STATUS_FIELDS
    };
    let status = object(get(fields, "status"), &status_path, names)?;
    let maturity = token(
        get(status, "maturity"),
        &format!("{status_path}.maturity"),
        MATURITY,
    )?;
    token(
        get(status, "identification"),
        &format!("{status_path}.identification"),
        &[((), "identified")],
    )?;
    token(
        get(status, "calibration"),
        &format!("{status_path}.calibration"),
        &[((), "not-applicable")],
    )?;
    let evidence = token(
        get(status, "evidence"),
        &format!("{status_path}.evidence"),
        &[
            (Evidence::NotAClaim, "not-a-claim"),
            (Evidence::Deferred, "deferred"),
        ],
    )?;
    let blocker = if claim {
        Some(blocker(
            get(status, "blocker"),
            &format!("{status_path}.blocker"),
        )?)
    } else {
        None
    };
    Ok((
        NodeDraft {
            id,
            maturity,
            payload,
        },
        NodeAssertion {
            evidence,
            blocker,
            path: status_path,
        },
    ))
}
fn edge(value: &Value, path: &str) -> Result<Edge, WireError> {
    let fields = object(value, path, EDGE_FIELDS)?;
    singleton(get(fields, "guard"), &format!("{path}.guard"), "always")?;
    Ok(Edge {
        relation: token(
            get(fields, "relation"),
            &format!("{path}.relation"),
            &[
                (RelationKind::DependsOn, "depends-on"),
                (RelationKind::CompatibleWith, "compatible-with"),
            ],
        )?,
        source: identity(get(fields, "source"), &format!("{path}.source"))?,
        target: identity(get(fields, "target"), &format!("{path}.target"))?,
        guard: Guard::Always,
    })
}
fn list<T>(
    value: &Value,
    path: &str,
    parse: impl Fn(&Value, &str) -> Result<T, WireError>,
) -> Result<Vec<T>, WireError> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, v)| parse(v, &format!("{path}[{i}]")))
        .collect()
}
fn coverage(value: &Value) -> Result<Coverage, WireError> {
    let fields = object(value, "$.coverage", COVERAGE_FIELDS)?;
    let closed = object(
        get(fields, "closed-world"),
        "$.coverage.closed-world",
        CLOSED_FIELDS,
    )?;
    Ok(Coverage {
        slice: list(get(fields, "slice"), "$.coverage.slice", identity)?,
        closed_world: ClosedWorld {
            referents: boolean(
                get(closed, "referents"),
                "$.coverage.closed-world.referents",
            )?,
            relations: boolean(
                get(closed, "relations"),
                "$.coverage.closed-world.relations",
            )?,
            audit: boolean(get(closed, "audit"), "$.coverage.closed-world.audit")?,
        },
        release_gate: token(
            get(fields, "release-gate"),
            "$.coverage.release-gate",
            &[
                (ReleaseGate::ExplicitPartial, "explicit-partial"),
                (ReleaseGate::Complete, "complete"),
            ],
        )?,
    })
}

struct BudgetAssertion {
    id: BudgetId,
    producer: ProducerId,
    pin: String,
    limits: Vec<(String, u64)>,
}
fn budget(value: &Value, path: &str) -> Result<BudgetAssertion, WireError> {
    let fields = object(value, path, BUDGET_FIELDS)?;
    token(
        get(fields, "profile"),
        &format!("{path}.profile"),
        &[((), "pinned-trial-limits-v2")],
    )?;
    let names = BudgetProfile::PinnedTrialLimitsV2
        .resolved()
        .fields()
        .map(|(name, _)| name);
    let limits_path = format!("{path}.limits");
    let limits = object(get(fields, "limits"), &limits_path, &names)?
        .iter()
        .map(|(name, v)| number(v, &format!("{limits_path}.{name}"), 64).map(|n| (name.clone(), n)))
        .collect::<Result<_, _>>()?;
    Ok(BudgetAssertion {
        id: token(get(fields, "id"), &format!("{path}.id"), BUDGETS)?,
        producer: token(
            get(fields, "producer"),
            &format!("{path}.producer"),
            PRODUCERS,
        )?,
        pin: string(get(fields, "pin"), &format!("{path}.pin"))?.into(),
        limits,
    })
}
fn producer(value: &Value, path: &str) -> Result<ProducerExport, WireError> {
    let f = object(value, path, PRODUCER_FIELDS)?;
    token(
        get(f, "region"),
        &format!("{path}.region"),
        &[((), "ambient-product-x-no-assumptions")],
    )?;
    Ok(ProducerExport {
        id: token(get(f, "id"), &format!("{path}.id"), PRODUCERS)?,
        claim: identity(get(f, "claim"), &format!("{path}.claim"))?,
        dependencies: list(
            get(f, "dependencies"),
            &format!("{path}.dependencies"),
            identity,
        )?,
        capabilities: list(
            get(f, "capabilities"),
            &format!("{path}.capabilities"),
            |v, p| token(v, p, CAPS),
        )?,
        budget: token(get(f, "budget"), &format!("{path}.budget"), BUDGETS)?,
        revision: u32::try_from(number(get(f, "revision"), &format!("{path}.revision"), 32)?)
            .expect("u32 range checked"),
        target: number(get(f, "target"), &format!("{path}.target"), 64)?,
        region: RegionContract::AmbientProductXNoAssumptions,
    })
}
fn capability(value: &Value, path: &str) -> Result<CapabilityExport, WireError> {
    let f = object(value, path, CAPABILITY_FIELDS)?;
    Ok(CapabilityExport {
        id: token(get(f, "id"), &format!("{path}.id"), CAPS)?,
        availability: token(
            get(f, "availability"),
            &format!("{path}.availability"),
            AVAILABILITY,
        )?,
        adapter_implemented: boolean(
            get(f, "adapter-implemented"),
            &format!("{path}.adapter-implemented"),
        )?,
        evidence: string(get(f, "evidence"), &format!("{path}.evidence"))?.into(),
    })
}

fn assertion(path: &str, position: Option<usize>, fault: ProjectionFault) -> ProjectionDiagnostic {
    ProjectionDiagnostic {
        rule: RuleId::M037,
        path: path.into(),
        position,
        fault,
    }
}
fn pin(actual: &str, path: &str, position: Option<usize>) -> Result<(), ProjectionDiagnostic> {
    if actual == ENGINE_PIN.0 {
        Ok(())
    } else {
        Err(assertion(
            path,
            position,
            ProjectionFault::PinMismatch {
                expected: ENGINE_PIN.0.into(),
                actual: actual.into(),
            },
        ))
    }
}
fn unique<T: PartialEq>(
    values: &[T],
    expected: &[(T, &str)],
    path: &str,
    complete: bool,
) -> Vec<ProjectionDiagnostic> {
    expected
        .iter()
        .filter_map(|(id, record)| {
            let positions: Vec<_> = values
                .iter()
                .enumerate()
                .filter_map(|(i, x)| (x == id).then_some(i))
                .collect();
            (positions.len() > 1 || (complete && positions.is_empty())).then(|| {
                assertion(
                    path,
                    positions.first().copied(),
                    ProjectionFault::ContractMultiplicity {
                        record: (*record).into(),
                        expected: 1,
                        actual: positions.len(),
                        positions,
                    },
                )
            })
        })
        .collect()
}
fn equal(
    expected: ProjectionValue,
    actual: ProjectionValue,
    path: &str,
    position: Option<usize>,
) -> Result<(), ProjectionDiagnostic> {
    if expected == actual {
        Ok(())
    } else {
        Err(assertion(
            path,
            position,
            ProjectionFault::ContractMismatch {
                expected: Box::new(expected),
                actual: Box::new(actual),
            },
        ))
    }
}
fn text(value: impl Into<String>) -> ProjectionValue {
    ProjectionValue::Text(value.into())
}

fn check_contracts(
    producers: &mut [ProducerExport],
    capabilities: &[CapabilityExport],
    budgets: &[BudgetAssertion],
) -> Result<Vec<BudgetExport>, Vec<ProjectionDiagnostic>> {
    let mut errors = Vec::new();
    let budget_duplicates = unique(
        &budgets.iter().map(|b| b.id).collect::<Vec<_>>(),
        BUDGETS,
        "$.budgets",
        false,
    );
    let producer_duplicates = unique(
        &producers.iter().map(|p| p.id).collect::<Vec<_>>(),
        PRODUCERS,
        "$.producers",
        true,
    );
    let capability_duplicates = unique(
        &capabilities.iter().map(|c| c.id).collect::<Vec<_>>(),
        CAPS,
        "$.capabilities",
        true,
    );
    let mut checked = Vec::new();
    if budget_duplicates.is_empty() {
        for (i, budget) in budgets.iter().enumerate() {
            if let Some(export) = check_budget(i, budget, &mut errors) {
                checked.push(export);
            }
        }
    }
    if producer_duplicates.is_empty() {
        for (i, producer) in producers.iter_mut().enumerate() {
            check_producer(i, producer, &mut errors);
        }
    }
    if capability_duplicates.is_empty() {
        for (i, capability) in capabilities.iter().enumerate() {
            check_capability(i, capability, &mut errors);
        }
    }
    errors.extend(budget_duplicates);
    errors.extend(producer_duplicates);
    errors.extend(capability_duplicates);
    errors.sort_by_key(|error| contract_key(error, producers, capabilities, budgets));
    if errors.is_empty() {
        Ok(checked)
    } else {
        Err(errors)
    }
}
fn contract_key(
    error: &ProjectionDiagnostic,
    producers: &[ProducerExport],
    capabilities: &[CapabilityExport],
    budgets: &[BudgetAssertion],
) -> (u8, String, String, String) {
    let index = error
        .path
        .split_once('[')
        .and_then(|(_, s)| s.split_once(']'))
        .and_then(|(i, _)| i.parse::<usize>().ok());
    let (rank, record) = if error.path.starts_with("$.budgets") {
        (
            0,
            index
                .and_then(|i| budgets.get(i))
                .map(|b| spelling(b.id, BUDGETS)),
        )
    } else if error.path.starts_with("$.producers") {
        (
            1,
            index
                .and_then(|i| producers.get(i))
                .map(|p| spelling(p.id, PRODUCERS)),
        )
    } else {
        (
            2,
            index
                .and_then(|i| capabilities.get(i))
                .map(|c| spelling(c.id, CAPS)),
        )
    };
    let record = record.map_or_else(
        || match &error.fault {
            ProjectionFault::ContractMultiplicity { record, .. } => record.clone(),
            _ => String::new(),
        },
        str::to_owned,
    );
    let field = error
        .path
        .split_once(']')
        .map_or("", |(_, field)| field)
        .to_owned();
    (rank, record, field, format!("{:?}", error.fault))
}
fn collect_assertion(
    errors: &mut Vec<ProjectionDiagnostic>,
    result: Result<(), ProjectionDiagnostic>,
) {
    if let Err(error) = result {
        errors.push(error);
    }
}
fn check_budget(
    i: usize,
    budget: &BudgetAssertion,
    errors: &mut Vec<ProjectionDiagnostic>,
) -> Option<BudgetExport> {
    let before = errors.len();
    let root = format!("$.budgets[{i}]");
    collect_assertion(errors, pin(&budget.pin, &format!("{root}.pin"), Some(i)));
    let expected = BudgetSpec::reviewed(budget.producer);
    collect_assertion(
        errors,
        equal(
            text(spelling(expected.id, BUDGETS)),
            text(spelling(budget.id, BUDGETS)),
            &format!("{root}.id"),
            Some(i),
        ),
    );
    for (name, expected) in expected.profile.resolved().fields() {
        let actual = budget
            .limits
            .iter()
            .find(|(key, _)| key == name)
            .expect("exact limits shape")
            .1;
        if expected != actual {
            errors.push(assertion(
                &format!("{root}.limits.{name}"),
                Some(i),
                ProjectionFault::BudgetLimitsMismatch { expected, actual },
            ));
        }
    }
    (errors.len() == before).then_some(BudgetExport {
        spec: expected,
        limits: expected.profile.resolved(),
    })
}

/// Load a strict projection, checking contracts, base status, history and stores.
///
/// # Errors
/// Returns the first earlier-phase fault or the complete graph/addressing diagnostics.
/// Fixed-assertion faults accumulate in component, semantic-record and field order.
///
/// # Panics
/// Panics only if an internal exact-shape or closed-table invariant is violated,
/// which input bytes cannot arrange.
pub fn load_projection(bytes: &[u8]) -> Result<LoadedProjection, ProjectionError> {
    load_with_authority(bytes, &crate::tag_history::seed_history())
}

fn load_with_authority(
    bytes: &[u8],
    authority: &[HistoryEvent],
) -> Result<LoadedProjection, ProjectionError> {
    let value = syntax::parse(bytes)?;
    let root = object(&value, "$", ROOT)?;
    let schema = get(root, "schema");
    let Value::Integer(schema) = schema else {
        return Err(wrong(schema, "$.schema", "integer").into());
    };
    if schema != "1" {
        return Err(WireError {
            rule: RuleId::M003,
            location: WireLocation::Path("$.schema".into()),
            fault: WireFault::SchemaValue {
                expected: "1".into(),
                actual: schema.clone(),
            },
        }
        .into());
    }
    let engine_pin = string(get(root, "engine-pin"), "$.engine-pin")?.to_owned();
    let (nodes, assertions): (Vec<_>, Vec<_>) = list(get(root, "nodes"), "$.nodes", node)?
        .into_iter()
        .unzip();
    let edges = list(get(root, "edges"), "$.edges", edge)?;
    let coverage = coverage(get(root, "coverage"))?;
    let budget_assertions = list(get(root, "budgets"), "$.budgets", budget)?;
    let mut producers = list(get(root, "producers"), "$.producers", producer)?;
    let capabilities = list(get(root, "capabilities"), "$.capabilities", capability)?;
    let tags = list(get(root, "tags"), "$.tags", tag_record)?;
    let history = list(get(root, "history"), "$.history", history_record)?;
    let stores = stores_record(get(root, "stores"))?;
    let mut errors = Vec::new();
    collect_assertion(&mut errors, pin(&engine_pin, "$.engine-pin", None));
    let budgets = match check_contracts(&mut producers, &capabilities, &budget_assertions) {
        Ok(budgets) => budgets,
        Err(faults) => {
            errors.extend(faults);
            Vec::new()
        }
    };
    if !errors.is_empty() {
        return Err(ProjectionError::Assertion(errors));
    }
    let manifest = validate(ManifestDraft {
        nodes,
        edges,
        coverage,
        budgets: budgets.iter().map(|b| b.spec).collect(),
    })
    .map_err(ProjectionError::Graph)?;
    for (node, asserted) in manifest.nodes().iter().zip(assertions) {
        let status = Status::derive(node);
        for (axis, expected, actual) in [
            (
                "evidence",
                evidence_token(status.evidence()).into(),
                evidence_token(asserted.evidence).into(),
            ),
            (
                "blocker",
                format!("{:?}", status.blocker()),
                format!("{:?}", asserted.blocker),
            ),
        ] {
            if expected != actual {
                return Err(WireError {
                    rule: RuleId::M012,
                    location: WireLocation::Path(format!("{}.{axis}", asserted.path)),
                    fault: WireFault::StatusMismatch {
                        node: node.id,
                        expected,
                        actual,
                    },
                }
                .into());
            }
        }
    }
    let book = admit_addressing(&manifest, &tags, &history, &stores, authority)?;
    let mut live: Vec<_> = book.live().collect();
    let mut retired: Vec<_> = book.retired().cloned().collect();
    let mut out_of_slice: Vec<_> = book.out_of_slice().copied().collect();
    // Retain input positions for the independent source comparator, while
    // taking every assignment and cause exclusively from the checked derivation.
    let position = |tag| stores.iter().find(|r| r.tag == tag).map(|r| r.position);
    live.sort_by_key(|tag| position(*tag));
    retired.sort_by_key(|entry| position(entry.tag()));
    out_of_slice.sort_by_key(|entry| position(entry.tag()));
    Ok(LoadedProjection {
        tags: tags.iter().map(|(tag, _)| tag.descriptor()).collect(),
        history,
        live,
        retired,
        out_of_slice,
        manifest,
        pin: engine_pin,
        budgets,
        producers,
        capabilities,
    })
}

fn s(value: impl Into<String>) -> Value {
    Value::String(value.into())
}
fn evidence_token(evidence: Evidence) -> &'static str {
    match evidence {
        Evidence::NotAClaim => "not-a-claim",
        Evidence::Deferred => "deferred",
    }
}
fn n(value: u64) -> Value {
    Value::Integer(value.to_string())
}
#[cfg(test)]
fn obj<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn arr<T>(values: impl IntoIterator<Item = T>, convert: impl Fn(T) -> Value) -> Value {
    Value::Array(values.into_iter().map(convert).collect())
}
fn kind(value: &str) -> Value {
    ordered(KIND_FIELDS, [("kind", s(value))])
}
fn awaiting_value(value: Awaiting) -> Value {
    match value {
        Awaiting::AuditBinding => kind("audit-binding"),
        Awaiting::Capability(c) => ordered(
            CAPABILITY_AWAIT_FIELDS,
            [
                ("kind", s("capability")),
                ("capability", s(spelling(c, CAPS))),
            ],
        ),
    }
}
fn audit_value(value: ClaimAudit) -> Value {
    match value {
        ClaimAudit::Awaiting(a) => ordered(
            AWAITING_FIELDS,
            [("kind", s("awaiting")), ("awaits", awaiting_value(a))],
        ),
        ClaimAudit::Bound {
            producer,
            budget,
            routes: AuditRoutes::Unconditional,
        } => ordered(
            BOUND_FIELDS,
            [
                ("kind", s("bound")),
                ("producer", s(spelling(producer, PRODUCERS))),
                ("budget", s(spelling(budget, BUDGETS))),
                ("routes", kind("unconditional")),
            ],
        ),
    }
}
fn node_value(node: &NodeDraft) -> Value {
    let status = Status::derive(node);
    let mut axes = vec![
        ("maturity".into(), s(spelling(status.maturity(), MATURITY))),
        ("identification".into(), s("identified")),
        ("calibration".into(), s("not-applicable")),
        (
            "evidence".into(),
            s(match status.evidence() {
                Evidence::NotAClaim => "not-a-claim",
                Evidence::Deferred => "deferred",
            }),
        ),
    ];
    if let Some(blocker) = status.blocker() {
        axes.push((
            "blocker".into(),
            match blocker {
                Blocker::AuditRunRequired => kind("audit-run-required"),
                Blocker::Awaiting(a) => ordered(
                    AWAITING_FIELDS,
                    [("kind", s("awaiting")), ("awaits", awaiting_value(a))],
                ),
            },
        ));
    }
    let mut fields = vec![
        ("id".into(), s(node.id.semantic_name())),
        (
            "kind".into(),
            s(match node.payload.kind() {
                NodeKind::Quantity => "quantity",
                NodeKind::Claim => "claim",
            }),
        ),
        ("description".into(), s(node.payload.description())),
        (
            "status".into(),
            ordered_fields(
                if node.payload.kind() == NodeKind::Claim {
                    CLAIM_STATUS_FIELDS
                } else {
                    QUANTITY_STATUS_FIELDS
                },
                axes,
            ),
        ),
    ];
    if let NodePayload::Claim { audit, .. } = node.payload {
        fields.push(("audit".into(), audit_value(audit)));
    }
    ordered_fields(
        if node.payload.kind() == NodeKind::Claim {
            CLAIM_FIELDS
        } else {
            QUANTITY_FIELDS
        },
        fields,
    )
}
fn producer_value(id: ProducerId) -> Value {
    let p = id.descriptor();
    let mut dependencies = p.dependencies.to_vec();
    dependencies.sort();
    let mut capabilities = p.capabilities.to_vec();
    capabilities.sort_by_key(|c| spelling(*c, CAPS));
    ordered(
        PRODUCER_FIELDS,
        [
            ("id", s(spelling(id, PRODUCERS))),
            ("claim", s(p.claim.semantic_name())),
            (
                "dependencies",
                arr(dependencies, |id| s(id.semantic_name())),
            ),
            ("capabilities", arr(capabilities, |c| s(spelling(c, CAPS)))),
            ("budget", s(spelling(p.budget, BUDGETS))),
            ("revision", n(u64::from(p.revision))),
            ("target", n(p.target as u64)),
            ("region", s("ambient-product-x-no-assumptions")),
        ],
    )
}
fn render(value: &Value, out: &mut String) {
    match value {
        Value::Object(fields) => {
            out.push('{');
            for (i, (key, value)) in fields.iter().enumerate() {
                if i != 0 {
                    out.push(',');
                }
                render(&s(key), out);
                out.push(':');
                render(value, out);
            }
            out.push('}');
        }
        Value::Array(values) => {
            out.push('[');
            for (i, value) in values.iter().enumerate() {
                if i != 0 {
                    out.push(',');
                }
                render(value, out);
            }
            out.push(']');
        }
        Value::String(value) => {
            out.push_str(
                &serde_json::to_string(value).expect("string serialization is infallible"),
            );
        }
        Value::Integer(value) => out.push_str(value),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Null => out.push_str("null"),
    }
}
/// The sole canonical projection writer. Fixed assertions are checked before bytes escape.
///
/// # Errors
/// Returns fixed-contract or addressing faults before emitting a projection.
///
/// # Panics
/// Panics only if an internal exact-shape or string-serialization invariant is
/// violated, which an admitted graph cannot arrange.
pub fn projection_bytes(graph: &ValidatedManifest) -> Result<Vec<u8>, ProjectionError> {
    let book = crate::addressing(graph).map_err(|e| addressing_error(&e))?;
    write_projection(&book)
}

#[cfg(test)]
fn projection_with_authority(
    graph: &ValidatedManifest,
    authority: &[HistoryEvent],
) -> Result<Vec<u8>, ProjectionError> {
    let book = crate::tag_validation::construct(
        graph,
        TagId::ALL.map(TagId::descriptor).to_vec(),
        authority.to_vec(),
    )
    .map_err(|e| addressing_error(&e))?;
    write_projection(&book)
}

pub(crate) fn write_projection(book: &AddressBook<'_>) -> Result<Vec<u8>, ProjectionError> {
    let graph = book.graph();
    let mut nodes: Vec<_> = graph.nodes().iter().collect();
    nodes.sort_by_key(|n| n.id);
    let mut edges = graph.edges().to_vec();
    edges.sort();
    let mut slice = graph.coverage().slice.clone();
    slice.sort();
    let mut budgets = graph.budgets().to_vec();
    budgets.sort_by_key(|b| spelling(b.id, BUDGETS));
    let closed = graph.coverage().closed_world;
    let mut producers = ProducerId::ALL;
    producers.sort_by_key(|id| spelling(*id, PRODUCERS));
    let mut capabilities = CAPABILITIES;
    capabilities.sort_by_key(|c| spelling(c.id, CAPS));
    let value = ordered(
        ROOT,
        [
            ("schema", n(1)),
            ("engine-pin", s(ENGINE_PIN.0)),
            ("nodes", arr(nodes, node_value)),
            (
                "edges",
                arr(edges, |e| {
                    ordered(
                        EDGE_FIELDS,
                        [
                            ("relation", s(e.relation.name())),
                            ("source", s(e.source.semantic_name())),
                            ("target", s(e.target.semantic_name())),
                            ("guard", kind("always")),
                        ],
                    )
                }),
            ),
            (
                "coverage",
                ordered(
                    COVERAGE_FIELDS,
                    [
                        ("slice", arr(slice, |id| s(id.semantic_name()))),
                        (
                            "closed-world",
                            ordered(
                                CLOSED_FIELDS,
                                [
                                    ("referents", Value::Bool(closed.referents)),
                                    ("relations", Value::Bool(closed.relations)),
                                    ("audit", Value::Bool(closed.audit)),
                                ],
                            ),
                        ),
                        (
                            "release-gate",
                            s(match graph.coverage().release_gate {
                                ReleaseGate::ExplicitPartial => "explicit-partial",
                                ReleaseGate::Complete => "complete",
                            }),
                        ),
                    ],
                ),
            ),
            ("budgets", arr(budgets, budget_value)),
            ("producers", arr(producers, producer_value)),
            ("capabilities", arr(capabilities, capability_value)),
            (
                "tags",
                arr(book.descriptors.iter(), |d| {
                    ordered(
                        TAG_FIELDS,
                        [
                            ("tag", s(d.spelling())),
                            ("target", s(d.target().semantic_name())),
                        ],
                    )
                }),
            ),
            ("history", arr(book.history(), history_value)),
            ("stores", stores_value(book)),
        ],
    );
    let mut output = String::new();
    render(&value, &mut output);
    output.push('\n');
    load_with_authority(output.as_bytes(), &book.history)?;
    Ok(output.into_bytes())
}

fn capability_value(c: crate::CapabilitySpec) -> Value {
    ordered(
        CAPABILITY_FIELDS,
        [
            ("id", s(spelling(c.id, CAPS))),
            ("availability", s(spelling(c.availability, AVAILABILITY))),
            ("adapter-implemented", Value::Bool(c.adapter_implemented)),
            ("evidence", s(c.evidence)),
        ],
    )
}
fn budget_value(budget: BudgetSpec) -> Value {
    ordered(
        BUDGET_FIELDS,
        [
            ("id", s(spelling(budget.id, BUDGETS))),
            ("producer", s(spelling(budget.producer, PRODUCERS))),
            ("profile", s("pinned-trial-limits-v2")),
            ("pin", s(budget.pin.0)),
            (
                "limits",
                Value::Object(
                    budget
                        .profile
                        .resolved()
                        .fields()
                        .into_iter()
                        .map(|(key, value)| (key.into(), n(value)))
                        .collect(),
                ),
            ),
        ],
    )
}

fn check_producer(i: usize, p: &mut ProducerExport, errors: &mut Vec<ProjectionDiagnostic>) {
    let root = format!("$.producers[{i}]");
    let expected = p.id.descriptor();
    let duplicates = unique(
        &p.dependencies,
        &NodeId::ALL.map(|id| (id, id.semantic_name())),
        &format!("{root}.dependencies"),
        false,
    );
    if duplicates.is_empty() {
        p.dependencies.sort();
        let mut expected_dependencies = expected.dependencies.to_vec();
        expected_dependencies.sort();
        collect_assertion(
            errors,
            equal(
                ProjectionValue::Identities(expected_dependencies),
                ProjectionValue::Identities(p.dependencies.clone()),
                &format!("{root}.dependencies"),
                Some(i),
            ),
        );
    }
    errors.extend(duplicates);
    let duplicates = unique(
        &p.capabilities,
        CAPS,
        &format!("{root}.capabilities"),
        false,
    );
    if duplicates.is_empty() {
        p.capabilities.sort_by_key(|c| spelling(*c, CAPS));
        let mut expected_capabilities = expected.capabilities.to_vec();
        expected_capabilities.sort_by_key(|c| spelling(*c, CAPS));
        collect_assertion(
            errors,
            equal(
                ProjectionValue::Capabilities(expected_capabilities),
                ProjectionValue::Capabilities(p.capabilities.clone()),
                &format!("{root}.capabilities"),
                Some(i),
            ),
        );
    }
    errors.extend(duplicates);
    for (field, e, a) in [
        (
            "claim",
            text(expected.claim.semantic_name()),
            text(p.claim.semantic_name()),
        ),
        (
            "budget",
            text(spelling(expected.budget, BUDGETS)),
            text(spelling(p.budget, BUDGETS)),
        ),
        (
            "revision",
            ProjectionValue::Integer(u64::from(expected.revision)),
            ProjectionValue::Integer(u64::from(p.revision)),
        ),
        (
            "target",
            ProjectionValue::Integer(expected.target as u64),
            ProjectionValue::Integer(p.target),
        ),
    ] {
        collect_assertion(errors, equal(e, a, &format!("{root}.{field}"), Some(i)));
    }
}
fn check_capability(i: usize, c: &CapabilityExport, errors: &mut Vec<ProjectionDiagnostic>) {
    let expected = CAPABILITIES
        .iter()
        .find(|e| e.id == c.id)
        .expect("closed capability");
    let root = format!("$.capabilities[{i}]");
    for (field, e, a) in [
        (
            "availability",
            text(spelling(expected.availability, AVAILABILITY)),
            text(spelling(c.availability, AVAILABILITY)),
        ),
        (
            "adapter-implemented",
            ProjectionValue::Boolean(expected.adapter_implemented),
            ProjectionValue::Boolean(c.adapter_implemented),
        ),
        ("evidence", text(expected.evidence), text(&c.evidence)),
    ] {
        collect_assertion(errors, equal(e, a, &format!("{root}.{field}"), Some(i)));
    }
}

fn ordered<const N: usize>(order: &[&str], fields: [(&str, Value); N]) -> Value {
    ordered_fields(
        order,
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn ordered_fields(order: &[&str], mut fields: Vec<(String, Value)>) -> Value {
    fields.sort_by_key(|(key, _)| {
        order
            .iter()
            .position(|expected| expected == key)
            .expect("writer field belongs to the exact grammar")
    });
    Value::Object(fields)
}
