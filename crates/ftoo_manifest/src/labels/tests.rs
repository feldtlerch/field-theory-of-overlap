use super::*;
use crate::{
    DocumentDraft, DocumentEntryDraft, LanguageCheckedUnits, LocatedEntryDraft, LocatedString,
    UnitDraft,
};
use std::fmt::Write;

const RECORDS: &str =
    "· `proposal:manifest:kernel-bite`\n· `proposal:manifest:addressing-lifecycle-bite`\n";
fn corpus(label: &str) -> LanguageCheckedUnits {
    corpus_labels(&[label])
}
fn corpus_labels(labels: &[&str]) -> LanguageCheckedUnits {
    let site = UnitSite {
        file: "units/statements/test.toml".into(),
        path: vec![],
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: site.clone(),
    };
    crate::admit_unit_drafts(
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
        labels
            .iter()
            .map(|label| UnitDraft {
                id: located(label),
                document: located("main"),
                table_site: site.clone(),
                math_site: site.clone(),
                texts_site: site.clone(),
                math: BTreeMap::new(),
                texts: BTreeMap::from([(
                    "en".into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("The coefficient is considered."),
                    },
                )]),
            })
            .collect(),
    )
    .unwrap()
}
fn identity_origin() -> MintOrigin {
    MintOrigin {
        kind: MintKind::RustIdentity,
        site: LabelSite::RustIdentity {
            file: "crates/ftoo_manifest/src/identity.rs".into(),
            identity: NodeId::LangevinCoefficientOne,
            field: "semantic_name".into(),
        },
    }
}
fn metadata_origin(label: &str) -> MintOrigin {
    MintOrigin {
        kind: MintKind::UnitMetadata,
        site: LabelSite::Unit {
            unit: label.into(),
            site: UnitSite {
                file: "units/statements/test.toml".into(),
                path: vec![PathPart::Key("unit".into()), PathPart::Key("id".into())],
                anchor: SourceAnchor::Unavailable,
                decoded: None,
                occurrence: None,
            },
        },
    }
}
fn duplicate(label: &str, origins: Vec<MintOrigin>) -> LabelDiagnostic {
    LabelDiagnostic {
        rule: RuleId::M043,
        stage: LabelStage::Namespace,
        unit: None,
        language: None,
        site: origins[0].site.clone(),
        fault: LabelFault::DuplicateMint {
            label: label.into(),
            expected: 1,
            actual: origins.len(),
            origins,
        },
    }
}

#[test]
fn w04_permanent_mint_survives_empty_slice_and_absent_identity() {
    let directory =
        std::env::temp_dir().join(format!("ftoo-permanent-label-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("docs")).unwrap();
    let path = directory.join("docs/weld-collision.md");
    let bytes = b"# Collision \xc2\xb7 `def:langevin:coefficient-one`\n";
    let corpus = corpus_labels(&[
        "sec:coefficients:first-value",
        "sec:coefficients:value-context",
    ]);
    for absent in [false, true] {
        let mut draft = crate::seed_draft();
        draft.coverage.slice.clear();
        if absent {
            draft
                .nodes
                .retain(|n| n.id != NodeId::LangevinCoefficientOne);
            draft.edges.retain(|e| {
                e.source != NodeId::LangevinCoefficientOne
                    && e.target != NodeId::LangevinCoefficientOne
            });
        }
        let graph = crate::validate(draft).unwrap();
        let book = crate::addressing(&graph).unwrap();
        let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
        assert!(resolved.tags().is_empty());
        let clean = check_markdown_sources(vec![source("records.md", RECORDS)]).unwrap();
        assert_eq!(
            integrate_labels(&resolved, &clean).unwrap().mints().len(),
            8
        );
        std::fs::write(&path, bytes).unwrap();
        let markdown = check_markdown_sources(vec![
            source("records.md", RECORDS),
            MarkdownSource {
                file: "docs/weld-collision.md".into(),
                bytes: std::fs::read(&path).unwrap(),
            },
        ])
        .unwrap();
        let error = integrate_labels(&resolved, &markdown).unwrap_err();
        assert_eq!(
            error.diagnostics(),
            [duplicate(
                "def:langevin:coefficient-one",
                vec![
                    identity_origin(),
                    MintOrigin {
                        kind: MintKind::Markdown,
                        site: markdown_site("docs/weld-collision.md", 12..45)
                    }
                ]
            )]
        );
        assert_eq!(
            error.to_string(),
            "M043 Namespace unit=None language=None at rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\": DuplicateMint { label: \"def:langevin:coefficient-one\", expected: 1, actual: 2, origins: [MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }, MintOrigin { kind: Markdown, site: Markdown { file: \"docs/weld-collision.md\", bytes: 12..45 } }] }\n"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(&path).unwrap();
        let restored = check_markdown_sources(vec![source("records.md", RECORDS)]).unwrap();
        assert_eq!(
            integrate_labels(&resolved, &restored).unwrap().mints(),
            integrate_labels(&resolved, &clean).unwrap().mints()
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn w04_metadata_and_three_way_collisions_retain_honest_unavailable_sites() {
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    for (label, markdown, origins) in [
        (
            "def:langevin:coefficient-one",
            RECORDS.to_owned(),
            vec![
                identity_origin(),
                metadata_origin("def:langevin:coefficient-one"),
            ],
        ),
        (
            "dec:weld:equal-address-sets",
            format!("· `dec:weld:equal-address-sets`\n{RECORDS}"),
            vec![
                MintOrigin {
                    kind: MintKind::Markdown,
                    site: markdown_site("records.md", 0..32),
                },
                metadata_origin("dec:weld:equal-address-sets"),
            ],
        ),
        (
            "def:langevin:coefficient-one",
            format!("· `def:langevin:coefficient-one`\n{RECORDS}"),
            vec![
                identity_origin(),
                MintOrigin {
                    kind: MintKind::Markdown,
                    site: markdown_site("records.md", 0..33),
                },
                metadata_origin("def:langevin:coefficient-one"),
            ],
        ),
    ] {
        let corpus = corpus(label);
        let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
        let markdown = check_markdown_sources(vec![source("records.md", &markdown)]).unwrap();
        let error = integrate_labels(&resolved, &markdown).unwrap_err();
        let count = origins.len();
        assert_eq!(error.diagnostics(), [duplicate(label, origins)]);
        let metadata = format!(
            "MintOrigin {{ kind: UnitMetadata, site: Unit {{ unit: {label:?}, site: UnitSite {{ file: \"units/statements/test.toml\", path: [Key(\"unit\"), Key(\"id\")], anchor: Unavailable, decoded: None, occurrence: None }} }} }}"
        );
        let identity = "MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }";
        let (site, origins) = if label == "dec:weld:equal-address-sets" {
            (
                "markdown \"records.md\" bytes 0..32",
                format!(
                    "MintOrigin {{ kind: Markdown, site: Markdown {{ file: \"records.md\", bytes: 0..32 }} }}, {metadata}"
                ),
            )
        } else if count == 3 {
            (
                "rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\"",
                format!(
                    "{identity}, MintOrigin {{ kind: Markdown, site: Markdown {{ file: \"records.md\", bytes: 0..33 }} }}, {metadata}"
                ),
            )
        } else {
            (
                "rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\"",
                format!("{identity}, {metadata}"),
            )
        };
        assert_eq!(
            error.to_string(),
            format!(
                "M043 Namespace unit=None language=None at {site}: DuplicateMint {{ label: {label:?}, expected: 1, actual: {count}, origins: [{origins}] }}\n"
            )
        );
    }
}

#[test]
fn w04_private_duplicate_export_is_counted_before_indexing() {
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    let corpus = corpus("sec:fixture:unit");
    let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
    let markdown = check_markdown_sources(vec![source("records.md", RECORDS)]).unwrap();
    let mints = joint_mints(
        &resolved,
        &markdown,
        &[
            NodeId::LangevinCoefficientOne,
            NodeId::LangevinCoefficientOne,
        ],
    );
    assert_eq!(
        duplicates(&mints, LabelStage::Namespace),
        [duplicate(
            "def:langevin:coefficient-one",
            vec![identity_origin(), identity_origin()]
        )]
    );
    assert_eq!(
        LabelErrors(duplicates(&mints, LabelStage::Namespace)).to_string(),
        "M043 Namespace unit=None language=None at rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\": DuplicateMint { label: \"def:langevin:coefficient-one\", expected: 1, actual: 2, origins: [MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }, MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }] }\n"
    );
}

#[test]
fn w05_private_well_shaped_authority_and_retirements_are_resolved() {
    let mut draft = crate::seed_draft();
    draft.coverage.slice.clear();
    draft
        .nodes
        .retain(|n| n.id != NodeId::LangevinCoefficientThree);
    draft.edges.retain(|e| {
        e.source != NodeId::LangevinCoefficientThree && e.target != NodeId::LangevinCoefficientThree
    });
    let graph = crate::validate(draft).unwrap();
    let corpus = corpus("sec:fixture:unit");
    let markdown = check_markdown_sources(vec![source("records.md", RECORDS)]).unwrap();
    let mut history = crate::tag_history::seed_history();
    history[0].authority_record = "proposal:fixture:missing".into();
    for (ordinal, change) in [
        (
            9,
            HistoryChange::RetireTag {
                tag: crate::TagId::LangevinCoefficientOne,
            },
        ),
        (
            10,
            HistoryChange::RetireIdentity {
                identity: NodeId::LangevinCoefficientThree,
            },
        ),
    ] {
        history.push(crate::HistoryEvent {
            ordinal,
            authority_record: "proposal:fixture:missing".into(),
            reason: "Fixture retirement.".into(),
            change,
        });
    }
    let book = crate::tag_validation::construct(
        &graph,
        crate::TagId::ALL.map(crate::TagId::descriptor).to_vec(),
        history,
    )
    .unwrap();
    let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
    let error = integrate_labels(&resolved, &markdown).unwrap_err();
    let expected: Vec<_> = [1, 9, 10]
        .into_iter()
        .map(|ordinal| LabelDiagnostic {
            rule: RuleId::M044,
            stage: LabelStage::RustCitation,
            unit: None,
            language: None,
            site: LabelSite::RustHistory {
                file: "crates/ftoo_manifest/src/tag_history.rs".into(),
                ordinal,
                field: "authority_record".into(),
            },
            fault: LabelFault::UnresolvedRustCitation {
                label: "proposal:fixture:missing".into(),
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::MarkdownOnly,
            },
        })
        .collect();
    assert_eq!(error.diagnostics(), expected);
    let mut display = String::new();
    for ordinal in [1, 9, 10] {
        writeln!(display,"M044 RustCitation unit=None language=None at rust-history \"crates/ftoo_manifest/src/tag_history.rs\" event {ordinal} field \"authority_record\": UnresolvedRustCitation {{ label: \"proposal:fixture:missing\", expected: 1, actual: 0, domain: MarkdownOnly }}").unwrap();
    }
    assert_eq!(error.to_string(), display);
    let book = crate::addressing(&graph).unwrap();
    let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
    assert_eq!(
        integrate_labels(&resolved, &markdown)
            .unwrap()
            .rust_citations()
            .len(),
        8
    );
}

fn source(file: &str, text: &str) -> MarkdownSource {
    MarkdownSource {
        file: file.into(),
        bytes: text.as_bytes().to_vec(),
    }
}
#[test]
fn w01_shared_literal_harvest_inventory() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/fixtures/label-harvest.json"
    ))
    .unwrap();
    let ranges = [
        (vec![(22, 32)], vec![]),
        (vec![(0, 10)], vec![(12, 19), (20, 27)]),
        (vec![], vec![]),
        (vec![], vec![(10, 17)]),
        (vec![], vec![(1, 8), (9, 16)]),
        (vec![], vec![(2, 9), (13, 20)]),
        (vec![(4, 14), (24, 34)], vec![(36, 43), (53, 60)]),
        (vec![(1, 11)], vec![(4, 11)]),
        (vec![(24, 34), (35, 45)], vec![(1, 8), (15, 22)]),
        (vec![(4, 14)], vec![(17, 24)]),
    ];
    assert_eq!(cases.as_array().unwrap().len(), ranges.len());
    for (case, (mint_ranges, cite_ranges)) in cases.as_array().unwrap().iter().zip(ranges) {
        let (mints, cites) = harvest(case["text"].as_str().unwrap(), "test.md");
        assert_eq!(
            serde_json::json!(mints.iter().map(|o| &o.label).collect::<Vec<_>>()),
            case["mints"],
            "{}",
            case["name"]
        );
        assert_eq!(
            mints.iter().map(|o| &o.site).collect::<Vec<_>>(),
            mint_ranges
                .into_iter()
                .map(|(start, end)| markdown_site("test.md", start..end))
                .collect::<Vec<_>>()
                .iter()
                .collect::<Vec<_>>(),
            "{}",
            case["name"]
        );
        assert_eq!(
            cites.iter().map(|o| &o.site).collect::<Vec<_>>(),
            cite_ranges
                .into_iter()
                .map(|(start, end)| markdown_site("test.md", start..end))
                .collect::<Vec<_>>()
                .iter()
                .collect::<Vec<_>>(),
            "{}",
            case["name"]
        );
        assert_eq!(
            serde_json::json!(cites.iter().map(|o| &o.label).collect::<Vec<_>>()),
            case["cites"],
            "{}",
            case["name"]
        );
    }
    assert_eq!(
        harvest("é\r\n· `a:b:c`\r(\r`a:b:c`\r\n)", "test.md"),
        (
            vec![LabelOccurrence {
                label: "a:b:c".into(),
                site: markdown_site("test.md", 4..14)
            }],
            vec![LabelOccurrence {
                label: "a:b:c".into(),
                site: markdown_site("test.md", 17..24)
            }]
        )
    );
    assert_eq!(
        harvest("(· `a:b:c`)", "test.md"),
        (
            vec![LabelOccurrence {
                label: "a:b:c".into(),
                site: markdown_site("test.md", 1..11)
            }],
            vec![LabelOccurrence {
                label: "a:b:c".into(),
                site: markdown_site("test.md", 4..11)
            }]
        )
    );
}
#[test]
fn w02_source_names_and_encoding() {
    for name in ["", "/a.md", "a//b.md", "./a.md", "a/../b.md", "a\\b.md"] {
        let error = check_markdown_sources(vec![source(name, "")]).unwrap_err();
        assert_eq!(
            error,
            LabelSourceError::Input(LabelSourceFault::InvalidSourceName { value: name.into() })
        );
        assert_eq!(
            error.to_string(),
            format!("Input(InvalidSourceName {{ value: {name:?} }})")
        );
    }
    let error = check_markdown_sources(vec![
        source("a.md", ""),
        source("b.md", ""),
        source("a.md", ""),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        LabelSourceError::Input(LabelSourceFault::DuplicateSource {
            file: "a.md".into(),
            positions: vec![0, 2]
        })
    );
    assert_eq!(
        error.to_string(),
        "Input(DuplicateSource { file: \"a.md\", positions: [0, 2] })"
    );
    for (bytes, range, fault, display) in [
        (
            vec![b'a', 255],
            1..2,
            LabelFault::InvalidUtf8 {
                offset: 1,
                error_len: Some(1),
            },
            "InvalidUtf8 { offset: 1, error_len: Some(1) }",
        ),
        (
            vec![b'a', 0xe2, 0x82],
            1..3,
            LabelFault::InvalidUtf8 {
                offset: 1,
                error_len: None,
            },
            "InvalidUtf8 { offset: 1, error_len: None }",
        ),
        (vec![0xef, 0xbb, 0xbf, b'a'], 0..3, LabelFault::Bom, "Bom"),
    ] {
        let error = check_markdown_sources(vec![MarkdownSource {
            file: "a.md".into(),
            bytes,
        }])
        .unwrap_err();
        assert_eq!(
            error,
            LabelSourceError::Admission(LabelErrors(vec![LabelDiagnostic {
                rule: RuleId::M001,
                stage: LabelStage::Encoding,
                unit: None,
                language: None,
                site: markdown_site("a.md", range.clone()),
                fault
            }]))
        );
        assert_eq!(
            error.to_string(),
            format!(
                "M001 Encoding unit=None language=None at markdown \"a.md\" bytes {}..{}: {display}\n",
                range.start, range.end
            )
        );
    }
}
#[test]
fn w03_duplicate_mints_precede_all_references() {
    let error =
        check_markdown_sources(vec![source("a.md", "· `a:b:c` · `a:b:c` (`x:y:z`)")]).unwrap_err();
    assert_eq!(
        error,
        LabelSourceError::Admission(LabelErrors(vec![LabelDiagnostic {
            rule: RuleId::M043,
            stage: LabelStage::Markdown,
            unit: None,
            language: None,
            site: markdown_site("a.md", 0..10),
            fault: LabelFault::DuplicateMint {
                label: "a:b:c".into(),
                expected: 1,
                actual: 2,
                origins: vec![
                    MintOrigin {
                        kind: MintKind::Markdown,
                        site: markdown_site("a.md", 0..10)
                    },
                    MintOrigin {
                        kind: MintKind::Markdown,
                        site: markdown_site("a.md", 11..21)
                    }
                ]
            }
        }]))
    );
    assert_eq!(
        error.to_string(),
        "M043 Markdown unit=None language=None at markdown \"a.md\" bytes 0..10: DuplicateMint { label: \"a:b:c\", expected: 1, actual: 2, origins: [MintOrigin { kind: Markdown, site: Markdown { file: \"a.md\", bytes: 0..10 } }, MintOrigin { kind: Markdown, site: Markdown { file: \"a.md\", bytes: 11..21 } }] }\n"
    );
    let checked =
        check_markdown_sources(vec![source("a.md", "· `a:b:c` (`a:b:c` `a:b:c`)")]).unwrap();
    assert_eq!(checked.mints().len(), 1);
    assert_eq!(
        checked
            .citations()
            .iter()
            .map(|o| o.site.clone())
            .collect::<Vec<_>>(),
        vec![markdown_site("a.md", 12..19), markdown_site("a.md", 20..27)]
    );
}
#[test]
fn w03_unresolved_citations_retain_every_site_and_markdown_domain() {
    let error = check_markdown_sources(vec![source("a.md", "(`x:y:z` `x:y:z`)")]).unwrap_err();
    let expected: Vec<_> = [1..8, 9..16]
        .into_iter()
        .map(|bytes| LabelDiagnostic {
            rule: RuleId::M043,
            stage: LabelStage::Markdown,
            unit: None,
            language: None,
            site: markdown_site("a.md", bytes),
            fault: LabelFault::UnresolvedMarkdownCitation {
                label: "x:y:z".into(),
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::MarkdownOnly,
            },
        })
        .collect();
    assert_eq!(error, LabelSourceError::Admission(LabelErrors(expected)));
    assert_eq!(
        error.to_string(),
        concat!(
            "M043 Markdown unit=None language=None at markdown \"a.md\" bytes 1..8: UnresolvedMarkdownCitation { label: \"x:y:z\", expected: 1, actual: 0, domain: MarkdownOnly }\n",
            "M043 Markdown unit=None language=None at markdown \"a.md\" bytes 9..16: UnresolvedMarkdownCitation { label: \"x:y:z\", expected: 1, actual: 0, domain: MarkdownOnly }\n"
        )
    );
}

#[test]
fn w03_multiple_cross_file_duplicates_sort_by_label_and_suppress_references() {
    let error = check_markdown_sources(vec![
        source("z.md", "· `d:e:f` · `a:b:c` (`x:y:z`)"),
        source("a.md", "· `d:e:f` · `a:b:c`"),
    ])
    .unwrap_err();
    let expected: Vec<_> = [("a:b:c", 11..21), ("d:e:f", 0..10)]
        .into_iter()
        .map(|(label, bytes)| LabelDiagnostic {
            rule: RuleId::M043,
            stage: LabelStage::Markdown,
            unit: None,
            language: None,
            site: markdown_site("a.md", bytes.clone()),
            fault: LabelFault::DuplicateMint {
                label: label.into(),
                expected: 1,
                actual: 2,
                origins: vec![
                    MintOrigin {
                        kind: MintKind::Markdown,
                        site: markdown_site("a.md", bytes.clone()),
                    },
                    MintOrigin {
                        kind: MintKind::Markdown,
                        site: markdown_site("z.md", bytes),
                    },
                ],
            },
        })
        .collect();
    assert_eq!(error, LabelSourceError::Admission(LabelErrors(expected)));
    let mut display = String::new();
    for (label, start, end) in [("a:b:c", 11, 21), ("d:e:f", 0, 10)] {
        writeln!(display,"M043 Markdown unit=None language=None at markdown \"a.md\" bytes {start}..{end}: DuplicateMint {{ label: {label:?}, expected: 1, actual: 2, origins: [MintOrigin {{ kind: Markdown, site: Markdown {{ file: \"a.md\", bytes: {start}..{end} }} }}, MintOrigin {{ kind: Markdown, site: Markdown {{ file: \"z.md\", bytes: {start}..{end} }} }}] }}").unwrap();
    }
    assert_eq!(error.to_string(), display);
}

#[test]
fn w03_rust_and_unit_names_cannot_supply_markdown_mints() {
    for (label, end) in [
        ("def:langevin:coefficient-one", 31),
        ("sec:coefficients:first-value", 31),
    ] {
        let error =
            check_markdown_sources(vec![source("a.md", &format!("(`{label}`)"))]).unwrap_err();
        assert_eq!(
            error,
            LabelSourceError::Admission(LabelErrors(vec![LabelDiagnostic {
                rule: RuleId::M043,
                stage: LabelStage::Markdown,
                unit: None,
                language: None,
                site: markdown_site("a.md", 1..end),
                fault: LabelFault::UnresolvedMarkdownCitation {
                    label: label.into(),
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::MarkdownOnly
                }
            }]))
        );
        assert_eq!(
            error.to_string(),
            format!(
                "M043 Markdown unit=None language=None at markdown \"a.md\" bytes 1..{end}: UnresolvedMarkdownCitation {{ label: {label:?}, expected: 1, actual: 0, domain: MarkdownOnly }}\n"
            )
        );
    }
}

#[test]
fn w05_unit_metadata_cannot_replace_a_missing_authority_record() {
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    let corpus = corpus("proposal:manifest:kernel-bite");
    let resolved = crate::resolve_unit_tags(&corpus, &book).unwrap();
    let markdown = check_markdown_sources(vec![source(
        "records.md",
        "· `proposal:manifest:addressing-lifecycle-bite`",
    )])
    .unwrap();
    let error = integrate_labels(&resolved, &markdown).unwrap_err();
    let expected: Vec<_> = (1..=4)
        .map(|ordinal| LabelDiagnostic {
            rule: RuleId::M044,
            stage: LabelStage::RustCitation,
            unit: None,
            language: None,
            site: LabelSite::RustHistory {
                file: "crates/ftoo_manifest/src/tag_history.rs".into(),
                ordinal,
                field: "authority_record".into(),
            },
            fault: LabelFault::UnresolvedRustCitation {
                label: "proposal:manifest:kernel-bite".into(),
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::MarkdownOnly,
            },
        })
        .collect();
    assert_eq!(error.diagnostics(), expected);
    let mut display = String::new();
    for ordinal in 1..=4 {
        writeln!(display,"M044 RustCitation unit=None language=None at rust-history \"crates/ftoo_manifest/src/tag_history.rs\" event {ordinal} field \"authority_record\": UnresolvedRustCitation {{ label: \"proposal:manifest:kernel-bite\", expected: 1, actual: 0, domain: MarkdownOnly }}").unwrap();
    }
    assert_eq!(error.to_string(), display);
}

#[test]
fn w05_malformed_authority_remains_an_m033_prerequisite() {
    let graph = crate::seed().unwrap();
    let mut history = crate::tag_history::seed_history();
    history[0].authority_record = "malformed".into();
    let error = crate::tag_validation::construct(
        &graph,
        crate::TagId::ALL.map(crate::TagId::descriptor).to_vec(),
        history,
    )
    .unwrap_err();
    assert_eq!(
        error.diagnostics(),
        [crate::AddressingDiagnostic {
            rule: RuleId::M033,
            stage: crate::AddressingStage::HistoryAdmission,
            location: crate::AddressingLocation::History(0),
            tag: None,
            identity: Some(NodeId::LangevinCoefficientOne),
            fault: crate::AddressingFault::InvalidAuthorityRecord {
                value: "malformed".into()
            }
        }]
    );
    assert_eq!(
        error.to_string(),
        "M033 HistoryAdmission at History(0): InvalidAuthorityRecord { value: \"malformed\" }\n"
    );
}
