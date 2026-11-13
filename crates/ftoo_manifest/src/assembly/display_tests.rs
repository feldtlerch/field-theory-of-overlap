// Keep each complete fault matrix beside its restoration controls.
#![allow(clippy::too_many_lines)]
use crate::*;
use std::collections::BTreeMap;

fn site() -> UnitSite {
    UnitSite {
        file: "units/displays/fixture.toml".into(),
        path: vec![
            PathPart::Key("display".into()),
            PathPart::Key("bodies".into()),
            PathPart::Key("markdown".into()),
            PathPart::Key("en".into()),
        ],
        anchor: SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    }
}
fn source(value: &str) -> LocatedString {
    LocatedString {
        value: value.into(),
        site: site(),
    }
}
fn refusal(
    kind: DisplayKind,
    target: DocumentTarget,
    text: &str,
    range: std::ops::Range<usize>,
    fault: AssemblyInputFault,
) {
    let mut expected_site = site();
    expected_site.decoded = Some(range);
    let expected_display = format!("{fault:?}");
    let actual = parse_display_body(kind, target, &source(text)).unwrap_err();
    assert_eq!(*actual, (expected_site, fault));
    assert_eq!(actual.1.to_string(), expected_display);
}
fn syntax(
    kind: DisplayKind,
    reason: DisplaySyntaxReason,
    token: &str,
    expected: &[&str],
) -> AssemblyInputFault {
    AssemblyInputFault::DisplaySyntax {
        kind,
        reason,
        token: token.into(),
        expected: expected.iter().map(|s| (*s).into()).collect(),
    }
}
#[test]
fn display_layouts_keep_exact_regions_and_span_protected_delimiters() {
    for (target, text) in [
        (
            DocumentTarget::Markdown,
            "| @@tag{left | right} |\n| --- | --- |\n| A | B |",
        ),
        (
            DocumentTarget::Latex,
            r"\begin{tabular}{ll} @@tag{left & right} \\ A & B \\ \end{tabular}",
        ),
    ] {
        let body = parse_display_body(DisplayKind::Table, target, &source(text)).unwrap();
        assert_eq!(
            body.regions()
                .iter()
                .map(|region| region.source().value.trim())
                .collect::<Vec<_>>(),
            vec!["@@tag{left", "right}", "A", "B"]
        );
        assert_eq!(
            body.regions()
                .iter()
                .map(|region| region.tags().len())
                .sum::<usize>(),
            0
        );
    }
    let markdown = "| ``a|b`` | @tag{c:1} |\n| :--- | ---: |\n| α | @math{x} |\n";
    let body = parse_display_body(
        DisplayKind::Table,
        DocumentTarget::Markdown,
        &source(markdown),
    )
    .unwrap();
    assert_eq!(
        body.layout(),
        &DisplayLayout::Table {
            columns: vec![":---".into(), "---:".into()],
            rows: vec![vec![0, 1], vec![2, 3]],
            rules: vec![]
        }
    );
    assert_eq!(
        body.regions()
            .iter()
            .map(|r| (&markdown[r.bytes()], r.bytes()))
            .collect::<Vec<_>>(),
        vec![
            (" ``a|b`` ", 1..10),
            (" @tag{c:1} ", 11..22),
            (" α ", 41..45),
            (" @math{x} ", 46..56)
        ]
    );
    let latex = r"\begin{tabular}{l c} ``a&b`` & @tag{c:1} \\ \hline α & @math{x} \\ \end{tabular}";
    let body =
        parse_display_body(DisplayKind::Table, DocumentTarget::Latex, &source(latex)).unwrap();
    assert_eq!(
        body.layout(),
        &DisplayLayout::Table {
            columns: vec!["l".into(), "c".into()],
            rows: vec![vec![0, 1], vec![2, 3]],
            rules: vec![1]
        }
    );
    assert_eq!(
        body.regions()
            .iter()
            .map(|r| r.source().value.as_str())
            .collect::<Vec<_>>(),
        vec!["``a&b`` ", " @tag{c:1} ", "α ", " @math{x} "]
    );
    let body = parse_display_body(
        DisplayKind::Figure,
        DocumentTarget::Markdown,
        &source("> @tag{c:1} → @tag{c:3}\n>\n> End"),
    )
    .unwrap();
    assert_eq!(
        body.layout(),
        &DisplayLayout::Quote {
            rows: vec![Some(0), None, Some(1)]
        }
    );
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let text = format!(
                "\\begin{{picture}}(120,20)\\put(0,10){{@tag{{c:1}}}}\\put(35,10){{\\vector({dx},{dy}){{40}}}}\\put(90,10){{@tag{{c:3}}}}\\end{{picture}}"
            );
            let body =
                parse_display_body(DisplayKind::Figure, DocumentTarget::Latex, &source(&text))
                    .unwrap();
            assert_eq!(
                body.layout(),
                &DisplayLayout::Picture {
                    width: 120,
                    height: 20,
                    commands: vec![
                        PictureCommand::Label {
                            x: 0,
                            y: 10,
                            region: 0
                        },
                        PictureCommand::Line {
                            x: 35,
                            y: 10,
                            dx,
                            dy,
                            length: 40,
                            vector: true
                        },
                        PictureCommand::Label {
                            x: 90,
                            y: 10,
                            region: 1
                        }
                    ]
                }
            );
        }
    }
}
#[test]
fn display_layout_refusals_pin_full_tokens_and_coordinates() {
    use DisplayKind::{Figure, Table};
    use DisplaySyntaxReason::{
        BadDelimiter, EmptyBody, MissingAlignment, MissingRow, TrailingContent, ZeroDirection,
    };
    use DocumentTarget::{Latex, Markdown};
    for (kind, target, text, range, reason, token, expected) in [
        (Table, Markdown, "", 0..0, EmptyBody, "", vec!["content"]),
        (Figure, Markdown, " ", 0..1, EmptyBody, " ", vec!["content"]),
        (
            Table,
            Markdown,
            "a",
            0..1,
            BadDelimiter,
            "a",
            vec!["| cells |"],
        ),
        (
            Table,
            Markdown,
            "| A |",
            5..5,
            MissingAlignment,
            "",
            vec!["alignment row"],
        ),
        (
            Table,
            Markdown,
            "| A |\n| --- |",
            13..13,
            MissingRow,
            "",
            vec!["data row"],
        ),
        (Figure, Markdown, "x", 0..1, BadDelimiter, "x", vec!["> "]),
        (
            Figure,
            Latex,
            r"\begin{picture}(2,2)\put(0,0){\line(0,0){1}}\end{picture}",
            36..39,
            ZeroDirection,
            "0,0",
            vec!["nonzero direction"],
        ),
        (
            Figure,
            Latex,
            r"\begin{picture}(2,2)\put(0,0){A}\end{picture}x",
            45..46,
            TrailingContent,
            "x",
            vec![],
        ),
    ] {
        refusal(
            kind,
            target,
            text,
            range,
            syntax(kind, reason, token, &expected),
        );
    }
    for (token, minimum, maximum) in [
        ("0", 1, 4096),
        ("4097", 1, 4096),
        ("999999999999999999999", 1, 4096),
        ("-1", 1, 4096),
    ] {
        let text = format!("\\begin{{picture}}({token},2)\\put(0,0){{A}}\\end{{picture}}");
        refusal(
            Figure,
            Latex,
            &text,
            16..16 + token.len(),
            AssemblyInputFault::Coordinate {
                field: "W".into(),
                value: token.into(),
                minimum,
                maximum,
            },
        );
    }
    refusal(
        Table,
        Markdown,
        "| A |\n| --- |\n| B | C |",
        14..23,
        AssemblyInputFault::TableColumns {
            row: 2,
            expected: 1,
            actual: 2,
        },
    );
}
#[test]
fn display_inline_failures_keep_nested_fault_and_region_path() {
    let text = "| A |\n| --- |\n| $ |";
    let mut expected = site();
    expected.path.extend([
        PathPart::Key("regions".into()),
        PathPart::Index(2),
        PathPart::Index(0),
    ]);
    expected.decoded = Some(16..17);
    let fault = AssemblyInputFault::DisplayInline {
        cause: UnitFault::UnsupportedTextCharacter { value: "$".into() },
    };
    assert_eq!(
        *parse_display_body(DisplayKind::Table, DocumentTarget::Markdown, &source(text))
            .unwrap_err(),
        (expected, fault.clone())
    );
    assert_eq!(
        fault.to_string(),
        "DisplayInline { cause: UnsupportedTextCharacter { value: \"$\" } }"
    );
}
#[test]
fn display_limits_have_boundary_success_and_first_excess() {
    let header = format!("|{}|", vec![" A "; 64].join("|"));
    let align = format!("|{}|", vec![" --- "; 64].join("|"));
    let text = format!("{header}\n{align}\n{header}");
    assert_eq!(
        parse_display_body(DisplayKind::Table, DocumentTarget::Markdown, &source(&text))
            .unwrap()
            .regions()
            .len(),
        128
    );
    let header = format!("|{}|", vec![" A "; 65].join("|"));
    refusal(
        DisplayKind::Table,
        DocumentTarget::Markdown,
        &header,
        257..260,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyColumns,
            limit: 64,
            actual: 65,
        },
    );
    let text = vec!["> A"; 1024].join("\n");
    assert_eq!(
        parse_display_body(
            DisplayKind::Figure,
            DocumentTarget::Markdown,
            &source(&text)
        )
        .unwrap()
        .regions()
        .len(),
        1024
    );
    let excess = format!("{text}\n> A");
    refusal(
        DisplayKind::Figure,
        DocumentTarget::Markdown,
        &excess,
        4096..4099,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyRows,
            limit: 1024,
            actual: 1025,
        },
    );
    let text = format!("> {}", "a".repeat(262_142));
    assert_eq!(
        parse_display_body(
            DisplayKind::Figure,
            DocumentTarget::Markdown,
            &source(&text)
        )
        .unwrap()
        .source()
        .value
        .len(),
        262_144
    );
    refusal(
        DisplayKind::Figure,
        DocumentTarget::Markdown,
        &format!("{text}a"),
        262_144..262_145,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyBytes,
            limit: 262_144,
            actual: 262_145,
        },
    );
    let text = format!("> {}", "@tag{c:1}".repeat(4096));
    assert_eq!(
        parse_display_body(
            DisplayKind::Figure,
            DocumentTarget::Markdown,
            &source(&text)
        )
        .unwrap()
        .regions()[0]
            .tags()
            .len(),
        4096
    );
    refusal(
        DisplayKind::Figure,
        DocumentTarget::Markdown,
        &format!("{text}@tag{{c:1}}"),
        36866..36875,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyOccurrences,
            limit: 4096,
            actual: 4097,
        },
    );
}
fn corpus() -> LanguageCheckedUnits {
    let site = site();
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: site.clone(),
    };
    let documents = DocumentDraft {
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
    };
    let units = ["heading", "caption"].map(|name| UnitDraft {
        id: located(&format!("sec:fixture:{name}")),
        document: located("main"),
        table_site: site.clone(),
        math_site: site.clone(),
        texts_site: site.clone(),
        math: BTreeMap::new(),
        texts: BTreeMap::from([(
            "en".into(),
            LocatedEntryDraft {
                key_site: site.clone(),
                value: located("Caption prose."),
            },
        )]),
    });
    admit_unit_drafts(documents, units.into()).unwrap()
}
fn companion(markdown: &str, latex: &str) -> DisplayDraft {
    DisplayDraft {
        unit: source("sec:fixture:caption"),
        kind: DisplayKind::Figure,
        bodies: BTreeMap::from([
            (
                DocumentTarget::Markdown,
                DisplayTargetDraft {
                    site: site(),
                    languages: BTreeMap::from([("en".into(), source(markdown))]),
                },
            ),
            (
                DocumentTarget::Latex,
                DisplayTargetDraft {
                    site: site(),
                    languages: BTreeMap::from([("en".into(), source(latex))]),
                },
            ),
        ]),
    }
}
fn with_context(check: impl FnOnce(&CheckedSkeletons<'_>, &WeldedUnits<'_, '_, '_, '_, '_, '_>)) {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    with_book(&book, check);
}
fn with_book(
    book: &AddressBook<'_>,
    check: impl FnOnce(&CheckedSkeletons<'_>, &WeldedUnits<'_, '_, '_, '_, '_, '_>),
) {
    let corpus = corpus();
    let resolved = resolve_unit_tags(&corpus, book).unwrap();
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"records.md".into(),bytes:"· `proposal:manifest:kernel-bite`\n· `proposal:manifest:addressing-lifecycle-bite`\n· `dec:fixture:source`\n· `rec:test:retirement`\n".as_bytes().to_vec()}]).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let draft = SkeletonDraft {
        document: source("main"),
        sections_site: site(),
        sections: vec![SkeletonSection {
            heading: source("sec:fixture:heading"),
            children: vec![SkeletonChild::Unit(source("sec:fixture:caption"))],
        }],
    };
    let metadata = corpus
        .units()
        .iter()
        .map(|u| (u.label().as_str().into(), site()))
        .collect();
    let skeletons = check_skeletons(
        &corpus,
        &[draft],
        &metadata,
        &metadata,
        &BTreeMap::from([("main".into(), site())]),
    )
    .unwrap();
    check(&skeletons, &welded);
}

#[test]
fn display_lifecycle_uses_checked_stores_in_every_layout_region() {
    use NodeId::{LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as Value};
    for case in [
        "unknown",
        "tag-retirement",
        "identity-retirement",
        "absent",
        "claim-outside",
        "quantity-outside",
    ] {
        let mut draft = seed_draft();
        if matches!(case, "absent" | "identity-retirement") {
            draft.nodes.retain(|n| n.id != Three);
            draft
                .edges
                .retain(|e| e.source != Three && e.target != Three);
            draft.coverage.slice.clear();
        }
        if matches!(case, "claim-outside" | "quantity-outside") {
            draft.coverage.slice.clear();
        }
        let graph = validate(draft).unwrap();
        let mut history = crate::tag_history::seed_history();
        let event = HistoryEvent {
            ordinal: 9,
            change: if case == "identity-retirement" {
                HistoryChange::RetireIdentity { identity: Three }
            } else {
                HistoryChange::RetireTag {
                    tag: TagId::LangevinCoefficientThree,
                }
            },
            authority_record: "rec:test:retirement".into(),
            reason: "Withdrawn for correction.".into(),
        };
        if matches!(case, "tag-retirement" | "identity-retirement") {
            history.push(event.clone());
        }
        let book = crate::tag_validation::construct(
            &graph,
            TagId::ALL.map(TagId::descriptor).into(),
            history,
        )
        .unwrap();
        let handle = if case == "unknown" {
            "missing"
        } else if case == "claim-outside" {
            "v:3"
        } else {
            "c:3"
        };
        let cause = match case {
            "unknown" => TagResolutionFault::UnknownTag {
                input: handle.into(),
            },
            "tag-retirement" | "identity-retirement" => TagResolutionFault::RetiredTag {
                input: handle.into(),
                tag: TagId::LangevinCoefficientThree,
                target: Three,
                cause: if case == "tag-retirement" {
                    RetirementCause::TagRetirement(event)
                } else {
                    RetirementCause::IdentityRetirement(event)
                },
            },
            _ => TagResolutionFault::OutOfSliceTag {
                input: handle.into(),
                tag: if case == "claim-outside" {
                    TagId::LangevinCoefficientThreeValue
                } else {
                    TagId::LangevinCoefficientThree
                },
                target: if case == "claim-outside" {
                    Value
                } else {
                    Three
                },
                cause: match case {
                    "absent" => OutOfSliceCause::TargetAbsent,
                    "claim-outside" => OutOfSliceCause::ClaimNotSelected,
                    _ => OutOfSliceCause::QuantityOutsideSupport,
                },
            },
        };
        with_book(&book, |skeletons, welded| {
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                for (kind, region) in [
                    (DisplayKind::Table, 0),
                    (DisplayKind::Table, 1),
                    (DisplayKind::Figure, 0),
                    (DisplayKind::Figure, 1),
                ] {
                    let original = if kind == DisplayKind::Table {
                        let mut d = companion(
                            "| A |\n| --- |\n| B |",
                            r"\begin{tabular}{l} A \\ B \\ \end{tabular}",
                        );
                        d.kind = kind;
                        d
                    } else {
                        companion(
                            "> A\n> B",
                            r"\begin{picture}(1,1)\put(0,0){A}\put(1,1){B}\end{picture}",
                        )
                    };
                    let baseline =
                        check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
                    let mut changed = original.clone();
                    let body = changed
                        .bodies
                        .get_mut(&target)
                        .unwrap()
                        .languages
                        .get_mut("en")
                        .unwrap();
                    let marker = if region == 0 { "A" } else { "B" };
                    let start = body.value.find(marker).unwrap();
                    let insertion = format!("@tag{{{handle}}}");
                    body.value = body.value.replacen(marker, &insertion, 1);
                    let path_region = if kind == DisplayKind::Table
                        && target == DocumentTarget::Markdown
                        && region == 1
                    {
                        2
                    } else {
                        region
                    };
                    let mut location =
                        body_site(path_region, start..start + insertion.len(), Some(0));
                    if kind == DisplayKind::Table {
                        location.path.push(PathPart::Index(0));
                    }
                    let expected = diagnostic(
                        target,
                        location,
                        AssemblyInputFault::DisplayTag {
                            cause: cause.clone(),
                        },
                    );
                    let errors = check_displays(&[changed], skeletons, welded).unwrap_err();
                    assert_eq!(errors, vec![expected.clone()]);
                    assert_eq!(
                        errors[0].to_string(),
                        format!(
                            "M058 DisplayResolution document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=Some(\"en\") target=Some({target:?}) at {}: {cause}",
                            expected.site
                        )
                    );
                    assert_eq!(
                        check_displays(&[original], skeletons, welded)
                            .unwrap()
                            .bodies(),
                        baseline.bodies()
                    );
                }
            }
        });
    }
}
#[test]
fn display_ownership_and_language_faults_restore_complete_bodies() {
    with_context(|skeletons, welded| {
        let original = companion("> A", r"\begin{picture}(1,1)\put(0,0){A}\end{picture}");
        let baseline = check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
        for (owner, fault) in [
            (
                "sec:fixture:unknown",
                AssemblyInputFault::UnknownUnit {
                    label: "sec:fixture:unknown".into(),
                    available: vec!["sec:fixture:caption".into(), "sec:fixture:heading".into()],
                },
            ),
            (
                "sec:fixture:heading",
                AssemblyInputFault::DisplayHeading {
                    unit: "sec:fixture:heading".into(),
                    display_at: site(),
                },
            ),
        ] {
            let mut changed = original.clone();
            changed.unit.value = owner.into();
            let expected = AssemblyInputDiagnostic {
                rule: RuleId::M058,
                stage: AssemblyStage::Ownership,
                document: (owner == "sec:fixture:heading").then(|| "main".into()),
                unit: Some(owner.into()),
                language: None,
                target: None,
                site: AssemblySite::Source(site()),
                fault,
            };
            let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
            assert_eq!(actual, vec![expected.clone()]);
            assert_eq!(
                actual[0].to_string(),
                format!(
                    "M058 Ownership document={:?} unit={:?} language=None target=None at {}: {}",
                    expected.document, expected.unit, expected.site, expected.fault
                )
            );
            assert_eq!(
                check_displays(std::slice::from_ref(&original), skeletons, welded)
                    .unwrap()
                    .bodies(),
                baseline.bodies()
            );
        }
        let mut changed = original.clone();
        let languages = &mut changed
            .bodies
            .get_mut(&DocumentTarget::Latex)
            .unwrap()
            .languages;
        let body = languages.remove("en").unwrap();
        languages.insert("fr".into(), body);
        let expected = AssemblyInputDiagnostic {
            rule: RuleId::M058,
            stage: AssemblyStage::Ownership,
            document: Some("main".into()),
            unit: Some("sec:fixture:caption".into()),
            language: None,
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Source(site()),
            fault: AssemblyInputFault::BodyLanguages {
                target: DocumentTarget::Latex,
                required: vec!["en".into()],
                missing: vec!["en".into()],
                unexpected: vec!["fr".into()],
            },
        };
        assert_eq!(
            check_displays(&[changed], skeletons, welded).unwrap_err(),
            vec![expected]
        );
        assert_eq!(
            check_displays(&[original], skeletons, welded)
                .unwrap()
                .bodies(),
            baseline.bodies()
        );
    });
}
#[test]
fn display_and_documentary_register_statements_are_exact() {
    assert_eq!(
        RULES[57..]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        vec![
            (
                RuleId::M058,
                "Every display has one existing unit owner, exactly one admitted body per target and required language, every body occurrence resolves under that unit's checked context, and a display unit occurs only as a skeleton leaf.",
                "All display companion records and every cell or figure-label occurrence.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M059,
                "Within each display, addressed tag sets and cited label sets are separately equal across all target and language bodies, retaining every occurrence independently of order and multiplicity.",
                "All display bodies after complete body reference resolution.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M060,
                "Documentary handles have immutable bindings to uniquely minted non-theory labels; their ordered provenance ledger admits each handle once and permits only terminal retirement; label-shaped documentary handles are refused and each documentary label has at most one live handle throughout replay; retained target and authority mints resolve, derived live and retired stores partition all admitted documentary handles, and resolution returns the bound documentary label only for a live exact handle.",
                "The complete documentary tag registry, its joint-mint binding context and common-resolver documentary branch, including unused and retired handles.",
                RuleState::Enforced,
                None
            ),
        ]
    );
}

#[test]
fn display_duplicate_owners_empty_corpus_and_missing_targets_restore() {
    with_context(|skeletons, welded| {
        assert!(
            check_displays(&[], skeletons, welded)
                .unwrap()
                .bodies()
                .is_empty()
        );
        let original = companion("> A", r"\begin{picture}(1,1)\put(0,0){A}\end{picture}");
        let baseline = check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
        let expected = AssemblyInputDiagnostic {
            rule: RuleId::M058,
            stage: AssemblyStage::Ownership,
            document: Some("main".into()),
            unit: Some("sec:fixture:caption".into()),
            language: None,
            target: None,
            site: AssemblySite::Source(site()),
            fault: AssemblyInputFault::DisplayMultiplicity {
                unit: "sec:fixture:caption".into(),
                expected: 1,
                actual: 2,
                sites: vec![site(), site()],
            },
        };
        let actual =
            check_displays(&[original.clone(), original.clone()], skeletons, welded).unwrap_err();
        assert_eq!(actual, vec![expected.clone()]);
        assert_eq!(
            actual[0].to_string(),
            format!(
                "M058 Ownership document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=None target=None at {}: {}",
                expected.site, expected.fault
            )
        );
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let mut changed = original.clone();
            changed.bodies.remove(&target);
            let expected = AssemblyInputDiagnostic {
                rule: RuleId::M058,
                stage: AssemblyStage::Ownership,
                document: Some("main".into()),
                unit: Some("sec:fixture:caption".into()),
                language: None,
                target: Some(target),
                site: AssemblySite::Source(site()),
                fault: AssemblyInputFault::BodyLanguages {
                    target,
                    required: vec!["en".into()],
                    missing: vec!["en".into()],
                    unexpected: vec![],
                },
            };
            let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
            assert_eq!(actual, vec![expected.clone()]);
            assert_eq!(
                actual[0].to_string(),
                format!(
                    "M058 Ownership document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=None target=Some({target:?}) at {}: {}",
                    expected.site, expected.fault
                )
            );
            assert_eq!(
                check_displays(std::slice::from_ref(&original), skeletons, welded)
                    .unwrap()
                    .bodies(),
                baseline.bodies()
            );
        }
    });
}

#[test]
fn display_coordinate_fields_and_every_alignment_have_exact_bounds() {
    for alignment in ["---", ":---", "---:", ":---:"] {
        let text = format!("| A |\n| {alignment} |\n| B |");
        assert_eq!(
            parse_display_body(DisplayKind::Table, DocumentTarget::Markdown, &source(&text))
                .unwrap()
                .layout(),
            &DisplayLayout::Table {
                columns: vec![alignment.into()],
                rows: vec![vec![0], vec![1]],
                rules: vec![]
            }
        );
    }
    let positive =
        r"\begin{picture}(4096,4096)\put(4096,4096){A}\put(0,0){\line(-1,1){4096}}\end{picture}";
    assert_eq!(
        parse_display_body(
            DisplayKind::Figure,
            DocumentTarget::Latex,
            &source(positive)
        )
        .unwrap()
        .layout(),
        &DisplayLayout::Picture {
            width: 4096,
            height: 4096,
            commands: vec![
                PictureCommand::Label {
                    x: 4096,
                    y: 4096,
                    region: 0
                },
                PictureCommand::Line {
                    x: 0,
                    y: 0,
                    dx: -1,
                    dy: 1,
                    length: 4096,
                    vector: false
                }
            ]
        }
    );
    for (field, text, value, min, max) in [
        (
            "H",
            r"\begin{picture}(1,4097)\put(0,0){A}\end{picture}",
            "4097",
            1,
            4096,
        ),
        (
            "X",
            r"\begin{picture}(1,1)\put(2,0){A}\end{picture}",
            "2",
            0,
            1,
        ),
        (
            "Y",
            r"\begin{picture}(1,1)\put(0,-1){A}\end{picture}",
            "-1",
            0,
            1,
        ),
        (
            "DX",
            r"\begin{picture}(1,1)\put(0,0){\line(2,0){1}}\put(0,0){A}\end{picture}",
            "2",
            -1,
            1,
        ),
        (
            "DY",
            r"\begin{picture}(1,1)\put(0,0){\vector(0,-2){1}}\put(0,0){A}\end{picture}",
            "-2",
            -1,
            1,
        ),
        (
            "L",
            r"\begin{picture}(1,1)\put(0,0){\line(1,0){4097}}\put(0,0){A}\end{picture}",
            "4097",
            1,
            4096,
        ),
    ] {
        let at = text.find(value).unwrap();
        refusal(
            DisplayKind::Figure,
            DocumentTarget::Latex,
            text,
            at..at + value.len(),
            AssemblyInputFault::Coordinate {
                field: field.into(),
                value: value.into(),
                minimum: min,
                maximum: max,
            },
        );
    }
    for token in ["+1", "1.0", "1e0", "NaN", "--1"] {
        let text = format!("\\begin{{picture}}({token},1)\\put(0,0){{A}}\\end{{picture}}");
        refusal(
            DisplayKind::Figure,
            DocumentTarget::Latex,
            &text,
            16..16 + token.len(),
            syntax(
                DisplayKind::Figure,
                DisplaySyntaxReason::InvalidInteger,
                token,
                &["minimal decimal integer"],
            ),
        );
    }
}

#[test]
fn picture_integer_spellings_are_minimal_in_every_field_and_restore() {
    with_context(|skeletons, welded| {
        let template = r"\begin{picture}(W,H)\put(X,Y){\line(DX,DY){L}}\put(0,0){A}\end{picture}";
        let fields = [
            ("W", "4096", 1, 4096),
            ("H", "4096", 1, 4096),
            ("X", "0", 0, 4096),
            ("Y", "0", 0, 4096),
            ("DX", "-1", -1, 1),
            ("DY", "1", -1, 1),
            ("L", "4096", 1, 4096),
        ];
        let render = |field: &str, value: &str| {
            // Longest placeholders first, so X never consumes the end of DX.
            ["DX", "DY", "W", "H", "X", "Y", "L"].into_iter().fold(
                template.to_owned(),
                |text, name| {
                    let original = fields.iter().find(|(f, _, _, _)| *f == name).unwrap().1;
                    text.replace(name, if name == field { value } else { original })
                },
            )
        };
        let positive = render("", "");
        let original = companion("> A", &positive);
        let baseline = check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
        for (field, valid, minimum, maximum) in fields {
            for token in [
                "00",
                "01",
                "-0",
                "-01",
                "+1",
                "",
                "-",
                "١",
                "１",
                "1.0",
                "99999999999999999999",
            ] {
                let marked = render(field, "TOKEN");
                let at = marked.find("TOKEN").unwrap();
                let text = marked.replace("TOKEN", token);
                let mut changed = companion("> A", &text);
                let body = changed
                    .bodies
                    .get_mut(&DocumentTarget::Latex)
                    .unwrap()
                    .languages
                    .get_mut("en")
                    .unwrap();
                body.site.path[2] = PathPart::Key("latex".into());
                body.site.anchor = SourceAnchor::Bytes(100, 102 + text.len());
                let mut expected_site = body.site.clone();
                expected_site.decoded = Some(at..at + token.len());
                let fault = if token == "99999999999999999999" {
                    AssemblyInputFault::Coordinate {
                        field: field.into(),
                        value: token.into(),
                        minimum,
                        maximum,
                    }
                } else {
                    syntax(
                        DisplayKind::Figure,
                        DisplaySyntaxReason::InvalidInteger,
                        token,
                        &["minimal decimal integer"],
                    )
                };
                let expected = AssemblyInputDiagnostic {
                    rule: RuleId::M058,
                    stage: AssemblyStage::DisplaySyntax,
                    document: Some("main".into()),
                    unit: Some("sec:fixture:caption".into()),
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: AssemblySite::Source(expected_site),
                    fault,
                };
                let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
                assert_eq!(actual, vec![expected.clone()]);
                assert_eq!(
                    actual[0].to_string(),
                    format!(
                        "M058 DisplaySyntax document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=Some(\"en\") target=Some(Latex) at {}: {}",
                        expected.site, expected.fault
                    )
                );
                assert_eq!(
                    check_displays(std::slice::from_ref(&original), skeletons, welded)
                        .unwrap()
                        .bodies(),
                    baseline.bodies()
                );
            }
            for value in [
                minimum.to_string(),
                maximum.to_string(),
                format!(" \t{valid}\n "),
            ] {
                let restored = companion("> A", &render(field, &value));
                assert!(
                    check_displays(&[restored], skeletons, welded).is_ok(),
                    "{field} {value}"
                );
            }
        }
    });
}

#[test]
fn native_layout_row_and_column_limits_stop_at_the_first_excess() {
    let table = format!(
        "\\begin{{tabular}}{{l}}{}\\end{{tabular}}",
        r"A \\ ".repeat(1024)
    );
    let parsed =
        parse_display_body(DisplayKind::Table, DocumentTarget::Latex, &source(&table)).unwrap();
    assert_eq!(parsed.regions().len(), 1024);
    let prefix = format!("\\begin{{tabular}}{{l}}{}", r"A \\ ".repeat(1024));
    let excess = format!("{prefix}A \\\\ \\end{{tabular}}");
    let at = prefix.len();
    refusal(
        DisplayKind::Table,
        DocumentTarget::Latex,
        &excess,
        at..at + 2,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyRows,
            limit: 1024,
            actual: 1025,
        },
    );
    let columns = "l".repeat(64);
    let row = vec!["A"; 64].join(" & ");
    let table = format!("\\begin{{tabular}}{{{columns}}}{row} \\\\ {row} \\\\ \\end{{tabular}}");
    assert_eq!(
        parse_display_body(DisplayKind::Table, DocumentTarget::Latex, &source(&table))
            .unwrap()
            .regions()
            .len(),
        128
    );
    let excess = format!("\\begin{{tabular}}{{{columns}l}}");
    refusal(
        DisplayKind::Table,
        DocumentTarget::Latex,
        &excess,
        80..81,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyColumns,
            limit: 64,
            actual: 65,
        },
    );
    let commands = r"\put(0,0){A}".repeat(1024);
    let picture = format!("\\begin{{picture}}(1,1){commands}\\end{{picture}}");
    assert_eq!(
        parse_display_body(
            DisplayKind::Figure,
            DocumentTarget::Latex,
            &source(&picture)
        )
        .unwrap()
        .regions()
        .len(),
        1024
    );
    let at = 20 + commands.len();
    let excess = format!("\\begin{{picture}}(1,1){commands}\\put(0,0){{A}}\\end{{picture}}");
    refusal(
        DisplayKind::Figure,
        DocumentTarget::Latex,
        &excess,
        at..at,
        AssemblyInputFault::Limit {
            resource: AssemblyResource::BodyRows,
            limit: 1024,
            actual: 1025,
        },
    );
}

fn body_site(region: usize, bytes: std::ops::Range<usize>, occurrence: Option<usize>) -> UnitSite {
    let mut site = site();
    site.path
        .extend([PathPart::Key("regions".into()), PathPart::Index(region)]);
    site.decoded = Some(bytes);
    site.occurrence = occurrence;
    site
}
fn diagnostic(
    target: DocumentTarget,
    site: UnitSite,
    fault: AssemblyInputFault,
) -> AssemblyInputDiagnostic {
    AssemblyInputDiagnostic {
        rule: RuleId::M058,
        stage: AssemblyStage::DisplayResolution,
        document: Some("main".into()),
        unit: Some("sec:fixture:caption".into()),
        language: Some("en".into()),
        target: Some(target),
        site: AssemblySite::Source(site),
        fault,
    }
}
#[test]
fn display_every_reference_is_resolved_before_any_set_comparison() {
    with_context(|skeletons, welded| {
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            for (text, fault, occurrence) in [
                (
                    "@tag{missing}",
                    AssemblyInputFault::DisplayTag {
                        cause: TagResolutionFault::UnknownTag {
                            input: "missing".into(),
                        },
                    },
                    Some(0),
                ),
                (
                    "@math{missing}",
                    AssemblyInputFault::DisplayMathReference {
                        name: "missing".into(),
                        available: vec![],
                    },
                    Some(0),
                ),
                (
                    "`dec:missing:source`",
                    AssemblyInputFault::DisplayCitation {
                        label: "dec:missing:source".into(),
                        span_index: 0,
                        expected: 1,
                        actual: 0,
                    },
                    None,
                ),
            ] {
                for position in 0..4 {
                    let mut original = if position < 2 {
                        companion(
                            "| A |\n| --- |\n| B |",
                            r"\begin{tabular}{l} A \\ B \\ \end{tabular}",
                        )
                    } else {
                        companion(
                            "> A\n> B",
                            r"\begin{picture}(1,1)\put(0,0){A}\put(1,1){B}\end{picture}",
                        )
                    };
                    original.kind = if position < 2 {
                        DisplayKind::Table
                    } else {
                        DisplayKind::Figure
                    };
                    let baseline =
                        check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
                    let mut changed = original.clone();
                    let source = changed
                        .bodies
                        .get_mut(&target)
                        .unwrap()
                        .languages
                        .get_mut("en")
                        .unwrap();
                    let marker = if position % 2 == 0 { "A" } else { "B" };
                    let start = source.value.find(marker).unwrap();
                    source.value = source.value.replacen(marker, text, 1);
                    let row = if position == 1 && target == DocumentTarget::Markdown {
                        2
                    } else {
                        position % 2
                    };
                    let mut location = body_site(row, start..start + text.len(), occurrence);
                    if position < 2 {
                        location.path.push(PathPart::Index(0));
                    }
                    let expected = diagnostic(target, location, fault.clone());
                    let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
                    assert_eq!(actual, vec![expected.clone()]);
                    assert_eq!(
                        actual[0].to_string(),
                        format!(
                            "M058 DisplayResolution document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=Some(\"en\") target=Some({target:?}) at {}: {}",
                            expected.site, expected.fault
                        )
                    );
                    assert_eq!(
                        check_displays(&[original], skeletons, welded)
                            .unwrap()
                            .bodies(),
                        baseline.bodies()
                    );
                }
            }
        }
        let mentions = companion(
            "> ``dec:missing:source``",
            r"\begin{picture}(1,1)\put(0,0){``other:unknown:label``}\end{picture}",
        );
        let checked = check_displays(&[mentions], skeletons, welded).unwrap();
        assert_eq!(checked.bodies()["sec:fixture:caption"].len(), 2);
    });
}
#[test]
fn display_address_families_keep_both_complete_sides_and_all_repetitions() {
    with_context(|skeletons, welded| {
        let original = companion(
            "> @tag{c:1} @tag{c:1} `dec:fixture:source`",
            r"\begin{picture}(1,1)\put(0,0){`dec:fixture:source` @tag{c:1}}\end{picture}",
        );
        let baseline = check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
        let mut changed = original.clone();
        changed
            .bodies
            .get_mut(&DocumentTarget::Latex)
            .unwrap()
            .languages
            .get_mut("en")
            .unwrap()
            .value = r"\begin{picture}(1,1)\put(0,0){@tag{c:3}}\end{picture}".into();
        let side = |target, members| BodyAddressSide {
            target,
            language: "en".into(),
            site: site(),
            members,
        };
        let cases = [
            (
                AddressFamily::Tags,
                side(
                    DocumentTarget::Markdown,
                    vec![BodyAddressMember {
                        value: "c:1".into(),
                        target: Some(NodeId::LangevinCoefficientOne),
                        sites: vec![body_site(0, 2..11, Some(0)), body_site(0, 12..21, Some(1))],
                    }],
                ),
                side(
                    DocumentTarget::Latex,
                    vec![BodyAddressMember {
                        value: "c:3".into(),
                        target: Some(NodeId::LangevinCoefficientThree),
                        sites: vec![body_site(0, 30..39, Some(0))],
                    }],
                ),
                vec!["c:1".into()],
                vec!["c:3".into()],
            ),
            (
                AddressFamily::Citations,
                side(
                    DocumentTarget::Markdown,
                    vec![BodyAddressMember {
                        value: "dec:fixture:source".into(),
                        target: None,
                        sites: vec![body_site(0, 22..42, None)],
                    }],
                ),
                side(DocumentTarget::Latex, vec![]),
                vec!["dec:fixture:source".into()],
                vec![],
            ),
        ];
        let expected: Vec<_> = cases
            .into_iter()
            .map(
                |(family, reference, actual, missing, unexpected)| AssemblyInputDiagnostic {
                    rule: RuleId::M059,
                    stage: AssemblyStage::DisplayEquality,
                    document: Some("main".into()),
                    unit: Some("sec:fixture:caption".into()),
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: AssemblySite::Source(site()),
                    fault: AssemblyInputFault::DisplaySetMismatch {
                        family,
                        reference,
                        actual,
                        missing,
                        unexpected,
                    },
                },
            )
            .collect();
        let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
        assert_eq!(actual, expected);
        for error in &actual {
            assert_eq!(
                error.to_string(),
                format!(
                    "M059 DisplayEquality document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=Some(\"en\") target=Some(Latex) at {}: {}",
                    error.site, error.fault
                )
            );
        }
        assert_eq!(
            check_displays(&[original], skeletons, welded)
                .unwrap()
                .bodies(),
            baseline.bodies()
        );
        // Opposite arrows are a human semantic disagreement; identical addressed sets pass.
        let opposite = companion(
            "> @tag{c:1} → @tag{c:3}",
            r"\begin{picture}(120,20)\put(0,10){@tag{c:1}}\put(75,10){\vector(-1,0){40}}\put(90,10){@tag{c:3}}\end{picture}",
        );
        let checked = check_displays(&[opposite], skeletons, welded).unwrap();
        assert_eq!(checked.bodies()["sec:fixture:caption"].len(), 2);
    });
}

#[test]
fn display_citation_to_tag_swap_retains_distinct_address_families() {
    with_context(|skeletons, welded| {
        let label = "def:langevin:coefficient-one";
        let original = companion(
            &format!("> `{label}`"),
            &format!("\\begin{{picture}}(1,1)\\put(0,0){{`{label}`}}\\end{{picture}}"),
        );
        let baseline = check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
        let changed = companion(
            &format!("> `{label}`"),
            r"\begin{picture}(1,1)\put(0,0){@tag{c:1}}\end{picture}",
        );
        let side = |target, members| BodyAddressSide {
            target,
            language: "en".into(),
            site: site(),
            members,
        };
        let expected = [
            (
                AddressFamily::Tags,
                side(DocumentTarget::Markdown, vec![]),
                side(
                    DocumentTarget::Latex,
                    vec![BodyAddressMember {
                        value: "c:1".into(),
                        target: Some(NodeId::LangevinCoefficientOne),
                        sites: vec![body_site(0, 30..39, Some(0))],
                    }],
                ),
                vec![],
                vec!["c:1".into()],
            ),
            (
                AddressFamily::Citations,
                side(
                    DocumentTarget::Markdown,
                    vec![BodyAddressMember {
                        value: label.into(),
                        target: None,
                        sites: vec![body_site(0, 2..4 + label.len(), None)],
                    }],
                ),
                side(DocumentTarget::Latex, vec![]),
                vec![label.into()],
                vec![],
            ),
        ]
        .into_iter()
        .map(
            |(family, reference, actual, missing, unexpected)| AssemblyInputDiagnostic {
                rule: RuleId::M059,
                stage: AssemblyStage::DisplayEquality,
                document: Some("main".into()),
                unit: Some("sec:fixture:caption".into()),
                language: Some("en".into()),
                target: Some(DocumentTarget::Latex),
                site: AssemblySite::Source(site()),
                fault: AssemblyInputFault::DisplaySetMismatch {
                    family,
                    reference,
                    actual,
                    missing,
                    unexpected,
                },
            },
        )
        .collect::<Vec<_>>();
        let actual = check_displays(&[changed], skeletons, welded).unwrap_err();
        assert_eq!(actual, expected);
        for error in &actual {
            assert_eq!(
                error.to_string(),
                format!(
                    "M059 DisplayEquality document=Some(\"main\") unit=Some(\"sec:fixture:caption\") language=Some(\"en\") target=Some(Latex) at {}: {}",
                    error.site, error.fault
                )
            );
        }
        assert_eq!(
            check_displays(&[original], skeletons, welded)
                .unwrap()
                .bodies(),
            baseline.bodies()
        );
    });
}

#[test]
fn display_three_languages_resolve_each_site_and_use_the_smallest_reference_language() {
    let at = site();
    let located = |value: &str| LocatedString {
        value: value.into(),
        site: at.clone(),
    };
    let declaration = |languages: &[&str]| DocumentEntryDraft {
        key_site: at.clone(),
        table_site: at.clone(),
        languages_site: at.clone(),
        required_languages: languages.iter().map(|l| located(l)).collect(),
    };
    let documents = DocumentDraft {
        documents_site: at.clone(),
        documents: BTreeMap::from([
            ("main".into(), declaration(&["en"])),
            ("explainer".into(), declaration(&["fr", "en", "de"])),
        ]),
    };
    let units = [
        ("main", "main", vec!["en"]),
        ("heading", "explainer", vec!["de", "en", "fr"]),
        ("caption", "explainer", vec!["de", "en", "fr"]),
    ]
    .map(|(name, document, languages)| UnitDraft {
        id: located(&format!("sec:fixture:{name}")),
        document: located(document),
        table_site: at.clone(),
        math_site: at.clone(),
        texts_site: at.clone(),
        math: BTreeMap::new(),
        texts: languages
            .into_iter()
            .map(|l| {
                (
                    l.into(),
                    LocatedEntryDraft {
                        key_site: at.clone(),
                        value: located("Caption."),
                    },
                )
            })
            .collect(),
    });
    let corpus = admit_unit_drafts(documents, units.into()).unwrap();
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let markdown = check_markdown_sources(vec![MarkdownSource {
        file: "records.md".into(),
        bytes:
            "· `proposal:manifest:kernel-bite`\n· `proposal:manifest:addressing-lifecycle-bite`\n"
                .as_bytes()
                .into(),
    }])
    .unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let drafts = [
        ("main", "main", vec![]),
        (
            "explainer",
            "heading",
            vec![SkeletonChild::Unit(located("sec:fixture:caption"))],
        ),
    ]
    .map(|(document, heading, children)| SkeletonDraft {
        document: located(document),
        sections_site: at.clone(),
        sections: vec![SkeletonSection {
            heading: located(&format!("sec:fixture:{heading}")),
            children,
        }],
    });
    let metadata = corpus
        .units()
        .iter()
        .map(|u| (u.label().as_str().into(), at.clone()))
        .collect();
    let declarations = BTreeMap::from([
        ("main".into(), at.clone()),
        ("explainer".into(), at.clone()),
    ]);
    let skeletons = check_skeletons(&corpus, &drafts, &metadata, &metadata, &declarations).unwrap();
    let mut original = companion(
        "> @tag{c:1}",
        r"\begin{picture}(1,1)\put(0,0){@tag{c:1}}\end{picture}",
    );
    for target in original.bodies.values_mut() {
        let body = target.languages["en"].clone();
        target.languages = BTreeMap::from([
            ("de".into(), body.clone()),
            ("en".into(), body.clone()),
            ("fr".into(), body),
        ]);
    }
    let baseline = check_displays(std::slice::from_ref(&original), &skeletons, &welded).unwrap();
    assert_eq!(baseline.bodies()["sec:fixture:caption"].len(), 6);
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        for language in ["de", "en", "fr"] {
            let mut changed = original.clone();
            let body = changed
                .bodies
                .get_mut(&target)
                .unwrap()
                .languages
                .get_mut(language)
                .unwrap();
            body.value = body.value.replace("c:1", "missing");
            let start = if target == DocumentTarget::Markdown {
                2
            } else {
                30
            };
            let mut expected = diagnostic(
                target,
                body_site(0, start..start + 13, Some(0)),
                AssemblyInputFault::DisplayTag {
                    cause: TagResolutionFault::UnknownTag {
                        input: "missing".into(),
                    },
                },
            );
            expected.document = Some("explainer".into());
            expected.language = Some(language.into());
            assert_eq!(
                check_displays(&[changed], &skeletons, &welded).unwrap_err(),
                vec![expected]
            );
            assert_eq!(
                check_displays(std::slice::from_ref(&original), &skeletons, &welded)
                    .unwrap()
                    .bodies(),
                baseline.bodies()
            );
        }
    }
    let mut changed = original.clone();
    changed
        .bodies
        .get_mut(&DocumentTarget::Latex)
        .unwrap()
        .languages
        .get_mut("fr")
        .unwrap()
        .value = original.bodies[&DocumentTarget::Latex].languages["fr"]
        .value
        .replace("c:1", "c:3");
    let side = |target, language: &str, value: &str, id, start| BodyAddressSide {
        target,
        language: language.into(),
        site: at.clone(),
        members: vec![BodyAddressMember {
            value: value.into(),
            target: Some(id),
            sites: vec![body_site(0, start..start + 9, Some(0))],
        }],
    };
    let expected = AssemblyInputDiagnostic {
        rule: RuleId::M059,
        stage: AssemblyStage::DisplayEquality,
        document: Some("explainer".into()),
        unit: Some("sec:fixture:caption".into()),
        language: Some("fr".into()),
        target: Some(DocumentTarget::Latex),
        site: AssemblySite::Source(at.clone()),
        fault: AssemblyInputFault::DisplaySetMismatch {
            family: AddressFamily::Tags,
            reference: side(
                DocumentTarget::Markdown,
                "de",
                "c:1",
                NodeId::LangevinCoefficientOne,
                2,
            ),
            actual: side(
                DocumentTarget::Latex,
                "fr",
                "c:3",
                NodeId::LangevinCoefficientThree,
                30,
            ),
            missing: vec!["c:1".into()],
            unexpected: vec!["c:3".into()],
        },
    };
    assert_eq!(
        check_displays(&[changed], &skeletons, &welded).unwrap_err(),
        vec![expected]
    );
    assert_eq!(
        check_displays(&[original], &skeletons, &welded)
            .unwrap()
            .bodies(),
        baseline.bodies()
    );
}
#[test]
fn display_cross_unit_body_swaps_cannot_cancel_each_owners_mismatch() {
    let located = |value: &str| source(value);
    let documents = DocumentDraft {
        documents_site: site(),
        documents: BTreeMap::from([(
            "main".into(),
            DocumentEntryDraft {
                key_site: site(),
                table_site: site(),
                languages_site: site(),
                required_languages: vec![located("en")],
            },
        )]),
    };
    let units = ["heading", "first", "second"].map(|name| UnitDraft {
        id: located(&format!("sec:fixture:{name}")),
        document: located("main"),
        table_site: site(),
        math_site: site(),
        texts_site: site(),
        math: BTreeMap::new(),
        texts: BTreeMap::from([(
            "en".into(),
            LocatedEntryDraft {
                key_site: site(),
                value: located("Caption."),
            },
        )]),
    });
    let corpus = admit_unit_drafts(documents, units.into()).unwrap();
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let markdown = check_markdown_sources(vec![MarkdownSource {file:"records.md".into(),bytes:b"\xC2\xB7 `proposal:manifest:kernel-bite`\n\xC2\xB7 `proposal:manifest:addressing-lifecycle-bite`\n".to_vec()}]).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let draft = SkeletonDraft {
        document: located("main"),
        sections_site: site(),
        sections: vec![SkeletonSection {
            heading: located("sec:fixture:heading"),
            children: vec![
                SkeletonChild::Unit(located("sec:fixture:first")),
                SkeletonChild::Unit(located("sec:fixture:second")),
            ],
        }],
    };
    let metadata = corpus
        .units()
        .iter()
        .map(|unit| (unit.label().as_str().into(), site()))
        .collect();
    let skeletons = check_skeletons(
        &corpus,
        &[draft],
        &metadata,
        &metadata,
        &BTreeMap::from([("main".into(), site())]),
    )
    .unwrap();
    let original: Vec<_> = [("first", "c:1"), ("second", "c:3")]
        .into_iter()
        .map(|(owner, tag)| {
            let mut display = companion(
                &format!("> @tag{{{tag}}}"),
                &format!("\\begin{{picture}}(1,1)\\put(0,0){{@tag{{{tag}}}}}\\end{{picture}}"),
            );
            display.unit = located(&format!("sec:fixture:{owner}"));
            display
        })
        .collect();
    let baseline = check_displays(&original, &skeletons, &welded).unwrap();
    let mut changed = original.clone();
    for (index, display) in changed.iter_mut().enumerate() {
        display.bodies.insert(
            DocumentTarget::Latex,
            original[1 - index].bodies[&DocumentTarget::Latex].clone(),
        );
    }
    let member = |tag: &str, id, start| BodyAddressMember {
        value: tag.into(),
        target: Some(id),
        sites: vec![body_site(0, start..start + 9, Some(0))],
    };
    let side = |target, member| BodyAddressSide {
        target,
        language: "en".into(),
        site: site(),
        members: vec![member],
    };
    let expected: Vec<_> = [
        (
            "first",
            "c:1",
            NodeId::LangevinCoefficientOne,
            "c:3",
            NodeId::LangevinCoefficientThree,
        ),
        (
            "second",
            "c:3",
            NodeId::LangevinCoefficientThree,
            "c:1",
            NodeId::LangevinCoefficientOne,
        ),
    ]
    .into_iter()
    .map(
        |(owner, wanted, id, received, other)| AssemblyInputDiagnostic {
            rule: RuleId::M059,
            stage: AssemblyStage::DisplayEquality,
            document: Some("main".into()),
            unit: Some(format!("sec:fixture:{owner}")),
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: AssemblySite::Source(site()),
            fault: AssemblyInputFault::DisplaySetMismatch {
                family: AddressFamily::Tags,
                reference: side(DocumentTarget::Markdown, member(wanted, id, 2)),
                actual: side(DocumentTarget::Latex, member(received, other, 30)),
                missing: vec![wanted.into()],
                unexpected: vec![received.into()],
            },
        },
    )
    .collect();
    let actual = check_displays(&changed, &skeletons, &welded).unwrap_err();
    assert_eq!(actual, expected);
    for error in actual {
        assert_eq!(
            error.to_string(),
            format!(
                "M059 DisplayEquality document=Some(\"main\") unit={:?} language=Some(\"en\") target=Some(Latex) at {}: {}",
                error.unit, error.site, error.fault
            )
        );
    }
    assert_eq!(
        check_displays(&original, &skeletons, &welded)
            .unwrap()
            .bodies(),
        baseline.bodies()
    );
}

fn with_documentary(
    events: Vec<DocumentaryEventDraft>,
    check: impl FnOnce(&CheckedSkeletons<'_>, &WeldedUnits<'_, '_, '_, '_, '_, '_>),
) {
    let corpus = corpus();
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"records.md".into(),bytes:"· `proposal:manifest:kernel-bite`\n· `proposal:manifest:addressing-lifecycle-bite`\n· `dec:fixture:source`\n· `rec:test:retirement`\n".as_bytes().to_vec()}]).unwrap();
    let mints = collect_joint_mints(&corpus, &markdown).unwrap();
    let graph = seed().unwrap();
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&mints, events)
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_joint_labels(&resolved, &mints).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let draft = SkeletonDraft {
        document: source("main"),
        sections_site: site(),
        sections: vec![SkeletonSection {
            heading: source("sec:fixture:heading"),
            children: vec![SkeletonChild::Unit(source("sec:fixture:caption"))],
        }],
    };
    let metadata = corpus
        .units()
        .iter()
        .map(|u| (u.label().as_str().into(), site()))
        .collect();
    let skeletons = check_skeletons(
        &corpus,
        &[draft],
        &metadata,
        &metadata,
        &BTreeMap::from([("main".into(), site())]),
    )
    .unwrap();
    check(&skeletons, &welded);
}

#[test]
fn documentary_display_tags_refuse_live_domain_and_retirement() {
    let admit = DocumentaryEventDraft {
        site: site(),
        ordinal: source("1"),
        handle: source("record"),
        label: Some(source("dec:fixture:source")),
        authority_record: source("dec:fixture:source"),
        reason: source("Fixture admission."),
    };
    let retire = DocumentaryEventDraft {
        site: site(),
        ordinal: source("2"),
        label: None,
        reason: source("Fixture retirement."),
        ..admit.clone()
    };
    for retired in [false, true] {
        let events = if retired {
            vec![admit.clone(), retire.clone()]
        } else {
            vec![admit.clone()]
        };
        with_documentary(events, |skeletons, welded| {
            let original = companion("> A", r"\begin{picture}(1,1)\put(0,0){A}\end{picture}");
            let baseline =
                check_displays(std::slice::from_ref(&original), skeletons, welded).unwrap();
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                let mut bad = original.clone();
                let body = bad
                    .bodies
                    .get_mut(&target)
                    .unwrap()
                    .languages
                    .get_mut("en")
                    .unwrap();
                let start = body.value.find('A').unwrap();
                body.value = body.value.replacen('A', "@tag{record}", 1);
                let fault = if retired {
                    AssemblyInputFault::DisplayTag {
                        cause: TagResolutionFault::RetiredDocumentaryTag {
                            input: "record".into(),
                            label: "dec:fixture:source".into(),
                            event: DocumentaryRetirement {
                                ordinal: 2,
                                authority_record: "dec:fixture:source".into(),
                                reason: "Fixture retirement.".into(),
                                site: site(),
                            },
                        },
                    }
                } else {
                    AssemblyInputFault::DocumentaryTagTarget {
                        input: "record".into(),
                        label: "dec:fixture:source".into(),
                    }
                };
                let expected = diagnostic(target, body_site(0, start..start + 12, Some(0)), fault);
                let actual = check_displays(&[bad], skeletons, welded).unwrap_err();
                assert_eq!(actual, vec![expected.clone()]);
                assert_eq!(actual[0].to_string(), expected.to_string());
                assert_eq!(
                    check_displays(std::slice::from_ref(&original), skeletons, welded)
                        .unwrap()
                        .bodies(),
                    baseline.bodies()
                );
            }
        });
    }
}
