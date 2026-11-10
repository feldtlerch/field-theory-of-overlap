use super::*;
use crate::seed;

fn encoded(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    render(value, &mut out);
    out.into_bytes()
}
fn refusal(bytes: &[u8], rule: RuleId, location: WireLocation, fault: WireFault) {
    assert_eq!(
        load_projection(bytes),
        Err(ProjectionError::Wire(WireError {
            rule,
            location,
            fault
        }))
    );
    assert!(load_projection(GOLDEN.as_bytes()).is_ok());
}
fn path_fault(bytes: &[u8], path: &str, fault: WireFault) {
    refusal(bytes, RuleId::M004, WireLocation::Path(path.into()), fault);
}
fn lc(line: usize, column: usize, path: &str) -> WireLocation {
    WireLocation::LineColumn {
        line,
        column,
        path: path.into(),
    }
}

#[test]
fn utf8_bom_locations_and_display() {
    refusal(
        b"a\xff",
        RuleId::M001,
        WireLocation::ByteOffset(1),
        WireFault::InvalidUtf8,
    );
    refusal(
        b"\xef\xbb\xbf{}",
        RuleId::M001,
        WireLocation::ByteOffset(0),
        WireFault::Bom,
    );
    assert_eq!(
        load_projection(b"\xff").unwrap_err().to_string(),
        "M001 at ByteOffset(0): InvalidUtf8"
    );
}
#[test]
fn syntax_precedes_shapes_in_unknown_children() {
    for (bytes, column, path, key) in [
        (r#"{"x":null,"x":1}"#, 15, "$", "x"),
        (r#"{"id":1,"\u0069d":2}"#, 19, "$", "id"),
        (r#"{"other":{"a":null,"a":0}}"#, 24, "$.other", "a"),
        (r#"{"kind":"quantity","kind":"claim"}"#, 27, "$", "kind"),
    ] {
        refusal(
            bytes.as_bytes(),
            RuleId::M002,
            lc(1, column, path),
            WireFault::DuplicateKey { key: key.into() },
        );
    }
    for token in ["1.0", "1e0", "1e999999999999999999999999"] {
        let bytes = format!("{{\"other\":{token}}}");
        refusal(
            bytes.as_bytes(),
            RuleId::M003,
            lc(1, 10, "$.other"),
            WireFault::NonIntegerNumber {
                token: token.into(),
            },
        );
    }
    for bytes in ["+1", "01", "NaN", "{", "[1,]"] {
        let column = match bytes {
            "+1" | "NaN" | "{" => 1,
            "01" => 2,
            _ => 4,
        };
        refusal(
            bytes.as_bytes(),
            RuleId::M002,
            lc(1, column, "$"),
            WireFault::Syntax,
        );
    }
    refusal(
        b"{} {}",
        RuleId::M002,
        lc(1, 4, "$"),
        WireFault::TrailingData,
    );
}
#[test]
fn depth_preflight_boundary_strings_and_unicode_coordinates() {
    let accepted = format!("{}0{}", "[".repeat(128), "]".repeat(128));
    assert!(syntax::parse(accepted.as_bytes()).is_ok());
    let rejected = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    refusal(
        rejected.as_bytes(),
        RuleId::M002,
        lc(1, 129, "$"),
        WireFault::DepthLimit { maximum: 128 },
    );
    assert_eq!(
        syntax::parse(br#""[[[\"{]""#).unwrap(),
        Value::String("[[[\"{]".into())
    );
    refusal(
        "{\"é\":0,\n\"x\":1e0}".as_bytes(),
        RuleId::M003,
        lc(2, 5, "$.x"),
        WireFault::NonIntegerNumber {
            token: "1e0".into(),
        },
    );
}
#[test]
fn schema_unsigned_ranges_and_fake_raw_marker() {
    for actual in ["2", "-0", "99999999999999999999999999999999999999"] {
        let bytes = GOLDEN.replacen("\"schema\":1", &format!("\"schema\":{actual}"), 1);
        refusal(
            bytes.as_bytes(),
            RuleId::M003,
            WireLocation::Path("$.schema".into()),
            WireFault::SchemaValue {
                expected: "1".into(),
                actual: actual.into(),
            },
        );
    }
    for (field, width, path) in [
        ("revision", 32, "$.producers[0].revision"),
        ("target", 64, "$.producers[0].target"),
    ] {
        for actual in ["-0", "-1", "18446744073709551616"] {
            let bytes = GOLDEN.replacen(
                &format!("\"{field}\":1"),
                &format!("\"{field}\":{actual}"),
                1,
            );
            path_fault(
                bytes.as_bytes(),
                path,
                WireFault::IntegerRange {
                    expected: format!("u{width}"),
                    actual: actual.into(),
                },
            );
        }
    }
    let bytes = GOLDEN.replacen("\"revision\":1", "\"revision\":4294967296", 1);
    path_fault(
        bytes.as_bytes(),
        "$.producers[0].revision",
        WireFault::IntegerRange {
            expected: "u32".into(),
            actual: "4294967296".into(),
        },
    );
    for name in [
        "tags",
        "history",
        "stores",
        "$serde_json::private::RawValue",
    ] {
        let bytes = GOLDEN.replacen('{', &format!("{{\"{name}\":[],"), 1);
        path_fault(
            bytes.as_bytes(),
            "$",
            WireFault::UnknownField {
                name: name.into(),
                expected: ROOT.iter().map(ToString::to_string).collect(),
            },
        );
    }
}

#[derive(Clone)]
enum Step {
    Key(String),
    Index(usize),
}
fn at_mut<'a>(value: &'a mut Value, steps: &[Step]) -> &'a mut Value {
    let Some((first, rest)) = steps.split_first() else {
        return value;
    };
    match (first, value) {
        (Step::Key(key), Value::Object(fields)) => at_mut(
            &mut fields.iter_mut().find(|(k, _)| k == key).unwrap().1,
            rest,
        ),
        (Step::Index(index), Value::Array(values)) => at_mut(&mut values[*index], rest),
        _ => panic!("fixture path"),
    }
}
type ObjectFixture = (Vec<Step>, String, Vec<(String, Value)>);
fn objects(value: &Value, steps: &[Step], path: &str, out: &mut Vec<ObjectFixture>) {
    match value {
        Value::Object(fields) => {
            out.push((steps.to_vec(), path.into(), fields.clone()));
            for (key, child) in fields {
                let mut next = steps.to_vec();
                next.push(Step::Key(key.clone()));
                objects(child, &next, &format!("{path}.{key}"), out);
            }
        }
        Value::Array(values) => {
            for (i, child) in values.iter().enumerate() {
                let mut next = steps.to_vec();
                next.push(Step::Index(i));
                objects(child, &next, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}
fn shape_matrix(value: &Value) -> usize {
    assert!(load_projection(&encoded(value)).is_ok());
    let mut inventory = Vec::new();
    objects(value, &[], "$", &mut inventory);
    let mut cases = 0;
    for (steps, path, fields) in inventory {
        let names: Vec<_> = fields.iter().map(|(k, _)| k.clone()).collect();
        let mut extra = value.clone();
        let Value::Object(target) = at_mut(&mut extra, &steps) else {
            panic!("object fixture");
        };
        target.push(("unexpected".into(), Value::Bool(true)));
        path_fault(
            &encoded(&extra),
            &path,
            WireFault::UnknownField {
                name: "unexpected".into(),
                expected: names,
            },
        );
        cases += 1;
        for (key, original) in fields {
            let mut missing = value.clone();
            let Value::Object(target) = at_mut(&mut missing, &steps) else {
                panic!("object fixture");
            };
            target.retain(|(k, _)| k != &key);
            path_fault(
                &encoded(&missing),
                &path,
                WireFault::MissingField { name: key.clone() },
            );
            cases += 1;
            let mut field_steps = steps.clone();
            field_steps.push(Step::Key(key.clone()));
            for (replacement, fault) in [
                (
                    Value::Null,
                    WireFault::Null {
                        expected: original.kind().into(),
                    },
                ),
                (
                    if matches!(original, Value::Bool(_)) {
                        s("wrong")
                    } else {
                        Value::Bool(false)
                    },
                    WireFault::WrongType {
                        expected: original.kind().into(),
                        actual: if matches!(original, Value::Bool(_)) {
                            "string"
                        } else {
                            "boolean"
                        }
                        .into(),
                    },
                ),
            ] {
                let mut mutated = value.clone();
                *at_mut(&mut mutated, &field_steps) = replacement;
                path_fault(&encoded(&mutated), &format!("{path}.{key}"), fault);
                cases += 1;
            }
            let mut duplicated = value.clone();
            let Value::Object(target) = at_mut(&mut duplicated, &steps) else {
                panic!("object fixture");
            };
            target.push((key.clone(), Value::Null));
            let bytes = encoded(&duplicated);
            let needle = format!("\"{key}\":null");
            let document = std::str::from_utf8(&bytes).unwrap();
            let column = document.find(&needle).unwrap() + needle.len() - 3;
            refusal(
                &bytes,
                RuleId::M002,
                lc(1, column, &path),
                WireFault::DuplicateKey { key },
            );
            cases += 1;
        }
        assert!(load_projection(&encoded(value)).is_ok());
    }
    cases
}
#[test]
fn exhaustive_core_object_field_shape_matrix() {
    let baseline = syntax::parse(GOLDEN.as_bytes()).unwrap();
    let count = shape_matrix(&baseline);
    assert_eq!(count, 1_143);
}

fn fixture_field<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let steps: Vec<_> = path
        .iter()
        .map(|p| {
            p.parse::<usize>()
                .map_or_else(|_| Step::Key((*p).into()), Step::Index)
        })
        .collect();
    at_mut(value, &steps)
}
#[test]
fn awaiting_variants_have_complete_shape_matrices_and_round_trip() {
    for (awaits, count) in [
        (kind("audit-binding"), 1_144),
        (
            obj([
                ("kind", s("capability")),
                ("capability", s("exact-rational-linear-equivalence")),
            ]),
            1_152,
        ),
    ] {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        *fixture_field(&mut value, &["coverage", "slice"]) = Value::Array(vec![]);
        *fixture_field(&mut value, &["coverage", "closed-world", "audit"]) = Value::Bool(false);
        *fixture_field(&mut value, &["nodes", "0", "audit"]) =
            obj([("kind", s("awaiting")), ("awaits", awaits.clone())]);
        *fixture_field(&mut value, &["nodes", "0", "status", "blocker"]) =
            obj([("kind", s("awaiting")), ("awaits", awaits)]);
        assert_eq!(shape_matrix(&value), count);
        let loaded = load_projection(&encoded(&value)).unwrap();
        assert_eq!(
            projection_bytes(loaded.manifest()).unwrap(),
            [encoded(&value), vec![b'\n']].concat()
        );
    }
}
fn token_table<T: Copy + PartialEq + std::fmt::Debug>(table: &[(T, &str)]) {
    for (value, name) in table {
        assert_eq!(token(&s(*name), "$.token", table).unwrap(), *value);
    }
    for actual in ["invented", "RustVariant"] {
        assert_eq!(
            token(&s(actual), "$.token", table),
            Err(WireError {
                rule: RuleId::M004,
                location: WireLocation::Path("$.token".into()),
                fault: WireFault::UnknownToken {
                    expected: table.iter().map(|(_, name)| (*name).into()).collect(),
                    actual: actual.into()
                }
            })
        );
        assert_eq!(token(&s(table[0].1), "$.token", table).unwrap(), table[0].0);
    }
}
#[test]
fn closed_token_tables_are_exhaustive_and_case_sensitive() {
    assert_eq!(
        PRODUCERS.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
        [
            "langevin-coefficient-one-value",
            "langevin-coefficient-three-value"
        ]
    );
    assert_eq!(
        BUDGETS.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
        ["coefficient-one-v1", "coefficient-three-v1"]
    );
    assert_eq!(
        CAPS.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
        [
            "exact-rational-linear-equivalence",
            "formal-truncated-power-series",
            "symbolic-trigonometry",
            "trigonometric-integration"
        ]
    );
    token_table(PRODUCERS);
    token_table(BUDGETS);
    token_table(CAPS);
    token_table(MATURITY);
    token_table(AVAILABILITY);
    token_table(&NodeId::ALL.map(|id| (id, id.semantic_name())));
    for names in [
        vec!["quantity", "claim"],
        vec!["identified"],
        vec!["not-applicable"],
        vec!["not-a-claim", "deferred"],
        vec!["bound", "awaiting"],
        vec!["audit-binding", "capability"],
        vec!["audit-run-required", "awaiting"],
        vec!["unconditional"],
        vec!["always"],
        vec!["depends-on", "compatible-with"],
        vec!["explicit-partial", "complete"],
        vec!["pinned-trial-limits-v1"],
        vec!["ambient-product-x-no-assumptions"],
    ] {
        token_table(
            &names
                .iter()
                .enumerate()
                .map(|(i, name)| (i, *name))
                .collect::<Vec<_>>(),
        );
    }
}

type TokenCase = (
    &'static [&'static str],
    &'static str,
    &'static [&'static str],
);
const ENUM_REFUSALS: &[TokenCase] = &[
    (&["nodes", "0", "kind"], "Claim", &["quantity", "claim"]),
    (
        &["nodes", "0", "status", "maturity"],
        "Proposed",
        &["proposed", "retained"],
    ),
    (
        &["nodes", "0", "status", "identification"],
        "Identified",
        &["identified"],
    ),
    (
        &["nodes", "0", "status", "calibration"],
        "NotApplicable",
        &["not-applicable"],
    ),
    (
        &["nodes", "0", "status", "evidence"],
        "Deferred",
        &["not-a-claim", "deferred"],
    ),
    (
        &["nodes", "0", "audit", "kind"],
        "Bound",
        &["bound", "awaiting"],
    ),
    (
        &["nodes", "0", "status", "blocker", "kind"],
        "AuditRunRequired",
        &["audit-run-required", "awaiting"],
    ),
    (
        &["nodes", "0", "audit", "routes", "kind"],
        "Unconditional",
        &["unconditional"],
    ),
    (
        &["edges", "0", "relation"],
        "CompatibleWith",
        &["depends-on", "compatible-with"],
    ),
    (&["edges", "0", "guard", "kind"], "Always", &["always"]),
    (
        &["coverage", "release-gate"],
        "ExplicitPartial",
        &["explicit-partial", "complete"],
    ),
    (
        &["budgets", "0", "profile"],
        "PinnedTrialLimitsV1",
        &["pinned-trial-limits-v1"],
    ),
    (
        &["producers", "0", "region"],
        "AmbientProductXNoAssumptions",
        &["ambient-product-x-no-assumptions"],
    ),
    (
        &["capabilities", "0", "availability"],
        "Present",
        &["present", "absent-with-evidence", "not-established"],
    ),
];
#[test]
fn rust_variant_spellings_are_refused_at_actual_grammar_positions() {
    for (steps, actual, expected) in ENUM_REFUSALS {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        *fixture_field(&mut value, steps) = s(*actual);
        let mut path = String::from("$");
        for step in *steps {
            use std::fmt::Write;
            if let Ok(index) = step.parse::<usize>() {
                write!(path, "[{index}]").unwrap();
            } else {
                write!(path, ".{step}").unwrap();
            }
        }
        path_fault(
            &encoded(&value),
            &path,
            WireFault::UnknownToken {
                expected: expected.iter().map(ToString::to_string).collect(),
                actual: (*actual).into(),
            },
        );
    }
    assert_eq!(
        awaits(&kind("AuditBinding"), "$.awaits"),
        Err(shape(
            "$.awaits.kind",
            WireFault::UnknownToken {
                expected: vec!["audit-binding".into(), "capability".into()],
                actual: "AuditBinding".into()
            }
        ))
    );
    assert_eq!(
        awaits(&kind("audit-binding"), "$.awaits").unwrap(),
        Awaiting::AuditBinding
    );
}
#[test]
fn new_register_entries_have_literal_statements_and_scopes() {
    assert_eq!(
        crate::RULES[36..]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        vec![
            (
                RuleId::M037,
                "A projection preserves every graph record and fixed authority assertion, including complete budgets, bindings, descriptors, history and store event references.",
                "Projection loading of fixed assertions and generation/check semantic comparison against Rust authority.",
                crate::RuleState::Enforced,
                None
            ),
            (
                RuleId::M038,
                "Generation and check share one canonical writer, and a generated projection must equal its regenerated bytes.",
                "The generated manifest artifact and its shared generation/check path.",
                crate::RuleState::Enforced,
                None
            ),
        ]
    );
}
#[test]
fn contract_multiplicities_preserve_all_original_positions() {
    for component in ["budgets", "producers", "capabilities"] {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        let Value::Array(values) = fixture_field(&mut value, &[component]) else {
            panic!("array fixture");
        };
        let second = values.len();
        values.push(values[0].clone());
        assert_eq!(
            load_projection(&encoded(&value)),
            Err(ProjectionError::Assertion(vec![ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.{component}"),
                position: Some(0),
                fault: ProjectionFault::ContractMultiplicity {
                    record: match component {
                        "budgets" => "coefficient-one-v1",
                        "producers" => "langevin-coefficient-one-value",
                        _ => "exact-rational-linear-equivalence",
                    }
                    .into(),
                    expected: 1,
                    actual: 2,
                    positions: vec![0, second]
                }
            }]))
        );
        values_restore_control();
    }
    for requirement in ["dependencies", "capabilities"] {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        let Value::Array(values) = fixture_field(&mut value, &["producers", "0", requirement])
        else {
            panic!("array fixture");
        };
        values.push(values[0].clone());
        assert_eq!(
            load_projection(&encoded(&value)),
            Err(ProjectionError::Assertion(vec![ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.producers[0].{requirement}"),
                position: Some(0),
                fault: ProjectionFault::ContractMultiplicity {
                    record: match requirement {
                        "dependencies" => "def:langevin:coefficient-one",
                        _ => "formal-truncated-power-series",
                    }
                    .into(),
                    expected: 1,
                    actual: 2,
                    positions: vec![0, 1]
                }
            }]))
        );
        values_restore_control();
    }
    for component in ["producers", "capabilities"] {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        let Value::Array(values) = fixture_field(&mut value, &[component]) else {
            panic!("array fixture");
        };
        values.remove(0);
        assert_eq!(
            load_projection(&encoded(&value)),
            Err(ProjectionError::Assertion(vec![ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.{component}"),
                position: None,
                fault: ProjectionFault::ContractMultiplicity {
                    record: match component {
                        "budgets" => "coefficient-one-v1",
                        "producers" => "langevin-coefficient-one-value",
                        _ => "exact-rational-linear-equivalence",
                    }
                    .into(),
                    expected: 1,
                    actual: 0,
                    positions: vec![]
                }
            }]))
        );
        values_restore_control();
    }
}
fn values_restore_control() {
    assert_eq!(
        projection_bytes(load_projection(GOLDEN.as_bytes()).unwrap().manifest()).unwrap(),
        GOLDEN.as_bytes()
    );
}

#[test]
fn independent_fixed_faults_accumulate_in_semantic_order_with_original_positions() {
    for reversed in [false, true] {
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        *fixture_field(&mut value, &["budgets", "0", "limits", "work"]) = n(8_000_001);
        *fixture_field(&mut value, &["budgets", "0", "limits", "arithmetic-steps"]) = n(500_001);
        *fixture_field(&mut value, &["budgets", "1", "limits", "work"]) = n(8_000_002);
        *fixture_field(&mut value, &["producers", "0", "revision"]) = n(2);
        *fixture_field(&mut value, &["capabilities", "0", "adapter-implemented"]) =
            Value::Bool(true);
        if reversed {
            for component in ["budgets", "producers", "capabilities"] {
                let Value::Array(records) = fixture_field(&mut value, &[component]) else {
                    panic!("records");
                };
                records.reverse();
            }
        }
        let first = usize::from(reversed);
        let second = 1 - first;
        let cap = if reversed { 3 } else { 0 };
        let expected = vec![
            ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.budgets[{first}].limits.arithmetic-steps"),
                position: Some(first),
                fault: ProjectionFault::BudgetLimitsMismatch {
                    expected: 500_000,
                    actual: 500_001,
                },
            },
            ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.budgets[{first}].limits.work"),
                position: Some(first),
                fault: ProjectionFault::BudgetLimitsMismatch {
                    expected: 8_000_000,
                    actual: 8_000_001,
                },
            },
            ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.budgets[{second}].limits.work"),
                position: Some(second),
                fault: ProjectionFault::BudgetLimitsMismatch {
                    expected: 8_000_000,
                    actual: 8_000_002,
                },
            },
            ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.producers[{first}].revision"),
                position: Some(first),
                fault: ProjectionFault::ContractMismatch {
                    expected: Box::new(ProjectionValue::Integer(1)),
                    actual: Box::new(ProjectionValue::Integer(2)),
                },
            },
            ProjectionDiagnostic {
                rule: RuleId::M037,
                path: format!("$.capabilities[{cap}].adapter-implemented"),
                position: Some(cap),
                fault: ProjectionFault::ContractMismatch {
                    expected: Box::new(ProjectionValue::Boolean(false)),
                    actual: Box::new(ProjectionValue::Boolean(true)),
                },
            },
        ];
        assert_eq!(
            load_projection(&encoded(&value)),
            Err(ProjectionError::Assertion(expected))
        );
        values_restore_control();
    }
}

#[test]
fn graph_admission_from_bytes_matches_direct_drafts_and_whole_rule_sets() {
    let baseline = load_projection(GOLDEN.as_bytes()).unwrap();
    for case in 0..6 {
        let graph = baseline.manifest();
        let mut draft = ManifestDraft {
            nodes: graph.nodes().to_vec(),
            edges: graph.edges().to_vec(),
            coverage: graph.coverage().clone(),
            budgets: graph.budgets().to_vec(),
        };
        let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
        let expected = match case {
            0 => {
                draft.nodes.push(draft.nodes[0].clone());
                let Value::Array(nodes) = fixture_field(&mut value, &["nodes"]) else {
                    panic!("nodes");
                };
                nodes.push(nodes[0].clone());
                vec![RuleId::M006]
            }
            1 => {
                draft.edges.push(draft.edges[0]);
                let Value::Array(edges) = fixture_field(&mut value, &["edges"]) else {
                    panic!("edges");
                };
                edges.push(edges[0].clone());
                vec![RuleId::M009]
            }
            2 => {
                draft.coverage.slice.push(draft.coverage.slice[0]);
                let Value::Array(slice) = fixture_field(&mut value, &["coverage", "slice"]) else {
                    panic!("slice");
                };
                slice.push(slice[0].clone());
                vec![RuleId::M013]
            }
            3 => {
                draft.nodes.remove(2);
                let Value::Array(nodes) = fixture_field(&mut value, &["nodes"]) else {
                    panic!("nodes");
                };
                nodes.remove(2);
                vec![RuleId::M007]
            }
            4 => {
                let NodePayload::Quantity { description } = &mut draft.nodes[2].payload else {
                    panic!("quantity");
                };
                *description = " ".into();
                *fixture_field(&mut value, &["nodes", "2", "description"]) = s(" ");
                vec![RuleId::M006]
            }
            _ => {
                draft.edges.push(draft.edges[0]);
                draft.coverage.slice.push(draft.coverage.slice[0]);
                let Value::Array(edges) = fixture_field(&mut value, &["edges"]) else {
                    panic!("edges");
                };
                edges.push(edges[0].clone());
                let Value::Array(slice) = fixture_field(&mut value, &["coverage", "slice"]) else {
                    panic!("slice");
                };
                slice.push(slice[0].clone());
                vec![RuleId::M009, RuleId::M013]
            }
        };
        let errors = validate(draft).unwrap_err();
        let rules: std::collections::BTreeSet<_> =
            errors.diagnostics().iter().map(|d| d.rule).collect();
        assert_eq!(rules.into_iter().collect::<Vec<_>>(), expected);
        assert_eq!(
            load_projection(&encoded(&value)),
            Err(ProjectionError::Graph(errors))
        );
        values_restore_control();
    }
}
#[test]
fn quantity_evidence_and_bound_blocker_assertions_cannot_change_base_status() {
    let bytes = GOLDEN.replacen(
        "\"evidence\":\"not-a-claim\"",
        "\"evidence\":\"deferred\"",
        1,
    );
    refusal(
        bytes.as_bytes(),
        RuleId::M012,
        WireLocation::Path("$.nodes[2].status.evidence".into()),
        WireFault::StatusMismatch {
            node: NodeId::LangevinCoefficientOne,
            expected: "not-a-claim".into(),
            actual: "deferred".into(),
        },
    );
    let bytes = GOLDEN.replacen(
        "\"blocker\":{\"kind\":\"audit-run-required\"}",
        "\"blocker\":{\"kind\":\"awaiting\",\"awaits\":{\"kind\":\"audit-binding\"}}",
        1,
    );
    refusal(
        bytes.as_bytes(),
        RuleId::M012,
        WireLocation::Path("$.nodes[0].status.blocker".into()),
        WireFault::StatusMismatch {
            node: NodeId::LangevinCoefficientOneValue,
            expected: "Some(AuditRunRequired)".into(),
            actual: "Some(Awaiting(AuditBinding))".into(),
        },
    );
}
#[test]
fn huge_integer_lexemes_and_phase_order_do_not_use_machine_number_coercion() {
    let huge = "9".repeat(500);
    assert_eq!(
        syntax::parse(huge.as_bytes()).unwrap(),
        Value::Integer(huge)
    );
    refusal(
        br#"{"earlier":1.0,"later":{"x":0,"x":1}}"#,
        RuleId::M003,
        lc(1, 12, "$.earlier"),
        WireFault::NonIntegerNumber {
            token: "1.0".into(),
        },
    );
    refusal(
        br#"{"earlier":{"x":0,"x":1},"later":1.0}"#,
        RuleId::M002,
        lc(1, 23, "$.earlier"),
        WireFault::DuplicateKey { key: "x".into() },
    );
    refusal(
        b"{\n\"child\": [1,]}",
        RuleId::M002,
        lc(2, 13, "$"),
        WireFault::Syntax,
    );
}
#[test]
fn canonical_descriptions_preserve_all_controls_and_unicode_without_normalization() {
    let descriptions = [
        "literal / quote \" slash \\ é e\u{301}".to_owned(),
        (0u8..32).map(char::from).chain("literal".chars()).collect(),
    ];
    for description in descriptions {
        let mut draft = crate::seed_draft();
        draft.nodes[0].payload = NodePayload::Quantity {
            description: description.clone(),
        };
        let graph = validate(draft).unwrap();
        let bytes = projection_bytes(&graph).unwrap();
        let loaded = load_projection(&bytes).unwrap();
        assert_eq!(
            loaded
                .manifest()
                .node(NodeId::LangevinCoefficientOne)
                .unwrap()
                .payload
                .description(),
            description
        );
        assert_eq!(projection_bytes(loaded.manifest()).unwrap(), bytes);
    }
}

#[test]
fn every_resolved_limit_assertion_is_compared_before_profile_conversion() {
    for budget in ["0", "1"] {
        for (name, expected) in EXPECTED_LIMITS {
            let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
            *fixture_field(&mut value, &["budgets", budget, "limits", name]) = n(expected + 1);
            assert_eq!(
                load_projection(&encoded(&value)),
                Err(ProjectionError::Assertion(vec![ProjectionDiagnostic {
                    rule: RuleId::M037,
                    path: format!("$.budgets[{budget}].limits.{name}"),
                    position: Some(budget.parse().unwrap()),
                    fault: ProjectionFault::BudgetLimitsMismatch {
                        expected: *expected,
                        actual: expected + 1
                    }
                }]))
            );
            values_restore_control();
        }
    }
}

#[test]
fn canonical_reordering_escaping_and_control_characters() {
    fn reverse(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.reverse();
                for (_, v) in fields {
                    reverse(v);
                }
            }
            Value::Array(values) => {
                values.reverse();
                for v in values {
                    reverse(v);
                }
            }
            _ => {}
        }
    }
    let mut value = syntax::parse(GOLDEN.as_bytes()).unwrap();
    reverse(&mut value);
    let bytes = encoded(&value);
    let loaded = load_projection(&bytes).unwrap();
    assert_eq!(
        projection_bytes(loaded.manifest()).unwrap(),
        GOLDEN.as_bytes()
    );
    let escaped = GOLDEN.replace("claim:", "\\u0063laim:");
    assert_eq!(
        projection_bytes(load_projection(escaped.as_bytes()).unwrap().manifest()).unwrap(),
        GOLDEN.as_bytes()
    );
    for (input, expected) in [
        ("\"\\/ée\u{301}", "\"\\\"\\\\/ée\u{301}\""),
        ("\u{8}\t\n\u{c}\r", "\"\\b\\t\\n\\f\\r\""),
    ] {
        assert_eq!(String::from_utf8(encoded(&s(input))).unwrap(), expected);
    }
    for control in 0u8..32 {
        let actual = encoded(&s(char::from(control).to_string()));
        let expected = match control {
            8 => "\"\\b\"".into(),
            9 => "\"\\t\"".into(),
            10 => "\"\\n\"".into(),
            12 => "\"\\f\"".into(),
            13 => "\"\\r\"".into(),
            _ => format!("\"\\u{control:04x}\""),
        };
        assert_eq!(actual, expected.as_bytes());
        assert_eq!(
            syntax::parse(&actual).unwrap(),
            s(char::from(control).to_string())
        );
    }
}

#[test]
fn canonical_seed_independent_golden_and_complete_budget_inventory() {
    let graph = seed().unwrap();
    assert_eq!(projection_bytes(&graph).unwrap(), GOLDEN.as_bytes());
    let loaded = load_projection(GOLDEN.as_bytes()).unwrap();
    assert_eq!(loaded.manifest().nodes().len(), 4);
    assert_eq!(loaded.manifest().edges().len(), 6);
    assert_eq!(loaded.budgets().len(), 2);
    assert_eq!(loaded.producers().len(), 2);
    assert_eq!(loaded.capabilities().len(), 4);
    assert_eq!(
        BudgetProfile::PinnedTrialLimitsV1.resolved().fields(),
        EXPECTED_LIMITS
    );
    for budget in loaded.budgets() {
        assert_eq!(budget.limits().fields(), EXPECTED_LIMITS);
    }
    assert_eq!(
        projection_bytes(loaded.manifest()).unwrap(),
        GOLDEN.as_bytes()
    );
}

const GOLDEN: &str = r#"{"schema":1,"engine-pin":"a5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad","nodes":[{"id":"claim:langevin:coefficient-one-value","kind":"claim","description":"The first coefficient is exactly 1/3.","status":{"maturity":"proposed","identification":"identified","calibration":"not-applicable","evidence":"deferred","blocker":{"kind":"audit-run-required"}},"audit":{"kind":"bound","producer":"langevin-coefficient-one-value","budget":"coefficient-one-v1","routes":{"kind":"unconditional"}}},{"id":"claim:langevin:coefficient-three-value","kind":"claim","description":"The third coefficient is exactly -1/45.","status":{"maturity":"proposed","identification":"identified","calibration":"not-applicable","evidence":"deferred","blocker":{"kind":"audit-run-required"}},"audit":{"kind":"bound","producer":"langevin-coefficient-three-value","budget":"coefficient-three-v1","routes":{"kind":"unconditional"}}},{"id":"def:langevin:coefficient-one","kind":"quantity","description":"The coefficient of k in the three-dimensional Langevin expansion.","status":{"maturity":"proposed","identification":"identified","calibration":"not-applicable","evidence":"not-a-claim"}},{"id":"def:langevin:coefficient-three","kind":"quantity","description":"The coefficient of k cubed in the same expansion.","status":{"maturity":"proposed","identification":"identified","calibration":"not-applicable","evidence":"not-a-claim"}}],"edges":[{"relation":"compatible-with","source":"def:langevin:coefficient-one","target":"def:langevin:coefficient-three","guard":{"kind":"always"}},{"relation":"compatible-with","source":"def:langevin:coefficient-three","target":"def:langevin:coefficient-one","guard":{"kind":"always"}},{"relation":"depends-on","source":"claim:langevin:coefficient-one-value","target":"def:langevin:coefficient-one","guard":{"kind":"always"}},{"relation":"depends-on","source":"claim:langevin:coefficient-three-value","target":"claim:langevin:coefficient-one-value","guard":{"kind":"always"}},{"relation":"depends-on","source":"claim:langevin:coefficient-three-value","target":"def:langevin:coefficient-three","guard":{"kind":"always"}},{"relation":"depends-on","source":"def:langevin:coefficient-three","target":"def:langevin:coefficient-one","guard":{"kind":"always"}}],"coverage":{"slice":["claim:langevin:coefficient-one-value","claim:langevin:coefficient-three-value"],"closed-world":{"referents":false,"relations":false,"audit":true},"release-gate":"explicit-partial"},"budgets":[{"id":"coefficient-one-v1","producer":"langevin-coefficient-one-value","profile":"pinned-trial-limits-v1","pin":"a5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad","limits":{"arithmetic-steps":500000,"boundary-nodes":128,"case-arms":256,"coefficient-maps":256,"coefficient-ops":2000000,"component-bits":4096,"component-splits":128,"condition-atoms":256,"condition-steps":100000,"cost-quota":4,"decimal-exponent":128,"delta-order":8,"denominator-factors":256,"depth":128,"diagnostic-records":128,"edges":200000,"expansion-credits":20000,"family-parameters":64,"frontier-labels":8192,"gcd-steps":100000,"hypothesis-depth":32,"integer-exponent":256,"intermediate-bits":8192,"isolation-steps":100000,"jet-entries":4096,"jet-order":8,"linear-pivots":256,"literal-bytes":256,"local-branches":64,"local-contexts":128,"match-attempts":50000,"match-batch":32,"matrix-entries":4096,"name-bytes":256,"orbit-members":256,"orbit-permutations":40320,"orbit-terms":262144,"original-occurrences":10000,"output-bytes":1048576,"pairing-steps":4096,"payload-bytes":16777216,"peak-expression-size":50000,"polynomial-degree":64,"polynomial-terms":4096,"polynomial-variables":16,"progress-quota":1,"proof-nodes":20000,"pullback-zeros":64,"queue-entries":16384,"refinement-pairs":4096,"region-cells":256,"region-nodes":2048,"replay-work":4500000,"resolvent-degree":32,"resolvent-depth":8,"selection-slots":64,"solution-branches":256,"solve-identities":8192,"source-bytes":262144,"staged-edges":200000,"staged-payload-bytes":16777216,"staged-vertices":50000,"symmetric-steps":4096,"test-nodes":256,"tower-degree":256,"tower-levels":8,"tower-reductions":100000,"transitions":20000,"vertices":50000,"witness-bytes":1048576,"work":8000000}},{"id":"coefficient-three-v1","producer":"langevin-coefficient-three-value","profile":"pinned-trial-limits-v1","pin":"a5cb0ab0d1763e8ed620f7c765fdd8d3b26b65ad","limits":{"arithmetic-steps":500000,"boundary-nodes":128,"case-arms":256,"coefficient-maps":256,"coefficient-ops":2000000,"component-bits":4096,"component-splits":128,"condition-atoms":256,"condition-steps":100000,"cost-quota":4,"decimal-exponent":128,"delta-order":8,"denominator-factors":256,"depth":128,"diagnostic-records":128,"edges":200000,"expansion-credits":20000,"family-parameters":64,"frontier-labels":8192,"gcd-steps":100000,"hypothesis-depth":32,"integer-exponent":256,"intermediate-bits":8192,"isolation-steps":100000,"jet-entries":4096,"jet-order":8,"linear-pivots":256,"literal-bytes":256,"local-branches":64,"local-contexts":128,"match-attempts":50000,"match-batch":32,"matrix-entries":4096,"name-bytes":256,"orbit-members":256,"orbit-permutations":40320,"orbit-terms":262144,"original-occurrences":10000,"output-bytes":1048576,"pairing-steps":4096,"payload-bytes":16777216,"peak-expression-size":50000,"polynomial-degree":64,"polynomial-terms":4096,"polynomial-variables":16,"progress-quota":1,"proof-nodes":20000,"pullback-zeros":64,"queue-entries":16384,"refinement-pairs":4096,"region-cells":256,"region-nodes":2048,"replay-work":4500000,"resolvent-degree":32,"resolvent-depth":8,"selection-slots":64,"solution-branches":256,"solve-identities":8192,"source-bytes":262144,"staged-edges":200000,"staged-payload-bytes":16777216,"staged-vertices":50000,"symmetric-steps":4096,"test-nodes":256,"tower-degree":256,"tower-levels":8,"tower-reductions":100000,"transitions":20000,"vertices":50000,"witness-bytes":1048576,"work":8000000}}],"producers":[{"id":"langevin-coefficient-one-value","claim":"claim:langevin:coefficient-one-value","dependencies":["def:langevin:coefficient-one"],"capabilities":["formal-truncated-power-series"],"budget":"coefficient-one-v1","revision":1,"target":1,"region":"ambient-product-x-no-assumptions"},{"id":"langevin-coefficient-three-value","claim":"claim:langevin:coefficient-three-value","dependencies":["claim:langevin:coefficient-one-value","def:langevin:coefficient-three"],"capabilities":["formal-truncated-power-series"],"budget":"coefficient-three-v1","revision":1,"target":3,"region":"ambient-product-x-no-assumptions"}],"capabilities":[{"id":"exact-rational-linear-equivalence","availability":"present","adapter-implemented":false,"evidence":"Accepted trial linear-equivalence and coefficient-identity witnesses; no production linear adapter."},{"id":"formal-truncated-power-series","availability":"present","adapter-implemented":true,"evidence":"Fixed object Compose, evaluated residual and Member. Source/reciprocal are registered but not exposed. Product polynomial Defined is unavailable: source requires it and region partition rejects Product."},{"id":"symbolic-trigonometry","availability":"not-established","adapter-implemented":false,"evidence":"No present-pin evidence in this adapter."},{"id":"trigonometric-integration","availability":"not-established","adapter-implemented":false,"evidence":"No present-pin evidence in this adapter."}]}
"#;

const EXPECTED_LIMITS: &[(&str, u64)] = &[
    ("arithmetic-steps", 500_000),
    ("boundary-nodes", 128),
    ("case-arms", 256),
    ("coefficient-maps", 256),
    ("coefficient-ops", 2_000_000),
    ("component-bits", 4096),
    ("component-splits", 128),
    ("condition-atoms", 256),
    ("condition-steps", 100_000),
    ("cost-quota", 4),
    ("decimal-exponent", 128),
    ("delta-order", 8),
    ("denominator-factors", 256),
    ("depth", 128),
    ("diagnostic-records", 128),
    ("edges", 200_000),
    ("expansion-credits", 20_000),
    ("family-parameters", 64),
    ("frontier-labels", 8192),
    ("gcd-steps", 100_000),
    ("hypothesis-depth", 32),
    ("integer-exponent", 256),
    ("intermediate-bits", 8192),
    ("isolation-steps", 100_000),
    ("jet-entries", 4096),
    ("jet-order", 8),
    ("linear-pivots", 256),
    ("literal-bytes", 256),
    ("local-branches", 64),
    ("local-contexts", 128),
    ("match-attempts", 50_000),
    ("match-batch", 32),
    ("matrix-entries", 4096),
    ("name-bytes", 256),
    ("orbit-members", 256),
    ("orbit-permutations", 40_320),
    ("orbit-terms", 262_144),
    ("original-occurrences", 10_000),
    ("output-bytes", 1_048_576),
    ("pairing-steps", 4096),
    ("payload-bytes", 16_777_216),
    ("peak-expression-size", 50_000),
    ("polynomial-degree", 64),
    ("polynomial-terms", 4096),
    ("polynomial-variables", 16),
    ("progress-quota", 1),
    ("proof-nodes", 20_000),
    ("pullback-zeros", 64),
    ("queue-entries", 16_384),
    ("refinement-pairs", 4096),
    ("region-cells", 256),
    ("region-nodes", 2048),
    ("replay-work", 4_500_000),
    ("resolvent-degree", 32),
    ("resolvent-depth", 8),
    ("selection-slots", 64),
    ("solution-branches", 256),
    ("solve-identities", 8192),
    ("source-bytes", 262_144),
    ("staged-edges", 200_000),
    ("staged-payload-bytes", 16_777_216),
    ("staged-vertices", 50_000),
    ("symmetric-steps", 4096),
    ("test-nodes", 256),
    ("tower-degree", 256),
    ("tower-levels", 8),
    ("tower-reductions", 100_000),
    ("transitions", 20_000),
    ("vertices", 50_000),
    ("witness-bytes", 1_048_576),
    ("work", 8_000_000),
];
