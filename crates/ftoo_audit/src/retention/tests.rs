use super::wire;
use wire::Codec;

pub(crate) fn assert_fixture(
    graph: &ftoo_manifest::ValidatedManifest,
    report: &crate::AuditReport,
) {
    use sha2::{Digest, Sha256};
    let projection = ftoo_manifest::projection_bytes(graph).unwrap();
    let key = super::InputKey {
        projection: projection.clone(),
        contract: 1,
        engine: ftoo_manifest::ENGINE_PIN.0.into(),
    };
    let dto = super::production::report(
        report,
        String::from_utf8(projection.clone()).unwrap(),
        wire::hex(&Sha256::digest(&projection)),
        key.engine.clone(),
    )
    .unwrap();
    let bytes = wire::write(&dto).unwrap();
    assert_eq!(wire::decode(&bytes), Ok(dto.clone()));
    let report = crate::AuditReport::from_outcomes(
        graph,
        report.snapshot().clone(),
        report.reached().to_vec(),
        report.outcomes().to_vec(),
        report.measurements().to_vec(),
    )
    .unwrap();
    let production = std::sync::Arc::new(super::Produced {
        key,
        report: std::sync::Arc::new(report),
        dto,
        bytes: bytes.clone(),
        info: super::InfoRecord::collect(super::Run::Fresh1),
    });
    // Seam acceptance exercises plumbing; the round-trip assertions above are genuine.
    let mut memo = std::collections::VecDeque::new();
    let accepted =
        super::acceptance::accept_with(projection, bytes.clone(), &mut memo, &mut |_, _| {
            Ok(std::sync::Arc::clone(&production))
        })
        .unwrap();
    assert_eq!(accepted.bytes(), bytes);
    super::matrix_tests::proof_matrix(&production);
    super::verdict_tests::matrix(&production);
    super::wire_tests::alternative_shapes(&production);
    assert_eq!(accepted.report().gated(), production.report.gated());
    println!(
        "retention fixture bytes={} outcomes={:?}",
        bytes.len(),
        accepted
            .report()
            .outcomes()
            .iter()
            .map(|row| row.verdict().class())
            .collect::<Vec<_>>()
    );
    for row in accepted.report().outcomes() {
        if let crate::Verdict::Refuted(receipt) = row.verdict() {
            println!(
                "retention contrary evidence sizes={:?} counterstatement={}",
                receipt.evidence().iter().map(Vec::len).collect::<Vec<_>>(),
                receipt.counterstatement().unwrap().witness().len()
            );
        }
    }
}

#[test]
fn canonical_seed_roundtrip_preserves_all_gated_bytes() {
    use sha2::{Digest, Sha256};
    use std::{sync::Arc, time::Instant};
    let graph = ftoo_manifest::seed().unwrap();
    let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
    let started = Instant::now();
    let engine = super::revision::linked().unwrap();
    let revision_time = started.elapsed();
    let started = Instant::now();
    let report = Arc::new(crate::audit(&graph).unwrap());
    let production_time = started.elapsed();
    assert_eq!(
        report
            .outcomes()
            .iter()
            .map(|row| row.verdict().class())
            .collect::<Vec<_>>(),
        vec![
            ftoo_manifest::VerdictClass::Certified,
            ftoo_manifest::VerdictClass::Certified
        ]
    );
    let started = Instant::now();
    let dto = super::production::report(
        &report,
        String::from_utf8(projection.clone()).unwrap(),
        wire::hex(&Sha256::digest(&projection)),
        engine.clone(),
    )
    .unwrap();
    let bytes = wire::write(&dto).unwrap();
    let encode_time = started.elapsed();
    let started = Instant::now();
    let reread = wire::decode(&bytes).unwrap();
    let decode_time = started.elapsed();
    assert_eq!(dto, reread);
    assert_eq!(wire::write(&reread).unwrap(), bytes);
    let started = Instant::now();
    let info = super::InfoRecord::collect(super::Run::Fresh1);
    let info_time = started.elapsed();
    let production = Arc::new(super::Produced {
        key: super::InputKey {
            projection: projection.clone(),
            contract: 1,
            engine,
        },
        report: Arc::clone(&report),
        dto,
        bytes: bytes.clone(),
        info,
    });
    // Seam acceptance exercises plumbing; the round-trip assertions above are genuine.
    let mut memo = std::collections::VecDeque::new();
    let accepted =
        super::acceptance::accept_with(projection, bytes.clone(), &mut memo, &mut |_, _| {
            Ok(Arc::clone(&production))
        })
        .unwrap();
    assert_eq!(accepted.bytes(), bytes);
    super::matrix_tests::proof_matrix(&production);
    super::verdict_tests::matrix(&production);
    assert_eq!(accepted.report().gated(), report.gated());
    println!(
        "retention seed production={production_time:?} encode={encode_time:?} decode={decode_time:?} revision={revision_time:?} diagnostic-collection={info_time:?} report-bytes={} measurements={:?}",
        bytes.len(),
        report.measurements()
    );
    for row in report.outcomes() {
        let crate::Verdict::Certified(receipt) = row.verdict() else {
            unreachable!()
        };
        println!(
            "retention evidence claim={} sizes={:?}",
            row.claim().semantic_name(),
            receipt.evidence().iter().map(Vec::len).collect::<Vec<_>>()
        );
    }
}

#[test]
fn byte_differences_keep_complete_bytes_and_runs() {
    use super::{
        difference::{Difference, Slot, Window},
        info::{InfoRecord, Run},
    };
    let left_info = InfoRecord::collect(Run::Retained);
    let right_info = left_info.clone();
    let mut difference =
        Difference::between(b"abcd", b"axcyz", left_info.clone(), right_info.clone()).unwrap();
    assert_eq!(
        difference,
        Difference {
            claim: None,
            slot: Slot::None,
            path: "$".into(),
            left: b"abcd".to_vec(),
            right: b"axcyz".to_vec(),
            offset: 1,
            runs: 2,
            left_window: Window {
                start: 0,
                bytes: b"abcd".to_vec(),
                eof: true
            },
            right_window: Window {
                start: 0,
                bytes: b"axcyz".to_vec(),
                eof: true
            },
            left_info,
            right_info
        }
    );
    for (slot, token) in [
        (Slot::None, "none"),
        (Slot::Source, "source"),
        (Slot::Residual, "residual"),
        (Slot::Member, "member"),
        (Slot::Counterstatement, "counterstatement"),
    ] {
        difference.slot = slot;
        assert_eq!(difference.left, b"abcd");
        assert_eq!(
            difference.to_string(),
            format!(
                "claim=none slot={token} path=\"$\" left-length=4 right-length=5 offset=1 runs=2 left-window-start=0 left-window=\"61626364\" left-eof=true right-window-start=0 right-window=\"617863797a\" right-eof=true left=retained right=retained"
            )
        );
    }
}

#[test]
fn required_exhaustion_components_roundtrip_losslessly() {
    for reason in [
        nrz_cas::Exhaustion::Limit,
        nrz_cas::Exhaustion::ArithmeticRange,
        nrz_cas::Exhaustion::Allocation,
        nrz_cas::Exhaustion::Capacity,
    ] {
        for logical_id in [None, Some(0), Some(u64::MAX)] {
            let value = wire::Exhaustion {
                resource: nrz_cas::limits::BudgetKind::Work,
                reason,
                limit: u64::MAX,
                attempted: u64::MAX,
                location: wire::Location {
                    operation: "operation\n\"\\é".into(),
                    logical_id,
                },
            };
            let encoded = value.value();
            let decoded = wire::Exhaustion::read(&encoded, "$.exhaustion").unwrap();
            assert_eq!(decoded, value);
            assert_eq!(decoded.value(), encoded);
        }
    }
}

#[test]
fn exact_shape_refusals_and_restoration() {
    use super::syntax::Value;
    use ftoo_manifest::{RuleId, WireError, WireFault, WireLocation};
    let original = wire::Location {
        operation: "audit".into(),
        logical_id: Some(3),
    };
    for name in ["operation", "logical-id"] {
        let Value::Object(mut fields) = original.value() else {
            unreachable!()
        };
        fields.retain(|(key, _)| key != name);
        let expected = wire::WireRefusal::Wire(WireError {
            rule: RuleId::M004,
            location: WireLocation::Path("$.location".into()),
            fault: WireFault::MissingField { name: name.into() },
        });
        assert_eq!(
            wire::Location::read(&Value::Object(fields), "$.location"),
            Err(expected)
        );
        assert_eq!(
            wire::Location::read(&original.value(), "$.location"),
            Ok(original.clone())
        );
    }
    let actual = wire::Location::read(&Value::Null, "$.location");
    assert_eq!(
        actual,
        Err(wire::WireRefusal::Wire(WireError {
            rule: RuleId::M004,
            location: WireLocation::Path("$.location".into()),
            fault: WireFault::Null {
                expected: "object".into()
            }
        }))
    );
    assert_eq!(
        actual.unwrap_err().to_string(),
        "retention shape M004 at json-path \"$.location\": M004 at Path(\"$.location\"): Null { expected: \"object\" }"
    );
    assert_eq!(
        wire::Location::read(&original.value(), "$.location"),
        Ok(original)
    );
}

#[test]
fn rational_lexemes_refuse_and_restore() {
    use super::syntax::Value;
    for (numerator, denominator) in [
        ("+1", "1"),
        ("-0", "1"),
        ("01", "1"),
        ("1", "00"),
        ("1", "0"),
        ("0", "2"),
        ("1", "-1"),
        ("2", "6"),
        ("-2", "6"),
        ("9223372036854775808", "1"),
        ("-9223372036854775809", "1"),
        ("1", "18446744073709551616"),
    ] {
        let value = Value::Object(vec![
            ("numerator".into(), Value::String(numerator.into())),
            ("denominator".into(), Value::String(denominator.into())),
        ]);
        assert_eq!(
            wire::Rational::read(&value, "$.rational"),
            Err(wire::WireRefusal::InvalidRational {
                path: "$.rational".into(),
                numerator: numerator.into(),
                denominator: denominator.into()
            })
        );
        assert_eq!(
            wire::Rational::read(&value, "$.rational")
                .unwrap_err()
                .to_string(),
            format!(
                "retention shape M004 at json-path \"$.rational\": invalid-rational numerator=\"{numerator}\" denominator=\"{denominator}\""
            )
        );
        let restored = wire::Rational {
            numerator: "-1".into(),
            denominator: "45".into(),
        };
        assert_eq!(
            wire::Rational::read(&restored.value(), "$.rational"),
            Ok(restored)
        );
    }
}

#[test]
fn revision_uses_resolved_fragment_and_refuses_ambiguity() {
    use super::revision::{self, Reason, Source, Unavailable};
    let revision = "598ff683a57caca07cf9d44e3fd28c716afb81ef";
    let source = format!("git+https://example.invalid/engine?rev=old#{revision}");
    let entry =
        format!("[[package]]\nname = \"nrz-cas\"\nversion = \"0.1.0\"\nsource = \"{source}\"\n");
    assert_eq!(
        revision::resolved(&entry, Source::CurrentLock),
        Ok(revision.into())
    );
    assert_eq!(revision::linked(), Ok(revision.into()));
    assert_eq!(
        revision::manifest(include_str!("../../Cargo.toml")),
        Ok(revision.into())
    );
    let error = Unavailable {
        source: Source::CurrentLock,
        path: "Cargo.lock",
        package: "nrz-cas",
        reason: Reason::Missing,
    };
    assert_eq!(
        revision::resolved("", Source::CurrentLock),
        Err(error.clone())
    );
    assert_eq!(
        error.to_string(),
        "retention provenance M061 at file \"Cargo.lock\": engine-revision-unavailable source=current-lock package=\"nrz-cas\" reason=missing"
    );
    assert_eq!(
        revision::resolved(&entry, Source::CurrentLock),
        Ok(revision.into())
    );
    assert_eq!(
        revision::resolved(&format!("{entry}{entry}"), Source::CurrentLock),
        Err(Unavailable {
            source: Source::CurrentLock,
            path: "Cargo.lock",
            package: "nrz-cas",
            reason: Reason::Ambiguous {
                entries: vec![("0.1.0".into(), source.clone()), ("0.1.0".into(), source)]
            }
        })
    );
    assert_eq!(
        revision::resolved(&entry, Source::CurrentLock),
        Ok(revision.into())
    );
}

#[test]
fn informational_records_are_separate_and_explicitly_unavailable() {
    use super::info::{Closure, Compiler, InfoRecord, Run, Unavailable};
    assert_eq!(
        InfoRecord::collect(Run::Retained).json(),
        "{\"run\":\"retained\",\"compiler\":{\"state\":\"unavailable\",\"reason\":\"not-captured\"},\"closure\":{\"state\":\"unavailable\",\"reason\":\"not-captured\"}}"
    );
    let record = InfoRecord {
        run: Run::Fresh2,
        compiler: Compiler::Unavailable(Unavailable::ReadFailed),
        closure: Closure::Unavailable(Unavailable::Ambiguous),
    };
    assert_eq!(
        record.json(),
        "{\"run\":\"fresh-2\",\"compiler\":{\"state\":\"unavailable\",\"reason\":\"read-failed\"},\"closure\":{\"state\":\"unavailable\",\"reason\":\"ambiguous\"}}"
    );
    let first = InfoRecord::collect(Run::Fresh1);
    let second = InfoRecord::collect(Run::Fresh1);
    assert_eq!(first, second);
}

#[test]
fn unsupported_exhaustion_components_refuse_without_panicking() {
    let original = nrz_cas::Fault::exhausted(
        nrz_cas::limits::BudgetKind::Work,
        1,
        2,
        nrz_cas::Location::new("test"),
    );
    let mut changed = original.clone();
    changed.series_evidence = Some(std::sync::Arc::new(
        nrz_cas::SeriesRefusalEvidence::Precision([1, 2, 3]),
    ));
    let error = super::production::exhaustion_record(&changed).unwrap_err();
    assert_eq!(
        error,
        super::RetentionError::UnsupportedComponent {
            component: "exhaustion.series-evidence"
        }
    );
    assert_eq!(
        error.to_string(),
        "retention reproduction M061 at file \"generated/audit-report.json\": unsupported-component \"exhaustion.series-evidence\""
    );
    let invalid =
        nrz_cas::Fault::invalid(nrz_cas::InvalidCode::Rule, nrz_cas::Location::new("test"));
    let error = super::production::exhaustion_record(&invalid).unwrap_err();
    assert_eq!(
        error,
        super::RetentionError::UnsupportedComponent {
            component: "exhaustion.kind"
        }
    );
    assert_eq!(
        error.to_string(),
        "retention reproduction M061 at file \"generated/audit-report.json\": unsupported-component \"exhaustion.kind\""
    );
    assert_eq!(
        super::production::exhaustion_record(&original).unwrap(),
        wire::Exhaustion {
            resource: nrz_cas::limits::BudgetKind::Work,
            reason: nrz_cas::Exhaustion::Limit,
            limit: 1,
            attempted: 2,
            location: wire::Location {
                operation: "test".into(),
                logical_id: None
            }
        }
    );
}
