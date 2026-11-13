use super::{acceptance, matrix_tests::expected_difference, wire, *};
use std::{
    collections::VecDeque,
    fs,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
pub(super) fn reject(
    production: &Arc<Produced>,
    changed: &wire::Report,
    mut differences: Vec<Difference>,
) {
    let path = std::env::temp_dir().join(format!(
        "retention-verdict-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, &production.bytes).unwrap();
    let mut memo = VecDeque::new();
    let before = acceptance::accept_with(
        production.key.projection.clone(),
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, _| Ok(Arc::clone(production)),
    )
    .unwrap();
    assert_eq!(before.bytes(), production.bytes);
    let retained = wire::write(changed).unwrap();
    fs::write(&path, &retained).unwrap();
    let mut calls = 0;
    let mut second = production.as_ref().clone();
    second.info.run = Run::Fresh2;
    let second = Arc::new(second);
    let actual = acceptance::accept_with(
        production.key.projection.clone(),
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, run| {
            assert_eq!(run, Run::Fresh2);
            calls += 1;
            Ok(Arc::clone(&second))
        },
    )
    .unwrap_err();
    differences.push(expected_difference(
        &retained,
        &production.bytes,
        InfoRecord::collect(Run::Retained),
        production.info.clone(),
    ));
    let expected = RetentionError::NonReproducibility {
        key: Box::new(production.key.clone()),
        basis: Basis::RetainedAgainstFresh,
        retained,
        fresh1: production.bytes.clone(),
        fresh2: production.bytes.clone(),
        information: Box::new([
            InfoRecord::collect(Run::Retained),
            production.info.clone(),
            second.info.clone(),
        ]),
        differences,
    };
    assert_eq!(
        actual.to_string(),
        super::classifier_tests::expected_display(&expected)
    );
    assert_eq!(actual, expected);
    assert_eq!(calls, 1);
    fs::write(&path, &production.bytes).unwrap();
    let restored = acceptance::accept_with(
        production.key.projection.clone(),
        fs::read(&path).unwrap(),
        &mut memo,
        &mut |_, _| panic!("restored production is reused"),
    )
    .unwrap();
    assert_eq!(restored.bytes(), production.bytes);
    fs::remove_file(path).unwrap();
}
fn named(
    production: &Produced,
    index: usize,
    path: &str,
    slot: Slot,
    left: &[u8],
    right: &[u8],
) -> Difference {
    let mut result = expected_difference(
        left,
        right,
        InfoRecord::collect(Run::Retained),
        production.info.clone(),
    );
    result.claim = Some(production.dto.outcomes[index].claim);
    result.path = format!("$.outcomes[{index}].verdict.{path}");
    result.slot = slot;
    result
}
pub(super) fn matrix(production: &Arc<Produced>) {
    for index in 0..production.dto.outcomes.len() {
        let mut changed = production.dto.clone();
        match &mut changed.outcomes[index].verdict {
            wire::Verdict::Certified(receipt) | wire::Verdict::Refuted { receipt, .. } => {
                let source = receipt.source.clone();
                let residual = receipt.residual.clone();
                std::mem::swap(&mut receipt.source, &mut receipt.residual);
                reject(
                    production,
                    &changed,
                    vec![
                        named(
                            production,
                            index,
                            "receipt.source-hex",
                            Slot::Source,
                            &residual,
                            &source,
                        ),
                        named(
                            production,
                            index,
                            "receipt.residual-hex",
                            Slot::Residual,
                            &source,
                            &residual,
                        ),
                    ],
                );
            }
            wire::Verdict::Undecided { .. } => exhaustion(production, index),
            wire::Verdict::Deferred(_) => (),
        }
        let mut changed = production.dto.clone();
        if let wire::Verdict::Refuted {
            counterstatement, ..
        } = &mut changed.outcomes[index].verdict
        {
            let original = serde_json::to_string(&counterstatement.claimed.numerator).unwrap();
            counterstatement.claimed.numerator = "999".into();
            reject(
                production,
                &changed,
                vec![named(
                    production,
                    index,
                    "counterstatement.claimed.numerator",
                    Slot::None,
                    b"\"999\"",
                    original.as_bytes(),
                )],
            );
        }
    }
}
fn exhaustion(production: &Arc<Produced>, index: usize) {
    for mutation in 0..4 {
        let mut changed = production.dto.clone();
        let wire::Verdict::Undecided { exhaustion, .. } = &mut changed.outcomes[index].verdict
        else {
            unreachable!()
        };
        let (field, original, altered) = match mutation {
            0 => {
                let original = exhaustion.limit.to_string();
                exhaustion.limit += 1;
                ("limit", original, exhaustion.limit.to_string())
            }
            1 => {
                let original = exhaustion.attempted.to_string();
                exhaustion.attempted += 1;
                ("attempted", original, exhaustion.attempted.to_string())
            }
            2 => {
                let original = serde_json::to_string(&exhaustion.location.logical_id).unwrap();
                exhaustion.location.logical_id = Some(999);
                ("location.logical-id", original, "999".into())
            }
            3 => {
                let original = serde_json::to_string(&exhaustion.location.operation).unwrap();
                exhaustion.location.operation.push('!');
                (
                    "location.operation",
                    original,
                    serde_json::to_string(&exhaustion.location.operation).unwrap(),
                )
            }
            _ => unreachable!(),
        };
        reject(
            production,
            &changed,
            vec![named(
                production,
                index,
                &format!("exhaustion.{field}"),
                Slot::None,
                altered.as_bytes(),
                original.as_bytes(),
            )],
        );
    }
}

#[test]
fn identical_seed_exports_do_not_gain_a_false_claim_swap_refusal() {
    let projection = ftoo_manifest::projection_bytes(&ftoo_manifest::seed().unwrap()).unwrap();
    let production = produce(projection.clone()).unwrap();
    let mut changed = production.dto.clone();
    let (first, rest) = changed.outcomes.split_at_mut(1);
    let wire::Verdict::Certified(a) = &mut first[0].verdict else {
        unreachable!()
    };
    let wire::Verdict::Certified(b) = &mut rest[0].verdict else {
        unreachable!()
    };
    assert_eq!(a.source, b.source);
    assert_eq!(a.residual, b.residual);
    assert_eq!(a.member, b.member);
    std::mem::swap(&mut a.source, &mut b.source);
    std::mem::swap(&mut a.residual, &mut b.residual);
    std::mem::swap(&mut a.member, &mut b.member);
    let bytes = wire::write(&changed).unwrap();
    assert_eq!(bytes, production.bytes);
    assert_eq!(accept(projection, bytes).unwrap().bytes(), production.bytes);
}
