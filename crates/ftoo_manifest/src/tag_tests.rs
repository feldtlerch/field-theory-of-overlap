use crate::tag_history::seed_history;
use crate::tag_validation::{Candidate, construct, construct_candidate};
use crate::*;
use AddressingFault as F;
use AddressingLocation as L;
use AddressingStage as S;
use HistoryChange as H;
use NodeId::{
    LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
    LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as ThreeValue,
};
use OutOfSliceCause as O;
use TagId::{
    LangevinCoefficientOne as C1, LangevinCoefficientOneValue as V1,
    LangevinCoefficientThree as C3, LangevinCoefficientThreeValue as V3,
};
use std::{collections::BTreeSet, fmt::Write};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn descriptors() -> Vec<TagDescriptor> {
    TagId::ALL.map(TagId::descriptor).to_vec()
}
fn event(ordinal: u32, change: H) -> HistoryEvent {
    HistoryEvent {
        ordinal,
        change,
        authority_record: "rec:test:retirement".into(),
        reason: "Withdrawn for correction.".into(),
    }
}
fn renumber(events: &mut [HistoryEvent]) {
    for (ordinal, e) in (1_u32..).zip(events) {
        e.ordinal = ordinal;
    }
}
fn diagnostic(
    rule: RuleId,
    stage: S,
    location: L,
    tag: Option<TagId>,
    identity: Option<NodeId>,
    fault: F,
) -> AddressingDiagnostic {
    AddressingDiagnostic {
        rule,
        stage,
        location,
        tag,
        identity,
        fault,
    }
}
fn history_fault(
    rule: RuleId,
    index: usize,
    tag: Option<TagId>,
    identity: NodeId,
    fault: F,
) -> AddressingDiagnostic {
    diagnostic(
        rule,
        S::HistoryAdmission,
        L::History(index),
        tag,
        Some(identity),
        fault,
    )
}
fn refuse(
    result: Result<AddressBook<'_>, AddressingErrors>,
    expected: &[AddressingDiagnostic],
    messages: &[&str],
) {
    let errors = result.unwrap_err();
    assert_eq!(errors.diagnostics(), expected);
    assert_eq!(
        errors
            .diagnostics()
            .iter()
            .map(|d| d.rule)
            .collect::<BTreeSet<_>>(),
        expected.iter().map(|d| d.rule).collect()
    );
    assert_eq!(
        errors
            .diagnostics()
            .iter()
            .map(|d| d.fault.to_string())
            .collect::<Vec<_>>(),
        messages
    );
    let mut display = String::new();
    for (d, message) in expected.iter().zip(messages) {
        writeln!(
            display,
            "{:?} {:?} at {:?}: {message}",
            d.rule, d.stage, d.location
        )
        .unwrap();
    }
    assert_eq!(errors.to_string(), display);
}
fn control(graph: &ValidatedManifest) -> TestResult {
    construct(graph, descriptors(), seed_history())?;
    Ok(())
}
fn slice(ids: &[NodeId]) -> ValidatedManifest {
    let mut draft = seed_draft();
    draft.coverage.slice = ids.to_vec();
    validate(draft).unwrap()
}
fn tombstone() -> ValidatedManifest {
    let mut draft = seed_draft();
    draft.nodes.retain(|n| n.id != Three);
    draft
        .edges
        .retain(|e| e.source != Three && e.target != Three);
    draft.coverage.slice.clear();
    // Only graph admission: the unchanged audit roster is deliberately not run.
    validate(draft).unwrap()
}
fn partition(book: &AddressBook<'_>, live: &[TagId], retired: &[TagId], outside: &[(TagId, O)]) {
    assert_eq!(book.live().collect::<Vec<_>>(), live);
    assert_eq!(
        book.retired().map(RetiredEntry::tag).collect::<Vec<_>>(),
        retired
    );
    assert_eq!(
        book.out_of_slice()
            .map(|e| (e.tag(), e.cause()))
            .collect::<Vec<_>>(),
        outside
    );
    let mut all: Vec<_> = book
        .live()
        .chain(book.retired().map(RetiredEntry::tag))
        .chain(book.out_of_slice().map(|e| e.tag()))
        .collect();
    all.sort();
    assert_eq!(all, TagId::ALL);
}

#[test]
fn seed_descriptors_history_resolution_and_snapshot() -> TestResult {
    use RelationKind::{CompatibleWith as C, DependsOn as D};
    let graph = seed()?;
    let snapshot = Snapshot::capture(&graph);
    let statuses: Vec<_> = NodeId::ALL.map(|id| graph.status(id)).into();
    let book = addressing(&graph)?;
    partition(&book, &[C1, C3, V1, V3], &[], &[]);
    let expected = [
        (C1, "c:1", One, "def:langevin:coefficient-one"),
        (C3, "c:3", Three, "def:langevin:coefficient-three"),
        (V1, "v:1", OneValue, "claim:langevin:coefficient-one-value"),
        (
            V3,
            "v:3",
            ThreeValue,
            "claim:langevin:coefficient-three-value",
        ),
    ];
    for (tag, spelling, target, name) in expected {
        assert_eq!(
            (
                tag.descriptor().tag(),
                tag.descriptor().spelling(),
                tag.descriptor().target()
            ),
            (tag, spelling, target)
        );
        let resolved = book.resolve(spelling)?;
        assert_eq!(resolved, target);
        assert_eq!(resolved.semantic_name(), name);
        assert_eq!(graph.node(resolved).unwrap().id, target);
        assert_eq!(
            graph.status(resolved).unwrap().evidence(),
            match target.expected_kind() {
                NodeKind::Quantity => Evidence::NotAClaim,
                NodeKind::Claim => Evidence::Deferred,
            }
        );
    }
    assert_seed_history(&book);
    let edges: BTreeSet<_> = graph
        .edges()
        .iter()
        .map(|e| (e.relation, e.source, e.target, e.guard))
        .collect();
    assert_eq!(
        edges,
        [
            (D, Three, One, Guard::Always),
            (D, OneValue, One, Guard::Always),
            (D, ThreeValue, Three, Guard::Always),
            (D, ThreeValue, OneValue, Guard::Always),
            (C, One, Three, Guard::Always),
            (C, Three, One, Guard::Always)
        ]
        .into()
    );
    assert_eq!(Snapshot::capture(&graph), snapshot);
    assert_eq!(NodeId::ALL.map(|id| graph.status(id)).to_vec(), statuses);
    assert!(std::ptr::eq(book.graph(), &raw const graph));
    Ok(())
}

fn assert_seed_history(book: &AddressBook<'_>) {
    let expected_history = [
        (
            1,
            H::AdmitIdentity {
                identity: One,
                original_formulation:
                    "The coefficient of k in the three-dimensional Langevin expansion.".into(),
            },
            "proposal:manifest:kernel-bite",
            "Retrospective registration of the M1 seed identity.",
        ),
        (
            2,
            H::AdmitIdentity {
                identity: Three,
                original_formulation: "The coefficient of k cubed in the same expansion.".into(),
            },
            "proposal:manifest:kernel-bite",
            "Retrospective registration of the M1 seed identity.",
        ),
        (
            3,
            H::AdmitIdentity {
                identity: OneValue,
                original_formulation: "The first coefficient is exactly 1/3.".into(),
            },
            "proposal:manifest:kernel-bite",
            "Retrospective registration of the M1 seed identity.",
        ),
        (
            4,
            H::AdmitIdentity {
                identity: ThreeValue,
                original_formulation: "The third coefficient is exactly -1/45.".into(),
            },
            "proposal:manifest:kernel-bite",
            "Retrospective registration of the M1 seed identity.",
        ),
        (
            5,
            H::AdmitTag { tag: C1 },
            "proposal:manifest:addressing-lifecycle-bite",
            "The handle addresses the existing seed referent def:langevin:coefficient-one.",
        ),
        (
            6,
            H::AdmitTag { tag: C3 },
            "proposal:manifest:addressing-lifecycle-bite",
            "The handle addresses the existing seed referent def:langevin:coefficient-three.",
        ),
        (
            7,
            H::AdmitTag { tag: V1 },
            "proposal:manifest:addressing-lifecycle-bite",
            "The handle addresses the existing seed referent claim:langevin:coefficient-one-value.",
        ),
        (
            8,
            H::AdmitTag { tag: V3 },
            "proposal:manifest:addressing-lifecycle-bite",
            "The handle addresses the existing seed referent claim:langevin:coefficient-three-value.",
        ),
    ];
    assert_eq!(
        book.history()
            .map(|e| (
                e.ordinal(),
                e.change().clone(),
                e.authority_record(),
                e.reason()
            ))
            .collect::<Vec<_>>(),
        expected_history
    );
}

#[test]
fn m031_spelling_and_duplicate_positions() -> TestResult {
    let graph = seed()?;
    for (spelling, message) in [
        ("", "InvalidTagSpelling { spelling: \"\" }"),
        ("a b", "InvalidTagSpelling { spelling: \"a b\" }"),
        ("a\u{a0}b", "InvalidTagSpelling { spelling: \"a\\u{a0}b\" }"),
        ("a\0b", "InvalidTagSpelling { spelling: \"a\\0b\" }"),
    ] {
        let mut ds = descriptors();
        ds[0].spelling = spelling;
        refuse(
            construct(&graph, ds, seed_history()),
            &[diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(0),
                Some(C1),
                Some(One),
                F::InvalidTagSpelling {
                    spelling: spelling.into(),
                },
            )],
            &[message],
        );
        control(&graph)?;
    }
    let mut ds = descriptors();
    ds[1].spelling = "c:1";
    let fault = F::DuplicateTagSpelling {
        spelling: "c:1".into(),
        positions: vec![0, 1],
    };
    refuse(
        construct(&graph, ds, seed_history()),
        &[
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(0),
                Some(C1),
                Some(One),
                fault.clone(),
            ),
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(1),
                Some(C3),
                Some(Three),
                fault,
            ),
        ],
        &[
            "DuplicateTagSpelling { spelling: \"c:1\", positions: [0, 1] }",
            "DuplicateTagSpelling { spelling: \"c:1\", positions: [0, 1] }",
        ],
    );
    control(&graph)
}

#[test]
fn m032_missing_admissions_and_completeness_suppression() -> TestResult {
    let graph = seed()?;
    let mut hs = seed_history();
    hs.remove(4);
    renumber(&mut hs);
    let missing_tag = diagnostic(
        RuleId::M032,
        S::HistoryAdmission,
        L::Store(C1),
        Some(C1),
        Some(One),
        F::MissingTagAdmission,
    );
    refuse(
        construct(&graph, descriptors(), hs),
        std::slice::from_ref(&missing_tag),
        &["MissingTagAdmission"],
    );
    control(&graph)?;
    let mut hs = seed_history();
    hs.remove(4);
    hs.remove(0);
    renumber(&mut hs);
    refuse(
        construct(&graph, descriptors(), hs),
        &[
            diagnostic(
                RuleId::M032,
                S::HistoryAdmission,
                L::Identity(One),
                None,
                Some(One),
                F::MissingIdentityAdmission,
            ),
            missing_tag,
        ],
        &["MissingIdentityAdmission", "MissingTagAdmission"],
    );
    control(&graph)
}

#[test]
fn m032_duplicates_premature_and_ordinal_suppression() -> TestResult {
    let graph = seed()?;
    for (change, tag, node, fault, message) in [
        (
            seed_history()[0].change.clone(),
            None,
            One,
            F::DuplicateIdentityAdmission,
            "DuplicateIdentityAdmission",
        ),
        (
            H::AdmitTag { tag: C1 },
            Some(C1),
            One,
            F::DuplicateTagAdmission,
            "DuplicateTagAdmission",
        ),
    ] {
        let mut hs = seed_history();
        hs.push(event(9, change));
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(RuleId::M032, 8, tag, node, fault)],
            &[message],
        );
        control(&graph)?;
    }
    let mut hs = seed_history();
    hs.swap(0, 4);
    renumber(&mut hs);
    refuse(
        construct(&graph, descriptors(), hs),
        &[history_fault(
            RuleId::M032,
            0,
            Some(C1),
            One,
            F::TargetNotAdmitted,
        )],
        &["TargetNotAdmitted"],
    );
    control(&graph)?;
    for (ordinal, message) in [
        (0, "InvalidHistoryOrder { expected: 1, actual: 0 }"),
        (2, "InvalidHistoryOrder { expected: 1, actual: 2 }"),
    ] {
        let mut hs = seed_history();
        hs[0].ordinal = ordinal;
        hs.push(event(9, H::RetireIdentity { identity: One }));
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(
                RuleId::M032,
                0,
                None,
                One,
                F::InvalidHistoryOrder {
                    expected: 1,
                    actual: ordinal,
                },
            )],
            &[message],
        );
        control(&graph)?;
    }
    let mut hs = seed_history();
    hs[7].ordinal = 10;
    refuse(
        construct(&graph, descriptors(), hs),
        &[history_fault(
            RuleId::M032,
            7,
            Some(V3),
            ThreeValue,
            F::InvalidHistoryOrder {
                expected: 8,
                actual: 10,
            },
        )],
        &["InvalidHistoryOrder { expected: 8, actual: 10 }"],
    );
    control(&graph)
}

#[test]
fn m033_metadata_and_independent_defects() -> TestResult {
    let graph = seed()?;
    let mut hs = seed_history();
    hs.push(event(9, H::RetireTag { tag: C1 }));
    let good = hs.clone();
    hs[8].reason = " \n\t".into();
    refuse(
        construct(&graph, descriptors(), hs),
        &[history_fault(
            RuleId::M033,
            8,
            Some(C1),
            One,
            F::BlankHistoryReason,
        )],
        &["BlankHistoryReason"],
    );
    construct(&graph, descriptors(), good.clone())?;
    for value in [
        "bad",
        "Rec:test:bad",
        "rec::bad",
        "rec:test:bad:extra",
        "rec:test:bad_name",
        "réc:test:bad",
    ] {
        let mut hs = good.clone();
        hs[8].authority_record = value.into();
        let message = format!("InvalidAuthorityRecord {{ value: {value:?} }}");
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(
                RuleId::M033,
                8,
                Some(C1),
                One,
                F::InvalidAuthorityRecord {
                    value: value.into(),
                },
            )],
            &[&message],
        );
        construct(&graph, descriptors(), good.clone())?;
    }
    let mut hs = seed_history();
    hs[0].change = H::AdmitIdentity {
        identity: One,
        original_formulation: " \u{2003}".into(),
    };
    refuse(
        construct(&graph, descriptors(), hs.clone()),
        &[history_fault(
            RuleId::M033,
            0,
            None,
            One,
            F::BlankOriginalFormulation,
        )],
        &["BlankOriginalFormulation"],
    );
    control(&graph)?;
    // The invalid identity metadata suppresses its dependent tag checks only.
    let mut ds = descriptors();
    ds[1].spelling = "";
    hs.push(event(9, H::RetireTag { tag: C1 }));
    hs.push(event(10, H::RetireTag { tag: V3 }));
    hs[9].reason.clear();
    refuse(
        construct(&graph, ds, hs),
        &[
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(1),
                Some(C3),
                Some(Three),
                F::InvalidTagSpelling {
                    spelling: String::new(),
                },
            ),
            history_fault(RuleId::M033, 0, None, One, F::BlankOriginalFormulation),
            history_fault(RuleId::M033, 9, Some(V3), ThreeValue, F::BlankHistoryReason),
        ],
        &[
            "InvalidTagSpelling { spelling: \"\" }",
            "BlankOriginalFormulation",
            "BlankHistoryReason",
        ],
    );
    control(&graph)
}

#[test]
fn m034_retirement_before_admission_and_duplicates() -> TestResult {
    let graph = seed()?;
    for (change, tag) in [
        (H::RetireIdentity { identity: One }, None),
        (H::RetireTag { tag: C1 }, Some(C1)),
    ] {
        let mut hs = seed_history();
        hs.insert(0, event(1, change));
        renumber(&mut hs);
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(
                RuleId::M034,
                0,
                tag,
                One,
                F::RetirementBeforeAdmission,
            )],
            &["RetirementBeforeAdmission"],
        );
        control(&graph)?;
    }
    let graph = tombstone();
    for (change, tag) in [
        (H::RetireIdentity { identity: Three }, None),
        (H::RetireTag { tag: C3 }, Some(C3)),
    ] {
        let mut hs = seed_history();
        hs.push(event(9, change.clone()));
        construct(&graph, descriptors(), hs.clone())?;
        hs.push(event(10, change));
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(
                RuleId::M034,
                9,
                tag,
                Three,
                F::AlreadyRetired { ordinal: 9 },
            )],
            &["AlreadyRetired { ordinal: 9 }"],
        );
    }
    control(&graph)
}

#[test]
fn m034_terminal_target_and_present_identity() -> TestResult {
    let graph = tombstone();
    let mut hs = seed_history();
    hs.insert(5, event(6, H::RetireIdentity { identity: Three }));
    renumber(&mut hs);
    refuse(
        construct(&graph, descriptors(), hs),
        &[history_fault(
            RuleId::M034,
            6,
            Some(C3),
            Three,
            F::TargetRetired { ordinal: 6 },
        )],
        &["TargetRetired { ordinal: 6 }"],
    );
    control(&graph)?;
    let mut hs = seed_history();
    hs.push(event(9, H::RetireIdentity { identity: Three }));
    construct(&graph, descriptors(), hs.clone())?;
    hs.push(event(10, H::RetireTag { tag: C3 }));
    refuse(
        construct(&graph, descriptors(), hs),
        &[history_fault(
            RuleId::M034,
            9,
            Some(C3),
            Three,
            F::AlreadyRetired { ordinal: 9 },
        )],
        &["AlreadyRetired { ordinal: 9 }"],
    );
    for (retirement, admission, tag, fault, message) in [
        (
            H::RetireIdentity { identity: Three },
            seed_history()[1].change.clone(),
            None,
            F::DuplicateIdentityAdmission,
            "DuplicateIdentityAdmission",
        ),
        (
            H::RetireTag { tag: C3 },
            H::AdmitTag { tag: C3 },
            Some(C3),
            F::DuplicateTagAdmission,
            "DuplicateTagAdmission",
        ),
    ] {
        let mut hs = seed_history();
        hs.push(event(9, retirement));
        construct(&graph, descriptors(), hs.clone())?;
        hs.push(event(10, admission));
        refuse(
            construct(&graph, descriptors(), hs),
            &[history_fault(RuleId::M032, 9, tag, Three, fault)],
            &[message],
        );
    }
    let mut hs = seed_history();
    hs.push(event(9, H::RetireIdentity { identity: Three }));
    let present = seed()?;
    refuse(
        construct(&present, descriptors(), hs.clone()),
        &[diagnostic(
            RuleId::M034,
            S::StoreConstruction,
            L::Identity(Three),
            None,
            Some(Three),
            F::RetiredIdentityPresent { ordinal: 9 },
        )],
        &["RetiredIdentityPresent { ordinal: 9 }"],
    );
    construct(&graph, descriptors(), hs)?;
    control(&present)
}

#[test]
fn m035_partition_multiplicity_and_wrong_support() -> TestResult {
    let graph = slice(&[OneValue]);
    let good: Candidate = vec![
        (C1, StoreState::Live),
        (C3, StoreState::OutOfSlice(O::QuantityOutsideSupport)),
        (V1, StoreState::Live),
        (V3, StoreState::OutOfSlice(O::ClaimNotSelected)),
    ];
    for (candidate, actual) in [
        (good[1..].to_vec(), 0),
        (
            {
                let mut c = good.clone();
                c.push((C1, StoreState::Live));
                c
            },
            2,
        ),
    ] {
        let message = format!("StorePartitionMismatch {{ expected: 1, actual: {actual} }}");
        refuse(
            construct_candidate(&graph, descriptors(), seed_history(), Some(candidate)),
            &[diagnostic(
                RuleId::M035,
                S::StoreConstruction,
                L::Store(C1),
                Some(C1),
                Some(One),
                F::StorePartitionMismatch {
                    expected: 1,
                    actual,
                },
            )],
            &[&message],
        );
        construct_candidate(&graph, descriptors(), seed_history(), Some(good.clone()))?;
    }
    let mut wrong = good.clone();
    wrong[1].1 = StoreState::Live;
    refuse(
        construct_candidate(&graph, descriptors(), seed_history(), Some(wrong)),
        &[diagnostic(
            RuleId::M035,
            S::StoreConstruction,
            L::Store(C3),
            Some(C3),
            Some(Three),
            F::StoreStateMismatch {
                expected: StoreState::OutOfSlice(O::QuantityOutsideSupport),
                actual: StoreState::Live,
            },
        )],
        &["StoreStateMismatch { expected: OutOfSlice(QuantityOutsideSupport), actual: Live }"],
    );
    construct_candidate(&graph, descriptors(), seed_history(), Some(good))?;
    Ok(())
}

#[test]
fn m036_identity_without_tag_catalog_half() -> TestResult {
    let graph = seed()?;
    let mut ds = descriptors();
    ds[1].target = One;
    refuse(
        construct(&graph, ds, seed_history()),
        &[diagnostic(
            RuleId::M036,
            S::HistoryAdmission,
            L::Identity(Three),
            None,
            Some(Three),
            F::IdentityWithoutTag,
        )],
        &["IdentityWithoutTag"],
    );
    control(&graph)
}

fn resolution(book: &AddressBook<'_>, input: &str, expected: TagResolutionFault, message: &str) {
    let result = book.resolve(input);
    assert_eq!(result.as_ref().unwrap_err().to_string(), message);
    assert_eq!(result, Err(expected));
}
fn outside(
    book: &AddressBook<'_>,
    input: &str,
    tag: TagId,
    target: NodeId,
    cause: O,
    message: &str,
) {
    resolution(
        book,
        input,
        TagResolutionFault::OutOfSliceTag {
            input: input.into(),
            tag,
            target,
            cause,
        },
        message,
    );
}

#[test]
fn resolution_unknown_exact_inputs() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    for (input, message) in [
        ("c:01", "tag \"c:01\": unknown"),
        ("C:1", "tag \"C:1\": unknown"),
        (" c:1 ", "tag \" c:1 \": unknown"),
        ("", "tag \"\": unknown"),
        ("x:1", "tag \"x:1\": unknown"),
        ("c:1\n\0", "tag \"c:1\\n\\0\": unknown"),
        (
            "def:langevin:coefficient-one",
            "tag \"def:langevin:coefficient-one\": unknown",
        ),
    ] {
        resolution(
            &book,
            input,
            TagResolutionFault::UnknownTag {
                input: input.into(),
            },
            message,
        );
        assert_eq!(book.resolve("c:1")?, One);
    }
    let owned = {
        let input = String::from("unregistered");
        book.resolve(&input).unwrap_err()
    };
    assert_eq!(
        owned,
        TagResolutionFault::UnknownTag {
            input: "unregistered".into()
        }
    );
    assert_eq!(owned.to_string(), "tag \"unregistered\": unknown");
    Ok(())
}

#[test]
fn resolution_opaque_unicode_and_misleading_prefix() -> TestResult {
    let graph = seed()?;
    let mut ds = descriptors();
    ds[0].spelling = "v:quantity!?";
    ds[1].spelling = "é";
    ds[2].spelling = "e\u{301}";
    ds[3].spelling = "λ→值";
    let book = construct(&graph, ds, seed_history())?;
    for (input, target) in [
        ("v:quantity!?", One),
        ("é", Three),
        ("e\u{301}", OneValue),
        ("λ→值", ThreeValue),
    ] {
        assert_eq!(book.resolve(input)?, target);
    }
    resolution(
        &book,
        "v:quantity",
        TagResolutionFault::UnknownTag {
            input: "v:quantity".into(),
        },
        "tag \"v:quantity\": unknown",
    );
    assert_eq!(book.resolve("v:quantity!?")?, One);
    assert_eq!(book.live().collect::<Vec<_>>(), [V1, C1, C3, V3]);
    Ok(())
}

#[test]
fn partitions_claim_one_empty_absent_and_transitive_support() -> TestResult {
    let graph = slice(&[OneValue]);
    let book = addressing(&graph)?;
    partition(
        &book,
        &[C1, V1],
        &[],
        &[(C3, O::QuantityOutsideSupport), (V3, O::ClaimNotSelected)],
    );
    outside(
        &book,
        "c:3",
        C3,
        Three,
        O::QuantityOutsideSupport,
        "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause quantity-outside-support",
    );
    outside(
        &book,
        "v:3",
        V3,
        ThreeValue,
        O::ClaimNotSelected,
        "tag \"v:3\": out of slice; label claim:langevin:coefficient-three-value; cause claim-not-selected",
    );
    let empty = slice(&[]);
    let book = addressing(&empty)?;
    partition(
        &book,
        &[],
        &[],
        &[
            (C1, O::QuantityOutsideSupport),
            (C3, O::QuantityOutsideSupport),
            (V1, O::ClaimNotSelected),
            (V3, O::ClaimNotSelected),
        ],
    );
    let absent = tombstone();
    let book = addressing(&absent)?;
    partition(
        &book,
        &[],
        &[],
        &[
            (C1, O::QuantityOutsideSupport),
            (C3, O::TargetAbsent),
            (V1, O::ClaimNotSelected),
            (V3, O::ClaimNotSelected),
        ],
    );
    outside(
        &book,
        "c:3",
        C3,
        Three,
        O::TargetAbsent,
        "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause target-absent",
    );
    // Structurally admitted, but M3 independently refuses its missing selected
    // claim premise. Traversal does not promote that unselected claim to live.
    let mut draft = seed_draft();
    draft.coverage.slice = vec![ThreeValue];
    draft
        .edges
        .retain(|e| !(e.source == ThreeValue && e.target == OneValue));
    let transitive = validate(draft)?;
    let book = addressing(&transitive)?;
    partition(&book, &[C1, C3, V3], &[], &[(V1, O::ClaimNotSelected)]);
    let selected_three = slice(&[ThreeValue]);
    partition(
        &addressing(&selected_three)?,
        &[C1, C3, V3],
        &[],
        &[(V1, O::ClaimNotSelected)],
    );
    let restored = seed()?;
    let book = addressing(&restored)?;
    assert_eq!(book.resolve("c:3")?, Three);
    assert_eq!(book.resolve("v:3")?, ThreeValue);
    Ok(())
}

#[test]
fn retirement_stores_and_all_terminal_causes() -> TestResult {
    let full = seed()?;
    let one = slice(&[OneValue]);
    for graph in [&full, &one] {
        let mut hs = seed_history();
        hs.push(event(9, H::RetireTag { tag: C3 }));
        let book = construct(graph, descriptors(), hs)?;
        resolution(
            &book,
            "c:3",
            TagResolutionFault::RetiredTag {
                input: "c:3".into(),
                tag: C3,
                target: Three,
                cause: RetirementCause::TagRetirement(event(9, H::RetireTag { tag: C3 })),
            },
            "tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority rec:test:retirement; reason \"Withdrawn for correction.\"",
        );
        assert_eq!(book.retired().count(), 1);
        control(graph)?;
    }
    let graph = tombstone();
    let mut hs = seed_history();
    hs.push(event(9, H::RetireIdentity { identity: Three }));
    let book = construct(&graph, descriptors(), hs)?;
    partition(
        &book,
        &[],
        &[C3],
        &[
            (C1, O::QuantityOutsideSupport),
            (V1, O::ClaimNotSelected),
            (V3, O::ClaimNotSelected),
        ],
    );
    resolution(
        &book,
        "c:3",
        TagResolutionFault::RetiredTag {
            input: "c:3".into(),
            tag: C3,
            target: Three,
            cause: RetirementCause::IdentityRetirement(event(
                9,
                H::RetireIdentity { identity: Three },
            )),
        },
        "tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority rec:test:retirement; reason \"Withdrawn for correction.\"",
    );
    let mut hs = seed_history();
    hs.push(event(9, H::RetireTag { tag: C3 }));
    hs.push(event(10, H::RetireIdentity { identity: Three }));
    let book = construct(&graph, descriptors(), hs)?;
    resolution(
        &book,
        "c:3",
        TagResolutionFault::RetiredTag {
            input: "c:3".into(),
            tag: C3,
            target: Three,
            cause: RetirementCause::TagRetirement(event(9, H::RetireTag { tag: C3 })),
        },
        "tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority rec:test:retirement; reason \"Withdrawn for correction.\"",
    );
    assert_eq!(book.history().count(), 10);
    assert_eq!(
        book.history().last().unwrap().change(),
        &H::RetireIdentity { identity: Three }
    );
    control(&graph)
}

#[test]
fn reversed_inputs_keep_semantics_and_original_locations() -> TestResult {
    let graph = seed()?;
    let mut ds = descriptors();
    ds.reverse();
    let mut hs = seed_history();
    hs.reverse();
    let book = construct(&graph, ds.clone(), hs.clone())?;
    partition(&book, &[C1, C3, V1, V3], &[], &[]);
    assert_eq!(book.history().cloned().collect::<Vec<_>>(), seed_history());
    ds[3].spelling = "";
    hs[0].reason.clear();
    refuse(
        construct(&graph, ds, hs),
        &[
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(3),
                Some(C1),
                Some(One),
                F::InvalidTagSpelling {
                    spelling: String::new(),
                },
            ),
            history_fault(RuleId::M033, 0, Some(V3), ThreeValue, F::BlankHistoryReason),
        ],
        &[
            "InvalidTagSpelling { spelling: \"\" }",
            "BlankHistoryReason",
        ],
    );
    control(&graph)
}

#[test]
fn awaiting_claim_keeps_its_audit_blocker_outside_addressing_slice() -> TestResult {
    for (awaiting, expected) in [
        (Awaiting::AuditBinding, DeferredReason::AuditBinding),
        (
            Awaiting::Capability(CapabilityId::ExactRationalLinearEquivalence),
            DeferredReason::Capability(CapabilityId::ExactRationalLinearEquivalence),
        ),
    ] {
        let mut draft = seed_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        for node in &mut draft.nodes {
            if node.id == OneValue
                && let NodePayload::Claim { audit, .. } = &mut node.payload
            {
                *audit = ClaimAudit::Awaiting(awaiting);
            }
        }
        let graph = validate(draft)?;
        let before = Snapshot::capture(&graph);
        let statuses = NodeId::ALL.map(|id| graph.status(id));
        let book = addressing(&graph)?;
        outside(
            &book,
            "v:1",
            V1,
            OneValue,
            O::ClaimNotSelected,
            "tag \"v:1\": out of slice; label claim:langevin:coefficient-one-value; cause claim-not-selected",
        );
        assert_eq!(
            expected_deferral(&graph, graph.node(OneValue).unwrap(), &[]),
            Some(expected)
        );
        assert_eq!(
            graph.status(OneValue).unwrap().evidence(),
            Evidence::Deferred
        );
        assert_eq!(Snapshot::capture(&graph), before);
        assert_eq!(NodeId::ALL.map(|id| graph.status(id)), statuses);
    }
    let graph = seed()?;
    assert_eq!(addressing(&graph)?.resolve("v:1")?, OneValue);
    Ok(())
}

#[test]
fn m031_retired_descriptors_still_require_valid_spelling() -> TestResult {
    let graph = seed()?;
    let mut hs = seed_history();
    hs.push(event(9, H::RetireTag { tag: C1 }));
    let mut ds = descriptors();
    ds[0].spelling = "";
    refuse(
        construct(&graph, ds, hs.clone()),
        &[diagnostic(
            RuleId::M031,
            S::CatalogAdmission,
            L::Descriptor(0),
            Some(C1),
            Some(One),
            F::InvalidTagSpelling {
                spelling: String::new(),
            },
        )],
        &["InvalidTagSpelling { spelling: \"\" }"],
    );
    construct(&graph, descriptors(), hs)?;
    Ok(())
}

#[test]
fn reversed_duplicate_spellings_sort_by_identity_before_location() -> TestResult {
    let graph = seed()?;
    let mut ds = descriptors();
    ds[1].spelling = "c:1";
    ds.reverse();
    let fault = F::DuplicateTagSpelling {
        spelling: "c:1".into(),
        positions: vec![2, 3],
    };
    refuse(
        construct(&graph, ds, seed_history()),
        &[
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(3),
                Some(C1),
                Some(One),
                fault.clone(),
            ),
            diagnostic(
                RuleId::M031,
                S::CatalogAdmission,
                L::Descriptor(2),
                Some(C3),
                Some(Three),
                fault,
            ),
        ],
        &[
            "DuplicateTagSpelling { spelling: \"c:1\", positions: [2, 3] }",
            "DuplicateTagSpelling { spelling: \"c:1\", positions: [2, 3] }",
        ],
    );
    control(&graph)
}

#[test]
fn m4_register_statements_scopes_and_consumer_inventory() {
    let expected = [
        (
            RuleId::M030,
            "Tag targets and spellings are fixed; no lifecycle operation can rebind or rename a tag or reactivate a retired tag.",
            "Every public addressing value and operation.",
            RuleState::Structural,
        ),
        (
            RuleId::M031,
            "Tag spellings are nonempty, contain no whitespace or control character, and are unique by exact UTF-8 bytes.",
            "CatalogAdmission over all descriptors including retired handles.",
            RuleState::Enforced,
        ),
        (
            RuleId::M032,
            "History admits every permanent identity and tag exactly once, in a contiguous event order, with identity admission before tag admission.",
            "HistoryAdmission over the whole ledger, not just present nodes.",
            RuleState::Enforced,
        ),
        (
            RuleId::M033,
            "History records carry nonblank reasons, valid authority-record labels and nonblank original identity formulations.",
            "HistoryAdmission for every event's metadata.",
            RuleState::Enforced,
        ),
        (
            RuleId::M034,
            "Retirement is terminal, follows admission, and a retired identity cannot remain in the addressed graph.",
            "HistoryAdmission and StoreConstruction.",
            RuleState::Enforced,
        ),
        (
            RuleId::M035,
            "The live, retired and out-of-slice stores partition every admitted tag according to retirement and declared-slice dependency support.",
            "StoreConstruction over the full catalog.",
            RuleState::Enforced,
        ),
        (
            RuleId::M036,
            "Every permanent identity has a tag, and resolution returns its bound NodeId only for a live exact handle, otherwise its exact named refusal.",
            "Completed catalog coverage and the resolution consumer.",
            RuleState::Enforced,
        ),
    ];
    assert_eq!(
        RULES[29..36]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state))
            .collect::<Vec<_>>(),
        expected
    );
    assert!(RULES[29..36].iter().all(|r| r.deferred_reason.is_none()));
    // M036 resolution has no RuleId diagnostic. Only its catalog half does.
    let diagnostic_consumers: [(RuleId, fn() -> TestResult); 6] = [
        (RuleId::M031, m031_spelling_and_duplicate_positions),
        (
            RuleId::M032,
            m032_missing_admissions_and_completeness_suppression,
        ),
        (RuleId::M033, m033_metadata_and_independent_defects),
        (RuleId::M034, m034_terminal_target_and_present_identity),
        (RuleId::M035, m035_partition_multiplicity_and_wrong_support),
        (RuleId::M036, m036_identity_without_tag_catalog_half),
    ];
    assert_eq!(
        diagnostic_consumers.map(|(id, _)| id),
        [
            RuleId::M031,
            RuleId::M032,
            RuleId::M033,
            RuleId::M034,
            RuleId::M035,
            RuleId::M036
        ]
    );
}

#[test]
fn predecessor_register_m001_m015_statements_and_scopes_remain_literal() {
    let expected = [
        ("Input is valid UTF-8 without a BOM.", "Every byte load."),
        (
            "Decoded object keys are unique and input contains exactly one document.",
            "Every object at every depth.",
        ),
        (
            "Numeric tokens are integers and the wire schema is exactly integer 1.",
            "Every numeric token at the byte boundary.",
        ),
        (
            "Wire fields have exact required shapes and exclusive variant fields.",
            "Every wire object and scalar.",
        ),
        (
            "Identifiers, kinds, relations, guards, status, binding, capabilities, budgets, verdict classifications and Unconditional routes are closed; payload determines kind.",
            "Every admitted Rust value.",
        ),
        (
            "Node IDs are unique, payload kinds match identity, and descriptions are nonblank.",
            "Every draft node.",
        ),
        (
            "Every edge endpoint and coverage member resolves to a present node.",
            "All references regardless of closure flags.",
        ),
        (
            "Endpoint kinds satisfy the relation's own permitted pair table.",
            "Edges with resolved, valid nodes.",
        ),
        (
            "Edge tuples are unique and self edges obey the relation policy.",
            "Every directed (relation, source, target, guard) record.",
        ),
        (
            "Each acyclic relation has no cycle in its entire subgraph.",
            "Edges passing endpoint and self-edge checks, including nodes outside the slice; multiplicity does not affect connectivity.",
        ),
        (
            "Only maturity is authored; base status derives from identity/payload and AuditedStatus only through private checked-report construction.",
            "Base and audited status APIs.",
        ),
        (
            "Assertions equal their surface's derivation: unaudited projection or replay-checked audited report.",
            "Every loaded status assertion.",
        ),
        (
            "Every declared slice member is unique and is a claim; M007 resolves membership before this shape check.",
            "Resolved members; shape precedes eligibility.",
        ),
        (
            "GraphAdmission requires each slice claim Bound; RegistryAdmission requires transitive claim premises selected; ReportConsumption requires reached IDs equal the declared slice.",
            "GraphAdmission, RegistryAdmission and ReportConsumption.",
        ),
        (
            "Audit closure true requires every claim Bound to its implementation; ExplicitPartial and Complete satisfy the stated report predicates, including nonvacuity.",
            "Every manifest, including an empty graph.",
        ),
    ];
    assert_eq!(
        RULES[..15]
            .iter()
            .map(|r| (r.statement, r.scope))
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn predecessor_register_m016_m029_statements_and_scopes_remain_literal() {
    let expected = [
        (
            "Shared weighting compares paths of the same kind only.",
            "Holding and covector payloads carrying weighting paths.",
        ),
        (
            "Reads-through is required only for field-anchored readings.",
            "Field readings, not latent readings.",
        ),
        (
            "Converters satisfy their declared input, conversion and output.",
            "Converter declarations, not inferred derivation contracts.",
        ),
        (
            "Indexed-space families and operations satisfy family-level typing at their node index.",
            "Every admitted indexed family and operation.",
        ),
        (
            "Every Bound claim has exactly its designated implementation and every implementation exactly one present Bound claim; admitted contracts pass their numbered consumers.",
            "Bound claims and the executing registry.",
        ),
        (
            "Every symmetric edge has its reverse with the same relation and guard.",
            "Edges passing M007, M008 and self-edge policy; compatibility only.",
        ),
        (
            "Fixed producers have exactly their declared dependencies; Certified premises unlock execution and target witnesses contain full coefficient dependencies.",
            "Registry and execution consumers.",
        ),
        (
            "Every Bound claim has its designated reviewed budget; all records are used, pin/profile agree and claim work shares one budget.",
            "Registry and execution consumers.",
        ),
        (
            "Supported means adapter-implemented at this pin solely from the table; unsupported requirements never execute and Awaiting supported contracts is refused.",
            "Registry and execution consumers.",
        ),
        (
            "Exactly one outcome exists per claim, none for quantities; claim, producer, revision, pin, budget and semantic snapshot match.",
            "Report domain and semantic provenance.",
        ),
        (
            "Certified/Refuted consume matching replay-checked evidence; only typed limits yield exhaustion and invalid construction never refutes.",
            "Private receipt construction and verdict interpretation.",
        ),
        (
            "Each execution has a fresh store and one budget; gated evidence is independent of measurements and previous caches.",
            "Fresh execution and complete gated report reproducibility.",
        ),
        (
            "Every unselected or blocked claim has its exact derived deferral; uncertified prerequisites cannot promote dependents.",
            "Total report blockers and Certified-premise scheduling.",
        ),
        (
            "Alternative routes execute alone and combine by a reviewed rule.",
            "Registry and execution consumers.",
        ),
    ];
    assert_eq!(
        RULES[15..29]
            .iter()
            .map(|r| (r.statement, r.scope))
            .collect::<Vec<_>>(),
        expected
    );
}
