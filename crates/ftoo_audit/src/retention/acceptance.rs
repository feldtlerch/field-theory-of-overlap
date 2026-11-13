//! Required-input admission, fresh derivation and mismatch-only classification.
use super::{
    difference::Difference,
    info::{InfoRecord, Run},
    production, revision,
    wire::{self, Codec},
};
use crate::{AuditError, AuditReport};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputKey {
    pub projection: Vec<u8>,
    pub contract: u32,
    pub engine: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Component {
    ProjectionBytes,
    ProjectionDigest,
    ContractRevision,
    EngineRevision,
    Pin,
    Target,
    Summary,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvenanceValue {
    Text(String),
    U32(u32),
    U64(u64),
    Summary(Box<wire::Summary>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvenanceDifference {
    Values {
        expected: ProvenanceValue,
        actual: ProvenanceValue,
    },
    Bytes(Box<Difference>),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    FreshProcesses,
    RetainedAgainstFresh,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusAxis {
    Maturity,
    Identification,
    Calibration,
    Evidence,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetentionError {
    UnsupportedComponent {
        component: &'static str,
    },
    Multiple(Vec<super::RetentionDiagnostic>),
    Wire(wire::WireRefusal),
    Projection {
        path: String,
        error: Box<ftoo_manifest::ProjectionError>,
    },
    EngineRevisionUnavailable(revision::Unavailable),
    ProvenanceMismatch {
        component: Component,
        path: String,
        difference: ProvenanceDifference,
    },
    Domain(Vec<ftoo_manifest::Diagnostic>),
    StatusDomain {
        expected: Vec<ftoo_manifest::NodeId>,
        actual: Vec<ftoo_manifest::NodeId>,
    },
    StatusAssertion {
        index: usize,
        axis: StatusAxis,
        expected: wire::Status,
        actual: wire::Status,
    },
    AssessmentAssertion {
        expected: wire::Assessment,
        actual: wire::Assessment,
    },
    Audit(AuditError),
    NonReproducibility {
        key: Box<InputKey>,
        basis: Basis,
        retained: Vec<u8>,
        fresh1: Vec<u8>,
        fresh2: Vec<u8>,
        information: Box<[InfoRecord; 3]>,
        differences: Vec<Difference>,
    },
    DiagnosticRerunFailed {
        key: Box<InputKey>,
        retained: Vec<u8>,
        fresh1: Vec<u8>,
        differences: Vec<Difference>,
        error: Box<AuditError>,
        info: Box<InfoRecord>,
    },
}
#[derive(Clone, Debug)]
pub struct Produced {
    pub(super) key: InputKey,
    pub(super) report: Arc<AuditReport>,
    pub(super) dto: wire::Report,
    pub(super) bytes: Vec<u8>,
    pub(super) info: InfoRecord,
}
/// Fresh checked evidence for exactly the accepted byte string.
/// ```compile_fail
/// use ftoo_audit::retention::AcceptedRetention;
/// let forged = AcceptedRetention { production: todo!(), bytes: Vec::new() };
/// ```
#[derive(Debug)]
pub struct AcceptedRetention {
    production: Arc<Produced>,
    bytes: Vec<u8>,
}
impl AcceptedRetention {
    #[must_use]
    pub fn information(&self) -> &InfoRecord {
        &self.production.info
    }
    #[must_use]
    pub fn report(&self) -> &AuditReport {
        &self.production.report
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
static MEMO: Mutex<VecDeque<Arc<Produced>>> = Mutex::new(VecDeque::new());
fn mismatch(
    component: Component,
    path: &str,
    expected: &[u8],
    actual: &[u8],
) -> Result<(), RetentionError> {
    if component == Component::ProjectionBytes {
        let Some(mut difference) = Difference::between(
            expected,
            actual,
            InfoRecord::collect(Run::Retained),
            InfoRecord::collect(Run::Retained),
        ) else {
            return Ok(());
        };
        difference.path = path.into();
        return Err(RetentionError::ProvenanceMismatch {
            component,
            path: path.into(),
            difference: ProvenanceDifference::Bytes(Box::new(difference)),
        });
    }
    let value = |bytes: &[u8]| {
        ProvenanceValue::Text(String::from_utf8(bytes.into()).expect("checked textual identity"))
    };
    field_mismatch(component, path, value(expected), value(actual))
}

fn field_mismatch(
    component: Component,
    path: &str,
    expected: ProvenanceValue,
    actual: ProvenanceValue,
) -> Result<(), RetentionError> {
    if expected == actual {
        Ok(())
    } else {
        Err(RetentionError::ProvenanceMismatch {
            component,
            path: path.into(),
            difference: ProvenanceDifference::Values { expected, actual },
        })
    }
}
fn key(projection: Vec<u8>) -> Result<InputKey, RetentionError> {
    let engine = revision::linked().map_err(RetentionError::EngineRevisionUnavailable)?;
    mismatch(
        Component::EngineRevision,
        "Cargo.lock",
        ftoo_manifest::ENGINE_PIN.0.as_bytes(),
        engine.as_bytes(),
    )?;
    let loaded = ftoo_manifest::load_projection(&projection).map_err(|error| {
        RetentionError::Projection {
            path: "generated/manifest.json".into(),
            error: Box::new(error),
        }
    })?;
    let canonical = ftoo_manifest::projection_bytes(loaded.manifest()).map_err(|error| {
        RetentionError::Projection {
            path: "generated/manifest.json".into(),
            error: Box::new(error),
        }
    })?;
    mismatch(
        Component::ProjectionBytes,
        "generated/manifest.json",
        &canonical,
        &projection,
    )?;
    Ok(InputKey {
        projection,
        contract: super::CONTRACT_REVISION,
        engine,
    })
}
pub(super) fn fresh(key: &InputKey, run: Run) -> Result<Produced, RetentionError> {
    let loaded = ftoo_manifest::load_projection(&key.projection).map_err(|error| {
        RetentionError::Projection {
            path: "generated/manifest.json".into(),
            error: Box::new(error),
        }
    })?;
    let report = crate::audit(loaded.manifest()).map_err(RetentionError::Audit)?;
    let projection = String::from_utf8(key.projection.clone()).expect("projection admitted UTF-8");
    let dto = production::report(
        &report,
        projection,
        wire::hex(&Sha256::digest(&key.projection)),
        key.engine.clone(),
    )?;
    let bytes = wire::write(&dto).map_err(RetentionError::Wire)?;
    wire::decode(&bytes).map_err(RetentionError::Wire)?;
    Ok(Produced {
        key: key.clone(),
        report: Arc::new(report),
        dto,
        bytes,
        info: InfoRecord::collect(run),
    })
}
/// Derive immutable canonical bytes through the closed audit registry.
/// # Errors
/// Returns complete projection, linked-engine, audit or serialization refusals.
/// # Panics
/// Panics only if a private checked-result invariant fails, which input cannot arrange.
pub fn produce(projection: Vec<u8>) -> Result<Arc<Produced>, RetentionError> {
    let key = key(projection)?;
    let mut memo = MEMO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(production) = memo.iter().find(|production| production.key == key) {
        return Ok(Arc::clone(production));
    }
    let production = Arc::new(fresh(&key, Run::Fresh1)?);
    if memo.len() == 2 {
        memo.pop_front();
    }
    memo.push_back(Arc::clone(&production));
    Ok(production)
}
fn bytes(value: &impl Codec) -> Vec<u8> {
    let mut output = String::new();
    wire::render(&value.value(), &mut output);
    output.into_bytes()
}
fn domain(
    dto: &wire::Report,
    graph: &ftoo_manifest::ValidatedManifest,
) -> Result<(), RetentionError> {
    let rows = dto
        .outcomes
        .iter()
        .map(|row| {
            let (class, reason, receipt) = match &row.verdict {
                wire::Verdict::Certified(receipt) => {
                    (ftoo_manifest::VerdictClass::Certified, None, Some(receipt))
                }
                wire::Verdict::Refuted { receipt, .. } => {
                    (ftoo_manifest::VerdictClass::Refuted, None, Some(receipt))
                }
                wire::Verdict::Undecided { .. } => {
                    (ftoo_manifest::VerdictClass::Undecided, None, None)
                }
                wire::Verdict::Deferred(reason) => {
                    (ftoo_manifest::VerdictClass::Deferred, Some(*reason), None)
                }
            };
            let mut draft = graph.node(row.claim).map_or(
                ftoo_manifest::OutcomeDraft {
                    claim: row.claim,
                    producer: None,
                    revision: None,
                    budget: None,
                    pin: ftoo_manifest::ENGINE_PIN,
                    verdict: class,
                    deferred: reason,
                },
                |node| ftoo_manifest::OutcomeDraft::new(node, class, reason),
            );
            if let Some(receipt) = receipt {
                draft.producer = Some(receipt.producer);
                draft.revision = Some(receipt.revision);
                draft.budget = Some(receipt.budget);
            }
            draft
        })
        .collect::<Vec<_>>();
    let diagnostics = ftoo_manifest::report_diagnostics(
        graph,
        &ftoo_manifest::Snapshot::capture(graph),
        &dto.reached,
        &rows,
    );
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(RetentionError::Domain(diagnostics))
    }
}
/// Accept exact owned bytes only after required-input equality and re-derivation.
/// # Errors
/// Returns complete input or reproduction refusals; never repairs the submitted bytes.
/// # Panics
/// Panics only if a private checked-result invariant fails, which input cannot arrange.
pub fn accept(projection: Vec<u8>, retained: Vec<u8>) -> Result<AcceptedRetention, RetentionError> {
    accept_after(retained, || Ok(projection), None)
}
/// Admit wire bytes once before collecting current inputs, then derive and compare.
/// An optional owned production serves only its exact input key during regeneration.
/// # Errors
/// Preserves wire, input and complete reproduction refusals in the caller's error type.
/// # Panics
/// Panics only if a private checked-result invariant fails, which input cannot arrange.
pub fn accept_after<E: From<RetentionError>>(
    retained: Vec<u8>,
    inputs: impl FnOnce() -> Result<Vec<u8>, E>,
    production: Option<&Arc<Produced>>,
) -> Result<AcceptedRetention, E> {
    let dto = wire::decode(&retained).map_err(RetentionError::Wire)?;
    let projection = inputs()?;
    let derive = &mut |key: &InputKey, run| fresh(key, run).map(Arc::new);
    if let Some(production) = production {
        return accept_decoded(
            projection,
            retained,
            &dto,
            &mut VecDeque::from([Arc::clone(production)]),
            derive,
        )
        .map_err(E::from);
    }
    let mut memo = MEMO
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    accept_decoded(projection, retained, &dto, &mut memo, derive).map_err(E::from)
}

#[cfg(test)]
pub(super) fn accept_with(
    projection: Vec<u8>,
    retained: Vec<u8>,
    memo: &mut VecDeque<Arc<Produced>>,
    derive: &mut impl FnMut(&InputKey, Run) -> Result<Arc<Produced>, RetentionError>,
) -> Result<AcceptedRetention, RetentionError> {
    let dto = wire::decode(&retained).map_err(RetentionError::Wire)?;
    accept_decoded(projection, retained, &dto, memo, derive)
}
fn accept_decoded(
    projection: Vec<u8>,
    retained: Vec<u8>,
    dto: &wire::Report,
    memo: &mut VecDeque<Arc<Produced>>,
    derive: &mut impl FnMut(&InputKey, Run) -> Result<Arc<Produced>, RetentionError>,
) -> Result<AcceptedRetention, RetentionError> {
    let key = key(projection)?;
    check_inputs(dto, &key)?;
    // Hold the production lock through comparison: a refused file must never
    // publish a new successful memo entry, including during concurrent calls.
    let cached = memo
        .iter()
        .find(|production| production.key == key)
        .cloned();
    let first = match &cached {
        Some(production) => Arc::clone(production),
        None => derive(&key, Run::Fresh1)?,
    };
    compare_receipts(&first.dto, dto)?;
    if first.dto.outcomes == dto.outcomes && first.dto.reached == dto.reached {
        compare_assertions(&first.dto, dto)?;
    }
    if retained == first.bytes {
        if cached.is_none() {
            if memo.len() == 2 {
                memo.pop_front();
            }
            memo.push_back(Arc::clone(&first));
        }
        return Ok(AcceptedRetention {
            production: first,
            bytes: retained,
        });
    }
    let original_differences = super::difference::reports(
        dto,
        &first.dto,
        &retained,
        &first.bytes,
        &InfoRecord::collect(Run::Retained),
        &first.info,
    );
    let second = match derive(&key, Run::Fresh2) {
        Ok(second) => second,
        Err(RetentionError::Audit(error)) => {
            return Err(RetentionError::DiagnosticRerunFailed {
                key: Box::new(key),
                retained,
                fresh1: first.bytes.clone(),
                differences: original_differences,
                error: Box::new(error),
                info: Box::new(InfoRecord::collect(Run::Fresh2)),
            });
        }
        Err(error) => return Err(error),
    };
    let basis = if first.bytes == second.bytes {
        Basis::RetainedAgainstFresh
    } else {
        Basis::FreshProcesses
    };
    let mut differences = super::difference::reports(
        &first.dto,
        &second.dto,
        &first.bytes,
        &second.bytes,
        &first.info,
        &second.info,
    );
    differences.extend(original_differences);
    Err(RetentionError::NonReproducibility {
        key: Box::new(key),
        basis,
        retained,
        fresh1: first.bytes.clone(),
        fresh2: second.bytes.clone(),
        information: Box::new([
            InfoRecord::collect(Run::Retained),
            first.info.clone(),
            second.info.clone(),
        ]),
        differences,
    })
}
fn check_inputs(dto: &wire::Report, key: &InputKey) -> Result<(), RetentionError> {
    ftoo_manifest::load_projection(dto.projection.as_bytes()).map_err(|error| {
        RetentionError::Projection {
            path: "$.provenance.projection-utf8".into(),
            error: Box::new(error),
        }
    })?;
    collect_faults([
        mismatch(
            Component::ProjectionBytes,
            "$.provenance.projection-utf8",
            &key.projection,
            dto.projection.as_bytes(),
        ),
        mismatch(
            Component::ProjectionDigest,
            "$.provenance.projection-sha256",
            wire::hex(&Sha256::digest(dto.projection.as_bytes())).as_bytes(),
            dto.digest.as_bytes(),
        ),
        field_mismatch(
            Component::ContractRevision,
            "$.provenance.inputs.contract-revision",
            ProvenanceValue::U32(key.contract),
            ProvenanceValue::U32(dto.contract),
        ),
        mismatch(
            Component::EngineRevision,
            "$.provenance.inputs.engine-revision",
            key.engine.as_bytes(),
            dto.engine.as_bytes(),
        ),
    ])?;
    let loaded = ftoo_manifest::load_projection(&key.projection).map_err(|error| {
        RetentionError::Projection {
            path: "generated/manifest.json".into(),
            error: Box::new(error),
        }
    })?;
    domain(dto, loaded.manifest())?;
    let mut expected_statuses: Vec<_> = ftoo_manifest::NodeId::ALL
        .into_iter()
        .filter(|id| loaded.manifest().node(*id).is_some())
        .collect();
    expected_statuses.sort();
    let actual_statuses: Vec<_> = dto.statuses.iter().map(|row| row.id).collect();
    if expected_statuses != actual_statuses {
        return Err(RetentionError::StatusDomain {
            expected: expected_statuses,
            actual: actual_statuses,
        });
    }
    receipt_inputs(dto)
}
fn collect_faults(
    results: impl IntoIterator<Item = Result<(), RetentionError>>,
) -> Result<(), RetentionError> {
    let mut faults: Vec<_> = results.into_iter().filter_map(Result::err).collect();
    match faults.len() {
        0 => Ok(()),
        1 => Err(faults.pop().expect("one fault")),
        _ => {
            let mut diagnostics: Vec<_> = faults
                .into_iter()
                .flat_map(|fault| fault.diagnostics())
                .collect();
            diagnostics.sort_by(|a, b| (a.rule, &a.site).cmp(&(b.rule, &b.site)));
            Err(RetentionError::Multiple(diagnostics))
        }
    }
}
fn compare_receipts(expected: &wire::Report, actual: &wire::Report) -> Result<(), RetentionError> {
    let mut checks = Vec::new();
    for (index, (expected, actual)) in expected.outcomes.iter().zip(&actual.outcomes).enumerate() {
        let receipt = |verdict: &wire::Verdict| match verdict {
            wire::Verdict::Certified(receipt) | wire::Verdict::Refuted { receipt, .. } => {
                Some(receipt.as_ref().clone())
            }
            _ => None,
        };
        if let (Some(expected), Some(actual)) =
            (receipt(&expected.verdict), receipt(&actual.verdict))
        {
            checks.push(field_mismatch(
                Component::Summary,
                &format!("$.outcomes[{index}].verdict.receipt.summary"),
                ProvenanceValue::Summary(Box::new(expected.summary)),
                ProvenanceValue::Summary(Box::new(actual.summary)),
            ));
        }
    }
    collect_faults(checks)
}
fn compare_assertions(
    expected: &wire::Report,
    actual: &wire::Report,
) -> Result<(), RetentionError> {
    let mut checks = Vec::new();
    for (index, (expected, actual)) in expected.statuses.iter().zip(&actual.statuses).enumerate() {
        for (axis, equal) in [
            (StatusAxis::Maturity, expected.maturity == actual.maturity),
            (
                StatusAxis::Identification,
                expected.identification == actual.identification,
            ),
            (
                StatusAxis::Calibration,
                expected.calibration == actual.calibration,
            ),
            (StatusAxis::Evidence, expected.evidence == actual.evidence),
        ] {
            if !equal {
                checks.push(Err(RetentionError::StatusAssertion {
                    index,
                    axis,
                    expected: expected.clone(),
                    actual: actual.clone(),
                }));
            }
        }
    }
    collect_faults(checks)?;
    if expected.assessment != actual.assessment {
        return Err(RetentionError::AssessmentAssertion {
            expected: expected.assessment.clone(),
            actual: actual.assessment.clone(),
        });
    }
    Ok(())
}

fn receipt_inputs(report: &wire::Report) -> Result<(), RetentionError> {
    let mut checks = Vec::new();
    for (index, row) in report.outcomes.iter().enumerate() {
        let (wire::Verdict::Certified(receipt) | wire::Verdict::Refuted { receipt, .. }) =
            &row.verdict
        else {
            continue;
        };
        let path = format!("$.outcomes[{index}].verdict.receipt");
        checks.push(mismatch(
            Component::Pin,
            &format!("{path}.pin"),
            ftoo_manifest::ENGINE_PIN.0.as_bytes(),
            receipt.pin.as_bytes(),
        ));
        let descriptor = receipt.producer.descriptor();
        checks.push(field_mismatch(
            Component::Target,
            &format!("{path}.target"),
            ProvenanceValue::U64(descriptor.target as u64),
            ProvenanceValue::U64(receipt.target),
        ));
    }
    collect_faults(checks)
}

impl Produced {
    /// Strictly accept a reread against this exact successful production.
    /// The owned production remains available even if the shared memo evicts it.
    /// # Errors
    /// Returns the same complete admission and reproduction refusals as `accept`.
    /// # Panics
    /// Panics only if a private checked-result invariant fails, which input cannot arrange.
    pub fn accept(
        self: &Arc<Self>,
        retained: Vec<u8>,
    ) -> Result<AcceptedRetention, RetentionError> {
        accept_after(retained, || Ok(self.key.projection.clone()), Some(self))
    }
    #[must_use]
    pub fn information(&self) -> &InfoRecord {
        &self.info
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    #[must_use]
    pub fn report(&self) -> &AuditReport {
        &self.report
    }
}
/// Check current source selections against the revision embedded in the linked build.
/// # Errors
/// Missing, malformed, unsupported or unequal engine identities refuse before audit.
pub fn check_current_engine(lock: &str, manifest: &str) -> Result<(), RetentionError> {
    let linked = revision::linked().map_err(RetentionError::EngineRevisionUnavailable)?;
    let current = revision::resolved(lock, revision::Source::CurrentLock)
        .map_err(RetentionError::EngineRevisionUnavailable)?;
    let requested =
        revision::manifest(manifest).map_err(RetentionError::EngineRevisionUnavailable)?;
    mismatch(
        Component::EngineRevision,
        "Cargo.lock",
        linked.as_bytes(),
        current.as_bytes(),
    )?;
    mismatch(
        Component::EngineRevision,
        "crates/ftoo_audit/Cargo.toml",
        linked.as_bytes(),
        requested.as_bytes(),
    )?;
    mismatch(
        Component::EngineRevision,
        "$.engine-pin",
        ftoo_manifest::ENGINE_PIN.0.as_bytes(),
        linked.as_bytes(),
    )
}
impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let q = |value: &str| serde_json::to_string(value).expect("strings serialize");
        match self {
            Self::Multiple(diagnostics) => {
                for (index, diagnostic) in diagnostics.iter().enumerate() {
                    if index != 0 {
                        f.write_str("\n")?;
                    }
                    diagnostic.fmt(f)?;
                }
                Ok(())
            }
            Self::UnsupportedComponent { component } => write!(
                f,
                "retention reproduction M061 at file \"generated/audit-report.json\": unsupported-component {}",
                q(component)
            ),
            Self::Wire(error) => error.fmt(f),
            Self::EngineRevisionUnavailable(error) => error.fmt(f),
            Self::Projection { path, error } => write!(
                f,
                "retention provenance {:?} at {} {}: {error}",
                super::diagnostic::projection_rule(error),
                if path.starts_with('$') {
                    "json-path"
                } else {
                    "file"
                },
                serde_json::to_string(path).expect("strings serialize")
            ),
            Self::Audit(error) => write!(
                f,
                "retention reproduction {:?} at file \"generated/audit-report.json\": {error}",
                super::diagnostic::audit_rule(error)
            ),
            Self::Domain(errors) => {
                for (i, error) in errors.iter().enumerate() {
                    if i != 0 {
                        f.write_str("\n")?;
                    }
                    super::diagnostic::domain(f, error)?;
                }
                Ok(())
            }
            Self::StatusDomain { .. }
            | Self::StatusAssertion { .. }
            | Self::AssessmentAssertion { .. } => assertion_display(self, f),
            Self::ProvenanceMismatch {
                component,
                path,
                difference,
            } => {
                let rule = component.rule();
                let component = component.token();
                write!(
                    f,
                    "retention provenance {rule:?} at {} {}: provenance-mismatch component={component} {}",
                    if path.starts_with('$') {
                        "json-path"
                    } else {
                        "file"
                    },
                    q(path),
                    difference
                )
            }
            Self::NonReproducibility {
                basis,
                differences,
                information,
                ..
            } => reproduction_display(f, *basis, differences, information),
            Self::DiagnosticRerunFailed {
                differences,
                error,
                info,
                ..
            } => {
                f.write_str(
                    "retention reproduction M061 at json-path \"$\": diagnostic-rerun-failed",
                )?;
                for difference in differences {
                    write!(f, "\n{difference}")?;
                }
                write!(f, "\nfault={error}")?;
                if let Some(first) = differences.first() {
                    write!(
                        f,
                        "\ninformation=[{},{},{}]",
                        first.left_info, first.right_info, info
                    )
                } else {
                    write!(f, "\ninformation=[{info}]")
                }
            }
        }
    }
}
fn reproduction_display(
    f: &mut std::fmt::Formatter<'_>,
    basis: Basis,
    differences: &[Difference],
    information: &[InfoRecord; 3],
) -> std::fmt::Result {
    let basis = match basis {
        Basis::FreshProcesses => "fresh-processes",
        Basis::RetainedAgainstFresh => "retained-against-fresh",
    };
    write!(
        f,
        "retention reproduction M061 at json-path \"$\": non-reproducibility basis={basis}"
    )?;
    for difference in differences {
        write!(f, "\n{difference}")?;
    }
    write!(
        f,
        "\ninformation=[{},{},{}]",
        information[0], information[1], information[2]
    )
}
impl std::error::Error for RetentionError {}

impl Component {
    pub(super) const fn rule(self) -> ftoo_manifest::RuleId {
        use ftoo_manifest::RuleId;
        match self {
            Self::ProjectionBytes
            | Self::ProjectionDigest
            | Self::ContractRevision
            | Self::EngineRevision => RuleId::M061,
            Self::Pin | Self::Target | Self::Summary => RuleId::M025,
        }
    }
    const fn token(self) -> &'static str {
        match self {
            Self::ProjectionBytes => "projection-bytes",
            Self::ProjectionDigest => "projection-digest",
            Self::ContractRevision => "contract-revision",
            Self::EngineRevision => "engine-revision",
            Self::Pin => "pin",
            Self::Target => "target",
            Self::Summary => "summary",
        }
    }
}

impl std::fmt::Display for ProvenanceDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Values { expected, actual } => write!(f, "expected={expected} actual={actual}"),
            Self::Bytes(difference) if difference.path == "generated/manifest.json" => {
                difference.fmt_labels(f, "canonical", "current")
            }
            Self::Bytes(difference) => difference.fmt_labels(f, "current", "retained"),
        }
    }
}

impl std::fmt::Display for ProvenanceValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(text) => write!(
                f,
                "{}",
                serde_json::to_string(text).expect("strings serialize")
            ),
            Self::U32(value) => write!(f, "{value}"),
            Self::U64(value) => write!(f, "{value}"),
            Self::Summary(value) => {
                f.write_str(&String::from_utf8(bytes(value.as_ref())).expect("codec UTF-8"))
            }
        }
    }
}

fn assertion_display(error: &RetentionError, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let q = |value: &str| serde_json::to_string(value).expect("strings serialize");
    match error {
        RetentionError::StatusDomain { expected, actual } => write!(
            f,
            "retention provenance M012 at json-path \"$.statuses\": status-domain expected={} actual={}",
            String::from_utf8(bytes(expected)).expect("codec UTF-8"),
            String::from_utf8(bytes(actual)).expect("codec UTF-8")
        ),
        RetentionError::StatusAssertion {
            index,
            axis,
            expected,
            actual,
        } => write!(
            f,
            "retention assertions M012 at json-path \"$.statuses[{index}]\": assertion-mismatch axis={} node={} expected={} actual={}",
            match axis {
                StatusAxis::Maturity => "maturity",
                StatusAxis::Identification => "identification",
                StatusAxis::Calibration => "calibration",
                StatusAxis::Evidence => "evidence",
            },
            q(expected.id.semantic_name()),
            String::from_utf8(bytes(expected)).expect("codec UTF-8"),
            String::from_utf8(bytes(actual)).expect("codec UTF-8")
        ),
        RetentionError::AssessmentAssertion { expected, actual } => write!(
            f,
            "retention assertions M015 at json-path \"$.assessment\": assertion-mismatch expected={} actual={}",
            String::from_utf8(bytes(expected)).expect("codec UTF-8"),
            String::from_utf8(bytes(actual)).expect("codec UTF-8")
        ),
        _ => unreachable!("assertion variants dispatched by Display"),
    }
}
