#![allow(clippy::too_many_lines)] // Complete citation records and restoration evidence stay together.
use super::Coverage;
use super::*;
use crate::documents::{input, latex, markdown, registry};
use ftoo_manifest::*;
const LABEL: &str = "dec:fixture:source";
const UNIT: &str = "sec:fixture:reader";
fn context(
    text: &str,
    ledger: &str,
    check: impl FnOnce(&AddressBook<'_>, &CheckedUnit, &WeldedUnits<'_, '_, '_, '_, '_, '_>, Block),
) {
    let graph = seed().unwrap();
    context_graph(&graph, text, ledger, check);
}
fn context_graph(
    graph: &ValidatedManifest,
    text: &str,
    ledger: &str,
    check: impl FnOnce(&AddressBook<'_>, &CheckedUnit, &WeldedUnits<'_, '_, '_, '_, '_, '_>, Block),
) {
    let site = input::site("fixture.toml");
    let corpus = literal_corpus(text, BTreeMap::new());
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"fixture.md".into(),bytes:"# Decision · `dec:fixture:source`\n# Authority · `dec:fixture:authority`\n# Kernel · `proposal:manifest:kernel-bite`\n# Lifecycle · `proposal:manifest:addressing-lifecycle-bite`\n".as_bytes().to_vec()}]).unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let events = registry::load(&crate::UnitSource {
        file: "fixture-registry.toml".into(),
        bytes: ledger.as_bytes().to_vec(),
    })
    .unwrap();
    let book = addressing(graph)
        .unwrap()
        .with_documentary(&joint, events)
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_joint_labels(&resolved, &joint).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let unit = &corpus.units()[0];
    let block = Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: UNIT.into(),
        depth: None,
        site: site.clone(),
        runs: runs(0, unit, "en", &welded, &BTreeMap::new()),
    };
    check(&book, unit, &welded, block);
}
fn literal_corpus(text: &str, math: BTreeMap<String, LocatedEntryDraft>) -> LanguageCheckedUnits {
    let site = input::site("fixture.toml");
    let value = |s: &str| LocatedString {
        value: s.into(),
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
                    required_languages: vec![value("en")],
                },
            )]),
        },
        vec![UnitDraft {
            id: value(UNIT),
            document: value("main"),
            table_site: site.clone(),
            math_site: site.clone(),
            texts_site: site.clone(),
            math,
            texts: BTreeMap::from([(
                "en".into(),
                LocatedEntryDraft {
                    key_site: site.clone(),
                    value: value(text),
                },
            )]),
        }],
    )
    .unwrap()
}
fn ledger(handle: &str, label: &str) -> String {
    format!(
        "schema = 1\n[[events]]\nordinal = 1\nchange = \"admit\"\nhandle = {handle:?}\nlabel = {label:?}\nauthority-record = \"dec:fixture:authority\"\nreason = \"Fixture admission.\"\n"
    )
}

#[test]
fn theory_citation_outside_mapping_preserves_mint_and_restores() {
    let label = "claim:langevin:coefficient-three-value";
    let text = format!("See `{label}`.");
    let mut draft = seed_draft();
    draft.coverage.slice = vec![NodeId::LangevinCoefficientOneValue];
    let graph = validate(draft).unwrap();
    context_graph(
        &graph,
        &text,
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site: UnitSite {
                                    decoded: Some(4..44),
                                    ..input::site("fixture.toml")
                                },
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    anchor: SourceAnchor::Unavailable,
                                    ..input::site("crates/ftoo_manifest/src/identity.rs")
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Theory(NodeId::LangevinCoefficientThreeValue),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: vec![ReaderTagCandidate {
                            handle: "v:3".into(),
                            outcome: Err(TagResolutionFault::OutOfSliceTag {
                                input: "v:3".into(),
                                tag: TagId::LangevinCoefficientThreeValue,
                                target: NodeId::LangevinCoefficientThreeValue,
                                cause: OutOfSliceCause::ClaimNotSelected,
                            }),
                        }],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        &text,
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            assert_eq!(
                check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                Ok(())
            );
            assert_eq!(
                check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
                Ok(())
            );
        },
    );
}

#[test]
fn theory_citation_retirement_mapping_keeps_complete_rejected_authority_evidence() {
    // Production projection admission rejects this extra authority act. Its
    // immutable diagnostic supplies an event for mapping, never a checked book.
    let original = include_bytes!("../../../../../generated/manifest.json");
    let mut bytes: serde_json::Value = serde_json::from_slice(original).unwrap();
    bytes["history"].as_array_mut().unwrap().push(serde_json::json!({
        "ordinal":9,"authority-record":"proposal:manifest:kernel-bite","reason":"Fixture retirement.",
        "change":{"kind":"retire-tag","tag":"c:1"},
    }));
    bytes["stores"]["live"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["tag"] != "c:1");
    bytes["stores"]["retired"] =
        serde_json::json!([{"tag":"c:1","cause":{"kind":"tag-retirement","event":9}}]);
    let refused = load_projection(&serde_json::to_vec(&bytes).unwrap()).unwrap_err();
    let ProjectionError::Assertion(ref errors) = refused else {
        panic!("authority refusal: {refused:?}")
    };
    let ProjectionFault::HistoryMismatch {
        actual: Some(value),
        ..
    } = &errors[0].fault
    else {
        panic!("history evidence")
    };
    let ProjectionValue::History(event) = value.as_ref() else {
        panic!("event evidence")
    };
    assert_eq!(
        (
            event.ordinal(),
            event.authority_record(),
            event.reason(),
            event.change()
        ),
        (
            9,
            "proposal:manifest:kernel-bite",
            "Fixture retirement.",
            &HistoryChange::RetireTag {
                tag: TagId::LangevinCoefficientOne
            }
        )
    );
    let expected_projection = ProjectionError::Assertion(vec![ProjectionDiagnostic {
        rule: RuleId::M037,
        path: "$.history[8]".into(),
        position: Some(8),
        fault: ProjectionFault::HistoryMismatch {
            expected: None,
            actual: Some(Box::new(ProjectionValue::History(event.clone()))),
        },
    }]);
    assert_eq!(refused, expected_projection);
    assert_eq!(refused.to_string(), expected_projection.to_string());
    let cause = RetirementCause::TagRetirement(event.clone());
    context(
        "See `def:langevin:coefficient-one`.",
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let Inline::Citation { identity, site, .. } = &block.runs[1] else {
                panic!("citation")
            };
            let origins = vec![
                SourceOrigin {
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: UnitSite {
                        decoded: Some(4..34),
                        ..input::site("fixture.toml")
                    },
                },
                SourceOrigin {
                    unit: None,
                    language: None,
                    target: None,
                    site: UnitSite {
                        anchor: SourceAnchor::Unavailable,
                        ..input::site("crates/ftoo_manifest/src/identity.rs")
                    },
                },
            ];
            assert_eq!(citation_origins(identity, site, UNIT, "en"), origins);
            let candidates = vec![ReaderTagCandidate {
                handle: "c:1".into(),
                outcome: Err(TagResolutionFault::RetiredTag {
                    input: "c:1".into(),
                    tag: TagId::LangevinCoefficientOne,
                    target: NodeId::LangevinCoefficientOne,
                    cause,
                }),
            }];
            let position = ReaderIdentityPosition::Citation {
                unit: UNIT.into(),
                span: 0,
                target: None,
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: position.clone(),
                        origins: origins.clone(),
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Theory(NodeId::LangevinCoefficientOne),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: candidates.clone(),
                    },
                }])
                .unwrap(),
            );
            let actual = reader_failure(
                ReaderIdentityFault {
                    target: AddressTarget::Theory(NodeId::LangevinCoefficientOne),
                    reason: ReaderIdentityReason::NoLiveTag,
                    candidates,
                },
                "main",
                "en",
                Some(UNIT),
                None,
                position,
                origins,
            );
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
        },
    );
    let restored = load_projection(original).unwrap();
    assert_eq!(
        restored.addressing().unwrap().resolve("c:1"),
        Ok(AddressTarget::Theory(NodeId::LangevinCoefficientOne))
    );
}

pub(in crate::documents) fn support_required_literal_build_controls(
    notation: &crate::RenderableNotation<'_, '_, '_>,
) {
    for (text, literal) in [
        ("Literal dec:fixture:source.", true),
        ("Literal ``dec:fixture:source``.", true),
        ("See `dec:fixture:source` and @tag{c:1}.", false),
    ] {
        context(text, &ledger("record", LABEL), |_, unit, welded, _| {
            let corpus = welded.integrated().resolved().corpus();
            let site = input::site("fixture.toml");
            let located = |value: &str| LocatedString {
                value: value.into(),
                site: site.clone(),
            };
            let metadata = BTreeMap::from([(UNIT.into(), site.clone())]);
            let skeletons = check_skeletons(
                corpus,
                &[SkeletonDraft {
                    document: located("main"),
                    sections_site: site.clone(),
                    sections: vec![SkeletonSection {
                        heading: located(UNIT),
                        children: vec![],
                    }],
                }],
                &metadata,
                &metadata,
                &BTreeMap::from([("main".into(), site.clone())]),
            )
            .unwrap();
            let displays = check_displays(&[], &skeletons, welded).unwrap();
            let mut errors = vec![];
            check_reader_literals(unit, "en", &displays, &mut errors);
            if literal {
                let mention = text.contains("``");
                let expected = AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::Source(UnitSite {
                        decoded: mention.then_some(8..30),
                        ..site.clone()
                    }),
                    fault: AssemblyFault::ReaderLabelLiteral {
                        value: LABEL.into(),
                        bytes: if mention { 10..28 } else { 8..26 },
                    },
                };
                assert_eq!(errors, vec![expected.clone()]);
                let expected = DocumentError::Assembly(
                    AssemblyErrors::from_diagnostics(vec![expected]).unwrap(),
                );
                let actual = build(&skeletons, welded, notation, &metadata, &displays)
                    .err()
                    .unwrap();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
            } else {
                assert_eq!(errors, vec![]);
                let plans = build(&skeletons, welded, notation, &metadata, &displays).unwrap();
                for plan in plans {
                    assert_eq!(
                        check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                        Ok(())
                    );
                    assert_eq!(
                        check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
                        Ok(())
                    );
                }
            }
        });
    }
}
fn plan(block: Block) -> DocumentPlan {
    DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![block],
    }
}

#[test]
fn tag_comment_cannot_become_a_citation_even_with_the_right_handle() {
    context(
        "Use @tag{c:1}.",
        "schema = 1\nevents = []\n",
        |_, _, _, block| {
            let plan = plan(block);
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                let source = if target == DocumentTarget::Latex {
                    latex::render(&plan)
                } else {
                    markdown::render(&plan)
                };
                let altered = source.replace(
                    "label def:langevin:coefficient-one",
                    "cite def:langevin:coefficient-one",
                );
                let site = UnitSite {
                    decoded: Some(4..13),
                    occurrence: Some(0),
                    ..input::site("fixture.toml")
                };
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "Use @tag{c:1}.".into(),
                    site: input::site("fixture.toml"),
                };
                let missing = EmissionRecord {
                    kind: EmissionKind::Tag,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: Some(0),
                    value: "def:langevin:coefficient-one".into(),
                    site: site.clone(),
                };
                let unexpected = EmissionRecord {
                    kind: EmissionKind::Citation(Box::new(crate::documents::CitationIdentity {
                        label: "def:langevin:coefficient-one".into(),
                        resolved: None,
                        handle: "c:1".into(),
                        token: if target == DocumentTarget::Latex {
                            "u0-t0".into()
                        } else {
                            String::new()
                        },
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        mint: None,
                    })),
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "def:langevin:coefficient-one".into(),
                    site: input::site("fixture.toml"),
                };
                let mut actual_paragraph = paragraph.clone();
                actual_paragraph.value = "Use `def:langevin:coefficient-one`.".into();
                let marker = if target == DocumentTarget::Latex {
                    "% unit "
                } else {
                    "<!-- unit "
                };
                let expected = crate::documents::failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    AssemblySite::Generated {
                        path: format!(
                            "generated/documents/main/en.{}",
                            if target == DocumentTarget::Latex {
                                "tex"
                            } else {
                                "md"
                            }
                        ),
                        bytes: altered.find(marker).unwrap()..altered.len(),
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: Some(target),
                                site: input::site("fixture.toml"),
                            },
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: Some(target),
                                site,
                            },
                        ],
                    },
                    AssemblyFault::EmissionMismatch {
                        missing: vec![missing],
                        unexpected: vec![unexpected],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph,
                            actual: actual_paragraph,
                        }],
                    },
                );
                let actual = check_emission(&plan, &altered, target).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn recovered_literal_labels_refuse_after_target_decoding_and_restore() {
    context(
        "Literal ordinary.",
        "schema = 1\nevents = []\n",
        |_, _, _, block| {
            let plan = plan(block);
            for (target, injected) in [
                (DocumentTarget::Markdown, "dec:fixture:so&#117;rce"),
                (DocumentTarget::Latex, r"\texttt{dec:fixture:source}"),
            ] {
                let source = if target == DocumentTarget::Latex {
                    latex::render(&plan)
                } else {
                    markdown::render(&plan)
                };
                let altered = source.replace("ordinary", injected);
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "Literal ordinary.".into(),
                    site: input::site("fixture.toml"),
                };
                let mut actual_paragraph = paragraph.clone();
                actual_paragraph.value = format!("Literal {injected}.");
                let marker = if target == DocumentTarget::Latex {
                    "% unit "
                } else {
                    "<!-- unit "
                };
                let expected = crate::documents::failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    AssemblySite::Generated {
                        path: format!(
                            "generated/documents/main/en.{}",
                            if target == DocumentTarget::Latex {
                                "tex"
                            } else {
                                "md"
                            }
                        ),
                        bytes: altered.find(marker).unwrap()..altered.len(),
                        origins: vec![SourceOrigin {
                            unit: Some(UNIT.into()),
                            language: Some("en".into()),
                            target: Some(target),
                            site: input::site("fixture.toml"),
                        }],
                    },
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph,
                            actual: actual_paragraph,
                        }],
                    },
                );
                let actual = check_emission(&plan, &altered, target).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}
#[test]
fn reader_citations_keep_both_address_families_and_span_indices() {
    context(
        "Literal ``ordinary``. @tag{c:1} and `def:langevin:coefficient-one`; see `dec:fixture:source` and `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, welded, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let identities = block
                .runs
                .iter()
                .filter_map(|run| {
                    if let Inline::Citation { identity, .. } = run {
                        Some((
                            identity.label.as_str(),
                            identity.handle.as_str(),
                            identity.token.as_str(),
                            identity.resolved.clone(),
                            identity.position.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                identities,
                vec![
                    (
                        "def:langevin:coefficient-one",
                        "c:1",
                        "u0-c1",
                        Some(AddressTarget::Theory(NodeId::LangevinCoefficientOne)),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 1,
                            target: None
                        }
                    ),
                    (
                        LABEL,
                        "record",
                        "u0-c2",
                        Some(AddressTarget::Documentary(LABEL.into())),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 2,
                            target: None
                        }
                    ),
                    (
                        LABEL,
                        "record",
                        "u0-c3",
                        Some(AddressTarget::Documentary(LABEL.into())),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 3,
                            target: None
                        }
                    )
                ]
            );
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                vec!["def:langevin:coefficient-one", LABEL, LABEL]
            );
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
                assert_eq!(
                    if target == DocumentTarget::Latex {
                        latex::size(&plan)
                    } else {
                        markdown::size(&plan)
                    },
                    source.len()
                );
            }
        },
    );
}
#[test]
fn reader_citation_missing_live_handle_retains_exact_target_and_origins() {
    context(
        "See `dec:fixture:source`.",
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let site = UnitSite {
                file: "fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: Some(4..24),
                occurrence: None,
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site,
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    file: "fixture.md".into(),
                                    path: vec![],
                                    anchor: SourceAnchor::Bytes(11, 34),
                                    decoded: None,
                                    occurrence: None,
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Documentary(LABEL.into()),
                        reason: ReaderIdentityReason::MissingTag,
                        candidates: vec![],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        "See `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            assert_eq!(
                check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                Ok(())
            );
        },
    );
}
#[test]
fn reader_citation_handle_encoding_and_independent_recovery_are_exact() {
    for (handle, encoded) in [
        ("record", "record"),
        ("é_#$&^~", r"\FtdocHandle{233,95,35,36,38,94,126}"),
    ] {
        context(
            "See `dec:fixture:source`.",
            &ledger(handle, LABEL),
            |book, _, _, mut block| {
                block.map_citations(book, "main", "en").unwrap();
                let mut native = String::new();
                latex::inline(&mut native, &block.runs);
                assert_eq!(
                    native,
                    format!("See \\FtdocTag{{{encoded}}}{{u0-c0}}% cite {LABEL}\n.")
                );
                assert_eq!(
                    recover_latex(&native),
                    Some(vec![
                        Recovered::Text("See ".into()),
                        Recovered::Citation {
                            handle: handle.into(),
                            label: LABEL.into(),
                            token: Some("u0-c0".into())
                        },
                        Recovered::Text(".".into())
                    ])
                );
                let plan = plan(block);
                for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                    let source = if target == DocumentTarget::Latex {
                        latex::render(&plan)
                    } else {
                        markdown::render(&plan)
                    };
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            },
        );
    }
    for encoded in [
        r"\FtdocHandle{0233}}",
        r"\FtdocHandle{1114112}}",
        r"\FtdocHandle{233,}}",
        r"\FtdocHandle{}}",
        r"re\allowbreak{}cord}",
    ] {
        assert_eq!(recover_handle(encoded), None);
    }
}

#[test]
fn reader_label_literals_refuse_text_and_mentions_without_changing_the_weld() {
    for (text, bytes, decoded) in [
        ("Literal dec:fixture:source.", 8..26, None),
        ("Literal ``dec:fixture:source``.", 10..28, Some(8..30)),
    ] {
        context(text, "schema = 1\nevents = []\n", |_, unit, welded, _| {
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                Vec::<&str>::new()
            );
            let mut errors = vec![];
            literal_region(
                unit,
                "en",
                None,
                text,
                unit.texts()["en"].site(),
                &assembly_text_spans(&unit.texts()["en"]),
                &[],
                0,
                &mut errors,
            );
            let expected = AssemblyDiagnostic {
                rule: RuleId::M055,
                stage: AssemblyStage::Emission,
                document: Some("main".into()),
                unit: Some(UNIT.into()),
                language: Some("en".into()),
                target: None,
                site: AssemblySite::Source(UnitSite {
                    file: "fixture.toml".into(),
                    path: vec![],
                    anchor: SourceAnchor::WholeFile,
                    decoded,
                    occurrence: None,
                }),
                fault: AssemblyFault::ReaderLabelLiteral {
                    value: LABEL.into(),
                    bytes,
                },
            };
            assert_eq!(errors, vec![expected.clone()]);
            assert_eq!(errors[0].to_string(), expected.to_string());
        });
        context(
            "Literal ordinary.",
            "schema = 1\nevents = []\n",
            |_, unit, _, block| {
                let mut errors = vec![];
                literal_region(
                    unit,
                    "en",
                    None,
                    unit.texts()["en"].text(),
                    unit.texts()["en"].site(),
                    &[],
                    &[],
                    0,
                    &mut errors,
                );
                assert_eq!(errors, vec![]);
                let plan = plan(block);
                for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                    let source = if target == DocumentTarget::Latex {
                        latex::render(&plan)
                    } else {
                        markdown::render(&plan)
                    };
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            },
        );
    }
    assert_eq!(
        reader_label_literals(
            "(dec:fixture:source), other:text and ABCdec:fixture:source and dec:fixture:source_suffix."
        ),
        vec![1..19]
    );
}

#[test]
fn citation_emission_checks_handles_comments_tokens_and_breaks_independently() {
    context(
        "See `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, _, mut block| {
            block.map_citations(book, "main", "en").unwrap();
            let plan = plan(block);
            let authored = UnitSite {
                file: "fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: Some(4..24),
                occurrence: None,
            };
            let identity = crate::documents::CitationIdentity {
                label: LABEL.into(),
                resolved: Some(AddressTarget::Documentary(LABEL.into())),
                handle: "record".into(),
                token: "u0-c0".into(),
                position: ReaderIdentityPosition::Citation {
                    unit: UNIT.into(),
                    span: 0,
                    target: None,
                },
                mint: Some(MintOrigin {
                    kind: MintKind::Markdown,
                    site: LabelSite::Markdown {
                        file: "fixture.md".into(),
                        bytes: 11..34,
                    },
                }),
            };
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                let source = if target == DocumentTarget::Markdown {
                    markdown::render(&plan)
                } else {
                    latex::render(&plan)
                };
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: format!("See `{LABEL}`."),
                    site: input::site("fixture.toml"),
                };
                let citation = EmissionRecord {
                    kind: EmissionKind::Citation(Box::new(identity.clone())),
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: LABEL.into(),
                    site: authored.clone(),
                };
                let origins = vec![
                    SourceOrigin {
                        unit: Some(UNIT.into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: input::site("fixture.toml"),
                    },
                    SourceOrigin {
                        unit: Some(UNIT.into()),
                        language: Some("en".into()),
                        target: None,
                        site: authored.clone(),
                    },
                    SourceOrigin {
                        unit: None,
                        language: None,
                        target: None,
                        site: UnitSite {
                            file: "fixture.md".into(),
                            path: vec![],
                            anchor: SourceAnchor::Bytes(11, 34),
                            decoded: None,
                            occurrence: None,
                        },
                    },
                ];
                let use_bytes = if target == DocumentTarget::Markdown {
                    format!("record<!-- cite {LABEL} -->")
                } else {
                    format!("\\FtdocTag{{record}}{{u0-c0}}% cite {LABEL}\n")
                };
                let mut cases = vec![];
                let other_label = "dec:fixture:other";
                let mut wrong_citation = citation.clone();
                wrong_citation.value = other_label.into();
                if let EmissionKind::Citation(ref mut identity) = wrong_citation.kind {
                    identity.label = other_label.into();
                    identity.resolved = None;
                    identity.mint = None;
                }
                let mut wrong_paragraph = paragraph.clone();
                wrong_paragraph.value = format!("See `{other_label}`.");
                cases.push((
                    source.replace(&format!("cite {LABEL}"), &format!("cite {other_label}")),
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![wrong_citation],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: wrong_paragraph,
                        }],
                    },
                ));
                if target == DocumentTarget::Markdown {
                    let mut actual = paragraph.clone();
                    actual.value = format!("See other<!-- cite {LABEL} -->.");
                    cases.push((
                        source.replace("record<!--", "other<!--"),
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                } else {
                    let old = format!("\\texttt{{{LABEL}}}% cite {LABEL}\n");
                    let mut actual = paragraph.clone();
                    actual.value = format!("See {old}.");
                    cases.push((
                        source.replace(&use_bytes, &old),
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                    let plans = std::slice::from_ref(&plan);
                    let runtime = crate::documents::runtime::render(book, plans).unwrap();
                    let row = runtime
                        .lines()
                        .find(|line| line.starts_with("\\prop_gput:Nxx \\g_ftdoc_sites_prop"))
                        .unwrap()
                        .to_owned()
                        + "\n";
                    let begin = runtime.find(&row).unwrap();
                    let record = EmissionRecord {
                        kind: EmissionKind::Tag,
                        unit: "main/en/u0-c0".into(),
                        language: "sites".into(),
                        target,
                        occurrence: None,
                        value: UNIT.into(),
                        site: UnitSite {
                            path: vec![
                                PathPart::Key("sites".into()),
                                PathPart::Key("main/en/u0-c0".into()),
                            ],
                            ..input::site("generated/addressing.tex")
                        },
                    };
                    for duplicate in [false, true] {
                        let altered = runtime.replace(
                            &row,
                            &if duplicate {
                                row.repeat(2)
                            } else {
                                String::new()
                            },
                        );
                        let expected = crate::documents::failure(
                            RuleId::M055,
                            AssemblyStage::Emission,
                            AssemblySite::Generated {
                                path: "generated/addressing.tex".into(),
                                bytes: if duplicate {
                                    begin + row.len()..begin + 2 * row.len()
                                } else {
                                    begin..begin
                                },
                                origins: origins.clone(),
                            },
                            AssemblyFault::EmissionMismatch {
                                missing: if duplicate {
                                    vec![]
                                } else {
                                    vec![record.clone()]
                                },
                                unexpected: if duplicate {
                                    vec![record.clone()]
                                } else {
                                    vec![]
                                },
                                changed: vec![],
                            },
                        );
                        let actual =
                            crate::documents::runtime::check(book, plans, &altered).unwrap_err();
                        assert_eq!(actual, expected);
                        assert_eq!(actual.to_string(), expected.to_string());
                        assert_eq!(
                            crate::documents::runtime::check(book, plans, &runtime),
                            Ok(())
                        );
                    }
                }
                if target == DocumentTarget::Latex {
                    for (from, to, changed_handle) in
                        [("{record}", "{other}", true), ("{u0-c0}", "{u0-c1}", false)]
                    {
                        let altered = source.replace(from, to);
                        let mut actual = citation.clone();
                        let EmissionKind::Citation(ref mut found) = actual.kind else {
                            unreachable!()
                        };
                        if changed_handle {
                            found.handle = "other".into();
                        } else {
                            found.token = "u0-c1".into();
                        }
                        cases.push((
                            altered,
                            AssemblyFault::EmissionMismatch {
                                missing: vec![citation.clone()],
                                unexpected: vec![actual],
                                changed: vec![],
                            },
                        ));
                    }
                }
                let repeated = source.replace(&use_bytes, &use_bytes.repeat(2));
                let mut repeated_paragraph = paragraph.clone();
                repeated_paragraph.value = format!("See `{LABEL}``{LABEL}`.");
                cases.push((
                    repeated,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![citation.clone()],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: repeated_paragraph,
                        }],
                    },
                ));
                let dropped = source.replace(&use_bytes, "");
                let mut dropped_paragraph = paragraph.clone();
                dropped_paragraph.value = "See .".into();
                cases.push((
                    dropped,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: dropped_paragraph,
                        }],
                    },
                ));
                let wrong_comment =
                    source.replace("cite dec:fixture:source", "label dec:fixture:source");
                let mut as_tag = citation.clone();
                as_tag.kind = EmissionKind::Tag;
                // A recovered tag has no planned authored-tag site to borrow.
                as_tag.site = input::site("fixture.toml");
                let mut tag_paragraph = paragraph.clone();
                tag_paragraph.value = "See @tag{record}.".into();
                cases.push((
                    wrong_comment,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![as_tag],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: tag_paragraph,
                        }],
                    },
                ));
                if target == DocumentTarget::Latex {
                    let altered = source.replace("{record}", r"{re\allowbreak{}cord}");
                    let mut actual = paragraph.clone();
                    actual.value = format!(
                        "See \\FtdocTag{{re\\allowbreak{{}}cord}}{{u0-c0}}% cite {LABEL}\n."
                    );
                    cases.push((
                        altered,
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                }
                for (altered, fault) in cases {
                    let marker = if target == DocumentTarget::Markdown {
                        "<!-- unit "
                    } else {
                        "% unit "
                    };
                    let expected = crate::documents::failure(
                        RuleId::M055,
                        AssemblyStage::Emission,
                        AssemblySite::Generated {
                            path: format!(
                                "generated/documents/main/en.{}",
                                if target == DocumentTarget::Markdown {
                                    "md"
                                } else {
                                    "tex"
                                }
                            ),
                            bytes: altered.find(marker).unwrap()..altered.len(),
                            origins: origins.clone(),
                        },
                        fault,
                    );
                    let actual = check_emission(&plan, &altered, target).unwrap_err();
                    assert_eq!(actual, expected, "{altered}");
                    assert_eq!(actual.to_string(), expected.to_string());
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            }
        },
    );
}

#[test]
fn unit_metadata_citations_keep_their_own_mint_and_live_handle() {
    context(
        "See `sec:fixture:reader`.",
        &ledger("self", UNIT),
        |book, _, welded, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let Inline::Citation {
                identity,
                site,
                text,
            } = &block.runs[1]
            else {
                panic!("citation")
            };
            assert_eq!(
                identity,
                &crate::documents::CitationIdentity {
                    label: UNIT.into(),
                    resolved: Some(AddressTarget::Documentary(UNIT.into())),
                    handle: "self".into(),
                    token: "u0-c0".into(),
                    position: ReaderIdentityPosition::Citation {
                        unit: UNIT.into(),
                        span: 0,
                        target: None
                    },
                    mint: Some(MintOrigin {
                        kind: MintKind::UnitMetadata,
                        site: LabelSite::Unit {
                            unit: UNIT.into(),
                            site: UnitSite {
                                file: "fixture.toml".into(),
                                path: vec![
                                    PathPart::Key("unit".into()),
                                    PathPart::Key("id".into())
                                ],
                                anchor: SourceAnchor::Unavailable,
                                decoded: None,
                                occurrence: None
                            }
                        }
                    })
                }
            );
            assert_eq!(text, UNIT);
            assert_eq!(
                site,
                &UnitSite {
                    decoded: Some(4..24),
                    ..input::site("fixture.toml")
                }
            );
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                vec![UNIT]
            );
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn retired_documentary_citation_keeps_the_complete_event_and_restores() {
    let live = ledger("record", LABEL);
    let retired = format!(
        "{live}[[events]]\nordinal = 2\nchange = \"retire\"\nhandle = \"record\"\nauthority-record = \"dec:fixture:authority\"\nreason = \"Fixture retirement.\"\n"
    );
    context(
        "See `dec:fixture:source`.",
        &retired,
        |book, _, _, mut block| {
            let event = DocumentaryRetirement {
                ordinal: 2,
                authority_record: "dec:fixture:authority".into(),
                reason: "Fixture retirement.".into(),
                site: UnitSite {
                    file: "fixture-registry.toml".into(),
                    path: vec![PathPart::Key("events".into()), PathPart::Index(1)],
                    anchor: SourceAnchor::Bytes(live.len(), retired.len() - 1),
                    decoded: None,
                    occurrence: None,
                },
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site: UnitSite {
                                    decoded: Some(4..24),
                                    ..input::site("fixture.toml")
                                },
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    file: "fixture.md".into(),
                                    path: vec![],
                                    anchor: SourceAnchor::Bytes(11, 34),
                                    decoded: None,
                                    occurrence: None,
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Documentary(LABEL.into()),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: vec![ReaderTagCandidate {
                            handle: "record".into(),
                            outcome: Err(TagResolutionFault::RetiredDocumentaryTag {
                                input: "record".into(),
                                label: LABEL.into(),
                                event,
                            }),
                        }],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        "See `dec:fixture:source`.",
        &live,
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn display_citation_sites_count_mentions_across_regions_and_languages() {
    let graph = seed().unwrap();
    let site = input::site("display-fixture.toml");
    let located = |s: &str| LocatedString {
        value: s.into(),
        site: site.clone(),
    };
    let languages = || {
        ["en", "fr"]
            .into_iter()
            .map(|language| {
                (
                    language.into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("Caption."),
                    },
                )
            })
            .collect()
    };
    let corpus = admit_unit_drafts(
        DocumentDraft {
            documents_site: site.clone(),
            documents: BTreeMap::from([
                (
                    "main".into(),
                    DocumentEntryDraft {
                        key_site: site.clone(),
                        table_site: site.clone(),
                        languages_site: site.clone(),
                        required_languages: vec![located("en")],
                    },
                ),
                (
                    "fixture".into(),
                    DocumentEntryDraft {
                        key_site: site.clone(),
                        table_site: site.clone(),
                        languages_site: site.clone(),
                        required_languages: vec![located("en"), located("fr")],
                    },
                ),
            ]),
        },
        ["sec:fixture:heading", UNIT]
            .into_iter()
            .map(|id| UnitDraft {
                id: located(id),
                document: located("fixture"),
                table_site: site.clone(),
                math_site: site.clone(),
                texts_site: site.clone(),
                math: BTreeMap::new(),
                texts: languages(),
            })
            .chain(std::iter::once(UnitDraft {
                id: located("sec:main:heading"),
                document: located("main"),
                table_site: site.clone(),
                math_site: site.clone(),
                texts_site: site.clone(),
                math: BTreeMap::new(),
                texts: BTreeMap::from([(
                    "en".into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("Heading."),
                    },
                )]),
            }))
            .collect(),
    )
    .unwrap();
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"fixture.md".into(),bytes:b"# Decision \xc2\xb7 `dec:fixture:source`\n# Authority \xc2\xb7 `dec:fixture:authority`\n# Kernel \xc2\xb7 `proposal:manifest:kernel-bite`\n# Lifecycle \xc2\xb7 `proposal:manifest:addressing-lifecycle-bite`\n".to_vec()}]).unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let events = registry::load(&crate::UnitSource {
        file: "fixture-registry.toml".into(),
        bytes: ledger("record", LABEL).into_bytes(),
    })
    .unwrap();
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, events)
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_joint_labels(&resolved, &joint).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let metadata = corpus
        .units()
        .iter()
        .map(|u| (u.label().as_str().into(), site.clone()))
        .collect();
    let skeletons = check_skeletons(
        &corpus,
        &[
            SkeletonDraft {
                document: located("main"),
                sections_site: site.clone(),
                sections: vec![SkeletonSection {
                    heading: located("sec:main:heading"),
                    children: vec![],
                }],
            },
            SkeletonDraft {
                document: located("fixture"),
                sections_site: site.clone(),
                sections: vec![SkeletonSection {
                    heading: located("sec:fixture:heading"),
                    children: vec![SkeletonChild::Unit(located(UNIT))],
                }],
            },
        ],
        &metadata,
        &metadata,
        &BTreeMap::from([
            ("fixture".into(), site.clone()),
            ("main".into(), site.clone()),
        ]),
    )
    .unwrap();
    let drafts=[DisplayDraft {unit:located(UNIT),kind:DisplayKind::Table,bodies:[(DocumentTarget::Markdown,"| ``ordinary`` | `dec:fixture:source` |\n| --- | --- |\n| `dec:fixture:source` | ``tail`` |"),(DocumentTarget::Latex,r"\begin{tabular}{ll} ``ordinary`` & `dec:fixture:source` \\ `dec:fixture:source` & ``tail`` \\ \end{tabular}")].into_iter().map(|(target,text)|(target,DisplayTargetDraft {site:site.clone(),languages:["en","fr"].into_iter().map(|language|(language.into(),located(text))).collect()})).collect()}];
    let displays = check_displays(&drafts, &skeletons, &welded).unwrap();
    for language in ["en", "fr"] {
        let mut errors = vec![];
        check_reader_literals(&corpus.units()[1], language, &displays, &mut errors);
        assert_eq!(errors, vec![]);
    }
    let mut altered_drafts = drafts.clone();
    for body in altered_drafts[0].bodies.values_mut() {
        for source in body.languages.values_mut() {
            source.value = source.value.replace("``ordinary``", "dec:fixture:source");
        }
    }
    let altered = check_displays(&altered_drafts, &skeletons, &welded).unwrap();
    for language in ["en", "fr"] {
        let mut actual = vec![];
        check_reader_literals(&corpus.units()[1], language, &altered, &mut actual);
        let expected: Vec<_> = [DocumentTarget::Markdown, DocumentTarget::Latex]
            .into_iter()
            .map(|target| {
                let start = if target == DocumentTarget::Latex {
                    20
                } else {
                    2
                };
                AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("fixture".into()),
                    unit: Some(UNIT.into()),
                    language: Some(language.into()),
                    target: Some(target),
                    site: AssemblySite::Source(UnitSite {
                        path: vec![
                            PathPart::Key("regions".into()),
                            PathPart::Index(0),
                            PathPart::Index(0),
                        ],
                        ..site.clone()
                    }),
                    fault: AssemblyFault::ReaderLabelLiteral {
                        value: LABEL.into(),
                        bytes: start..start + 18,
                    },
                }
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.iter().map(ToString::to_string).collect::<Vec<_>>(),
            expected.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
        let reread = check_displays(&drafts, &skeletons, &welded).unwrap();
        let mut restored = vec![];
        check_reader_literals(&corpus.units()[1], language, &reread, &mut restored);
        assert_eq!(restored, vec![]);
    }

    let unit = &corpus.units()[1];
    for language in ["en", "fr"] {
        let mut block = Block {
            unit: UNIT.into(),
            depth: None,
            site: site.clone(),
            runs: vec![Inline::Text("Caption.".into())],
            pending_names: BTreeMap::new(),
            displays: display_plans(1, unit, language, &welded, &BTreeMap::new(), &displays),
        };
        assert_eq!(block.map_citations(&book, "fixture", language), Ok(()));
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let name = if target == DocumentTarget::Markdown {
                "markdown"
            } else {
                "latex"
            };
            let found = block.displays[&target]
                .regions
                .iter()
                .flat_map(|r| &r.runs)
                .filter_map(|r| {
                    if let Inline::Citation { identity, .. } = r {
                        Some((
                            identity.token.clone(),
                            identity.position.clone(),
                            identity.label.clone(),
                            identity.handle.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                found,
                [1, 2].map(|span| (
                    format!("u1-d{name}-{language}-c{span}"),
                    ReaderIdentityPosition::Citation {
                        unit: UNIT.into(),
                        span,
                        target: Some(target)
                    },
                    LABEL.into(),
                    "record".into()
                ))
            );
        }
        let mut plan = plan(block);
        plan.document = "fixture".into();
        plan.language = language.into();
        for (target, source) in [
            (DocumentTarget::Markdown, markdown::render(&plan)),
            (DocumentTarget::Latex, latex::render(&plan)),
        ] {
            assert_eq!(check_emission(&plan, &source, target), Ok(()));
        }
    }
}

#[test]
fn literal_checker_protects_admitted_citation_tag_and_math_ranges() {
    context(
        "Plain text.",
        "schema = 1\nevents = []\n",
        |_, _, welded, _| {
            let site = input::site("fixture.toml");
            let located = |value: &str| LocatedString {
                value: value.into(),
                site: site.clone(),
            };
            let metadata = BTreeMap::from([(UNIT.into(), site.clone())]);
            let skeletons = check_skeletons(
                welded.integrated().resolved().corpus(),
                &[SkeletonDraft {
                    document: located("main"),
                    sections_site: site.clone(),
                    sections: vec![SkeletonSection {
                        heading: located(UNIT),
                        children: vec![],
                    }],
                }],
                &metadata,
                &metadata,
                &BTreeMap::from([("main".into(), site.clone())]),
            )
            .unwrap();
            let displays = check_displays(&[], &skeletons, welded).unwrap();
            let text = "See `dec:fixture:source` @tag{dec:fixture:source} @math{value}.";
            let corpus = literal_corpus(
                text,
                BTreeMap::from([(
                    "value".into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("x"),
                    },
                )]),
            );
            let unit = &corpus.units()[0];
            assert_eq!(
                assembly_text_spans(&unit.texts()["en"])
                    .into_iter()
                    .map(|(label, site, citation)| (label, site.decoded, citation))
                    .collect::<Vec<_>>(),
                vec![(LABEL.into(), Some(4..24), true)]
            );
            assert_eq!(
                unit.texts()["en"]
                    .tag_occurrences()
                    .iter()
                    .map(|o| (o.input(), o.site().decoded.clone()))
                    .collect::<Vec<_>>(),
                vec![(LABEL, Some(25..49))]
            );
            assert_eq!(
                unit.texts()["en"]
                    .occurrences()
                    .iter()
                    .map(|o| (o.name(), o.site().decoded.clone()))
                    .collect::<Vec<_>>(),
                vec![("value", Some(50..62))]
            );
            let mut errors = vec![];
            check_reader_literals(unit, "en", &displays, &mut errors);
            assert_eq!(errors, vec![]);
        },
    );
}
