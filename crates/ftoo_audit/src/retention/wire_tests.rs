use super::{
    syntax::Value,
    wire::{self, Codec},
};
use ftoo_manifest::{RuleId, WireError, WireFault, WireLocation};

const SEED: &[u8] = include_bytes!("../../../../generated/audit-report.json");
pub(super) fn actual_file_decode(bytes: &[u8]) -> Result<wire::Report, wire::WireRefusal> {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let production = super::produce(projection).unwrap();
    fixture_file_decode(&production, bytes)
}
fn fixture_file_decode(
    production: &std::sync::Arc<super::Produced>,
    bytes: &[u8],
) -> Result<wire::Report, wire::WireRefusal> {
    use super::{RetentionError, acceptance};
    use std::{
        collections::VecDeque,
        fs,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let projection = production.key.projection.clone();
    let path = std::env::temp_dir().join(format!(
        "retention-wire-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut memo = VecDeque::new();
    fs::write(&path, production.bytes.as_slice()).unwrap();
    let control = acceptance::accept_with(
        projection.clone(),
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, _| Ok(Arc::clone(production)),
    )
    .unwrap();
    assert_eq!(control.bytes(), production.bytes.as_slice());
    fs::write(&path, bytes).unwrap();
    let result = acceptance::accept_with(
        projection.clone(),
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, _| panic!("wire mutations refuse before production"),
    );
    let result = match result {
        Err(RetentionError::Wire(error)) => Err(error),
        Ok(accepted) => wire::decode(accepted.bytes()),
        Err(error) => panic!("unexpected complete fault: {error}"),
    };
    fs::write(&path, production.bytes.as_slice()).unwrap();
    let restored = acceptance::accept_with(
        projection,
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, _| panic!("restoration shares the exact successful production"),
    )
    .unwrap();
    assert_eq!(restored.bytes(), production.bytes.as_slice());
    fs::remove_file(path).unwrap();
    result
}
fn encoded(value: &Value) -> Vec<u8> {
    let mut text = String::new();
    wire::render(value, &mut text);
    text.push('\n');
    text.into_bytes()
}
fn at<'a>(value: &'a mut Value, indices: &[usize]) -> &'a mut Value {
    let Some((index, tail)) = indices.split_first() else {
        return value;
    };
    match value {
        Value::Object(fields) => at(&mut fields[*index].1, tail),
        Value::Array(values) => at(&mut values[*index], tail),
        _ => panic!("fixture location is a container"),
    }
}
type ObjectSite = (Vec<usize>, String, Vec<String>);
fn objects(value: &Value, indices: &[usize], path: &str, result: &mut Vec<ObjectSite>) {
    match value {
        Value::Object(fields) => {
            result.push((
                indices.into(),
                path.into(),
                fields.iter().map(|(name, _)| name.clone()).collect(),
            ));
            for (index, (name, value)) in fields.iter().enumerate() {
                let mut indices = indices.to_vec();
                indices.push(index);
                objects(value, &indices, &format!("{path}.{name}"), result);
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let mut indices = indices.to_vec();
                indices.push(index);
                objects(value, &indices, &format!("{path}[{index}]"), result);
            }
        }
        _ => (),
    }
}
fn shape_fault(path: &str, fault: WireFault) -> wire::WireRefusal {
    wire::WireRefusal::Wire(WireError {
        rule: RuleId::M004,
        location: WireLocation::Path(path.into()),
        fault,
    })
}
#[test]
fn every_seed_object_requires_each_field_and_refuses_extensions() {
    let original = wire::decode(SEED).unwrap();
    let value = original.value();
    let mut sites = Vec::new();
    objects(&value, &[], "$", &mut sites);
    for (indices, path, names) in sites {
        for name in &names {
            let mut changed = value.clone();
            let Value::Object(fields) = at(&mut changed, &indices) else {
                unreachable!()
            };
            fields.retain(|(key, _)| key != name);
            let actual = actual_file_decode(&encoded(&changed)).unwrap_err();
            assert_eq!(
                actual,
                shape_fault(&path, WireFault::MissingField { name: name.clone() })
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "retention shape M004 at json-path {path:?}: M004 at Path({path:?}): MissingField {{ name: {name:?} }}"
                )
            );
            assert_eq!(wire::decode(SEED), Ok(original.clone()));
        }
        let mut changed = value.clone();
        let Value::Object(fields) = at(&mut changed, &indices) else {
            unreachable!()
        };
        fields.push(("unexpected".into(), Value::Bool(true)));
        let actual = actual_file_decode(&encoded(&changed)).unwrap_err();
        assert_eq!(
            actual,
            shape_fault(
                &path,
                WireFault::UnknownField {
                    name: "unexpected".into(),
                    expected: names.clone()
                }
            )
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention shape M004 at json-path {path:?}: M004 at Path({path:?}): UnknownField {{ name: \"unexpected\", expected: {names:?} }}"
            )
        );
        assert_eq!(wire::decode(SEED), Ok(original.clone()));
    }
}

#[test]
fn every_seed_field_refuses_null_and_wrong_scalar() {
    let original = wire::decode(SEED).unwrap();
    let value = original.value();
    let mut sites = Vec::new();
    objects(&value, &[], "$", &mut sites);
    for (indices, path, names) in sites {
        for (index, name) in names.iter().enumerate() {
            for null in [false, true] {
                let mut changed = value.clone();
                let Value::Object(fields) = at(&mut changed, &indices) else {
                    unreachable!()
                };
                let expected = fields[index].1.kind().to_string();
                fields[index].1 = if null { Value::Null } else { Value::Bool(true) };
                let fault = if null {
                    WireFault::Null {
                        expected: expected.clone(),
                    }
                } else {
                    WireFault::WrongType {
                        expected: expected.clone(),
                        actual: "boolean".into(),
                    }
                };
                let field = format!("{path}.{name}");
                let actual = actual_file_decode(&encoded(&changed)).unwrap_err();
                assert_eq!(actual, shape_fault(&field, fault));
                let detail = if null {
                    format!("Null {{ expected: {expected:?} }}")
                } else {
                    format!("WrongType {{ expected: {expected:?}, actual: \"boolean\" }}")
                };
                assert_eq!(
                    actual.to_string(),
                    format!(
                        "retention shape M004 at json-path {field:?}: M004 at Path({field:?}): {detail}"
                    )
                );
                assert_eq!(wire::decode(SEED), Ok(original.clone()));
            }
        }
    }
}

const TOKEN_CASES: &[(&[usize], &str, &[&str])] = &[
    (
        &[3, 0, 0],
        "$.outcomes[0].claim",
        &[
            "def:langevin:coefficient-one",
            "def:langevin:coefficient-three",
            "claim:langevin:coefficient-one-value",
            "claim:langevin:coefficient-three-value",
        ],
    ),
    (
        &[3, 0, 1, 0],
        "$.outcomes[0].verdict.kind",
        &["certified", "refuted", "undecided", "deferred"],
    ),
    (
        &[3, 0, 1, 1, 0],
        "$.outcomes[0].verdict.receipt.producer",
        &[
            "langevin-coefficient-one-value",
            "langevin-coefficient-three-value",
        ],
    ),
    (
        &[3, 0, 1, 1, 2],
        "$.outcomes[0].verdict.receipt.budget",
        &["coefficient-one-v1", "coefficient-three-v1"],
    ),
    (
        &[3, 0, 1, 1, 6, 9],
        "$.outcomes[0].verdict.receipt.summary.region",
        &["ambient-product-x-no-assumptions"],
    ),
    (
        &[4, 0, 1],
        "$.statuses[0].maturity",
        &["proposed", "retained"],
    ),
    (&[4, 0, 2], "$.statuses[0].identification", &["identified"]),
    (&[4, 0, 3], "$.statuses[0].calibration", &["not-applicable"]),
    (
        &[4, 0, 4],
        "$.statuses[0].evidence",
        &[
            "not-a-claim",
            "certified",
            "refuted",
            "undecided",
            "deferred",
        ],
    ),
    (
        &[5, 0],
        "$.assessment.disposition",
        &[
            "pass",
            "input-error",
            "semantic-invalidity",
            "mathematical-failure",
            "incomplete",
        ],
    ),
];

#[test]
fn closed_tokens_and_mixed_verdict_fields_refuse_before_production() {
    let original = wire::decode(SEED).unwrap();
    for &(indices, path, expected) in TOKEN_CASES {
        let mut value = original.value();
        *at(&mut value, indices) = Value::String("foreign".into());
        let error = actual_file_decode(&encoded(&value)).unwrap_err();
        assert_eq!(
            error,
            shape_fault(
                path,
                WireFault::UnknownToken {
                    expected: expected.iter().map(|s| (*s).into()).collect(),
                    actual: "foreign".into()
                }
            )
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention shape M004 at json-path {path:?}: M004 at Path({path:?}): UnknownToken {{ expected: {expected:?}, actual: \"foreign\" }}"
            )
        );
    }
    let mut value = original.value();
    let Value::Object(verdict) = at(&mut value, &[3, 0, 1]) else {
        unreachable!()
    };
    verdict.push(("counterstatement".into(), Value::Null));
    let error = actual_file_decode(&encoded(&value)).unwrap_err();
    assert_eq!(
        error,
        shape_fault(
            "$.outcomes[0].verdict",
            WireFault::UnknownField {
                name: "counterstatement".into(),
                expected: vec!["kind".into(), "receipt".into()]
            }
        )
    );
    assert_eq!(
        error.to_string(),
        "retention shape M004 at json-path \"$.outcomes[0].verdict\": M004 at Path(\"$.outcomes[0].verdict\"): UnknownField { name: \"counterstatement\", expected: [\"kind\", \"receipt\"] }"
    );
}

#[test]
fn malformed_evidence_and_fixed_array_lengths_are_complete_shape_faults() {
    let original = wire::decode(SEED).unwrap();
    for value in ["0", "gg", " 00"] {
        let mut changed = original.value();
        *at(&mut changed, &[3, 0, 1, 1, 7]) = Value::String(value.into());
        let error = actual_file_decode(&encoded(&changed)).unwrap_err();
        assert_eq!(
            error,
            wire::WireRefusal::InvalidHex {
                path: "$.outcomes[0].verdict.receipt.source-hex".into(),
                value: value.into()
            }
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention shape M004 at json-path \"$.outcomes[0].verdict.receipt.source-hex\": invalid-hex value={value:?}"
            )
        );
    }
    let mut changed = original.value();
    let Value::Array(jet) = at(&mut changed, &[3, 0, 1, 1, 6, 1]) else {
        unreachable!()
    };
    jet.pop();
    let error = actual_file_decode(&encoded(&changed)).unwrap_err();
    assert_eq!(
        error,
        wire::WireRefusal::ArrayLength {
            path: "$.outcomes[0].verdict.receipt.summary.jet".into(),
            expected: 5,
            actual: 4
        }
    );
    assert_eq!(
        error.to_string(),
        "retention shape M004 at json-path \"$.outcomes[0].verdict.receipt.summary.jet\": array-length expected=5 actual=4"
    );
}

pub(super) fn alternative_shapes(production: &std::sync::Arc<super::Produced>) {
    let value = production.dto.value();
    let mut sites = Vec::new();
    objects(&value, &[], "$", &mut sites);
    for (indices, path, names) in sites {
        if !path.contains(".exhaustion")
            && !path.contains(".counterstatement")
            && !path.ends_with(".reason")
        {
            continue;
        }
        for name in &names {
            let mut changed = value.clone();
            let Value::Object(fields) = at(&mut changed, &indices) else {
                unreachable!()
            };
            fields.retain(|(key, _)| key != name);
            let actual = fixture_file_decode(production, &encoded(&changed)).unwrap_err();
            assert_eq!(
                actual,
                shape_fault(&path, WireFault::MissingField { name: name.clone() })
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "retention shape M004 at json-path {path:?}: M004 at Path({path:?}): MissingField {{ name: {name:?} }}"
                )
            );
        }
        let mut changed = value.clone();
        let Value::Object(fields) = at(&mut changed, &indices) else {
            unreachable!()
        };
        fields.push(("unexpected".into(), Value::Bool(true)));
        let actual = fixture_file_decode(production, &encoded(&changed)).unwrap_err();
        assert_eq!(
            actual,
            shape_fault(
                &path,
                WireFault::UnknownField {
                    name: "unexpected".into(),
                    expected: names.clone()
                }
            )
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention shape M004 at json-path {path:?}: M004 at Path({path:?}): UnknownField {{ name: \"unexpected\", expected: {names:?} }}"
            )
        );
    }
}

#[test]
fn scalar_bounds_accept_the_limit_and_refuse_the_first_excess() {
    for (dimension, maximum, path) in [
        (
            wire::Dimension::StringBytes,
            16 * 1024,
            "$.location.operation",
        ),
        (wire::Dimension::RationalBytes, 4096, "$.rational.numerator"),
    ] {
        let text = "1".repeat(maximum);
        let value = if dimension == wire::Dimension::StringBytes {
            Value::Object(vec![
                ("operation".into(), Value::String(text.clone())),
                ("logical-id".into(), Value::Null),
            ])
        } else {
            Value::Object(vec![
                ("numerator".into(), Value::String(text.clone())),
                ("denominator".into(), Value::String("1".into())),
            ])
        };
        let read = |value: &Value| {
            if dimension == wire::Dimension::StringBytes {
                wire::Location::read(value, "$.location").map(|value| value.value())
            } else {
                wire::Rational::read(value, "$.rational").map(|value| value.value())
            }
        };
        let expected = if dimension == wire::Dimension::StringBytes {
            Ok(value.clone())
        } else {
            Err(wire::WireRefusal::InvalidRational {
                path: "$.rational".into(),
                numerator: text.clone(),
                denominator: "1".into(),
            })
        };
        assert_eq!(read(&value), expected);
        let mut changed = value.clone();
        let Value::Object(fields) = &mut changed else {
            unreachable!()
        };
        fields[0].1 = Value::String(format!("{text}1"));
        let actual = read(&changed).unwrap_err();
        assert_eq!(
            actual,
            wire::WireRefusal::Limit {
                dimension,
                maximum,
                actual: maximum + 1,
                path: path.into()
            }
        );
        let token = if dimension == wire::Dimension::StringBytes {
            "string-bytes"
        } else {
            "rational-bytes"
        };
        assert_eq!(
            actual.to_string(),
            format!(
                "retention shape M061 at json-path \"{path}\": limit dimension={token} limit={maximum} actual={}",
                maximum + 1
            )
        );
        assert_eq!(read(&value), expected);
    }
}

fn limit(
    error: &wire::WireRefusal,
    dimension: wire::Dimension,
    maximum: usize,
    path: &str,
    token: &str,
) {
    assert_eq!(
        *error,
        wire::WireRefusal::Limit {
            dimension,
            maximum,
            actual: maximum + 1,
            path: path.into()
        }
    );
    assert_eq!(
        error.to_string(),
        format!(
            "retention shape M061 at json-path \"{path}\": limit dimension={token} limit={maximum} actual={}",
            maximum + 1
        )
    );
}
#[test]
fn projection_and_evidence_bounds_are_checked_before_expansion() {
    let mut report = wire::decode(SEED).unwrap();
    report.projection = " ".repeat(4 * 1024 * 1024);
    let bytes = wire::write(&report).unwrap();
    assert_eq!(wire::decode(&bytes), Ok(report.clone()));
    report.projection.push(' ');
    limit(
        &wire::write(&report).unwrap_err(),
        wire::Dimension::ProjectionBytes,
        4 * 1024 * 1024,
        "$.provenance.projection-utf8",
        "projection-bytes",
    );
    report.projection.pop();
    assert_eq!(
        wire::decode(&wire::write(&report).unwrap()),
        Ok(report.clone())
    );
    let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.source = vec![0; 1024 * 1024];
    let bytes = wire::write(&report).unwrap();
    assert_eq!(wire::decode(&bytes), Ok(report.clone()));
    let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.source.push(0);
    limit(
        &wire::write(&report).unwrap_err(),
        wire::Dimension::EvidenceBytes,
        1024 * 1024,
        "$.outcomes[0].verdict.receipt.source-hex",
        "evidence-bytes",
    );
    let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.source.pop();
    assert_eq!(wire::decode(&wire::write(&report).unwrap()), Ok(report));
}
#[test]
fn container_depth_array_and_total_value_limits_have_exact_edges() {
    use super::syntax;
    let array = Value::Array(vec![Value::Null; 4096]);
    assert_eq!(syntax::parse(&encoded(&array)), Ok(array.clone()));
    let excessive = Value::Array(vec![Value::Null; 4097]);
    limit(
        &syntax::parse(&encoded(&excessive)).unwrap_err(),
        wire::Dimension::ArrayElements,
        4096,
        "$",
        "array-elements",
    );
    assert_eq!(syntax::parse(&encoded(&array)), Ok(array));
    let mut values = vec![Value::Array(vec![Value::Null; 4095]); 16];
    values[15] = Value::Array(vec![Value::Null; 4094]);
    let original = Value::Array(values.clone());
    assert_eq!(syntax::parse(&encoded(&original)), Ok(original.clone()));
    values[15] = Value::Array(vec![Value::Null; 4095]);
    limit(
        &syntax::parse(&encoded(&Value::Array(values))).unwrap_err(),
        wire::Dimension::Values,
        65_536,
        "$[15][4094]",
        "values",
    );
    assert_eq!(syntax::parse(&encoded(&original)), Ok(original));
    let bytes = format!("{}null{}", "[".repeat(128), "]".repeat(128));
    let mut expected = Value::Null;
    for _ in 0..128 {
        expected = Value::Array(vec![expected]);
    }
    assert_eq!(syntax::parse(bytes.as_bytes()), Ok(expected.clone()));
    let excessive = format!("[{bytes}]");
    let actual = syntax::parse(excessive.as_bytes()).unwrap_err();
    assert_eq!(
        actual,
        wire::WireRefusal::Wire(WireError {
            rule: RuleId::M002,
            location: WireLocation::LineColumn {
                line: 1,
                column: 129,
                path: "$".into()
            },
            fault: WireFault::DepthLimit { maximum: 128 }
        })
    );
    assert_eq!(
        actual.to_string(),
        "retention syntax M002 at line-column 1:129 path \"$\": M002 at LineColumn { line: 1, column: 129, path: \"$\" }: DepthLimit { maximum: 128 }"
    );
    assert_eq!(syntax::parse(bytes.as_bytes()), Ok(expected));
}

#[test]
fn complete_input_and_output_ceiling_is_exact() {
    let mut report = wire::decode(SEED).unwrap();
    let row = report.outcomes[0].clone();
    report.outcomes = vec![row; 6];
    for row in &mut report.outcomes {
        let wire::Verdict::Certified(receipt) = &mut row.verdict else {
            unreachable!()
        };
        receipt.source.clear();
        receipt.residual.clear();
        receipt.member.clear();
    }
    let length = wire::write(&report).unwrap().len();
    if length % 2 != 0 {
        let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
            unreachable!()
        };
        receipt.summary.subject.push('x');
    }
    let mut remaining = (wire::MAX_INPUT - wire::write(&report).unwrap().len()) / 2;
    for row in &mut report.outcomes {
        let wire::Verdict::Certified(receipt) = &mut row.verdict else {
            unreachable!()
        };
        for slot in [
            &mut receipt.source,
            &mut receipt.residual,
            &mut receipt.member,
        ] {
            let size = remaining.min(1024 * 1024);
            slot.resize(size, 0);
            remaining -= size;
        }
    }
    assert_eq!(remaining, 0);
    let bytes = wire::write(&report).unwrap();
    assert_eq!(bytes.len(), 32 * 1024 * 1024);
    assert_eq!(wire::decode(&bytes), Ok(report.clone()));
    let mut excessive = bytes.clone();
    excessive.push(b' ');
    limit(
        &wire::decode(&excessive).unwrap_err(),
        wire::Dimension::InputBytes,
        32 * 1024 * 1024,
        "$",
        "input-bytes",
    );
    assert_eq!(wire::decode(&bytes), Ok(report.clone()));
    let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.summary.subject.push('x');
    limit(
        &wire::write(&report).unwrap_err(),
        wire::Dimension::OutputBytes,
        32 * 1024 * 1024,
        "$",
        "output-bytes",
    );
    let wire::Verdict::Certified(receipt) = &mut report.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.summary.subject.pop();
    assert_eq!(wire::write(&report).unwrap(), bytes);
}

#[test]
fn lexical_faults_precede_unknown_shapes_with_exact_coordinates() {
    let original = wire::decode(SEED).unwrap();
    for (text, column, path, key) in [
        (r#"{"x":null,"x":1}"#, 15, "$", "x"),
        (r#"{"id":1,"\u0069d":2}"#, 19, "$", "id"),
        (r#"{"other":{"a":null,"a":0}}"#, 24, "$.other", "a"),
    ] {
        let error = actual_file_decode(text.as_bytes()).unwrap_err();
        assert_eq!(
            error,
            wire::WireRefusal::Wire(WireError {
                rule: RuleId::M002,
                location: WireLocation::LineColumn {
                    line: 1,
                    column,
                    path: path.into()
                },
                fault: WireFault::DuplicateKey { key: key.into() }
            })
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention syntax M002 at line-column 1:{column} path \"{path}\": M002 at LineColumn {{ line: 1, column: {column}, path: \"{path}\" }}: DuplicateKey {{ key: \"{key}\" }}"
            )
        );
        assert_eq!(wire::decode(SEED), Ok(original.clone()));
    }
    for token in ["1.0", "1e0", "1e999999999999999999999999"] {
        let text = format!("{{\"other\":{token}}}");
        let error = actual_file_decode(text.as_bytes()).unwrap_err();
        assert_eq!(
            error,
            wire::WireRefusal::Wire(WireError {
                rule: RuleId::M003,
                location: WireLocation::LineColumn {
                    line: 1,
                    column: 10,
                    path: "$.other".into()
                },
                fault: WireFault::NonIntegerNumber {
                    token: token.into()
                }
            })
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention syntax M003 at line-column 1:10 path \"$.other\": M003 at LineColumn {{ line: 1, column: 10, path: \"$.other\" }}: NonIntegerNumber {{ token: \"{token}\" }}"
            )
        );
        assert_eq!(wire::decode(SEED), Ok(original.clone()));
    }
    for (text, column, trailing) in [
        ("+1", 1, false),
        ("01", 2, false),
        ("NaN", 1, false),
        ("{", 1, false),
        ("[1,]", 4, false),
        ("{} {}", 4, true),
    ] {
        let error = actual_file_decode(text.as_bytes()).unwrap_err();
        assert_eq!(
            error,
            wire::WireRefusal::Wire(WireError {
                rule: RuleId::M002,
                location: WireLocation::LineColumn {
                    line: 1,
                    column,
                    path: "$".into()
                },
                fault: if trailing {
                    WireFault::TrailingData
                } else {
                    WireFault::Syntax
                }
            })
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention syntax M002 at line-column 1:{column} path \"$\": M002 at LineColumn {{ line: 1, column: {column}, path: \"$\" }}: {}",
                if trailing { "TrailingData" } else { "Syntax" }
            )
        );
        assert_eq!(wire::decode(SEED), Ok(original.clone()));
    }
}

#[test]
fn schema_refusal_retains_the_full_integer_lexeme() {
    let original = wire::decode(SEED).unwrap();
    for token in ["2", "-0", "99999999999999999999999999999999999999"] {
        let text = std::str::from_utf8(SEED).unwrap().replacen(
            "\"schema\":1",
            &format!("\"schema\":{token}"),
            1,
        );
        let error = actual_file_decode(text.as_bytes()).unwrap_err();
        assert_eq!(
            error,
            wire::WireRefusal::Wire(WireError {
                rule: RuleId::M003,
                location: WireLocation::Path("$.schema".into()),
                fault: WireFault::SchemaValue {
                    expected: "1".into(),
                    actual: token.into()
                }
            })
        );
        assert_eq!(
            error.to_string(),
            format!(
                "retention syntax M003 at json-path \"$.schema\": M003 at Path(\"$.schema\"): SchemaValue {{ expected: \"1\", actual: \"{token}\" }}"
            )
        );
        assert_eq!(wire::decode(SEED), Ok(original.clone()));
    }
}
