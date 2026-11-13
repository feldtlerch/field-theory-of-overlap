use super::{acceptance, wire, *};
use std::{
    collections::VecDeque,
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub(super) fn expected_difference(
    left: &[u8],
    right: &[u8],
    left_info: InfoRecord,
    right_info: InfoRecord,
) -> Difference {
    let unequal: Vec<_> = (0..left.len().max(right.len()))
        .filter(|&i| left.get(i) != right.get(i))
        .collect();
    let offset = unequal[0];
    let runs = 1 + unequal
        .windows(2)
        .filter(|pair| pair[1] != pair[0] + 1)
        .count();
    let window = |bytes: &[u8]| {
        let start = offset.saturating_sub(64).min(bytes.len());
        let end = (offset + 257).min(bytes.len());
        Window {
            start,
            bytes: bytes[start..end].to_vec(),
            eof: end == bytes.len(),
        }
    };
    Difference {
        claim: None,
        slot: Slot::None,
        path: "$".into(),
        left: left.into(),
        right: right.into(),
        offset,
        runs,
        left_window: window(left),
        right_window: window(right),
        left_info,
        right_info,
    }
}
fn slot(verdict: &mut wire::Verdict, slot: Slot) -> Option<&mut Vec<u8>> {
    let receipt = match verdict {
        wire::Verdict::Certified(receipt) => receipt,
        wire::Verdict::Refuted {
            receipt,
            counterstatement,
        } => {
            if slot == Slot::Counterstatement {
                return Some(&mut counterstatement.witness);
            }
            receipt
        }
        _ => return None,
    };
    match slot {
        Slot::Source => Some(&mut receipt.source),
        Slot::Residual => Some(&mut receipt.residual),
        Slot::Member => Some(&mut receipt.member),
        Slot::Counterstatement | Slot::None => None,
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
pub(super) fn proof_matrix(production: &Arc<Produced>) {
    let path = std::env::temp_dir().join(format!(
        "retention-proof-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, &production.bytes).unwrap();
    let mut memo = VecDeque::new();
    restore(&path, production, &mut memo);
    for index in 0..production.dto.outcomes.len() {
        for (role, suffix) in [
            (Slot::Source, "receipt.source-hex"),
            (Slot::Residual, "receipt.residual-hex"),
            (Slot::Member, "receipt.member-hex"),
            (Slot::Counterstatement, "counterstatement.witness-hex"),
        ] {
            let mut dto = production.dto.clone();
            let Some(bytes) = slot(&mut dto.outcomes[index].verdict, role) else {
                continue;
            };
            let original = bytes.clone();
            for offset in [0, original.len() / 2, original.len()] {
                *bytes = original.clone();
                if offset == bytes.len() {
                    bytes.push(1);
                } else {
                    bytes[offset] ^= 1;
                }
                let changed_slot = bytes.clone();
                let mut changed = production.dto.clone();
                *slot(&mut changed.outcomes[index].verdict, role).unwrap() = changed_slot.clone();
                let retained = wire::write(&changed).unwrap();
                fs::write(&path, &retained).unwrap();
                let mut calls = 0;
                let actual = acceptance::accept_with(
                    production.key.projection.clone(),
                    fs::read(&path).unwrap(),
                    &mut memo,
                    &mut |_, run| {
                        calls += 1;
                        assert_eq!(run, Run::Fresh2);
                        let mut rerun = production.as_ref().clone();
                        rerun.info.run = run;
                        Ok(Arc::new(rerun))
                    },
                )
                .unwrap_err();
                let mut named = expected_difference(
                    &changed_slot,
                    &original,
                    InfoRecord::collect(Run::Retained),
                    production.info.clone(),
                );
                named.claim = Some(changed.outcomes[index].claim);
                named.slot = role;
                named.path = format!("$.outcomes[{index}].verdict.{suffix}");
                let expected = RetentionError::NonReproducibility {
                    key: Box::new(production.key.clone()),
                    basis: Basis::RetainedAgainstFresh,
                    retained: retained.clone(),
                    fresh1: production.bytes.clone(),
                    fresh2: production.bytes.clone(),
                    information: Box::new([
                        InfoRecord::collect(Run::Retained),
                        production.info.clone(),
                        InfoRecord {
                            run: Run::Fresh2,
                            ..production.info.clone()
                        },
                    ]),
                    differences: vec![
                        named,
                        expected_difference(
                            &retained,
                            &production.bytes,
                            InfoRecord::collect(Run::Retained),
                            production.info.clone(),
                        ),
                    ],
                };
                assert_eq!(
                    actual.to_string(),
                    super::classifier_tests::expected_display(&expected)
                );
                assert_eq!(actual, expected);
                assert_eq!(calls, 1);
                restore(&path, production, &mut memo);
            }
        }
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn canonical_variants_are_not_accepted_after_normalization() {
    use wire::Codec;
    let original = include_bytes!("../../../../generated/audit-report.json");
    let dto = wire::decode(original).unwrap();
    let text = std::str::from_utf8(original).unwrap();
    let mut cases = vec![
        text.trim_end().as_bytes().to_vec(),
        text.replacen('{', "{ ", 1).into_bytes(),
        text.replacen("schema", "\\u0073chema", 1).into_bytes(),
        text.replacen(
            &format!("\"engine-revision\":\"{}\"", ftoo_manifest::ENGINE_PIN.0),
            &format!(
                "\"engine-revision\":\"{}\"",
                ftoo_manifest::ENGINE_PIN.0.to_uppercase()
            ),
            1,
        )
        .into_bytes(),
    ];
    let mut value = dto.value();
    let super::syntax::Value::Object(fields) = &mut value else {
        unreachable!()
    };
    fields.swap(0, 1);
    let mut text = String::new();
    wire::render(&value, &mut text);
    text.push('\n');
    cases.push(text.into_bytes());
    let mut rows = dto.value();
    let super::syntax::Value::Object(fields) = &mut rows else {
        unreachable!()
    };
    let super::syntax::Value::Array(outcomes) = &mut fields
        .iter_mut()
        .find(|(name, _)| name == "outcomes")
        .unwrap()
        .1
    else {
        unreachable!()
    };
    outcomes.reverse();
    let mut text = String::new();
    wire::render(&rows, &mut text);
    text.push('\n');
    cases.push(text.into_bytes());
    for bytes in cases {
        let difference = expected_difference(
            &bytes,
            original,
            InfoRecord::collect(Run::Retained),
            InfoRecord::collect(Run::Retained),
        );
        let display = format!(
            "retention shape M061 at json-path \"$\": non-canonical canonical-side=right {}",
            super::classifier_tests::difference_display_labels(
                &difference,
                "retained",
                "canonical"
            )
        );
        let expected = wire::WireRefusal::NonCanonical {
            difference: Box::new(difference),
        };
        let actual = super::wire_tests::actual_file_decode(&bytes).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), display);
        assert_eq!(wire::decode(original), Ok(dto.clone()));
    }
}

#[test]
fn mismatch_diagnostic_production_is_measured_separately() {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let key = InputKey {
        projection,
        contract: 1,
        engine: ftoo_manifest::ENGINE_PIN.0.into(),
    };
    let first = Arc::new(acceptance::fresh(&key, Run::Fresh1).unwrap());
    let mut changed = first.dto.clone();
    let original = slot(&mut changed.outcomes[0].verdict, Slot::Source)
        .unwrap()
        .clone();
    let changed_slot = slot(&mut changed.outcomes[0].verdict, Slot::Source).unwrap();
    changed_slot[0] ^= 1;
    let changed_slot = changed_slot.clone();
    let retained = wire::write(&changed).unwrap();
    let mut memo = VecDeque::from([Arc::clone(&first)]);
    let mut calls = 0;
    let mut duration = std::time::Duration::ZERO;
    let started = std::time::Instant::now();
    let actual = acceptance::accept_with(
        key.projection.clone(),
        retained.clone(),
        &mut memo,
        &mut |key, run| {
            assert_eq!(run, Run::Fresh2);
            calls += 1;
            let started = std::time::Instant::now();
            let result = acceptance::fresh(key, run).map(Arc::new);
            duration = started.elapsed();
            result
        },
    )
    .unwrap_err();
    let total = started.elapsed();
    let mut named = expected_difference(
        &changed_slot,
        &original,
        InfoRecord::collect(Run::Retained),
        first.info.clone(),
    );
    named.claim = Some(ftoo_manifest::NodeId::LangevinCoefficientOneValue);
    named.slot = Slot::Source;
    named.path = "$.outcomes[0].verdict.receipt.source-hex".into();
    let expected = RetentionError::NonReproducibility {
        key: Box::new(key.clone()),
        basis: Basis::RetainedAgainstFresh,
        retained: retained.clone(),
        fresh1: first.bytes.clone(),
        fresh2: first.bytes.clone(),
        information: Box::new([
            InfoRecord::collect(Run::Retained),
            first.info.clone(),
            InfoRecord::collect(Run::Fresh2),
        ]),
        differences: vec![
            named,
            expected_difference(
                &retained,
                &first.bytes,
                InfoRecord::collect(Run::Retained),
                first.info.clone(),
            ),
        ],
    };
    assert_eq!(
        actual.to_string(),
        super::classifier_tests::expected_display(&expected)
    );
    assert_eq!(actual, expected);
    assert_eq!(calls, 1);
    let restored = acceptance::accept_with(
        key.projection,
        first.bytes.clone(),
        &mut memo,
        &mut |_, _| panic!("restored exact production is shared"),
    )
    .unwrap();
    assert_eq!(restored.bytes(), first.bytes);
    println!(
        "retention mismatch-only diagnostic-production={duration:?} total-classification={total:?} fresh-reruns={calls}"
    );
}

#[test]
fn informational_records_cannot_stale_or_change_gated_bytes() {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let production = produce(projection.clone()).unwrap();
    for information in [
        InfoRecord {
            run: Run::Fresh1,
            compiler: Compiler::Available {
                release: "another release".into(),
                commit_hash: "another commit".into(),
            },
            closure: Closure::Available(Vec::new()),
        },
        InfoRecord {
            run: Run::Fresh1,
            compiler: Compiler::Unavailable(Unavailable::ReadFailed),
            closure: Closure::Unavailable(Unavailable::TooLarge),
        },
    ] {
        let mut changed = production.as_ref().clone();
        changed.info = information.clone();
        assert_eq!(changed.key, production.key);
        assert_eq!(wire::write(&changed.dto).unwrap(), production.bytes);
        let changed = Arc::new(changed);
        let accepted = acceptance::accept_with(
            projection.clone(),
            production.bytes.clone(),
            &mut VecDeque::new(),
            &mut |_, _| Ok(Arc::clone(&changed)),
        )
        .unwrap();
        assert_eq!(accepted.bytes(), production.bytes);
        assert_eq!(accepted.information(), &information);
        let restored = accept(projection.clone(), production.bytes.clone()).unwrap();
        assert_eq!(restored.bytes(), production.bytes);
    }
}

fn restore(path: &std::path::Path, production: &Arc<Produced>, memo: &mut VecDeque<Arc<Produced>>) {
    fs::write(path, &production.bytes).unwrap();
    let accepted = acceptance::accept_with(
        production.key.projection.clone(),
        fs::read(path).unwrap(),
        memo,
        &mut |_, _| Ok(Arc::clone(production)),
    )
    .unwrap();
    assert_eq!(accepted.bytes(), production.bytes);
}
