//! Duplicate-preserving raw JSON admission. No semantic values are constructed here.
use super::wire::{Dimension, WireRefusal};
use std::{cell::Cell, collections::BTreeSet, fmt};

use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use serde_json::value::RawValue;

use ftoo_manifest::{RuleId, WireError, WireFault, WireLocation};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Value {
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    Integer(String),
    Bool(bool),
    Null,
}

impl Value {
    pub(super) const fn kind(&self) -> &'static str {
        match self {
            Self::Object(_) => "object",
            Self::Array(_) => "array",
            Self::String(_) => "string",
            Self::Integer(_) => "integer",
            Self::Bool(_) => "boolean",
            Self::Null => "null",
        }
    }
}

struct Container<'a> {
    document: &'a str,
    path: &'a str,
    depth: usize,
    fault: &'a mut Option<WireRefusal>,
    values: &'a Cell<usize>,
}
impl<'de> Visitor<'de> for Container<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON container")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Value, M::Error> {
        let mut keys = BTreeSet::new();
        let mut fields = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            // Check decoded keys as they arrive, before storing any field.
            let duplicate = !keys.insert(key.clone());
            let child = map.next_value::<&RawValue>()?;
            let result = if duplicate {
                let offset = child.get().as_ptr() as usize - self.document.as_ptr() as usize;
                Err(at(
                    self.document,
                    offset,
                    self.path,
                    RuleId::M002,
                    WireFault::DuplicateKey { key: key.clone() },
                ))
            } else {
                walk(
                    self.document,
                    child,
                    &format!("{}.{key}", self.path),
                    self.depth + 1,
                    self.values,
                )
            };
            match result {
                Ok(value) => fields.push((key, value)),
                Err(error) => {
                    *self.fault = Some(error);
                    return Err(serde::de::Error::custom("strict syntax refusal"));
                }
            }
        }
        Ok(Value::Object(fields))
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Value, S::Error> {
        let mut values = Vec::new();
        while let Some(child) = seq.next_element::<&RawValue>()? {
            if values.len() == 4096 {
                *self.fault = Some(WireRefusal::Limit {
                    dimension: Dimension::ArrayElements,
                    maximum: 4096,
                    actual: 4097,
                    path: self.path.into(),
                });
                return Err(serde::de::Error::custom("strict syntax refusal"));
            }
            match walk(
                self.document,
                child,
                &format!("{}[{}]", self.path, values.len()),
                self.depth + 1,
                self.values,
            ) {
                Ok(value) => values.push(value),
                Err(error) => {
                    *self.fault = Some(error);
                    return Err(serde::de::Error::custom("strict syntax refusal"));
                }
            }
        }
        Ok(Value::Array(values))
    }
}

fn coordinate(document: &str, offset: usize) -> (usize, usize) {
    let prefix = &document.as_bytes()[..offset];
    let line = prefix.split(|b| *b == b'\n').count();
    let column = prefix
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(offset + 1, |p| offset - p);
    (line, column)
}

fn at(document: &str, offset: usize, path: &str, rule: RuleId, fault: WireFault) -> WireRefusal {
    let (line, column) = coordinate(document, offset);
    WireError {
        rule,
        location: WireLocation::LineColumn {
            line,
            column,
            path: path.into(),
        },
        fault,
    }
    .into()
}

fn syntax(document: &str, origin: usize, path: &str, error: &serde_json::Error) -> WireRefusal {
    let (base_line, base_column) = coordinate(document, origin);
    WireError {
        rule: RuleId::M002,
        location: WireLocation::LineColumn {
            line: base_line + error.line().saturating_sub(1),
            column: if error.line() == 1 {
                base_column - 1 + error.column()
            } else {
                error.column()
            },
            path: path.into(),
        },
        fault: WireFault::Syntax,
    }
    .into()
}

fn preflight(document: &str) -> Result<(), WireRefusal> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, byte) in document.bytes().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 128 {
                        return Err(at(
                            document,
                            offset,
                            "$",
                            RuleId::M002,
                            WireFault::DepthLimit { maximum: 128 },
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}

fn walk(
    document: &str,
    raw: &RawValue,
    path: &str,
    depth: usize,
    values: &Cell<usize>,
) -> Result<Value, WireRefusal> {
    let count = values.get() + 1;
    if count > 65_536 {
        return Err(WireRefusal::Limit {
            dimension: Dimension::Values,
            maximum: 65_536,
            actual: count,
            path: path.into(),
        });
    }
    values.set(count);
    let token = raw.get();
    let origin = token.as_ptr() as usize - document.as_ptr() as usize;
    if depth > 128 {
        return Err(at(
            document,
            origin,
            path,
            RuleId::M002,
            WireFault::DepthLimit { maximum: 128 },
        ));
    }
    match token.as_bytes()[0] {
        b'{' | b'[' => {
            let mut fault = None;
            let visitor = Container {
                document,
                path,
                depth,
                fault: &mut fault,
                values,
            };
            let mut de = serde_json::Deserializer::from_str(token);
            let result = if token.starts_with('{') {
                de.deserialize_map(visitor)
            } else {
                de.deserialize_seq(visitor)
            };
            if let Some(error) = fault {
                return Err(error);
            }
            result.map_err(|e| syntax(document, origin, path, &e))
        }
        b'"' => serde_json::from_str(token)
            .map(Value::String)
            .map_err(|e| syntax(document, origin, path, &e)),
        b't' => Ok(Value::Bool(true)),
        b'f' => Ok(Value::Bool(false)),
        b'n' => Ok(Value::Null),
        _ => {
            if token.contains(['.', 'e', 'E']) {
                return Err(at(
                    document,
                    origin,
                    path,
                    RuleId::M003,
                    WireFault::NonIntegerNumber {
                        token: token.into(),
                    },
                ));
            }
            Ok(Value::Integer(token.into()))
        }
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value, WireRefusal> {
    let document = std::str::from_utf8(bytes).map_err(|e| WireError {
        rule: RuleId::M001,
        location: WireLocation::ByteOffset(e.valid_up_to()),
        fault: WireFault::InvalidUtf8,
    })?;
    if document.starts_with('\u{feff}') {
        return Err(WireError {
            rule: RuleId::M001,
            location: WireLocation::ByteOffset(0),
            fault: WireFault::Bom,
        }
        .into());
    }
    preflight(document)?;
    let mut de = serde_json::Deserializer::from_str(document);
    let raw = <&RawValue>::deserialize(&mut de).map_err(|e| syntax(document, 0, "$", &e))?;
    let value = walk(document, raw, "$", 0, &Cell::new(0))?;
    de.end().map_err(|e| WireError {
        rule: RuleId::M002,
        location: WireLocation::LineColumn {
            line: e.line(),
            column: e.column(),
            path: "$".into(),
        },
        fault: WireFault::TrailingData,
    })?;
    Ok(value)
}
