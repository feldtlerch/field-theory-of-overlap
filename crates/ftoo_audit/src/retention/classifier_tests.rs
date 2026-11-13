use super::{acceptance, wire, *};
use std::{collections::VecDeque, sync::Arc};

fn info(run: Run) -> InfoRecord {
    InfoRecord {
        run,
        compiler: Compiler::Unavailable(Unavailable::NotCaptured),
        closure: Closure::Unavailable(Unavailable::NotCaptured),
    }
}
fn difference(
    left: &[u8],
    right: &[u8],
    offset: usize,
    claim: Option<ftoo_manifest::NodeId>,
    slot: Slot,
    path: &str,
    runs: (Run, Run),
) -> Difference {
    let start = offset.saturating_sub(64);
    let left_end = (offset + 257).min(left.len());
    let right_end = (offset + 257).min(right.len());
    Difference {
        claim,
        slot,
        path: path.into(),
        left: left.into(),
        right: right.into(),
        offset,
        runs: 1,
        left_window: Window {
            start,
            bytes: left[start..left_end].into(),
            eof: left_end == left.len(),
        },
        right_window: Window {
            start,
            bytes: right[start..right_end].into(),
            eof: right_end == right.len(),
        },
        left_info: info(runs.0),
        right_info: info(runs.1),
    }
}
fn comparisons(left: &Produced, right: &Produced, runs: (Run, Run)) -> Vec<Difference> {
    let wire::Verdict::Certified(left_receipt) = &left.dto.outcomes[0].verdict else {
        unreachable!()
    };
    let wire::Verdict::Certified(right_receipt) = &right.dto.outcomes[0].verdict else {
        unreachable!()
    };
    let offset = String::from_utf8(left.bytes.clone())
        .unwrap()
        .find("\"source-hex\":\"")
        .unwrap()
        + "\"source-hex\":\"".len()
        + 1;
    vec![
        difference(
            &left_receipt.source,
            &right_receipt.source,
            0,
            Some(ftoo_manifest::NodeId::LangevinCoefficientOneValue),
            Slot::Source,
            "$.outcomes[0].verdict.receipt.source-hex",
            runs,
        ),
        difference(
            &left.bytes,
            &right.bytes,
            offset,
            None,
            Slot::None,
            "$",
            runs,
        ),
    ]
}
#[test]
fn classifier_never_accepts_an_initial_mismatch_or_publishes_it_to_memo() {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let key = InputKey {
        projection: projection.clone(),
        contract: 1,
        engine: ftoo_manifest::ENGINE_PIN.0.into(),
    };
    let mut original = acceptance::fresh(&key, Run::Fresh1).unwrap();
    original.info = info(Run::Fresh1);
    let original = Arc::new(original);
    let mut altered = acceptance::fresh(&key, Run::Fresh1).unwrap();
    let wire::Verdict::Certified(receipt) = &mut altered.dto.outcomes[0].verdict else {
        unreachable!()
    };
    receipt.source[0] ^= 1;
    altered.bytes = wire::write(&altered.dto).unwrap();
    altered.info = info(Run::Fresh1);
    let altered = Arc::new(altered);
    let file = super::file_tests::FileFixture::new(&original);
    for (retained, first, second, basis) in [
        (&altered, &original, &original, Basis::RetainedAgainstFresh),
        (&original, &altered, &original, Basis::FreshProcesses),
        (&altered, &original, &altered, Basis::FreshProcesses),
    ] {
        let mut memo = VecDeque::new();
        let mut calls = Vec::new();
        let actual = acceptance::accept_with(
            projection.clone(),
            file.write_read(&retained.bytes),
            &mut memo,
            &mut |actual_key, run| {
                assert_eq!(actual_key, &key);
                calls.push(run);
                let mut value = (if run == Run::Fresh1 { first } else { second })
                    .as_ref()
                    .clone();
                value.info = info(run);
                Ok(Arc::new(value))
            },
        )
        .unwrap_err();
        let mut differences = if basis == Basis::FreshProcesses {
            comparisons(first, second, (Run::Fresh1, Run::Fresh2))
        } else {
            Vec::new()
        };
        differences.extend(comparisons(retained, first, (Run::Retained, Run::Fresh1)));
        let expected = RetentionError::NonReproducibility {
            key: Box::new(key.clone()),
            basis,
            retained: retained.bytes.clone(),
            fresh1: first.bytes.clone(),
            fresh2: second.bytes.clone(),
            information: Box::new([info(Run::Retained), info(Run::Fresh1), info(Run::Fresh2)]),
            differences,
        };
        assert_eq!(actual.to_string(), expected_display(&expected));
        assert_eq!(actual, expected);
        assert_eq!(calls, vec![Run::Fresh1, Run::Fresh2]);
        assert_eq!(memo.len(), 0);
        let mut restored_calls = Vec::new();
        let restored = acceptance::accept_with(
            projection.clone(),
            file.write_read(&original.bytes),
            &mut memo,
            &mut |_, run| {
                restored_calls.push(run);
                Ok(Arc::clone(&original))
            },
        )
        .unwrap();
        assert_eq!(restored.bytes(), original.bytes);
        assert_eq!(restored_calls, vec![Run::Fresh1]);
        assert_eq!(memo.len(), 1);
        let reread = acceptance::accept_with(
            projection.clone(),
            file.write_read(&original.bytes),
            &mut memo,
            &mut |_, _| panic!("successful exact-key production is shared"),
        )
        .unwrap();
        assert_eq!(reread.bytes(), original.bytes);
    }
    rerun_failure(&key, &original, &altered);
}

pub(super) fn difference_display(value: &Difference) -> String {
    let label = |run| match run {
        Run::Retained => "retained",
        Run::Fresh1 => "fresh-1",
        Run::Fresh2 => "fresh-2",
    };
    difference_display_labels(
        value,
        label(value.left_info.run),
        label(value.right_info.run),
    )
}
pub(super) fn difference_display_labels(value: &Difference, left: &str, right: &str) -> String {
    let q = |text: &str| serde_json::to_string(text).unwrap();
    let claim = value
        .claim
        .map_or_else(|| "none".into(), |id| q(id.semantic_name()));
    let slot = match value.slot {
        Slot::None => "none",
        Slot::Source => "source",
        Slot::Residual => "residual",
        Slot::Member => "member",
        Slot::Counterstatement => "counterstatement",
    };
    format!(
        "claim={claim} slot={slot} path={} left-length={} right-length={} offset={} runs={} left-window-start={} left-window={} left-eof={} right-window-start={} right-window={} right-eof={} left={left} right={right}",
        q(&value.path),
        value.left.len(),
        value.right.len(),
        value.offset,
        value.runs,
        value.left_window.start,
        q(&wire::hex(&value.left_window.bytes)),
        value.left_window.eof,
        value.right_window.start,
        q(&wire::hex(&value.right_window.bytes)),
        value.right_window.eof
    )
}
pub(super) fn expected_display(expected: &RetentionError) -> String {
    use std::fmt::Write;
    let (mut result, differences, tail) = match expected {
        RetentionError::NonReproducibility {
            basis,
            differences,
            information,
            ..
        } => (
            format!(
                "retention reproduction M061 at json-path \"$\": non-reproducibility basis={}",
                if *basis == Basis::FreshProcesses {
                    "fresh-processes"
                } else {
                    "retained-against-fresh"
                }
            ),
            differences,
            format!(
                "\ninformation=[{},{},{}]",
                information[0].json(),
                information[1].json(),
                information[2].json()
            ),
        ),
        RetentionError::DiagnosticRerunFailed {
            differences, info, ..
        } => (
            "retention reproduction M061 at json-path \"$\": diagnostic-rerun-failed".into(),
            differences,
            format!(
                "\nfault=Execution(Execution {{ claim: LangevinCoefficientOneValue, stage: Projection, cause: ProjectionMismatch }})\ninformation=[{},{},{}]",
                differences[0].left_info.json(),
                differences[0].right_info.json(),
                info.json()
            ),
        ),
        _ => panic!("expected an independently specified classifier result"),
    };
    for difference in differences {
        write!(&mut result, "\n{}", difference_display(difference)).unwrap();
    }
    result.push_str(&tail);
    result
}
fn rerun_failure(key: &InputKey, original: &Arc<Produced>, altered: &Arc<Produced>) {
    let failure = crate::AuditError::Execution(crate::RunError::Execution {
        claim: ftoo_manifest::NodeId::LangevinCoefficientOneValue,
        stage: crate::RunStage::Projection,
        cause: crate::RunCause::ProjectionMismatch,
    });
    let mut memo = VecDeque::new();
    let mut calls = Vec::new();
    let actual = acceptance::accept_with(
        key.projection.clone(),
        original.bytes.clone(),
        &mut memo,
        &mut |_, run| {
            calls.push(run);
            if run == Run::Fresh1 {
                Ok(Arc::clone(altered))
            } else {
                Err(RetentionError::Audit(failure.clone()))
            }
        },
    )
    .unwrap_err();
    let expected = RetentionError::DiagnosticRerunFailed {
        key: Box::new(key.clone()),
        retained: original.bytes.clone(),
        fresh1: altered.bytes.clone(),
        differences: comparisons(original, altered, (Run::Retained, Run::Fresh1)),
        error: Box::new(failure),
        info: Box::new(InfoRecord::collect(Run::Fresh2)),
    };
    assert_eq!(actual.to_string(), expected_display(&expected));
    assert_eq!(actual, expected);
    assert_eq!(calls, vec![Run::Fresh1, Run::Fresh2]);
    assert_eq!(memo.len(), 0);
    let restored = acceptance::accept_with(
        key.projection.clone(),
        original.bytes.clone(),
        &mut memo,
        &mut |_, _| Ok(Arc::clone(original)),
    )
    .unwrap();
    assert_eq!(restored.bytes(), original.bytes);
}

#[test]
fn successful_memo_retains_only_two_complete_exact_input_keys() {
    let mut productions = Vec::new();
    for suffix in ['a', 'b', 'c'] {
        let mut draft = ftoo_manifest::seed_draft();
        let ftoo_manifest::NodePayload::Quantity { description } = &mut draft.nodes[0].payload
        else {
            unreachable!()
        };
        description.push(suffix);
        let projection =
            ftoo_manifest::projection_bytes(&ftoo_manifest::validate(draft).unwrap()).unwrap();
        let key = InputKey {
            projection,
            contract: 1,
            engine: ftoo_manifest::ENGINE_PIN.0.into(),
        };
        productions.push(Arc::new(acceptance::fresh(&key, Run::Fresh1).unwrap()));
    }
    let mut memo = VecDeque::new();
    for production in &productions {
        let mut calls = 0;
        let accepted = acceptance::accept_with(
            production.key.projection.clone(),
            production.bytes.clone(),
            &mut memo,
            &mut |key, run| {
                assert_eq!(key, &production.key);
                assert_eq!(run, Run::Fresh1);
                calls += 1;
                Ok(Arc::clone(production))
            },
        )
        .unwrap();
        assert_eq!(accepted.bytes(), production.bytes);
        assert_eq!(calls, 1);
    }
    assert_eq!(
        memo.iter().map(|entry| &entry.key).collect::<Vec<_>>(),
        vec![&productions[1].key, &productions[2].key]
    );
    let evicted = &productions[0];
    let fixture = super::file_tests::FileFixture::new(evicted);
    let reread = evicted.accept(fixture.write_read(evicted.bytes())).unwrap();
    assert_eq!(reread.bytes(), evicted.bytes());
    assert_eq!(
        reread.report().measurements(),
        evicted.report().measurements()
    );
    assert!(std::ptr::eq(reread.report(), evicted.report()));
    fixture.restore();
    for production in &productions[1..] {
        assert_eq!(
            acceptance::accept_with(
                production.key.projection.clone(),
                production.bytes.clone(),
                &mut memo,
                &mut |_, _| panic!("exact resident key shares complete production")
            )
            .unwrap()
            .bytes(),
            production.bytes
        );
    }
    let mut calls = 0;
    let first = &productions[0];
    assert_eq!(
        acceptance::accept_with(
            first.key.projection.clone(),
            first.bytes.clone(),
            &mut memo,
            &mut |_, _| {
                calls += 1;
                Ok(Arc::clone(first))
            }
        )
        .unwrap()
        .bytes(),
        first.bytes
    );
    assert_eq!(calls, 1);
    assert_eq!(
        memo.iter().map(|entry| &entry.key).collect::<Vec<_>>(),
        vec![&productions[2].key, &productions[0].key]
    );
}
