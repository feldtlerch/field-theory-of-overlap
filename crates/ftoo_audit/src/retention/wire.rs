//! Closed, bounded report grammar. Decoding creates assertions, never audited values.
use super::syntax::{self, Value};
use ftoo_manifest::{RuleId, WireError, WireFault, WireLocation};

pub(super) const MAX_INPUT: usize = 32 * 1024 * 1024;
const MAX_PROJECTION: usize = 4 * 1024 * 1024;
const MAX_STRING: usize = 16 * 1024;
const MAX_EVIDENCE: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireRefusal {
    Wire(WireError),
    Limit {
        dimension: Dimension,
        maximum: usize,
        actual: usize,
        path: String,
    },
    InvalidHex {
        path: String,
        value: String,
    },
    InvalidRational {
        path: String,
        numerator: String,
        denominator: String,
    },
    ArrayLength {
        path: String,
        expected: usize,
        actual: usize,
    },
    NonCanonical {
        difference: Box<super::difference::Difference>,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dimension {
    InputBytes,
    ProjectionBytes,
    StringBytes,
    Values,
    ArrayElements,
    RationalBytes,
    EvidenceBytes,
    OutputBytes,
}
impl Dimension {
    const fn token(self) -> &'static str {
        match self {
            Self::InputBytes => "input-bytes",
            Self::ProjectionBytes => "projection-bytes",
            Self::StringBytes => "string-bytes",
            Self::Values => "values",
            Self::ArrayElements => "array-elements",
            Self::RationalBytes => "rational-bytes",
            Self::EvidenceBytes => "evidence-bytes",
            Self::OutputBytes => "output-bytes",
        }
    }
}
impl std::fmt::Display for WireRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let q = |value: &str| serde_json::to_string(value).expect("strings serialize");
        match self {
            Self::Wire(error) => {
                let stage = if matches!(error.rule, RuleId::M001 | RuleId::M002 | RuleId::M003) {
                    "syntax"
                } else {
                    "shape"
                };
                write!(f, "retention {stage} {:?} at ", error.rule)?;
                match &error.location {
                    WireLocation::ByteOffset(offset) => write!(f, "byte-offset {offset}"),
                    WireLocation::LineColumn { line, column, path } => {
                        write!(f, "line-column {line}:{column} path {}", q(path))
                    }
                    WireLocation::Path(path) => write!(f, "json-path {}", q(path)),
                }?;
                write!(f, ": {error}")
            }
            Self::Limit {
                dimension,
                maximum,
                actual,
                path,
            } => write!(
                f,
                "retention shape M061 at json-path {}: limit dimension={} limit={maximum} actual={actual}",
                q(path),
                dimension.token()
            ),
            Self::InvalidHex { path, value } => write!(
                f,
                "retention shape M004 at json-path {}: invalid-hex value={}",
                q(path),
                q(value)
            ),
            Self::InvalidRational {
                path,
                numerator,
                denominator,
            } => write!(
                f,
                "retention shape M004 at json-path {}: invalid-rational numerator={} denominator={}",
                q(path),
                q(numerator),
                q(denominator)
            ),
            Self::ArrayLength {
                path,
                expected,
                actual,
            } => write!(
                f,
                "retention shape M004 at json-path {}: array-length expected={expected} actual={actual}",
                q(path)
            ),
            Self::NonCanonical { difference } => {
                f.write_str(
                    "retention shape M061 at json-path \"$\": non-canonical canonical-side=right ",
                )?;
                difference.fmt_labels(f, "retained", "canonical")
            }
        }
    }
}
impl From<WireError> for WireRefusal {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    pub projection: String,
    pub digest: String,
    pub contract: u32,
    pub engine: String,
    pub reached: Vec<ftoo_manifest::NodeId>,
    pub outcomes: Vec<Outcome>,
    pub statuses: Vec<Status>,
    pub assessment: Assessment,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    pub claim: ftoo_manifest::NodeId,
    pub verdict: Verdict,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    Certified(Box<Receipt>),
    Refuted {
        receipt: Box<Receipt>,
        counterstatement: Counterstatement,
    },
    Undecided {
        stage: crate::RunStage,
        exhaustion: Exhaustion,
    },
    Deferred(ftoo_manifest::DeferredReason),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub producer: ftoo_manifest::ProducerId,
    pub revision: u32,
    pub budget: ftoo_manifest::BudgetId,
    pub pin: String,
    pub target: u64,
    pub limits: Vec<(String, u64)>,
    pub summary: Summary,
    pub source: Vec<u8>,
    pub residual: Vec<u8>,
    pub member: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counterstatement {
    pub claimed: Rational,
    pub witness: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rational {
    pub numerator: String,
    pub denominator: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Summary {
    pub subject: String,
    pub jet: [Rational; 5],
    pub source: String,
    pub clause: String,
    pub order: u32,
    pub derivative_order: u32,
    pub residual_order: u32,
    pub equation_degrees: [u32; 4],
    pub ode: String,
    pub region: ftoo_manifest::RegionContract,
    pub analytic_scope: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exhaustion {
    pub resource: nrz_cas::limits::BudgetKind,
    pub reason: nrz_cas::Exhaustion,
    pub limit: u64,
    pub attempted: u64,
    pub location: Location,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    pub operation: String,
    pub logical_id: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub id: ftoo_manifest::NodeId,
    pub maturity: ftoo_manifest::Maturity,
    pub identification: ftoo_manifest::Identification,
    pub calibration: ftoo_manifest::Calibration,
    pub evidence: crate::AuditedEvidence,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub disposition: ftoo_manifest::Disposition,
    pub exit_code: u32,
}

fn bound(
    dimension: Dimension,
    actual: usize,
    maximum: usize,
    path: &str,
) -> Result<(), WireRefusal> {
    if actual > maximum {
        Err(WireRefusal::Limit {
            dimension,
            maximum,
            actual: maximum + 1,
            path: path.into(),
        })
    } else {
        Ok(())
    }
}
fn fault(path: &str, fault: WireFault) -> WireRefusal {
    WireError {
        rule: RuleId::M004,
        location: WireLocation::Path(path.into()),
        fault,
    }
    .into()
}
fn wrong(value: &Value, path: &str, expected: &str) -> WireRefusal {
    fault(
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
fn fields<'a>(
    value: &'a Value,
    path: &str,
    names: &[&str],
) -> Result<&'a [(String, Value)], WireRefusal> {
    let Value::Object(fields) = value else {
        return Err(wrong(value, path, "object"));
    };
    for (key, _) in fields {
        if !names.contains(&key.as_str()) {
            return Err(fault(
                path,
                WireFault::UnknownField {
                    name: key.clone(),
                    expected: names.iter().map(ToString::to_string).collect(),
                },
            ));
        }
    }
    for name in names {
        if !fields.iter().any(|(key, _)| key == name) {
            return Err(fault(
                path,
                WireFault::MissingField {
                    name: (*name).into(),
                },
            ));
        }
    }
    Ok(fields)
}
fn get<'a>(fields: &'a [(String, Value)], key: &str) -> &'a Value {
    &fields
        .iter()
        .find(|(name, _)| name == key)
        .expect("exact fields checked")
        .1
}
fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}
fn s(text: &str) -> Value {
    Value::String(text.into())
}
pub(super) trait Codec: Sized {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal>;
    fn value(&self) -> Value;
}
impl Codec for String {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        string(value, path, Dimension::StringBytes, MAX_STRING)
    }
    fn value(&self) -> Value {
        s(self)
    }
}
fn string(
    value: &Value,
    path: &str,
    dimension: Dimension,
    maximum: usize,
) -> Result<String, WireRefusal> {
    let Value::String(text) = value else {
        return Err(wrong(value, path, "string"));
    };
    bound(dimension, text.len(), maximum, path)?;
    Ok(text.clone())
}
macro_rules! unsigned {
    ($ty:ty, $name:literal) => {
        impl Codec for $ty {
            fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
                let Value::Integer(text) = value else {
                    return Err(wrong(value, path, "integer"));
                };
                if !text.starts_with('-')
                    && let Ok(number) = text.parse()
                {
                    return Ok(number);
                }
                Err(fault(
                    path,
                    WireFault::IntegerRange {
                        expected: $name.into(),
                        actual: text.clone(),
                    },
                ))
            }
            fn value(&self) -> Value {
                Value::Integer(self.to_string())
            }
        }
    };
}
unsigned!(u32, "u32");
unsigned!(u64, "u64");
impl Codec for Option<u64> {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        if matches!(value, Value::Null) {
            Ok(None)
        } else {
            u64::read(value, path).map(Some)
        }
    }
    fn value(&self) -> Value {
        self.map_or(Value::Null, |number| number.value())
    }
}
impl<T: Codec> Codec for Vec<T> {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let Value::Array(values) = value else {
            return Err(wrong(value, path, "array"));
        };
        bound(Dimension::ArrayElements, values.len(), 4096, path)?;
        values
            .iter()
            .enumerate()
            .map(|(i, value)| T::read(value, &format!("{path}[{i}]")))
            .collect()
    }
    fn value(&self) -> Value {
        Value::Array(self.iter().map(Codec::value).collect())
    }
}
impl<T: Codec, const N: usize> Codec for [T; N] {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let values = Vec::<T>::read(value, path)?;
        let actual = values.len();
        values.try_into().map_err(|_| WireRefusal::ArrayLength {
            path: path.into(),
            expected: N,
            actual,
        })
    }
    fn value(&self) -> Value {
        Value::Array(self.iter().map(Codec::value).collect())
    }
}
macro_rules! record {
    ($ty:ty, $($field:ident => $key:literal),+ $(,)?) => {
        impl Codec for $ty {
            fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
                let fields = fields(value, path, &[$($key),+])?;
                Ok(Self { $($field: Codec::read(get(fields, $key), &format!("{path}.{}", $key))?),+ })
            }
            fn value(&self) -> Value { object([$(( $key, self.$field.value() )),+]) }
        }
    };
}
macro_rules! tokens {
    ($ty:ty, $($variant:path => $token:literal),+ $(,)?) => {
        impl Codec for $ty {
            fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
                let text = String::read(value, path)?;
                match text.as_str() { $($token => Ok($variant),)+ _ => Err(fault(path, WireFault::UnknownToken { expected: vec![$($token.into()),+], actual: text })) }
            }
            fn value(&self) -> Value { s(match self { $($variant => $token),+ }) }
        }
    };
}
record!(Outcome, claim => "claim", verdict => "verdict");
record!(Summary, subject => "subject", jet => "jet", source => "source", clause => "clause", order => "order", derivative_order => "derivative-order", residual_order => "residual-order", equation_degrees => "equation-degrees", ode => "ode", region => "region", analytic_scope => "analytic-scope");
record!(Exhaustion, resource => "resource", reason => "reason", limit => "limit", attempted => "attempted", location => "location");
record!(Location, operation => "operation", logical_id => "logical-id");
record!(Status, id => "id", maturity => "maturity", identification => "identification", calibration => "calibration", evidence => "evidence");
record!(Assessment, disposition => "disposition", exit_code => "exit-code");
tokens!(ftoo_manifest::Maturity, ftoo_manifest::Maturity::Proposed => "proposed", ftoo_manifest::Maturity::Retained => "retained");
tokens!(ftoo_manifest::Identification, ftoo_manifest::Identification::Identified => "identified");
tokens!(ftoo_manifest::Calibration, ftoo_manifest::Calibration::NotApplicable => "not-applicable");
tokens!(ftoo_manifest::RegionContract, ftoo_manifest::RegionContract::AmbientProductXNoAssumptions => "ambient-product-x-no-assumptions");
tokens!(crate::AuditedEvidence, crate::AuditedEvidence::NotAClaim => "not-a-claim", crate::AuditedEvidence::Certified => "certified", crate::AuditedEvidence::Refuted => "refuted", crate::AuditedEvidence::Undecided => "undecided", crate::AuditedEvidence::Deferred => "deferred");
tokens!(ftoo_manifest::VerdictClass, ftoo_manifest::VerdictClass::Certified => "certified", ftoo_manifest::VerdictClass::Refuted => "refuted", ftoo_manifest::VerdictClass::Undecided => "undecided", ftoo_manifest::VerdictClass::Deferred => "deferred");
tokens!(ftoo_manifest::Disposition, ftoo_manifest::Disposition::Pass => "pass", ftoo_manifest::Disposition::InputError => "input-error", ftoo_manifest::Disposition::SemanticInvalidity => "semantic-invalidity", ftoo_manifest::Disposition::MathematicalFailure => "mathematical-failure", ftoo_manifest::Disposition::Incomplete => "incomplete");
tokens!(nrz_cas::Exhaustion, nrz_cas::Exhaustion::Limit => "limit", nrz_cas::Exhaustion::ArithmeticRange => "arithmetic-range", nrz_cas::Exhaustion::Allocation => "allocation", nrz_cas::Exhaustion::Capacity => "capacity");
tokens!(ftoo_manifest::ProducerId, ftoo_manifest::ProducerId::LangevinCoefficientOneValue => "langevin-coefficient-one-value", ftoo_manifest::ProducerId::LangevinCoefficientThreeValue => "langevin-coefficient-three-value");
tokens!(ftoo_manifest::BudgetId, ftoo_manifest::BudgetId::CoefficientOneV1 => "coefficient-one-v1", ftoo_manifest::BudgetId::CoefficientThreeV1 => "coefficient-three-v1");
tokens!(ftoo_manifest::CapabilityId, ftoo_manifest::CapabilityId::ExactRationalLinearEquivalence => "exact-rational-linear-equivalence", ftoo_manifest::CapabilityId::FormalTruncatedPowerSeries => "formal-truncated-power-series", ftoo_manifest::CapabilityId::SymbolicTrigonometry => "symbolic-trigonometry", ftoo_manifest::CapabilityId::TrigonometricIntegration => "trigonometric-integration");
tokens!(crate::RunStage, crate::RunStage::Capability => "capability", crate::RunStage::Premise => "premise", crate::RunStage::Setup => "setup", crate::RunStage::Candidate => "candidate", crate::RunStage::Definedness => "definedness", crate::RunStage::Source => "source", crate::RunStage::ResidualMember => "residual-member", crate::RunStage::Export => "export", crate::RunStage::Expectation => "expectation", crate::RunStage::Replay => "replay", crate::RunStage::Projection => "projection");
impl Codec for ftoo_manifest::NodeId {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let text = String::read(value, path)?;
        Self::ALL
            .into_iter()
            .find(|id| id.semantic_name() == text)
            .ok_or_else(|| {
                fault(
                    path,
                    WireFault::UnknownToken {
                        expected: Self::ALL
                            .iter()
                            .map(|id| id.semantic_name().into())
                            .collect(),
                        actual: text,
                    },
                )
            })
    }
    fn value(&self) -> Value {
        s(self.semantic_name())
    }
}
fn kind<'a>(value: &'a Value, path: &str, allowed: &[&str]) -> Result<&'a str, WireRefusal> {
    let Value::Object(fields) = value else {
        return Err(wrong(value, path, "object"));
    };
    let Some((_, value)) = fields.iter().find(|(key, _)| key == "kind") else {
        return Err(fault(
            path,
            WireFault::MissingField {
                name: "kind".into(),
            },
        ));
    };
    let path = format!("{path}.kind");
    let Value::String(text) = value else {
        return Err(wrong(value, &path, "string"));
    };
    if allowed.contains(&text.as_str()) {
        Ok(text)
    } else {
        Err(fault(
            &path,
            WireFault::UnknownToken {
                expected: allowed.iter().map(ToString::to_string).collect(),
                actual: text.clone(),
            },
        ))
    }
}
pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 15)]),
            ]
        })
        .collect()
}
fn unhex(value: &Value, path: &str, maximum: usize) -> Result<Vec<u8>, WireRefusal> {
    let Value::String(text) = value else {
        return Err(wrong(value, path, "string"));
    };
    bound(
        Dimension::EvidenceBytes,
        text.len().div_ceil(2),
        maximum,
        path,
    )?;
    if text.len() % 2 != 0 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WireRefusal::InvalidHex {
            path: path.into(),
            value: text.clone(),
        });
    }
    let digit = |byte: u8| {
        if byte.is_ascii_digit() {
            byte - b'0'
        } else {
            byte.to_ascii_lowercase() - b'a' + 10
        }
    };
    Ok(text
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| digit(pair[0]) * 16 + digit(pair[1]))
        .collect())
}
fn revision(value: &Value, path: &str, size: usize) -> Result<String, WireRefusal> {
    let text = String::read(value, path)?;
    if text.len() != size || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(WireRefusal::InvalidHex {
            path: path.into(),
            value: text,
        });
    }
    Ok(text.to_ascii_lowercase())
}
fn minimal_integer(text: &str, signed: bool) -> bool {
    let digits = if signed {
        text.strip_prefix('-').unwrap_or(text)
    } else {
        text
    };
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0"
}
fn reduced(numerator: &str, denominator: &str) -> bool {
    let (Ok(numerator), Ok(denominator)) = (numerator.parse::<i64>(), denominator.parse::<u64>())
    else {
        return false;
    };
    let (mut a, mut b) = (numerator.unsigned_abs(), denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a == 1
}
impl Codec for Rational {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let fields = fields(value, path, &["numerator", "denominator"])?;
        let numerator = string(
            get(fields, "numerator"),
            &format!("{path}.numerator"),
            Dimension::RationalBytes,
            4096,
        )?;
        let denominator = string(
            get(fields, "denominator"),
            &format!("{path}.denominator"),
            Dimension::RationalBytes,
            4096,
        )?;
        if !minimal_integer(&numerator, true)
            || !minimal_integer(&denominator, false)
            || denominator == "0"
            || !reduced(&numerator, &denominator)
        {
            return Err(WireRefusal::InvalidRational {
                path: path.into(),
                numerator,
                denominator,
            });
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }
    fn value(&self) -> Value {
        object([
            ("numerator", self.numerator.value()),
            ("denominator", self.denominator.value()),
        ])
    }
}
impl Codec for Counterstatement {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        Self::read_bounded(value, path, MAX_EVIDENCE)
    }
    fn value(&self) -> Value {
        object([
            ("claimed", self.claimed.value()),
            ("witness-hex", s(&hex(&self.witness))),
        ])
    }
}
impl Counterstatement {
    fn read_bounded(value: &Value, path: &str, maximum: usize) -> Result<Self, WireRefusal> {
        let fields = fields(value, path, &["claimed", "witness-hex"])?;
        Ok(Self {
            claimed: Codec::read(get(fields, "claimed"), &format!("{path}.claimed"))?,
            witness: unhex(
                get(fields, "witness-hex"),
                &format!("{path}.witness-hex"),
                maximum,
            )?,
        })
    }
}
impl Codec for ftoo_manifest::DeferredReason {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let kind = kind(
            value,
            path,
            &["audit-binding", "capability", "outside-slice", "premise"],
        )?;
        let fields = fields(
            value,
            path,
            match kind {
                "capability" => &["kind", "capability"],
                "premise" => &["kind", "claim", "evidence"],
                _ => &["kind"],
            },
        )?;
        Ok(match kind {
            "audit-binding" => Self::AuditBinding,
            "outside-slice" => Self::OutsideSlice,
            "capability" => Self::Capability(Codec::read(
                get(fields, "capability"),
                &format!("{path}.capability"),
            )?),
            _ => Self::Premise {
                claim: Codec::read(get(fields, "claim"), &format!("{path}.claim"))?,
                evidence: Codec::read(get(fields, "evidence"), &format!("{path}.evidence"))?,
            },
        })
    }
    fn value(&self) -> Value {
        match self {
            Self::AuditBinding => object([("kind", s("audit-binding"))]),
            Self::OutsideSlice => object([("kind", s("outside-slice"))]),
            Self::Capability(capability) => object([
                ("kind", s("capability")),
                ("capability", capability.value()),
            ]),
            Self::Premise { claim, evidence } => object([
                ("kind", s("premise")),
                ("claim", claim.value()),
                ("evidence", evidence.value()),
            ]),
        }
    }
}
impl Codec for Verdict {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let kind = kind(
            value,
            path,
            &["certified", "refuted", "undecided", "deferred"],
        )?;
        let fields = fields(
            value,
            path,
            match kind {
                "certified" => &["kind", "receipt"],
                "refuted" => &["kind", "receipt", "counterstatement"],
                "undecided" => &["kind", "stage", "exhaustion"],
                _ => &["kind", "reason"],
            },
        )?;
        Ok(match kind {
            "certified" => Self::Certified(Box::new(Receipt::read(
                get(fields, "receipt"),
                &format!("{path}.receipt"),
            )?)),
            "refuted" => {
                let receipt = Box::new(Receipt::read(
                    get(fields, "receipt"),
                    &format!("{path}.receipt"),
                )?);
                let maximum = receipt
                    .limits
                    .iter()
                    .find(|(name, _)| name == "witness-bytes")
                    .expect("required limit")
                    .1
                    .min(MAX_EVIDENCE as u64);
                let counterstatement = Counterstatement::read_bounded(
                    get(fields, "counterstatement"),
                    &format!("{path}.counterstatement"),
                    usize::try_from(maximum).expect("bounded ceiling"),
                )?;
                Self::Refuted {
                    receipt,
                    counterstatement,
                }
            }
            "undecided" => Self::Undecided {
                stage: Codec::read(get(fields, "stage"), &format!("{path}.stage"))?,
                exhaustion: Codec::read(get(fields, "exhaustion"), &format!("{path}.exhaustion"))?,
            },
            _ => Self::Deferred(Codec::read(
                get(fields, "reason"),
                &format!("{path}.reason"),
            )?),
        })
    }
    fn value(&self) -> Value {
        match self {
            Self::Certified(receipt) => {
                object([("kind", s("certified")), ("receipt", receipt.value())])
            }
            Self::Refuted {
                receipt,
                counterstatement,
            } => object([
                ("kind", s("refuted")),
                ("receipt", receipt.value()),
                ("counterstatement", counterstatement.value()),
            ]),
            Self::Undecided { stage, exhaustion } => object([
                ("kind", s("undecided")),
                ("stage", stage.value()),
                ("exhaustion", exhaustion.value()),
            ]),
            Self::Deferred(reason) => object([("kind", s("deferred")), ("reason", reason.value())]),
        }
    }
}

// Names and canonical order come from the manifest; this table only maps engine fields.
fn limit_fields() -> [&'static str; 71] {
    ftoo_manifest::BudgetProfile::PinnedTrialLimitsV2
        .resolved()
        .fields()
        .map(|(name, _)| name)
}
macro_rules! engine_limits {
    ($($variant:ident => $field:ident),+ $(,)?) => {
        #[cfg(test)]
        #[test]
        fn engine_field_names_match_manifest_order() {
            assert_eq!([$(stringify!($field).replace('_', "-")),+], limit_fields().map(str::to_owned));
        }
        const BUDGET_KINDS: &[nrz_cas::limits::BudgetKind] = &[$(nrz_cas::limits::BudgetKind::$variant),+];
        pub(super) fn limits(limits: &nrz_cas::limits::Limits) -> Vec<(String, u64)> {
            limit_fields().into_iter().zip([$(limits.$field),+]).map(|(name, value)| (name.into(), value)).collect()
        }
        impl Codec for nrz_cas::limits::BudgetKind {
            fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
                let text = String::read(value, path)?;
                let names = limit_fields();
                names.iter().position(|name| *name == text).map(|index| BUDGET_KINDS[index]).ok_or_else(|| fault(path, WireFault::UnknownToken { expected: names.map(str::to_owned).into(), actual: text }))
            }
            fn value(&self) -> Value {
                match self { $(Self::$variant => ()),+ }
                s(limit_fields()[BUDGET_KINDS.iter().position(|kind| kind == self).expect("exhaustive engine kinds")])
            }
        }
    };
}
engine_limits!(
    ArithmeticSteps => arithmetic_steps,
    BoundaryNodes => boundary_nodes,
    CaseArms => case_arms,
    CoefficientMaps => coefficient_maps,
    CoefficientOps => coefficient_ops,
    ComponentBits => component_bits,
    ComponentSplits => component_splits,
    ConditionAtoms => condition_atoms,
    ConditionSteps => condition_steps,
    CostQuota => cost_quota,
    DecimalExponent => decimal_exponent,
    DeltaOrder => delta_order,
    DenominatorFactors => denominator_factors,
    Depth => depth,
    DiagnosticRecords => diagnostic_records,
    Edges => edges,
    ExpansionCredits => expansion_credits,
    FamilyParameters => family_parameters,
    FrontierLabels => frontier_labels,
    GcdSteps => gcd_steps,
    HypothesisDepth => hypothesis_depth,
    IntegerExponent => integer_exponent,
    IntermediateBits => intermediate_bits,
    IsolationSteps => isolation_steps,
    JetEntries => jet_entries,
    JetOrder => jet_order,
    LinearPivots => linear_pivots,
    LiteralBytes => literal_bytes,
    LocalBranches => local_branches,
    LocalContexts => local_contexts,
    MatchAttempts => match_attempts,
    MatchBatch => match_batch,
    MatrixEntries => matrix_entries,
    NameBytes => name_bytes,
    OrbitMembers => orbit_members,
    OrbitPermutations => orbit_permutations,
    OrbitTerms => orbit_terms,
    OriginalOccurrences => original_occurrences,
    OutputBytes => output_bytes,
    PairingSteps => pairing_steps,
    PayloadBytes => payload_bytes,
    PeakExpressionSize => peak_expression_size,
    PolynomialDegree => polynomial_degree,
    PolynomialTerms => polynomial_terms,
    PolynomialVariables => polynomial_variables,
    ProgressQuota => progress_quota,
    ProofNodes => proof_nodes,
    PullbackZeros => pullback_zeros,
    QueueEntries => queue_entries,
    RefinementPairs => refinement_pairs,
    RegionCells => region_cells,
    RegionNodes => region_nodes,
    ReplayWork => replay_work,
    ResolventDegree => resolvent_degree,
    ResolventDepth => resolvent_depth,
    SelectionSlots => selection_slots,
    SolutionBranches => solution_branches,
    SolveIdentities => solve_identities,
    SourceBytes => source_bytes,
    StagedEdges => staged_edges,
    StagedPayloadBytes => staged_payload_bytes,
    StagedVertices => staged_vertices,
    SymmetricSteps => symmetric_steps,
    TestNodes => test_nodes,
    TowerDegree => tower_degree,
    TowerLevels => tower_levels,
    TowerReductions => tower_reductions,
    Transitions => transitions,
    Vertices => vertices,
    WitnessBytes => witness_bytes,
    Work => work,
);

impl Codec for Receipt {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let fields = fields(
            value,
            path,
            &[
                "producer",
                "revision",
                "budget",
                "pin",
                "target",
                "limits",
                "summary",
                "source-hex",
                "residual-hex",
                "member-hex",
            ],
        )?;
        let limits_path = format!("{path}.limits");
        let limit_fields = limit_fields();
        let limits = self::fields(get(fields, "limits"), &limits_path, &limit_fields)?;
        let limits: Vec<(String, u64)> = limit_fields
            .iter()
            .map(|key| {
                Ok((
                    (*key).into(),
                    u64::read(get(limits, key), &format!("{limits_path}.{key}"))?,
                ))
            })
            .collect::<Result<_, WireRefusal>>()?;
        let ceiling = limits
            .iter()
            .find(|(key, _)| key == "witness-bytes")
            .expect("all limits checked")
            .1
            .min(MAX_EVIDENCE as u64);
        let ceiling = usize::try_from(ceiling).expect("bounded evidence ceiling");
        Ok(Self {
            producer: Codec::read(get(fields, "producer"), &format!("{path}.producer"))?,
            revision: Codec::read(get(fields, "revision"), &format!("{path}.revision"))?,
            budget: Codec::read(get(fields, "budget"), &format!("{path}.budget"))?,
            pin: revision(get(fields, "pin"), &format!("{path}.pin"), 40)?,
            target: Codec::read(get(fields, "target"), &format!("{path}.target"))?,
            limits,
            summary: Codec::read(get(fields, "summary"), &format!("{path}.summary"))?,
            source: unhex(
                get(fields, "source-hex"),
                &format!("{path}.source-hex"),
                ceiling,
            )?,
            residual: unhex(
                get(fields, "residual-hex"),
                &format!("{path}.residual-hex"),
                ceiling,
            )?,
            member: unhex(
                get(fields, "member-hex"),
                &format!("{path}.member-hex"),
                ceiling,
            )?,
        })
    }
    fn value(&self) -> Value {
        object([
            ("producer", self.producer.value()),
            ("revision", self.revision.value()),
            ("budget", self.budget.value()),
            ("pin", self.pin.value()),
            ("target", self.target.value()),
            (
                "limits",
                Value::Object(
                    self.limits
                        .iter()
                        .map(|(key, value)| (key.clone(), value.value()))
                        .collect(),
                ),
            ),
            ("summary", self.summary.value()),
            ("source-hex", s(&hex(&self.source))),
            ("residual-hex", s(&hex(&self.residual))),
            ("member-hex", s(&hex(&self.member))),
        ])
    }
}
impl Codec for Report {
    fn read(value: &Value, path: &str) -> Result<Self, WireRefusal> {
        let root = fields(
            value,
            path,
            &[
                "schema",
                "provenance",
                "reached",
                "outcomes",
                "statuses",
                "assessment",
            ],
        )?;
        let Value::Integer(schema) = get(root, "schema") else {
            return Err(wrong(get(root, "schema"), "$.schema", "integer"));
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
        let provenance = fields(
            get(root, "provenance"),
            "$.provenance",
            &["projection-utf8", "projection-sha256", "inputs"],
        )?;
        let inputs = fields(
            get(provenance, "inputs"),
            "$.provenance.inputs",
            &["contract-revision", "engine-revision"],
        )?;
        Ok(Self {
            projection: string(
                get(provenance, "projection-utf8"),
                "$.provenance.projection-utf8",
                Dimension::ProjectionBytes,
                MAX_PROJECTION,
            )?,
            digest: revision(
                get(provenance, "projection-sha256"),
                "$.provenance.projection-sha256",
                64,
            )?,
            contract: Codec::read(
                get(inputs, "contract-revision"),
                "$.provenance.inputs.contract-revision",
            )?,
            engine: revision(
                get(inputs, "engine-revision"),
                "$.provenance.inputs.engine-revision",
                40,
            )?,
            reached: Codec::read(get(root, "reached"), "$.reached")?,
            outcomes: Codec::read(get(root, "outcomes"), "$.outcomes")?,
            statuses: Codec::read(get(root, "statuses"), "$.statuses")?,
            assessment: Codec::read(get(root, "assessment"), "$.assessment")?,
        })
    }
    fn value(&self) -> Value {
        let mut reached = self.reached.clone();
        reached.sort_by_key(|id| id.semantic_name());
        let mut outcomes: Vec<_> = self.outcomes.iter().collect();
        outcomes.sort_by_key(|row| row.claim.semantic_name());
        let mut statuses: Vec<_> = self.statuses.iter().collect();
        statuses.sort_by_key(|row| row.id.semantic_name());
        object([
            ("schema", 1u32.value()),
            (
                "provenance",
                object([
                    ("projection-utf8", self.projection.value()),
                    ("projection-sha256", self.digest.value()),
                    (
                        "inputs",
                        object([
                            ("contract-revision", self.contract.value()),
                            ("engine-revision", self.engine.value()),
                        ]),
                    ),
                ]),
            ),
            ("reached", reached.value()),
            (
                "outcomes",
                Value::Array(outcomes.into_iter().map(Codec::value).collect()),
            ),
            (
                "statuses",
                Value::Array(statuses.into_iter().map(Codec::value).collect()),
            ),
            ("assessment", self.assessment.value()),
        ])
    }
}
pub(super) fn render(value: &Value, output: &mut String) {
    match value {
        Value::Object(fields) => {
            output.push('{');
            for (i, (key, value)) in fields.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                render(&s(key), output);
                output.push(':');
                render(value, output);
            }
            output.push('}');
        }
        Value::Array(values) => {
            output.push('[');
            for (i, value) in values.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                render(value, output);
            }
            output.push(']');
        }
        Value::String(value) => output
            .push_str(&serde_json::to_string(value).expect("strings serialize without refusal")),
        Value::Integer(value) => output.push_str(value),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Null => output.push_str("null"),
    }
}
pub(super) fn write(report: &Report) -> Result<Vec<u8>, WireRefusal> {
    let value = report.value();
    Report::read(&value, "$")?;
    let mut output = String::new();
    render(&value, &mut output);
    output.push('\n');
    bound(Dimension::OutputBytes, output.len(), MAX_INPUT, "$")?;
    Ok(output.into_bytes())
}
pub(super) fn decode(bytes: &[u8]) -> Result<Report, WireRefusal> {
    bound(Dimension::InputBytes, bytes.len(), MAX_INPUT, "$")?;
    let value = syntax::parse(bytes)?;
    let report = Report::read(&value, "$")?;
    let expected = write(&report)?;
    if bytes != expected {
        return Err(WireRefusal::NonCanonical {
            difference: Box::new(
                super::difference::Difference::between(
                    bytes,
                    &expected,
                    super::info::InfoRecord::collect(super::info::Run::Retained),
                    super::info::InfoRecord::collect(super::info::Run::Retained),
                )
                .expect("unequal canonical bytes"),
            ),
        });
    }
    Ok(report)
}
