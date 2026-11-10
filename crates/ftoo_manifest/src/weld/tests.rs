use super::*;
use crate::{
    DocumentDraft, DocumentEntryDraft, LocatedEntryDraft, LocatedString, NodeId, PathPart,
    SourceAnchor, UnitDraft, addressing, admit_unit_drafts, check_markdown_sources,
    integrate_labels, resolve_unit_tags, seed,
};
use std::fmt::Write;

const UNIT: &str = "sec:fixture:unit";
fn site(
    language: &str,
    decoded: Option<std::ops::Range<usize>>,
    occurrence: Option<usize>,
) -> UnitSite {
    UnitSite {
        file: "statement.toml".into(),
        path: ["unit", "texts", language]
            .map(|s| PathPart::Key(s.into()))
            .to_vec(),
        anchor: SourceAnchor::Bytes(10, 90),
        decoded,
        occurrence,
    }
}
fn drafts(texts: &[(&str, &str)]) -> (DocumentDraft, UnitDraft) {
    let location = site("en", None, None);
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: location.clone(),
    };
    let declaration = |languages: Vec<LocatedString>| DocumentEntryDraft {
        key_site: location.clone(),
        table_site: location.clone(),
        languages_site: location.clone(),
        required_languages: languages,
    };
    (
        DocumentDraft {
            documents_site: location.clone(),
            documents: BTreeMap::from([
                ("main".into(), declaration(vec![located("en")])),
                (
                    "explainer".into(),
                    declaration(texts.iter().map(|(l, _)| located(l)).collect()),
                ),
            ]),
        },
        UnitDraft {
            id: located(UNIT),
            document: located("explainer"),
            table_site: location.clone(),
            math_site: location.clone(),
            texts_site: location.clone(),
            math: BTreeMap::new(),
            texts: texts
                .iter()
                .map(|(l, t)| {
                    (
                        (*l).into(),
                        LocatedEntryDraft {
                            key_site: site(l, None, None),
                            value: LocatedString {
                                value: (*t).into(),
                                site: site(l, None, None),
                            },
                        },
                    )
                })
                .collect(),
        },
    )
}
fn corpus(texts: &[(&str, &str)]) -> crate::LanguageCheckedUnits {
    let (documents, unit) = drafts(texts);
    admit_unit_drafts(documents, vec![unit]).unwrap()
}
fn markdown() -> crate::CheckedMarkdown {
    check_markdown_sources(vec![crate::MarkdownSource { file:"records.md".into(),bytes:"· `proposal:manifest:kernel-bite`\n· `proposal:manifest:addressing-lifecycle-bite`\n· `sec:fixture:target`\n· `dec:weld:equal-address-sets`\n".as_bytes().to_vec() }]).unwrap()
}
fn tag_side(language: &str, entries: &[(&str, NodeId, usize, usize, usize)]) -> TagSide {
    let mut members: BTreeMap<&str, TagMember> = BTreeMap::new();
    for (input, target, start, end, index) in entries {
        members
            .entry(input)
            .or_insert_with(|| TagMember {
                input: (*input).into(),
                target: *target,
                sites: vec![],
            })
            .sites
            .push(site(language, Some(*start..*end), Some(*index)));
    }
    TagSide {
        language: language.into(),
        text_site: site(language, None, None),
        members: members.into_values().collect(),
    }
}
fn label_side(language: &str, entries: &[(&str, usize, usize, usize)]) -> LabelSide {
    let mut members: BTreeMap<&str, LabelMember> = BTreeMap::new();
    for (label, start, end, index) in entries {
        members
            .entry(label)
            .or_insert_with(|| LabelMember {
                label: (*label).into(),
                sites: vec![],
            })
            .sites
            .push(LabelUseSite {
                span_index: *index,
                site: site(language, Some(*start..*end), None),
            });
    }
    LabelSide {
        language: language.into(),
        text_site: site(language, None, None),
        members: members.into_values().collect(),
    }
}
// Independently spelled diagnostic templates: no production Display/Debug of sides or sites.
fn site_debug(s: &UnitSite) -> String {
    let language = match &s.path[2] {
        PathPart::Key(s) => s,
        PathPart::Index(_) => panic!("fixture path"),
    };
    let decoded = s.decoded.as_ref().map_or_else(
        || "None".into(),
        |r| format!("Some({}..{})", r.start, r.end),
    );
    format!(
        "UnitSite {{ file: \"statement.toml\", path: [Key(\"unit\"), Key(\"texts\"), Key({language:?})], anchor: Bytes(10, 90), decoded: {decoded}, occurrence: {:?} }}",
        s.occurrence
    )
}
fn tags_debug(s: &TagSide) -> String {
    let members = s
        .members
        .iter()
        .map(|m| {
            let target = match m.target {
                NodeId::LangevinCoefficientOne => "LangevinCoefficientOne",
                NodeId::LangevinCoefficientThree => "LangevinCoefficientThree",
                NodeId::LangevinCoefficientOneValue => "LangevinCoefficientOneValue",
                NodeId::LangevinCoefficientThreeValue => "LangevinCoefficientThreeValue",
            };
            format!(
                "TagMember {{ input: {:?}, target: {target}, sites: [{}] }}",
                m.input,
                m.sites
                    .iter()
                    .map(site_debug)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "TagSide {{ language: {:?}, text_site: {}, members: [{members}] }}",
        s.language,
        site_debug(&s.text_site)
    )
}
fn labels_debug(s: &LabelSide) -> String {
    let members = s
        .members
        .iter()
        .map(|m| {
            format!(
                "LabelMember {{ label: {:?}, sites: [{}] }}",
                m.label,
                m.sites
                    .iter()
                    .map(|u| format!(
                        "LabelUseSite {{ span_index: {}, site: {} }}",
                        u.span_index,
                        site_debug(&u.site)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "LabelSide {{ language: {:?}, text_site: {}, members: [{members}] }}",
        s.language,
        site_debug(&s.text_site)
    )
}
fn assert_errors(error: &WeldErrors, expected: &[WeldDiagnostic]) {
    assert_eq!(error.diagnostics(), expected);
    let mut display = String::new();
    for d in expected {
        let LabelSite::Unit { unit, site } = &d.site else {
            panic!("fixture unit site")
        };
        let language = d.language.as_ref().unwrap();
        let suffix = site
            .decoded
            .as_ref()
            .map_or_else(String::new, |r| format!(" decoded {}..{}", r.start, r.end));
        let fault = match &d.fault {
            WeldFault::UnresolvedUnitCitation {
                label,
                span_index,
                expected,
                actual,
                domain: _,
            } => format!(
                "UnresolvedUnitCitation {{ label: {label:?}, span_index: {span_index}, expected: {expected}, actual: {actual}, domain: Joint }}"
            ),
            WeldFault::TagSetMismatch {
                reference,
                actual,
                missing,
                unexpected,
            } => format!(
                "TagSetMismatch {{ reference: {}, actual: {}, missing: {missing:?}, unexpected: {unexpected:?} }}",
                tags_debug(reference),
                tags_debug(actual)
            ),
            WeldFault::LabelSetMismatch {
                reference,
                actual,
                missing,
                unexpected,
            } => format!(
                "LabelSetMismatch {{ reference: {}, actual: {}, missing: {missing:?}, unexpected: {unexpected:?} }}",
                labels_debug(reference),
                labels_debug(actual)
            ),
        };
        writeln!(display,"{:?} {:?} unit=Some({unit:?}) language=Some({language:?}) at unit {unit:?} at \"statement.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes 10..90{suffix}: {fault}",d.rule,d.stage).unwrap();
    }
    assert_eq!(error.to_string(), display);
}
fn inequality(unit: &str, language: &str, fault: WeldFault) -> WeldDiagnostic {
    WeldDiagnostic {
        rule: RuleId::M046,
        stage: WeldStage::Equality,
        unit: Some(unit.into()),
        language: Some(language.into()),
        site: LabelSite::Unit {
            unit: unit.into(),
            site: site(language, None, None),
        },
        fault,
    }
}

fn assert_unit_errors(
    errors: &crate::UnitErrors,
    expected: &[crate::UnitDiagnostic],
    faults: &[String],
) {
    assert_eq!(errors.diagnostics(), expected);
    assert_eq!(expected.len(), faults.len());
    let mut display = String::new();
    for (d, fault) in expected.iter().zip(faults) {
        let language = d.language.as_ref().unwrap();
        let range = d.site.decoded.as_ref().unwrap();
        let occurrence = d
            .site
            .occurrence
            .map_or_else(String::new, |i| format!(" occurrence {i}"));
        writeln!(display, "{:?} {:?} unit=Some(\"sec:fixture:unit\") language=Some({language:?}) at \"statement.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes 10..90 decoded {}..{}{occurrence}: {fault}", d.rule, d.stage, range.start, range.end).unwrap();
    }
    assert_eq!(errors.to_string(), display);
}

#[test]
fn w08_all_languages_cannot_mint_self_and_malformed_markers_remain_syntax_faults() {
    for (text, rule, fault, start, end, display) in [
        (
            "See · `sec:fixture:unit`.",
            RuleId::M045,
            crate::UnitFault::TextMintForbidden {
                span_index: 0,
                label: UNIT.into(),
            },
            4,
            25,
            "TextMintForbidden { span_index: 0, label: \"sec:fixture:unit\" }",
        ),
        (
            "See · `sec:fixture`.",
            RuleId::M041,
            crate::UnitFault::MalformedLabelSpan {
                span_index: 0,
                reason: crate::LabelSpanReason::InvalidLabel,
                fragment: "`sec:fixture`".into(),
            },
            7,
            20,
            "MalformedLabelSpan { span_index: 0, reason: InvalidLabel, fragment: \"`sec:fixture`\" }",
        ),
    ] {
        let (documents, unit) = drafts(&[("fr", text), ("en", text)]);
        let expected: Vec<_> = ["en", "fr"]
            .into_iter()
            .map(|language| crate::UnitDiagnostic {
                rule,
                stage: crate::UnitStage::Content,
                unit: Some(UNIT.into()),
                language: Some(language.into()),
                site: site(language, Some(start..end), None),
                fault: fault.clone(),
            })
            .collect();
        assert_unit_errors(
            &admit_unit_drafts(documents, vec![unit]).unwrap_err(),
            &expected,
            &[display.into(), display.into()],
        );
    }
}

#[test]
fn w12_private_retired_tags_precede_weld_at_first_middle_and_last_sites() {
    let graph = seed().unwrap();
    let event = crate::HistoryEvent {
        ordinal: 9,
        authority_record: "proposal:manifest:kernel-bite".into(),
        reason: "Withdrawn for correction.".into(),
        change: crate::HistoryChange::RetireTag {
            tag: crate::TagId::LangevinCoefficientThree,
        },
    };
    let mut history = crate::tag_history::seed_history();
    history.push(event.clone());
    let book = crate::tag_validation::construct(
        &graph,
        crate::TagId::ALL.map(crate::TagId::descriptor).to_vec(),
        history,
    )
    .unwrap();
    let corpus = corpus(&[
        (
            "en",
            "@tag{c:3} See ``ignored`` @tag{c:3} `sec:fixture:unit` @tag{c:3}",
        ),
        (
            "fr",
            "@tag{c:3} See ``ignored`` @tag{c:3} `sec:fixture:unit` @tag{c:3}",
        ),
    ]);
    let mut expected = Vec::new();
    for language in ["en", "fr"] {
        for (index, start) in [0, 26, 55].into_iter().enumerate() {
            expected.push(crate::UnitDiagnostic {
                rule: RuleId::M042,
                stage: crate::UnitStage::Resolution,
                unit: Some(UNIT.into()),
                language: Some(language.into()),
                site: site(language, Some(start..start + 9), Some(index)),
                fault: crate::UnitFault::ResolutionFault {
                    cause: crate::TagResolutionFault::RetiredTag {
                        input: "c:3".into(),
                        tag: crate::TagId::LangevinCoefficientThree,
                        target: NodeId::LangevinCoefficientThree,
                        cause: crate::RetirementCause::TagRetirement(event.clone()),
                    },
                },
            });
        }
    }
    assert_unit_errors(&resolve_unit_tags(&corpus, &book).unwrap_err(), &expected, &vec!["tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority proposal:manifest:kernel-bite; reason \"Withdrawn for correction.\"".into(); 6]);
    let live = addressing(&graph).unwrap();
    let resolved = resolve_unit_tags(&corpus, &live).unwrap();
    let markdown = markdown();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    for language in welded.units()[0].languages() {
        assert_eq!(
            language.tags(),
            &tag_side(
                language.language(),
                &[
                    ("c:3", NodeId::LangevinCoefficientThree, 0, 9, 0),
                    ("c:3", NodeId::LangevinCoefficientThree, 26, 35, 1),
                    ("c:3", NodeId::LangevinCoefficientThree, 55, 64, 2)
                ]
            )
        );
        assert_eq!(
            language.labels(),
            &label_side(language.language(), &[(UNIT, 36, 54, 1)])
        );
        assert_eq!(
            language.mentions(),
            &[LabelUseSite {
                span_index: 0,
                site: site(language.language(), Some(14..25), None)
            }]
        );
    }
}

#[test]
fn w10_tag_sets_missing_unexpected_and_both_directions() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let md = markdown();
    for (en, fr, reference, actual, missing, unexpected) in [
        (
            "See @tag{c:1}.",
            "Plain prose.",
            vec![("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0)],
            vec![],
            vec!["c:1"],
            vec![],
        ),
        (
            "Plain prose.",
            "See @tag{c:1}.",
            vec![],
            vec![("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0)],
            vec![],
            vec!["c:1"],
        ),
        (
            "See @tag{c:1}.",
            "See @tag{v:1}.",
            vec![("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0)],
            vec![("v:1", NodeId::LangevinCoefficientOneValue, 4, 13, 0)],
            vec!["c:1"],
            vec!["v:1"],
        ),
    ] {
        let corpus = corpus(&[("fr", fr), ("en", en)]);
        let resolved = resolve_unit_tags(&corpus, &book).unwrap();
        let integrated = integrate_labels(&resolved, &md).unwrap();
        let error = weld_units(&integrated).unwrap_err();
        assert_errors(
            &error,
            &[inequality(
                UNIT,
                "fr",
                WeldFault::TagSetMismatch {
                    reference: tag_side("en", &reference),
                    actual: tag_side("fr", &actual),
                    missing: missing.into_iter().map(str::to_owned).collect(),
                    unexpected: unexpected.into_iter().map(str::to_owned).collect(),
                },
            )],
        );
    }
}
#[test]
fn w08_w10_label_sets_and_self_citation_are_separate_from_metadata() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let md = markdown();
    for (en, fr, reference, actual, missing, unexpected) in [
        (
            "See `sec:fixture:target`.",
            "See ``sec:fixture:target``.",
            vec![("sec:fixture:target", 4, 24, 0)],
            vec![],
            vec!["sec:fixture:target"],
            vec![],
        ),
        (
            "Plain prose.",
            "See `sec:fixture:unit`.",
            vec![],
            vec![(UNIT, 4, 22, 0)],
            vec![],
            vec![UNIT],
        ),
        (
            "See `sec:fixture:unit`.",
            "See `sec:fixture:target`.",
            vec![(UNIT, 4, 22, 0)],
            vec![("sec:fixture:target", 4, 24, 0)],
            vec![UNIT],
            vec!["sec:fixture:target"],
        ),
    ] {
        let corpus = corpus(&[("fr", fr), ("en", en)]);
        let resolved = resolve_unit_tags(&corpus, &book).unwrap();
        let integrated = integrate_labels(&resolved, &md).unwrap();
        assert_eq!(
            integrated.mints().get(UNIT).unwrap().kind,
            crate::MintKind::UnitMetadata
        );
        let error = weld_units(&integrated).unwrap_err();
        assert_errors(
            &error,
            &[inequality(
                UNIT,
                "fr",
                WeldFault::LabelSetMismatch {
                    reference: label_side("en", &reference),
                    actual: label_side("fr", &actual),
                    missing: missing.into_iter().map(str::to_owned).collect(),
                    unexpected: unexpected.into_iter().map(str::to_owned).collect(),
                },
            )],
        );
    }
}
#[test]
fn w10_tag_to_bound_label_substitution_is_two_ordered_inequalities() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let md = markdown();
    let corpus = corpus(&[
        ("en", "See @tag{c:1}."),
        ("fr", "See `def:langevin:coefficient-one`."),
    ]);
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &md).unwrap();
    assert_errors(
        &weld_units(&integrated).unwrap_err(),
        &[
            inequality(
                UNIT,
                "fr",
                WeldFault::TagSetMismatch {
                    reference: tag_side("en", &[("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0)]),
                    actual: tag_side("fr", &[]),
                    missing: vec!["c:1".into()],
                    unexpected: vec![],
                },
            ),
            inequality(
                UNIT,
                "fr",
                WeldFault::LabelSetMismatch {
                    reference: label_side("en", &[]),
                    actual: label_side("fr", &[("def:langevin:coefficient-one", 4, 34, 0)]),
                    missing: vec![],
                    unexpected: vec!["def:langevin:coefficient-one".into()],
                },
            ),
        ],
    );
}
#[test]
fn w09_unknown_occurrences_suppress_equality_globally_and_mentions_resolve_nothing() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let md = markdown();
    let (documents, unit) = drafts(&[
        (
            "en",
            "See ``x`` `missing:label:one` and `missing:label:one`.",
        ),
        ("fr", "See `missing:label:two` @tag{v:1}."),
    ]);
    let (_, mut other) = drafts(&[("en", "See @tag{c:1}."), ("fr", "Plain prose.")]);
    other.id.value = "sec:fixture:other".into();
    let corpus = admit_unit_drafts(documents, vec![other, unit]).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &md).unwrap();
    let expected: Vec<_> = [
        ("en", "missing:label:one", 10, 29, 1),
        ("en", "missing:label:one", 34, 53, 2),
        ("fr", "missing:label:two", 4, 23, 0),
    ]
    .into_iter()
    .map(|(language, label, start, end, index)| WeldDiagnostic {
        rule: RuleId::M045,
        stage: WeldStage::UnitCitation,
        unit: Some(UNIT.into()),
        language: Some(language.into()),
        site: LabelSite::Unit {
            unit: UNIT.into(),
            site: site(language, Some(start..end), None),
        },
        fault: WeldFault::UnresolvedUnitCitation {
            label: label.into(),
            span_index: index,
            expected: 1,
            actual: 0,
            domain: ReferenceDomain::Joint,
        },
    })
    .collect();
    assert_errors(&weld_units(&integrated).unwrap_err(), &expected);
    let corpus = crate::weld::tests::corpus(&[("en", "See ``missing:label:one``.")]);
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &md).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let language = &welded.units()[0].languages()[0];
    assert_eq!(language.labels(), &label_side("en", &[]));
    assert!(language.citations().is_empty());
    assert_eq!(
        language.mentions(),
        [LabelUseSite {
            span_index: 0,
            site: site("en", Some(4..25), None)
        }]
    );
    assert!(!integrated.mints().contains_key("missing:label:one"));
}

fn assert_repeated_sets(languages: &[WeldedLanguage]) {
    for (language, tags, labels) in [
        (
            "de",
            vec![
                ("c:1", NodeId::LangevinCoefficientOne, 6, 15, 0),
                ("v:1", NodeId::LangevinCoefficientOneValue, 35, 44, 1),
            ],
            vec![(UNIT, 16, 34, 0)],
        ),
        (
            "en",
            vec![
                ("v:1", NodeId::LangevinCoefficientOneValue, 4, 13, 0),
                ("c:1", NodeId::LangevinCoefficientOne, 14, 23, 1),
                ("c:1", NodeId::LangevinCoefficientOne, 24, 33, 2),
            ],
            vec![(UNIT, 34, 52, 0)],
        ),
        (
            "fr",
            vec![
                ("v:1", NodeId::LangevinCoefficientOneValue, 24, 33, 0),
                ("c:1", NodeId::LangevinCoefficientOne, 43, 52, 2),
            ],
            vec![(UNIT, 5, 23, 0), (UNIT, 53, 71, 1)],
        ),
    ] {
        let actual = languages.iter().find(|l| l.language() == language).unwrap();
        assert_eq!(actual.tags(), &tag_side(language, &tags));
        assert_eq!(actual.labels(), &label_side(language, &labels));
        assert_eq!(actual.mentions(), []);
        assert_eq!(
            actual
                .citations()
                .iter()
                .map(|c| (c.label(), c.site()))
                .collect::<Vec<_>>(),
            labels
                .iter()
                .map(|(label, start, end, index)| (
                    *label,
                    LabelUseSite {
                        span_index: *index,
                        site: site(language, Some(*start..*end), None)
                    }
                ))
                .collect::<Vec<_>>()
                .iter()
                .map(|(label, site)| (*label, site))
                .collect::<Vec<_>>()
        );
        for citation in actual.citations() {
            assert_eq!(
                citation.origin(),
                &MintOrigin {
                    kind: crate::MintKind::UnitMetadata,
                    site: LabelSite::Unit {
                        unit: UNIT.into(),
                        site: UnitSite {
                            file: "statement.toml".into(),
                            path: vec![PathPart::Key("unit".into()), PathPart::Key("id".into())],
                            anchor: SourceAnchor::Unavailable,
                            decoded: None,
                            occurrence: None
                        }
                    },
                }
            );
        }
    }
}

#[test]
fn w09_w10_reordered_repeated_sets_and_different_math_sets_pass() {
    let graph = seed().unwrap();
    let before = crate::Snapshot::capture(&graph);
    let statuses = NodeId::ALL.map(|id| graph.status(id));
    let book = addressing(&graph).unwrap();
    let md = markdown();
    let (documents, mut unit) = drafts(&[
        (
            "fr",
            "Voir `sec:fixture:unit` @tag{v:1} @math{b} @tag{c:1} `sec:fixture:unit`.",
        ),
        (
            "de",
            "Siehe @tag{c:1} `sec:fixture:unit` @tag{v:1} @math{a}.",
        ),
        (
            "en",
            "See @tag{v:1} @tag{c:1} @tag{c:1} `sec:fixture:unit`.",
        ),
    ]);
    for name in ["a", "b"] {
        unit.math.insert(
            name.into(),
            LocatedEntryDraft {
                key_site: site("en", None, None),
                value: LocatedString {
                    value: "x".into(),
                    site: site("en", None, None),
                },
            },
        );
    }
    let corpus = admit_unit_drafts(documents, vec![unit]).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &md).unwrap();
    let welded = weld_units(&integrated).unwrap();
    assert!(std::ptr::eq(welded.integrated(), &raw const integrated));
    assert_eq!(welded.units().len(), 1);
    let languages = welded.units()[0].languages();
    assert_eq!(
        languages
            .iter()
            .map(WeldedLanguage::language)
            .collect::<Vec<_>>(),
        ["de", "en", "fr"]
    );
    assert_repeated_sets(languages);
    assert_eq!(crate::Snapshot::capture(&graph), before);
    assert_eq!(NodeId::ALL.map(|id| graph.status(id)), statuses);
}

#[test]
fn w10_smallest_language_without_english_and_independent_unit_sets() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let md = markdown();
    let (documents, first) = drafts(&[
        ("fr", "See @tag{v:1}."),
        ("de", "See @tag{c:1}."),
        ("ja", "See @tag{v:1}."),
    ]);
    let (_, mut second) = drafts(&[
        ("ja", "See @tag{c:1}."),
        ("de", "See @tag{v:1}."),
        ("fr", "See @tag{c:1}."),
    ]);
    second.id.value = "sec:fixture:second".into();
    // The document-wide union is identical in every language; each unit still differs.
    for units in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        let corpus = admit_unit_drafts(documents.clone(), units).unwrap();
        let resolved = resolve_unit_tags(&corpus, &book).unwrap();
        let integrated = integrate_labels(&resolved, &md).unwrap();
        let mut expected = Vec::new();
        for (unit, reference, actual, reference_id, actual_id) in [
            (
                "sec:fixture:second",
                "v:1",
                "c:1",
                NodeId::LangevinCoefficientOneValue,
                NodeId::LangevinCoefficientOne,
            ),
            (
                UNIT,
                "c:1",
                "v:1",
                NodeId::LangevinCoefficientOne,
                NodeId::LangevinCoefficientOneValue,
            ),
        ] {
            for language in ["fr", "ja"] {
                expected.push(inequality(
                    unit,
                    language,
                    WeldFault::TagSetMismatch {
                        reference: tag_side("de", &[(reference, reference_id, 4, 13, 0)]),
                        actual: tag_side(language, &[(actual, actual_id, 4, 13, 0)]),
                        missing: vec![reference.into()],
                        unexpected: vec![actual.into()],
                    },
                ));
            }
        }
        assert_errors(&weld_units(&integrated).unwrap_err(), &expected);
    }
    // Different complete per-unit sets are legal, including an empty one.
    let (documents, first) = drafts(&[("fr", "Plain."), ("de", "Plain.")]);
    let (_, mut second) = drafts(&[("de", "See @tag{v:1}."), ("fr", "See @tag{v:1}.")]);
    second.id.value = "sec:fixture:second".into();
    let corpus = admit_unit_drafts(documents, vec![first, second]).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &md).unwrap();
    let welded = weld_units(&integrated).unwrap();
    assert_eq!(
        welded
            .units()
            .iter()
            .map(WeldedUnit::unit)
            .collect::<Vec<_>>(),
        ["sec:fixture:second", UNIT]
    );
    for language in welded.units()[0].languages() {
        assert_eq!(
            language.tags(),
            &tag_side(
                language.language(),
                &[("v:1", NodeId::LangevinCoefficientOneValue, 4, 13, 0)]
            )
        );
        assert_eq!(language.labels(), &label_side(language.language(), &[]));
    }
    for language in welded.units()[1].languages() {
        assert_eq!(language.tags(), &tag_side(language.language(), &[]));
        assert_eq!(language.labels(), &label_side(language.language(), &[]));
    }
}

#[test]
fn w09_permanent_absent_excluded_and_retired_label_citations_resolve() {
    let full = seed().unwrap();
    let mut draft = crate::seed_draft();
    draft.coverage.slice.clear();
    let empty = crate::validate(draft.clone()).unwrap();
    draft
        .nodes
        .retain(|n| n.id != NodeId::LangevinCoefficientThree);
    draft.edges.retain(|e| {
        e.source != NodeId::LangevinCoefficientThree && e.target != NodeId::LangevinCoefficientThree
    });
    let absent = crate::validate(draft).unwrap();
    let md = markdown();
    let corpus = corpus(&[("en", "See `def:langevin:coefficient-three`.")]);
    for graph in [&full, &empty, &absent] {
        for retirement in [
            None,
            Some(crate::HistoryChange::RetireIdentity {
                identity: NodeId::LangevinCoefficientThree,
            }),
            Some(crate::HistoryChange::RetireTag {
                tag: crate::TagId::LangevinCoefficientThree,
            }),
        ] {
            if matches!(
                retirement,
                Some(crate::HistoryChange::RetireIdentity { .. })
            ) && graph.node(NodeId::LangevinCoefficientThree).is_some()
            {
                continue;
            }
            let mut history = crate::tag_history::seed_history();
            if let Some(change) = retirement {
                history.push(crate::HistoryEvent {
                    ordinal: 9,
                    authority_record: "proposal:manifest:kernel-bite".into(),
                    reason: "Withdrawn for correction.".into(),
                    change,
                });
            }
            let book = crate::tag_validation::construct(
                graph,
                crate::TagId::ALL.map(crate::TagId::descriptor).to_vec(),
                history,
            )
            .unwrap();
            let before = crate::Snapshot::capture(graph);
            let resolved = resolve_unit_tags(&corpus, &book).unwrap();
            let integrated = integrate_labels(&resolved, &md).unwrap();
            let welded = weld_units(&integrated).unwrap();
            let language = &welded.units()[0].languages()[0];
            assert_eq!(language.tags(), &tag_side("en", &[]));
            assert_eq!(
                language.labels(),
                &label_side("en", &[("def:langevin:coefficient-three", 4, 36, 0)])
            );
            assert_eq!(language.citations().len(), 1);
            assert_eq!(
                language.citations()[0].origin().kind,
                crate::MintKind::RustIdentity
            );
            assert_eq!(crate::Snapshot::capture(graph), before);
        }
    }
}

#[test]
fn w09_w11_first_middle_last_citations_and_other_unit_resolution() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let markdown = markdown();
    let (documents, unit) = drafts(&[(
        "en",
        "`sec:fixture:unit` See (`sec:fixture:other`) and `def:langevin:coefficient-one`.",
    )]);
    let (_, mut other) = drafts(&[("en", "Plain prose.")]);
    other.id.value = "sec:fixture:other".into();
    let corpus = admit_unit_drafts(documents, vec![unit, other]).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let language = &welded.units()[1].languages()[0];
    assert_eq!(language.tags(), &tag_side("en", &[]));
    assert_eq!(
        language.labels(),
        &label_side(
            "en",
            &[
                (UNIT, 0, 18, 0),
                ("sec:fixture:other", 24, 43, 1),
                ("def:langevin:coefficient-one", 49, 79, 2)
            ]
        )
    );
    assert_eq!(
        language
            .citations()
            .iter()
            .map(ResolvedUnitCitation::label)
            .collect::<Vec<_>>(),
        [UNIT, "sec:fixture:other", "def:langevin:coefficient-one"]
    );
    assert_eq!(
        language.citations()[1].origin(),
        &MintOrigin {
            kind: crate::MintKind::UnitMetadata,
            site: LabelSite::Unit {
                unit: "sec:fixture:other".into(),
                site: UnitSite {
                    file: "statement.toml".into(),
                    path: vec![PathPart::Key("unit".into()), PathPart::Key("id".into())],
                    anchor: SourceAnchor::Unavailable,
                    decoded: None,
                    occurrence: None
                }
            }
        }
    );
    assert_eq!(
        language.citations()[2].origin(),
        &MintOrigin {
            kind: crate::MintKind::RustIdentity,
            site: LabelSite::RustIdentity {
                file: "crates/ftoo_manifest/src/identity.rs".into(),
                identity: NodeId::LangevinCoefficientOne,
                field: "semantic_name".into()
            }
        }
    );
    let corpus = self::corpus(&[("en", "See `missing:label:one`.")]);
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    assert_errors(
        &weld_units(&integrated).unwrap_err(),
        &[WeldDiagnostic {
            rule: RuleId::M045,
            stage: WeldStage::UnitCitation,
            unit: Some(UNIT.into()),
            language: Some("en".into()),
            site: LabelSite::Unit {
                unit: UNIT.into(),
                site: site("en", Some(4..23), None),
            },
            fault: WeldFault::UnresolvedUnitCitation {
                label: "missing:label:one".into(),
                span_index: 0,
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::Joint,
            },
        }],
    );
}
