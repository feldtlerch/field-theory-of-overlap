#![allow(clippy::too_many_lines)] // Complete fault matrices stay beside their restoration controls.
use super::*;
use crate::*;
const LABEL: &str = "dec:fixture:target";
const AUTHORITY: &str = "dec:fixture:authority";

#[test]
fn documentary_phases_sort_by_event_then_field_and_restore() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let baseline = vec![event(0, "fixture", Some(LABEL))];
    let mut malformed = baseline.clone();
    malformed[0].ordinal.value = "2".into();
    malformed[0].reason.value.clear();
    malformed[0].authority_record.value = "invalid".into();
    let wanted = RegistryErrors(vec![
        RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::History,
            site: site(0, "authority-record"),
            fault: RegistryFault::EventMetadata {
                field: "authority-record".into(),
                value: "invalid".into(),
                reason: MetadataReason::InvalidLabel,
            },
        },
        RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::History,
            site: site(0, "ordinal"),
            fault: RegistryFault::EventOrdinal {
                index: 0,
                expected: 1,
                actual: "2".into(),
            },
        },
        RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::History,
            site: site(0, "reason"),
            fault: RegistryFault::EventMetadata {
                field: "reason".into(),
                value: String::new(),
                reason: MetadataReason::Blank,
            },
        },
    ]);
    let actual = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, malformed)
        .unwrap_err();
    assert_eq!(actual, wanted);
    assert_eq!(actual.to_string(), wanted.to_string());
    let mut malformed = baseline.clone();
    malformed[0].label.as_mut().unwrap().value = "dec:missing:target".into();
    malformed[0].authority_record.value = "dec:missing:authority".into();
    let wanted = RegistryErrors(vec![
        RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Targets,
            site: site(0, "authority-record"),
            fault: RegistryFault::AuthorityRecord {
                label: "dec:missing:authority".into(),
                actual: None,
            },
        },
        RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Targets,
            site: site(0, "label"),
            fault: RegistryFault::UnknownTarget {
                label: "dec:missing:target".into(),
            },
        },
    ]);
    let actual = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, malformed)
        .unwrap_err();
    assert_eq!(actual, wanted);
    assert_eq!(actual.to_string(), wanted.to_string());
    let restored = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, baseline)
        .unwrap();
    assert_eq!(
        restored.resolve("fixture"),
        Ok(AddressTarget::Documentary(LABEL.into()))
    );
}

#[test]
fn documentary_authority_requires_markdown_even_for_known_mints() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    for (label, origin) in [
        (
            "sec:fixture:unit",
            MintOrigin {
                kind: MintKind::UnitMetadata,
                site: LabelSite::Unit {
                    unit: "sec:fixture:unit".into(),
                    site: UnitSite {
                        file: "units/statements/fixture.toml".into(),
                        path: vec![PathPart::Key("unit".into()), PathPart::Key("id".into())],
                        anchor: SourceAnchor::Unavailable,
                        decoded: None,
                        occurrence: None,
                    },
                },
            },
        ),
        (
            "def:langevin:coefficient-one",
            MintOrigin {
                kind: MintKind::RustIdentity,
                site: LabelSite::RustIdentity {
                    file: "crates/ftoo_manifest/src/identity.rs".into(),
                    identity: NodeId::LangevinCoefficientOne,
                    field: "semantic_name".into(),
                },
            },
        ),
    ] {
        let mut altered = event(0, "fixture", Some(LABEL));
        altered.authority_record.value = label.into();
        let wanted = expected(
            RegistryStage::Targets,
            0,
            "authority-record",
            RegistryFault::AuthorityRecord {
                label: label.into(),
                actual: Some(origin),
            },
        );
        let actual = addressing(&graph)
            .unwrap()
            .with_documentary(&joint, vec![altered])
            .unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
        let restored = addressing(&graph)
            .unwrap()
            .with_documentary(&joint, vec![event(0, "fixture", Some(LABEL))])
            .unwrap();
        assert_eq!(
            restored.resolve("fixture"),
            Ok(AddressTarget::Documentary(LABEL.into()))
        );
    }
}
fn site(index: usize, field: &str) -> UnitSite {
    UnitSite {
        file: "registry/fixture.toml".into(),
        path: vec![
            PathPart::Key("events".into()),
            PathPart::Index(index),
            PathPart::Key(field.into()),
        ],
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    }
}
fn value(index: usize, field: &str, value: &str) -> LocatedString {
    LocatedString {
        value: value.into(),
        site: site(index, field),
    }
}
fn event(index: usize, handle: &str, label: Option<&str>) -> DocumentaryEventDraft {
    DocumentaryEventDraft {
        site: UnitSite {
            file: "registry/fixture.toml".into(),
            path: vec![PathPart::Key("events".into()), PathPart::Index(index)],
            anchor: SourceAnchor::Unavailable,
            decoded: None,
            occurrence: None,
        },
        ordinal: value(index, "ordinal", &(index + 1).to_string()),
        handle: value(index, "handle", handle),
        label: label.map(|s| value(index, "label", s)),
        authority_record: value(index, "authority-record", AUTHORITY),
        reason: value(index, "reason", "Fixture authority."),
    }
}
fn corpus() -> LanguageCheckedUnits {
    corpus_text("Fixture.")
}
fn corpus_text(text: &str) -> LanguageCheckedUnits {
    let site = UnitSite {
        file: "units/statements/fixture.toml".into(),
        path: vec![],
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: site.clone(),
    };
    admit_unit_drafts(
        DocumentDraft {
            documents_site: site.clone(),
            documents: BTreeMap::from([(
                "main".into(),
                DocumentEntryDraft {
                    key_site: site.clone(),
                    table_site: site.clone(),
                    languages_site: site.clone(),
                    required_languages: vec![located("en")],
                },
            )]),
        },
        vec![UnitDraft {
            id: located("sec:fixture:unit"),
            document: located("main"),
            table_site: site.clone(),
            math_site: site.clone(),
            texts_site: site.clone(),
            math: BTreeMap::new(),
            texts: BTreeMap::from([(
                "en".into(),
                LocatedEntryDraft {
                    key_site: site.clone(),
                    value: located(text),
                },
            )]),
        }],
    )
    .unwrap()
}

#[test]
fn documentary_direct_statement_tags_refuse_live_domain_and_retirement() {
    let graph = seed().unwrap();
    let corpus = corpus_text("@tag{fixture} names a record.");
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    for retired in [false, true] {
        let mut events = vec![event(0, "fixture", Some(LABEL))];
        if retired {
            events.push(event(1, "fixture", None));
        }
        let book = addressing(&graph)
            .unwrap()
            .with_documentary(&mints, events)
            .unwrap();
        let fault = if retired {
            UnitFault::ResolutionFault {
                cause: TagResolutionFault::RetiredDocumentaryTag {
                    input: "fixture".into(),
                    label: LABEL.into(),
                    event: DocumentaryRetirement {
                        ordinal: 2,
                        authority_record: AUTHORITY.into(),
                        reason: "Fixture authority.".into(),
                        site: event(1, "fixture", None).site,
                    },
                },
            }
        } else {
            UnitFault::DocumentaryTagTarget {
                input: "fixture".into(),
                label: LABEL.into(),
            }
        };
        let expected = UnitErrors::from_diagnostics(vec![UnitDiagnostic {
            rule: RuleId::M042,
            stage: UnitStage::Resolution,
            unit: Some("sec:fixture:unit".into()),
            language: Some("en".into()),
            site: UnitSite {
                file: "units/statements/fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::Unavailable,
                decoded: Some(0..13),
                occurrence: Some(0),
            },
            fault,
        }])
        .unwrap();
        let actual = resolve_unit_tags(&corpus, &book).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        let restored = corpus_text("@tag{c:1} names a quantity.");
        let resolved = resolve_unit_tags(&restored, &book).unwrap();
        assert_eq!(
            resolved
                .tags()
                .iter()
                .map(|tag| (tag.occurrence().input(), tag.target()))
                .collect::<Vec<_>>(),
            vec![("c:1", NodeId::LangevinCoefficientOne)]
        );
    }
}
fn markdown() -> CheckedMarkdown {
    check_markdown_sources(vec![MarkdownSource {
        file: "fixture.md".into(),
        bytes: "# Target · `dec:fixture:target`\n# Authority · `dec:fixture:authority`\n"
            .as_bytes()
            .to_vec(),
    }])
    .unwrap()
}
#[test]
fn documentary_joint_context_rejects_another_mint_inventory_and_restores() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let bytes = "# Target · `dec:fixture:target`\n# Authority · `dec:fixture:authority`\n# Kernel · `proposal:manifest:kernel-bite`\n# Lifecycle · `proposal:manifest:addressing-lifecycle-bite`\n";
    let markdown = check_markdown_sources(vec![MarkdownSource {
        file: "fixture.md".into(),
        bytes: bytes.as_bytes().to_vec(),
    }])
    .unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, vec![event(0, "fixture", Some(LABEL))])
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let baseline = integrate_joint_labels(&resolved, &joint).unwrap();
    let changed = check_markdown_sources(vec![MarkdownSource {
        file: "other.md".into(),
        bytes: bytes.as_bytes().to_vec(),
    }])
    .unwrap();
    let other = collect_joint_mints(&corpus, &changed).unwrap();
    let expected = vec![LabelDiagnostic {
        rule: RuleId::M060,
        stage: LabelStage::Namespace,
        unit: None,
        language: None,
        site: LabelSite::MintContext,
        fault: LabelFault::MintContextMismatch {
            expected: joint.mints().clone(),
            actual: other.mints().clone(),
        },
    }];
    let actual = integrate_joint_labels(&resolved, &other).unwrap_err();
    assert_eq!(actual.diagnostics(), expected);
    assert_eq!(actual.to_string(), format!("{}\n", expected[0]));
    assert_eq!(
        integrate_joint_labels(&resolved, &joint).unwrap().mints(),
        baseline.mints()
    );
}
fn expected(
    stage: RegistryStage,
    index: usize,
    field: &str,
    fault: RegistryFault,
) -> RegistryErrors {
    RegistryErrors(vec![RegistryDiagnostic {
        rule: if matches!(
            fault,
            RegistryFault::InvalidHandle { .. } | RegistryFault::HandleCollision { .. }
        ) {
            RuleId::M031
        } else {
            RuleId::M060
        },
        stage,
        site: site(index, field),
        fault,
    }])
}
fn restored(book: &AddressBook<'_>) {
    assert_eq!(
        book.resolve("fixture"),
        Ok(AddressTarget::Documentary(LABEL.into()))
    );
    assert_eq!(
        book.reader_identity(AddressTarget::Documentary(LABEL.into())),
        Ok("fixture".into())
    );
    assert_eq!(
        book.documentary_history(),
        &[event(0, "fixture", Some(LABEL))]
    );
    assert_eq!(
        book.documentary()
            .map(|b| (b.handle(), b.label(), b.state()))
            .collect::<Vec<_>>(),
        vec![("fixture", LABEL, &DocumentaryState::Live)]
    );
}
#[test]
fn documentary_history_and_target_faults_restore_complete_admission() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    let original = vec![event(0, "fixture", Some(LABEL))];
    let mut controls = vec![];
    for ordinal in ["0", "+1", "01", "1_0", "4294967296"] {
        let mut bad = original.clone();
        bad[0].ordinal.value = ordinal.into();
        controls.push((
            bad,
            expected(
                RegistryStage::History,
                0,
                "ordinal",
                RegistryFault::EventOrdinal {
                    index: 0,
                    expected: 1,
                    actual: ordinal.into(),
                },
            ),
        ));
    }
    for (field, text, reason) in [
        ("reason", " ", MetadataReason::Blank),
        ("authority-record", "bad", MetadataReason::InvalidLabel),
        ("label", "bad", MetadataReason::InvalidLabel),
    ] {
        let mut bad = original.clone();
        match field {
            "reason" => bad[0].reason.value = text.into(),
            "authority-record" => bad[0].authority_record.value = text.into(),
            _ => bad[0].label.as_mut().unwrap().value = text.into(),
        }
        controls.push((
            bad,
            expected(
                RegistryStage::History,
                0,
                field,
                RegistryFault::EventMetadata {
                    field: field.into(),
                    value: text.into(),
                    reason,
                },
            ),
        ));
    }
    controls.push((
        vec![event(0, "fixture", None)],
        expected(
            RegistryStage::History,
            0,
            "handle",
            RegistryFault::RetireBeforeAdmission {
                handle: "fixture".into(),
            },
        ),
    ));
    let first = DocumentaryRetirement {
        ordinal: 2,
        authority_record: AUTHORITY.into(),
        reason: "Fixture authority.".into(),
        site: event(1, "old", None).site,
    };
    controls.push((
        vec![
            original[0].clone(),
            event(1, "fixture", None),
            event(2, "fixture", None),
        ],
        expected(
            RegistryStage::History,
            2,
            "handle",
            RegistryFault::RetireTwice {
                handle: "fixture".into(),
                first,
            },
        ),
    ));
    controls.push((
        vec![event(0, "fixture", Some("dec:fixture:absent"))],
        expected(
            RegistryStage::Targets,
            0,
            "label",
            RegistryFault::UnknownTarget {
                label: "dec:fixture:absent".into(),
            },
        ),
    ));
    let mut bad = original.clone();
    bad[0].authority_record.value = "dec:fixture:absent".into();
    controls.push((
        bad,
        expected(
            RegistryStage::Targets,
            0,
            "authority-record",
            RegistryFault::AuthorityRecord {
                label: "dec:fixture:absent".into(),
                actual: None,
            },
        ),
    ));
    let theory = NodeId::LangevinCoefficientOne;
    controls.push((
        vec![event(0, "fixture", Some(theory.semantic_name()))],
        expected(
            RegistryStage::Targets,
            0,
            "label",
            RegistryFault::TargetDomain {
                label: theory.semantic_name().into(),
                actual: MintOrigin {
                    kind: MintKind::RustIdentity,
                    site: LabelSite::RustIdentity {
                        file: "crates/ftoo_manifest/src/identity.rs".into(),
                        identity: theory,
                        field: "semantic_name".into(),
                    },
                },
            },
        ),
    ));
    for tail in [false, true] {
        let mut bad = vec![original[0].clone(), event(1, "second", Some(LABEL))];
        if tail {
            bad.push(event(2, "fixture", None));
        }
        controls.push((
            bad,
            expected(
                RegistryStage::Targets,
                1,
                "label",
                RegistryFault::ConcurrentLiveTarget {
                    label: LABEL.into(),
                    live_handle: "fixture".into(),
                    live_admitted_at: site(0, "label"),
                    attempted_handle: "second".into(),
                },
            ),
        ));
    }
    for (bad, wanted) in controls {
        let actual = addressing(&graph)
            .unwrap()
            .with_documentary(&mints, bad)
            .unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
        restored(
            &addressing(&graph)
                .unwrap()
                .with_documentary(&mints, original.clone())
                .unwrap(),
        );
    }
}
#[test]
fn documentary_retirement_new_handle_and_empty_registry_preserve_domains() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    let empty = addressing(&graph)
        .unwrap()
        .with_documentary(&mints, vec![])
        .unwrap();
    let fault = ReaderIdentityFault {
        target: AddressTarget::Documentary(LABEL.into()),
        reason: ReaderIdentityReason::MissingTag,
        candidates: vec![],
    };
    assert_eq!(
        empty.reader_identity(AddressTarget::Documentary(LABEL.into())),
        Err(fault)
    );
    let old = event(0, "old", Some(LABEL));
    let retired = event(1, "old", None);
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&mints, vec![old.clone(), retired.clone()])
        .unwrap();
    let refusal = TagResolutionFault::RetiredDocumentaryTag {
        input: "old".into(),
        label: LABEL.into(),
        event: DocumentaryRetirement {
            ordinal: 2,
            authority_record: AUTHORITY.into(),
            reason: "Fixture authority.".into(),
            site: event(1, "old", None).site,
        },
    };
    assert_eq!(book.resolve("old"), Err(refusal.clone()));
    assert_eq!(
        refusal.to_string(),
        "tag \"old\": retired; label dec:fixture:target; event 2; authority dec:fixture:authority; reason \"Fixture authority.\""
    );
    assert_eq!(
        book.reader_identity(AddressTarget::Documentary(LABEL.into())),
        Err(ReaderIdentityFault {
            target: AddressTarget::Documentary(LABEL.into()),
            reason: ReaderIdentityReason::NoLiveTag,
            candidates: vec![ReaderTagCandidate {
                handle: "old".into(),
                outcome: Err(refusal.clone())
            }]
        })
    );
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(
            &mints,
            vec![
                old,
                retired,
                event(2, "new", Some(LABEL)),
                event(3, "unit", Some("sec:fixture:unit")),
            ],
        )
        .unwrap();
    assert_eq!(book.resolve("old"), Err(refusal));
    assert_eq!(
        book.reader_identity(AddressTarget::Documentary(LABEL.into())),
        Ok("new".into())
    );
    assert_eq!(
        book.resolve("unit"),
        Ok(AddressTarget::Documentary("sec:fixture:unit".into()))
    );
    assert_eq!(
        book.resolve("c:1"),
        Ok(AddressTarget::Theory(NodeId::LangevinCoefficientOne))
    );
}
#[test]
fn documentary_catalogue_collisions_and_literal_shapes_are_independent() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    for (handle, fault) in [
        (
            "",
            RegistryFault::InvalidHandle {
                handle: String::new(),
            },
        ),
        (
            "a b",
            RegistryFault::InvalidHandle {
                handle: "a b".into(),
            },
        ),
        (
            "dec:fixture:label",
            RegistryFault::LabelShapedHandle {
                handle: "dec:fixture:label".into(),
            },
        ),
    ] {
        let wanted = expected(RegistryStage::Catalogue, 0, "handle", fault);
        let actual = addressing(&graph)
            .unwrap()
            .with_documentary(&mints, vec![event(0, handle, Some(LABEL))])
            .unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
        restored(
            &addressing(&graph)
                .unwrap()
                .with_documentary(&mints, vec![event(0, "fixture", Some(LABEL))])
                .unwrap(),
        );
    }
    for descriptor in addressing(&graph).unwrap().descriptors() {
        let handle = descriptor.spelling();
        let wanted = expected(
            RegistryStage::Catalogue,
            0,
            "handle",
            RegistryFault::HandleCollision {
                handle: handle.into(),
                origins: vec![
                    HandleOrigin::Theory {
                        tag: descriptor.tag(),
                    },
                    HandleOrigin::Documentary {
                        index: 0,
                        site: site(0, "handle"),
                    },
                ],
            },
        );
        let actual = addressing(&graph)
            .unwrap()
            .with_documentary(&mints, vec![event(0, handle, Some(LABEL))])
            .unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
    }
    let actual = addressing(&graph)
        .unwrap()
        .with_documentary(
            &mints,
            vec![
                event(0, "fixture", Some(LABEL)),
                event(1, "fixture", None),
                event(2, "fixture", Some(LABEL)),
            ],
        )
        .unwrap_err();
    let wanted = expected(
        RegistryStage::Catalogue,
        0,
        "handle",
        RegistryFault::HandleCollision {
            handle: "fixture".into(),
            origins: vec![
                HandleOrigin::Documentary {
                    index: 0,
                    site: site(0, "handle"),
                },
                HandleOrigin::Documentary {
                    index: 2,
                    site: site(2, "handle"),
                },
            ],
        },
    );
    assert_eq!(actual, wanted);
    assert_eq!(actual.to_string(), wanted.to_string());
}

#[test]
fn documentary_partition_mutations_preserve_complete_vectors_and_restore() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    let events = vec![event(0, "fixture", Some(LABEL))];
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&mints, events.clone())
        .unwrap();
    let original = book.documentary.clone();
    let retirement = DocumentaryRetirement {
        ordinal: 2,
        authority_record: AUTHORITY.into(),
        reason: "Fixture authority.".into(),
        site: event(1, "old", None).site,
    };
    for control in 0..4 {
        let mut actual = original.clone();
        let mut missing = vec![];
        let mut repeated = vec![];
        let mut unexpected = vec![];
        let mut changed = vec![];
        match control {
            0 => {
                actual.clear();
                missing.push("fixture".into());
            }
            1 => {
                actual.push(original[0].clone());
                repeated.push(("fixture".into(), 2));
            }
            2 => {
                let mut extra = original[0].clone();
                extra.handle = "extra".into();
                actual.push(extra);
                unexpected.push("extra".into());
            }
            _ => {
                actual[0].state = DocumentaryState::Retired(retirement.clone());
                changed.push(DocumentaryStateChange {
                    handle: "fixture".into(),
                    expected: DocumentaryState::Live,
                    actual: DocumentaryState::Retired(retirement.clone()),
                });
            }
        }
        let wanted = RegistryErrors(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Stores,
            site: UnitSite {
                file: "registry/fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: None,
                occurrence: None,
            },
            fault: RegistryFault::DocumentaryPartition {
                missing,
                repeated,
                unexpected,
                changed,
            },
        }]);
        let error = partition(&events, &actual).unwrap_err();
        assert_eq!(error, wanted);
        assert_eq!(error.to_string(), wanted.to_string());
        assert_eq!(partition(&events, &original), Ok(()));
        restored(&book);
    }
}

#[test]
fn documentary_collides_with_retired_and_outside_theory_handles() {
    let graph = seed().unwrap();
    let corpus = corpus();
    let markdown = markdown();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    let mut history = crate::tag_history::seed_history();
    history.push(HistoryEvent {
        ordinal: 9,
        change: HistoryChange::RetireTag {
            tag: TagId::LangevinCoefficientThree,
        },
        authority_record: AUTHORITY.into(),
        reason: "Fixture authority.".into(),
    });
    let retired = crate::tag_validation::construct(
        &graph,
        TagId::ALL.map(TagId::descriptor).to_vec(),
        history,
    )
    .unwrap();
    let mut draft = seed_draft();
    draft.coverage.slice = vec![NodeId::LangevinCoefficientOneValue];
    let graph = validate(draft).unwrap();
    let outside = addressing(&graph).unwrap();
    for book in [retired, outside] {
        let wanted = expected(
            RegistryStage::Catalogue,
            0,
            "handle",
            RegistryFault::HandleCollision {
                handle: "c:3".into(),
                origins: vec![
                    HandleOrigin::Theory {
                        tag: TagId::LangevinCoefficientThree,
                    },
                    HandleOrigin::Documentary {
                        index: 0,
                        site: site(0, "handle"),
                    },
                ],
            },
        );
        let actual = book
            .with_documentary(&mints, vec![event(0, "c:3", Some(LABEL))])
            .unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
    }
}
