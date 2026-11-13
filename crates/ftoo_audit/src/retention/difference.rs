//! Complete byte evidence accompanies bounded, byte-coordinate diagnostic windows.
use super::{info::InfoRecord, wire};
use ftoo_manifest::NodeId;
use std::fmt;
use wire::Codec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Slot {
    None,
    Source,
    Residual,
    Member,
    Counterstatement,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Window {
    pub start: usize,
    pub bytes: Vec<u8>,
    pub eof: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Difference {
    pub claim: Option<NodeId>,
    pub slot: Slot,
    pub path: String,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
    pub offset: usize,
    pub runs: usize,
    pub left_window: Window,
    pub right_window: Window,
    pub left_info: InfoRecord,
    pub right_info: InfoRecord,
}
fn window(bytes: &[u8], offset: usize) -> Window {
    let start = offset.saturating_sub(64).min(bytes.len());
    let end = offset.saturating_add(257).min(bytes.len());
    Window {
        start,
        bytes: bytes[start..end].to_vec(),
        eof: end == bytes.len(),
    }
}
impl Difference {
    pub(super) fn between(
        left: &[u8],
        right: &[u8],
        left_info: InfoRecord,
        right_info: InfoRecord,
    ) -> Option<Self> {
        if left == right {
            return None;
        }
        let offset = left
            .iter()
            .zip(right)
            .position(|(a, b)| a != b)
            .unwrap_or(left.len().min(right.len()));
        let mut previous = false;
        let mut runs = 0;
        for position in 0..left.len().max(right.len()) {
            let differs = left.get(position) != right.get(position);
            if differs && !previous {
                runs += 1;
            }
            previous = differs;
        }
        Some(Self {
            claim: None,
            slot: Slot::None,
            path: "$".into(),
            left: left.into(),
            right: right.into(),
            offset,
            runs,
            left_window: window(left, offset),
            right_window: window(right, offset),
            left_info,
            right_info,
        })
    }
}

fn slots(verdict: &wire::Verdict) -> Vec<(Slot, &'static str, &[u8])> {
    let (receipt, counterstatement) = match verdict {
        wire::Verdict::Certified(receipt) => (receipt, None),
        wire::Verdict::Refuted {
            receipt,
            counterstatement,
        } => (receipt, Some(counterstatement)),
        _ => return Vec::new(),
    };
    let mut result = vec![
        (
            Slot::Source,
            "receipt.source-hex",
            receipt.source.as_slice(),
        ),
        (
            Slot::Residual,
            "receipt.residual-hex",
            receipt.residual.as_slice(),
        ),
        (
            Slot::Member,
            "receipt.member-hex",
            receipt.member.as_slice(),
        ),
    ];
    if let Some(counterstatement) = counterstatement {
        result.push((
            Slot::Counterstatement,
            "counterstatement.witness-hex",
            &counterstatement.witness,
        ));
    }
    result
}

pub(super) fn reports(
    left: &wire::Report,
    right: &wire::Report,
    left_bytes: &[u8],
    right_bytes: &[u8],
    left_info: &InfoRecord,
    right_info: &InfoRecord,
) -> Vec<Difference> {
    let mut differences = Vec::new();
    for (index, row) in left.outcomes.iter().enumerate() {
        let Some(other) = right.outcomes.iter().find(|other| other.claim == row.claim) else {
            continue;
        };
        let left_slots = slots(&row.verdict);
        let right_slots = slots(&other.verdict);
        for (slot, path) in [
            (Slot::Source, "receipt.source-hex"),
            (Slot::Residual, "receipt.residual-hex"),
            (Slot::Member, "receipt.member-hex"),
            (Slot::Counterstatement, "counterstatement.witness-hex"),
        ] {
            let bytes = left_slots
                .iter()
                .find(|(candidate, _, _)| *candidate == slot)
                .map_or(&[][..], |(_, _, bytes)| *bytes);
            let other_bytes = right_slots
                .iter()
                .find(|(candidate, _, _)| *candidate == slot)
                .map_or(&[][..], |(_, _, bytes)| *bytes);
            if let Some(mut difference) =
                Difference::between(bytes, other_bytes, left_info.clone(), right_info.clone())
            {
                difference.claim = Some(row.claim);
                difference.slot = slot;
                difference.path = format!("$.outcomes[{index}].verdict.{path}");
                differences.push(difference);
            }
        }
        let mut a = row.verdict.value();
        let mut b = other.verdict.value();
        strip_evidence(&mut a);
        strip_evidence(&mut b);
        metadata(
            Some(&a),
            Some(&b),
            &format!("$.outcomes[{index}].verdict"),
            row.claim,
            (left_info, right_info),
            &mut differences,
        );
    }
    if let Some(difference) = Difference::between(
        left_bytes,
        right_bytes,
        left_info.clone(),
        right_info.clone(),
    ) {
        differences.push(difference);
    }
    differences
}
fn strip_evidence(value: &mut super::syntax::Value) {
    use super::syntax::Value;
    match value {
        Value::Object(fields) => {
            fields.retain(|(name, _)| {
                !matches!(
                    name.as_str(),
                    "source-hex" | "residual-hex" | "member-hex" | "witness-hex"
                )
            });
            for (_, value) in fields {
                strip_evidence(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                strip_evidence(value);
            }
        }
        _ => (),
    }
}
fn metadata(
    left: Option<&super::syntax::Value>,
    right: Option<&super::syntax::Value>,
    path: &str,
    claim: NodeId,
    info: (&InfoRecord, &InfoRecord),
    differences: &mut Vec<Difference>,
) {
    use super::syntax::Value;
    if left == right {
        return;
    }
    match (left, right) {
        (Some(Value::Object(a)), Some(Value::Object(b))) => {
            let keys: std::collections::BTreeSet<_> =
                a.iter().chain(b).map(|(key, _)| key).collect();
            for key in keys {
                metadata(
                    a.iter()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| value),
                    b.iter()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| value),
                    &format!("{path}.{key}"),
                    claim,
                    info,
                    differences,
                );
            }
        }
        (Some(Value::Array(a)), Some(Value::Array(b))) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                metadata(
                    Some(a),
                    Some(b),
                    &format!("{path}[{index}]"),
                    claim,
                    info,
                    differences,
                );
            }
        }
        _ => {
            let mut a = String::new();
            let mut b = String::new();
            if let Some(value) = left {
                wire::render(value, &mut a);
            }
            if let Some(value) = right {
                wire::render(value, &mut b);
            }
            if let Some(mut difference) =
                Difference::between(a.as_bytes(), b.as_bytes(), info.0.clone(), info.1.clone())
            {
                difference.claim = Some(claim);
                difference.path = path.into();
                differences.push(difference);
            }
        }
    }
}
impl Difference {
    pub(super) fn fmt_labels(
        &self,
        f: &mut fmt::Formatter<'_>,
        left: &str,
        right: &str,
    ) -> fmt::Result {
        let q = |value: &str| serde_json::to_string(value).expect("strings serialize");
        let slot = match self.slot {
            Slot::None => "none",
            Slot::Source => "source",
            Slot::Residual => "residual",
            Slot::Member => "member",
            Slot::Counterstatement => "counterstatement",
        };
        let claim = self
            .claim
            .map_or_else(|| "none".into(), |claim| q(claim.semantic_name()));
        write!(
            f,
            "claim={claim} slot={slot} path={} left-length={} right-length={} offset={} runs={} left-window-start={} left-window={} left-eof={} right-window-start={} right-window={} right-eof={} left={left} right={right}",
            q(&self.path),
            self.left.len(),
            self.right.len(),
            self.offset,
            self.runs,
            self.left_window.start,
            q(&wire::hex(&self.left_window.bytes)),
            self.left_window.eof,
            self.right_window.start,
            q(&wire::hex(&self.right_window.bytes)),
            self.right_window.eof
        )
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_labels(f, self.left_info.run.token(), self.right_info.run.token())
    }
}
