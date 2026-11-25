use super::*;
use crate::{
    CheckedMarkdown, JointMints, LabelFault, MarkdownSource, MintKind, PathPart, SourceAnchor,
    check_markdown_sources, collect_joint_mints,
};

const FILE: &str = "registry/open-questions.toml";
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn site(path: Vec<PathPart>) -> UnitSite {
    UnitSite {
        file: FILE.into(),
        path,
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    }
}
fn at(index: usize, key: &str) -> UnitSite {
    site(vec![
        PathPart::Key("question".into()),
        PathPart::Index(index),
        PathPart::Key(key.into()),
    ])
}
fn text(index: usize, key: &str, value: &str) -> LocatedString {
    LocatedString {
        value: value.into(),
        site: at(index, key),
    }
}
fn reference(index: usize, key: &str, kind: ReferenceKind, value: &str) -> ReferenceDraft {
    ReferenceDraft {
        kind,
        value: text(index, key, value),
    }
}
fn draft(index: usize, slug: &str) -> OpenQuestionDraft {
    OpenQuestionDraft {
        site: site(vec![
            PathPart::Key("question".into()),
            PathPart::Index(index),
        ]),
        label: text(index, "label", &format!("open:test:{slug}")),
        title: text(index, "title", &format!("the {slug} subject")),
        concern: QuestionConcern::Theory,
        question: text(
            index,
            "question",
            &format!("Is the {slug} subject settled?"),
        ),
        about: vec![reference(
            index,
            "about",
            ReferenceKind::Node,
            "def:langevin:coefficient-one",
        )],
        settled_when: text(index, "settled-when", "A derivation answers it."),
        blocks: vec![],
        rests_on: vec![],
        state: StateDraft::Open,
    }
}
fn store_of(slugs: &[&str]) -> Vec<OpenQuestionDraft> {
    slugs
        .iter()
        .enumerate()
        .map(|(index, slug)| draft(index, slug))
        .collect()
}
fn domains() -> QuestionDomains {
    QuestionDomains {
        records: ["dec:paper:authored-latex".to_owned()].into(),
        units: ["stmt:test:unit".to_owned()].into(),
        paper: ["sem:claim:release-knee".to_owned(), "tbl:debts".to_owned()].into(),
        files: ["docs/glossary.md".to_owned()].into(),
    }
}
fn refused(drafts: Vec<OpenQuestionDraft>) -> Vec<(RuleId, QuestionFault)> {
    admit_open_questions(FILE, drafts, &domains())
        .expect_err("refused")
        .diagnostics()
        .iter()
        .map(|d| (d.rule, d.fault.clone()))
        .collect()
}
fn settled(index: usize, by: Vec<ReferenceDraft>) -> StateDraft {
    StateDraft::Settled {
        by,
        answer: text(index, "answer", "It is settled."),
    }
}

#[test]
fn closed_vocabularies_round_trip_their_names() {
    for concern in QuestionConcern::ALL {
        assert_eq!(QuestionConcern::parse(concern.name()), Some(concern));
    }
    for kind in ReferenceKind::ALL {
        assert_eq!(ReferenceKind::parse(kind.name()), Some(kind));
    }
    for state in QuestionStateKind::ALL {
        assert_eq!(QuestionStateKind::parse(state.name()), Some(state));
    }
    assert_eq!(QuestionConcern::parse("Theory"), None);
    assert_eq!(ReferenceKind::parse("url"), None);
    assert_eq!(QuestionStateKind::parse("reopened"), None);
}
#[test]
fn m066_every_closed_state_is_admitted_only_with_its_content() -> TestResult {
    let mut drafts = store_of(&["a-settled", "b-succeeded", "c-withdrawn", "d-open"]);
    drafts[0].state = settled(
        0,
        vec![reference(0, "settled-by", ReferenceKind::Rule, "M065")],
    );
    drafts[1].state = StateDraft::Succeeded {
        by: vec![reference(
            1,
            "succeeded-by",
            ReferenceKind::Open,
            "open:test:d-open",
        )],
        answer: None,
    };
    drafts[2].state = StateDraft::Withdrawn {
        because: text(2, "withdrawn-because", "It rests on a false premise."),
    };
    let store = admit_open_questions(FILE, drafts, &domains())?;
    let states: Vec<_> = store.records().iter().map(|r| r.state().clone()).collect();
    let QuestionState::Settled(settlement) = &states[0] else {
        return Err("settled".into());
    };
    assert_eq!(settlement.by(), [QuestionReference::Rule(RuleId::M065)]);
    assert_eq!(settlement.answer(), "It is settled.");
    let QuestionState::Succeeded(succession) = &states[1] else {
        return Err("succeeded".into());
    };
    assert_eq!(
        succession.by(),
        [QuestionReference::Open("open:test:d-open".into())]
    );
    assert_eq!(succession.answer(), None);
    let QuestionState::Withdrawn(withdrawal) = &states[2] else {
        return Err("withdrawn".into());
    };
    assert_eq!(withdrawal.because(), "It rests on a false premise.");
    assert_eq!(states[3], QuestionState::Open);
    assert_eq!(store.file(), FILE);
    Ok(())
}
#[test]
fn every_reference_kind_resolves_in_its_own_domain() -> TestResult {
    let mut drafts = store_of(&["a", "b"]);
    drafts[0].about = vec![
        reference(
            0,
            "about",
            ReferenceKind::Node,
            "claim:langevin:coefficient-one-value",
        ),
        reference(0, "about", ReferenceKind::Rule, "M001"),
        reference(0, "about", ReferenceKind::Unit, "stmt:test:unit"),
        reference(
            0,
            "about",
            ReferenceKind::Record,
            "dec:paper:authored-latex",
        ),
        reference(0, "about", ReferenceKind::Paper, "tbl:debts"),
        reference(0, "about", ReferenceKind::File, "docs/glossary.md"),
        reference(0, "about", ReferenceKind::Open, "open:test:b"),
    ];
    drafts[0].rests_on = vec![text(0, "rests-on", "open:test:b")];
    let store = admit_open_questions(FILE, drafts, &domains())?;
    assert_eq!(
        store.records()[0].about(),
        [
            QuestionReference::Node(NodeId::LangevinCoefficientOneValue),
            QuestionReference::Rule(RuleId::M001),
            QuestionReference::Unit("stmt:test:unit".into()),
            QuestionReference::Record("dec:paper:authored-latex".into()),
            QuestionReference::Paper("tbl:debts".into()),
            QuestionReference::File("docs/glossary.md".into()),
            QuestionReference::Open("open:test:b".into()),
        ]
    );
    assert_eq!(store.records()[0].rests_on(), ["open:test:b"]);
    Ok(())
}

// M067: store shape, labels and texts.
#[test]
fn m067_label_without_three_components_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].label.value = "open:test".into();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::InvalidLabel {
                label: "open:test".into()
            }
        )]
    );
}
#[test]
fn m067_label_of_another_kind_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].label.value = "rec:test:a".into();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::InvalidLabel {
                label: "rec:test:a".into()
            }
        )]
    );
}
#[test]
fn m067_duplicate_label_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[1].label.value = "open:test:a".into();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::DuplicateLabel {
                label: "open:test:a".into(),
                first: 0
            }
        )]
    );
}
#[test]
fn m067_labels_out_of_order_refuse() {
    let drafts = store_of(&["b", "a"]);
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::LabelOrder {
                label: "open:test:a".into(),
                previous: "open:test:b".into()
            }
        )]
    );
}
fn text_fault(text: QuestionText, value: &str, reason: TextReason) -> (RuleId, QuestionFault) {
    (
        RuleId::M067,
        QuestionFault::Text {
            text,
            value: value.into(),
            reason,
        },
    )
}
#[test]
fn m067_each_text_fault_refuses() {
    let cases: [(QuestionText, &str, TextReason); 10] = [
        (QuestionText::Title, " ", TextReason::Blank),
        (QuestionText::Title, " padded", TextReason::Padded),
        (
            QuestionText::Question,
            "Is it\nsettled?",
            TextReason::ControlCharacter,
        ),
        (
            QuestionText::Question,
            "Is `it` settled?",
            TextReason::ForbiddenCharacter,
        ),
        (QuestionText::Title, "a | b", TextReason::ForbiddenCharacter),
        (QuestionText::Title, "the subject.", TextReason::Terminal),
        (
            QuestionText::Question,
            "It is settled.",
            TextReason::Terminal,
        ),
        (
            QuestionText::SettledWhen,
            "A derivation answers it",
            TextReason::Terminal,
        ),
        (
            QuestionText::Question,
            "Is it? Is it settled?",
            TextReason::SeveralSentences,
        ),
        (
            QuestionText::SettledWhen,
            "It is. It ends.",
            TextReason::SeveralSentences,
        ),
    ];
    for (field, value, reason) in cases {
        let mut drafts = store_of(&["a"]);
        match field {
            QuestionText::Title => drafts[0].title.value = value.into(),
            QuestionText::Question => drafts[0].question.value = value.into(),
            QuestionText::SettledWhen => drafts[0].settled_when.value = value.into(),
            QuestionText::Answer | QuestionText::WithdrawnBecause => unreachable!(),
        }
        assert_eq!(
            refused(drafts),
            [text_fault(field, value, reason)],
            "{value:?}"
        );
    }
}
#[test]
fn m067_question_with_two_question_marks_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].question.value = "Is it settled, or is it open? or both?".into();
    assert_eq!(
        refused(drafts),
        [text_fault(
            QuestionText::Question,
            "Is it settled, or is it open? or both?",
            TextReason::SeveralSentences
        )]
    );
}
#[test]
fn m067_closure_texts_are_sentences() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[0].state = StateDraft::Settled {
        by: vec![reference(0, "settled-by", ReferenceKind::Rule, "M001")],
        answer: text(0, "answer", "it is settled"),
    };
    drafts[1].state = StateDraft::Withdrawn {
        because: text(1, "withdrawn-because", ""),
    };
    assert_eq!(
        refused(drafts),
        [
            text_fault(QuestionText::Answer, "it is settled", TextReason::Terminal),
            text_fault(QuestionText::WithdrawnBecause, "", TextReason::Blank),
        ]
    );
}
#[test]
fn m067_duplicate_title_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[1].title.value = "the a subject".into();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::DuplicateTitle {
                title: "the a subject".into(),
                first: 0
            }
        )]
    );
}
#[test]
fn m067_question_equal_after_whitespace_normalization_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[1].question.value = "Is  the a subject   settled?".into();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::DuplicateQuestion {
                question: "Is the a subject settled?".into(),
                first: 0
            }
        )]
    );
}
#[test]
fn m067_empty_closure_list_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].state = settled(0, vec![]);
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M067,
            QuestionFault::EmptyList {
                list: QuestionList::SettledBy
            }
        )]
    );
}
#[test]
fn m067_repeated_list_entry_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    let first = drafts[0].about[0].clone();
    drafts[0].about.push(first);
    drafts[1].rests_on = vec![
        text(1, "rests-on", "open:test:a"),
        text(1, "rests-on", "open:test:a"),
    ];
    assert_eq!(
        refused(drafts),
        [
            (
                RuleId::M067,
                QuestionFault::RepeatedReference {
                    list: QuestionList::About,
                    kind: ReferenceKind::Node,
                    value: "def:langevin:coefficient-one".into()
                }
            ),
            (
                RuleId::M067,
                QuestionFault::RepeatedReference {
                    list: QuestionList::RestsOn,
                    kind: ReferenceKind::Open,
                    value: "open:test:a".into()
                }
            ),
        ]
    );
}

// M069: references resolve, each in its own domain.
#[test]
fn m069_record_about_nothing_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].about.clear();
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M069,
            QuestionFault::EmptyList {
                list: QuestionList::About
            }
        )]
    );
}
#[test]
fn m069_each_kind_refuses_a_value_outside_its_domain() {
    // Cross-domain values and an unallocated rule refuse in their own domains.
    let cases = [
        (ReferenceKind::Node, "M001"),
        (ReferenceKind::Rule, "def:langevin:coefficient-one"),
        (ReferenceKind::Unit, "dec:paper:authored-latex"),
        (ReferenceKind::Record, "stmt:test:unit"),
        (ReferenceKind::Paper, "docs/glossary.md"),
        (ReferenceKind::File, "tbl:debts"),
        (ReferenceKind::Open, "open:test:missing"),
        (ReferenceKind::Rule, "M999"),
    ];
    for (kind, value) in cases {
        let mut drafts = store_of(&["a"]);
        drafts[0].about = vec![reference(0, "about", kind, value)];
        assert_eq!(
            refused(drafts),
            [(
                RuleId::M069,
                QuestionFault::UnresolvedReference {
                    list: QuestionList::About,
                    kind,
                    value: value.into()
                }
            )],
            "{kind:?} {value}"
        );
    }
}
#[test]
fn m069_malformed_file_paths_refuse_before_lookup() {
    for value in [
        "/etc/passwd",
        "../outside.md",
        "docs/./glossary.md",
        "docs//glossary.md",
        "docs\\glossary.md",
        "docs/a b.md",
        "",
    ] {
        let mut drafts = store_of(&["a"]);
        drafts[0].about = vec![reference(0, "about", ReferenceKind::File, value)];
        assert_eq!(
            refused(drafts),
            [(
                RuleId::M069,
                QuestionFault::MalformedPath {
                    list: QuestionList::About,
                    value: value.into()
                }
            )],
            "{value:?}"
        );
    }
}
#[test]
fn m069_self_reference_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].about = vec![reference(0, "about", ReferenceKind::Open, "open:test:a")];
    drafts[0].rests_on = vec![text(0, "rests-on", "open:test:a")];
    assert_eq!(
        refused(drafts),
        [
            (
                RuleId::M069,
                QuestionFault::SelfReference {
                    list: QuestionList::About
                }
            ),
            (
                RuleId::M069,
                QuestionFault::SelfReference {
                    list: QuestionList::RestsOn
                }
            ),
        ]
    );
}
#[test]
fn m069_succession_by_itself_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].state = StateDraft::Succeeded {
        by: vec![reference(
            0,
            "succeeded-by",
            ReferenceKind::Open,
            "open:test:a",
        )],
        answer: None,
    };
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M069,
            QuestionFault::SelfReference {
                list: QuestionList::SucceededBy
            }
        )]
    );
}
#[test]
fn m068_paper_tag_target_of_the_reserved_kind_refuses() {
    let rows = vec![
        ("c:x".to_owned(), "sem:claim:x".to_owned(), 3),
        ("o:x".to_owned(), "open:paper:x".to_owned(), 4),
    ];
    let error = crate::check_paper_tag_targets("documents/paper-build/paper-tags.tsv", &rows)
        .expect_err("reserved");
    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(error.diagnostics()[0].rule, RuleId::M068);
    assert!(matches!(
        &error.diagnostics()[0].fault,
        LabelFault::ReservedOpenKind { label, .. } if label == "open:paper:x"
    ));
    assert_eq!(
        error.diagnostics()[0].site,
        crate::LabelSite::PaperTagRow {
            file: "documents/paper-build/paper-tags.tsv".into(),
            line: 4
        }
    );
    assert_eq!(
        error.diagnostics()[0].site.to_string(),
        "paper tag table \"documents/paper-build/paper-tags.tsv\" row 4"
    );
    assert!(
        crate::check_paper_tag_targets("documents/paper-build/paper-tags.tsv", &rows[..1]).is_ok()
    );
}
// M068's Rust-identity clause needs a NodeId of kind open to be refused, which the closed
// identity catalogue does not contain; the same reserved-kind check covers those mints.
#[test]
fn m069_unresolved_rests_on_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].rests_on = vec![text(0, "rests-on", "open:test:missing")];
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M069,
            QuestionFault::UnresolvedReference {
                list: QuestionList::RestsOn,
                kind: ReferenceKind::Open,
                value: "open:test:missing".into()
            }
        )]
    );
}
#[test]
fn m069_blocks_naming_an_open_record_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[0].blocks = vec![reference(0, "blocks", ReferenceKind::Open, "open:test:b")];
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M069,
            QuestionFault::OpenInBlocks {
                value: "open:test:b".into()
            }
        )]
    );
}
#[test]
fn m069_closure_references_resolve() {
    let mut drafts = store_of(&["a"]);
    drafts[0].state = settled(
        0,
        vec![reference(0, "settled-by", ReferenceKind::Record, "dec:x:y")],
    );
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M069,
            QuestionFault::UnresolvedReference {
                list: QuestionList::SettledBy,
                kind: ReferenceKind::Record,
                value: "dec:x:y".into()
            }
        )]
    );
}

// M070: lifecycle relations.
#[test]
fn m070_open_record_resting_on_a_closed_record_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[0].state = StateDraft::Withdrawn {
        because: text(0, "withdrawn-because", "It was ill-posed."),
    };
    drafts[1].rests_on = vec![text(1, "rests-on", "open:test:a")];
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M070,
            QuestionFault::RestsOnClosed {
                label: "open:test:a".into(),
                state: QuestionStateKind::Withdrawn
            }
        )]
    );
}
#[test]
fn m070_rests_on_cycle_refuses_once_per_cycle() {
    let mut drafts = store_of(&["a", "b", "c", "d"]);
    drafts[0].rests_on = vec![text(0, "rests-on", "open:test:b")];
    drafts[1].rests_on = vec![text(1, "rests-on", "open:test:c")];
    drafts[2].rests_on = vec![text(2, "rests-on", "open:test:a")];
    drafts[3].rests_on = vec![text(3, "rests-on", "open:test:a")];
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M070,
            QuestionFault::Cycle {
                list: QuestionList::RestsOn,
                labels: vec![
                    "open:test:a".into(),
                    "open:test:b".into(),
                    "open:test:c".into()
                ]
            }
        )]
    );
}
#[test]
fn m070_succession_cycle_refuses() {
    let mut drafts = store_of(&["a", "b"]);
    drafts[0].state = StateDraft::Succeeded {
        by: vec![reference(
            0,
            "succeeded-by",
            ReferenceKind::Open,
            "open:test:b",
        )],
        answer: None,
    };
    drafts[1].state = StateDraft::Succeeded {
        by: vec![reference(
            1,
            "succeeded-by",
            ReferenceKind::Open,
            "open:test:a",
        )],
        answer: Some(text(1, "answer", "Part of it is answered.")),
    };
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M070,
            QuestionFault::Cycle {
                list: QuestionList::SucceededBy,
                labels: vec!["open:test:a".into(), "open:test:b".into()]
            }
        )]
    );
}
#[test]
fn m070_settlement_by_file_or_open_refuses() {
    for (kind, value) in [
        (ReferenceKind::File, "docs/glossary.md"),
        (ReferenceKind::Open, "open:test:b"),
    ] {
        let mut drafts = store_of(&["a", "b"]);
        drafts[0].state = settled(0, vec![reference(0, "settled-by", kind, value)]);
        assert_eq!(
            refused(drafts),
            [(RuleId::M070, QuestionFault::SettlementKind { kind })],
            "{kind:?}"
        );
    }
}
#[test]
fn m070_succession_by_anything_but_open_or_node_refuses() {
    let mut drafts = store_of(&["a"]);
    drafts[0].state = StateDraft::Succeeded {
        by: vec![reference(0, "succeeded-by", ReferenceKind::Rule, "M001")],
        answer: None,
    };
    assert_eq!(
        refused(drafts),
        [(
            RuleId::M070,
            QuestionFault::SuccessionKind {
                kind: ReferenceKind::Rule
            }
        )]
    );
}
#[test]
fn phases_report_the_earliest_failing_phase_only() {
    let mut drafts = store_of(&["a"]);
    drafts[0].title.value = "the subject.".into();
    drafts[0].about = vec![reference(0, "about", ReferenceKind::Rule, "M999")];
    let faults = refused(drafts);
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].0, RuleId::M067);
}

// M068: the kind is reserved to the store, and store labels join the joint inventory.
fn units(id: &str) -> crate::LanguageCheckedUnits {
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: site(vec![]),
    };
    crate::admit_unit_drafts(
        crate::DocumentDraft {
            documents_site: site(vec![]),
            documents: [(
                "main".to_owned(),
                crate::DocumentEntryDraft {
                    key_site: site(vec![]),
                    table_site: site(vec![]),
                    languages_site: site(vec![]),
                    required_languages: vec![located("en")],
                },
            )]
            .into(),
        },
        vec![crate::UnitDraft {
            econ: None,
            id: located(id),
            document: located("main"),
            table_site: site(vec![]),
            math_site: site(vec![]),
            texts_site: site(vec![]),
            documentary_targets: BTreeMap::new(),
            math: BTreeMap::new(),
            texts: BTreeMap::from([(
                "en".into(),
                crate::LocatedEntryDraft {
                    key_site: site(vec![]),
                    value: located("Fixture."),
                },
            )]),
        }],
    )
    .expect("one-unit corpus")
}
fn markdown(text: &str) -> CheckedMarkdown {
    check_markdown_sources(vec![MarkdownSource {
        file: "docs/records.md".into(),
        bytes: text.as_bytes().to_vec(),
    }])
    .expect("markdown")
}
fn admitted() -> OpenQuestions {
    admit_open_questions(FILE, store_of(&["a"]), &domains()).expect("store")
}
#[test]
fn m068_markdown_mint_of_the_reserved_kind_refuses() {
    let corpus = units("sec:fixture:unit");
    let markdown = markdown("# Q · `open:test:a`\n");
    let error = collect_joint_mints(&corpus, &markdown).expect_err("reserved");
    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(error.diagnostics()[0].rule, RuleId::M068);
    assert!(matches!(
        &error.diagnostics()[0].fault,
        LabelFault::ReservedOpenKind { label, origin }
            if label == "open:test:a" && origin.kind == MintKind::Markdown
    ));
}
#[test]
fn m068_unit_metadata_of_the_reserved_kind_refuses() {
    let corpus = units("open:fixture:unit");
    let markdown = markdown("# R · `rec:test:a`\n");
    let error = collect_joint_mints(&corpus, &markdown).expect_err("reserved");
    assert_eq!(error.diagnostics()[0].rule, RuleId::M068);
    assert!(matches!(
        &error.diagnostics()[0].fault,
        LabelFault::ReservedOpenKind { origin, .. } if origin.kind == MintKind::UnitMetadata
    ));
}
#[test]
fn m068_paper_mint_key_of_the_reserved_kind_refuses() {
    let corpus = units("sec:fixture:unit");
    let markdown = markdown("# R · `rec:test:a`\n");
    let error = collect_joint_mints(&corpus, &markdown)
        .expect("joint")
        .with_paper_mints(vec![(
            "open:test:a".into(),
            "documents/paper/main.tex".into(),
        )])
        .expect_err("reserved");
    assert_eq!(error.diagnostics()[0].rule, RuleId::M068);
    assert!(matches!(
        &error.diagnostics()[0].fault,
        LabelFault::ReservedOpenKind { label, origin }
            if label == "open:test:a" && origin.kind == MintKind::PaperTex
    ));
    assert_eq!(
        error.diagnostics()[0].site.to_string(),
        "paper mint file \"documents/paper/main.tex\""
    );
}
#[test]
fn m068_store_labels_are_checked_against_the_joint_inventory_and_a_second_mint_refuses() {
    let corpus = units("sec:fixture:unit");
    let markdown = markdown("# R · `rec:test:a`\n");
    let store = admitted();
    let joint: JointMints<'_, '_> = collect_joint_mints(&corpus, &markdown)
        .expect("joint")
        .with_open_questions(&store)
        .expect("store mints");
    assert_eq!(joint.mints()["open:test:a"].kind, MintKind::OpenQuestion);
    let error = joint.with_open_questions(&store).expect_err("second mint");
    assert_eq!(error.diagnostics()[0].rule, RuleId::M068);
    assert!(matches!(
        &error.diagnostics()[0].fault,
        LabelFault::DuplicateMint { label, actual: 2, .. } if label == "open:test:a"
    ));
}

#[test]
fn register_pins_the_open_question_rules() {
    use RuleId::{M066, M067, M068, M069, M070, M071, M072};
    assert_eq!(
        RULES[65..72]
            .iter()
            .map(|r| (r.number, r.state))
            .collect::<Vec<_>>(),
        [
            (M066, crate::RuleState::Structural),
            (M067, crate::RuleState::Enforced),
            (M068, crate::RuleState::Enforced),
            (M069, crate::RuleState::Enforced),
            (M070, crate::RuleState::Enforced),
            (M071, crate::RuleState::Enforced),
            (M072, crate::RuleState::DeferredPaperUnits),
        ]
    );
    assert!(RULES[65..71].iter().all(|r| r.deferred_reason.is_none()));
    assert_eq!(
        RULES[71].deferred_reason,
        Some(
            "The paper's appendices are authored LaTeX under ADR 014; the table is generated once they are statement units."
        )
    );
}
