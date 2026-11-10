use super::*;
use ftoo_manifest::{
    LabelSide, LabelSite, LabelUseSite, NodeId, PathPart, ReferenceDomain, RuleId, SourceAnchor,
    TagMember, TagSide, UnitDiagnostic, UnitFault, UnitSite, UnitStage, WeldDiagnostic, WeldFault,
    WeldStage,
};
use std::{
    collections::BTreeMap,
    fmt::Write,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
const UNIT: &str = "sec:fixture:unit";
const HEADER: &str =
    "schema=1\n[unit]\nid='sec:fixture:unit'\ndocument='explainer'\nmath={}\n[unit.texts]\n";
const BASE: &str = "See @tag{c:1} and `sec:fixture:target`.";
const DECLARATIONS: &str = "schema=1\n[documents.main]\nrequired-languages=['en']\n[documents.explainer]\nrequired-languages=['en','fr']\n";
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn statement(en: &str, fr: &str) -> String {
    format!("{HEADER}en='{en}'\nfr='{fr}'\n")
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ftoo-weld-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("units/statements")).unwrap();
    fs::write(root.join("units/documents.toml"), DECLARATIONS).unwrap();
    fs::write(
        root.join("units/statements/unit.toml"),
        statement(BASE, BASE),
    )
    .unwrap();
    for source in load_markdown_files(&self::root()).unwrap().sources() {
        let path = root.join(&source.file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &source.bytes).unwrap();
    }
    fs::write(
        root.join("docs/weld-fixture.md"),
        "· `sec:fixture:target`\n· `sec:fixture:other`\n",
    )
    .unwrap();
    root
}

fn assert_literal_sites(
    welded: &ftoo_manifest::WeldedUnits<'_, '_, '_, '_, '_, '_>,
    literal: &str,
) {
    for (language, start) in [
        ("en", HEADER.len() + 3),
        ("fr", HEADER.len() + literal.len() + 7),
    ] {
        let expected = |decoded, occurrence| UnitSite {
            file: "units/statements/unit.toml".into(),
            path: ["unit", "texts", language]
                .map(|k| PathPart::Key(k.into()))
                .to_vec(),
            anchor: SourceAnchor::Bytes(start, start + literal.len()),
            decoded,
            occurrence,
        };
        let actual = welded.units()[0]
            .languages()
            .iter()
            .find(|l| l.language() == language)
            .unwrap();
        assert_eq!(
            actual.tags(),
            &TagSide {
                language: language.into(),
                text_site: expected(None, None),
                members: vec![TagMember {
                    input: "c:1".into(),
                    target: NodeId::LangevinCoefficientOne,
                    sites: vec![expected(Some(19..28), Some(0))]
                }]
            }
        );
        assert_eq!(
            actual.mentions(),
            [LabelUseSite {
                span_index: 0,
                site: expected(Some(7..18), None)
            }]
        );
        assert_eq!(
            actual.labels(),
            &LabelSide {
                language: language.into(),
                text_site: expected(None, None),
                members: vec![ftoo_manifest::LabelMember {
                    label: "sec:fixture:target".into(),
                    sites: vec![LabelUseSite {
                        span_index: 1,
                        site: expected(Some(29..49), None)
                    }]
                }]
            }
        );
    }
}

#[test]
fn w11_real_literal_forms_keep_raw_anchors_and_separate_occurrence_indices() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let root = fixture();
    let original = snapshot(&root);
    let path = root.join("units/statements/unit.toml");
    let saved = fs::read(&path).unwrap();
    let baseline = check_weld_files(&root, &book).unwrap();
    let text = "é See ``exemple`` @tag{c:1} `sec:fixture:target`.";
    for literal in [
        "'é See ``exemple`` @tag{c:1} `sec:fixture:target`.'",
        "\"\\u00e9 See ``exemple`` @tag{c:1} `sec:fixture:target`.\"",
        "'''\né See ``exemple`` @tag{c:1} `sec:fixture:target`.'''",
        "\"\"\"\r\né See ``exemple`` @tag{c:1} `sec:fixture:target`.\"\"\"",
        "\"\"\"é See ``exemple`` \\\n  @tag{c:1} `sec:fixture:target`.\"\"\"",
    ] {
        let source = format!("{HEADER}en={literal}\nfr={literal}\n");
        fs::write(&path, &source).unwrap();
        let changed = snapshot(&root);
        assert_eq!(
            check_weld_files(&root, &book).unwrap(),
            WeldCheckSummary {
                mention_sites: 2,
                ..baseline
            }
        );
        let corpus = load_unit_files(&root).unwrap();
        let resolved = resolve_unit_tags(&corpus, &book).unwrap();
        let markdown = load_markdown_files(&root).unwrap();
        let integrated = integrate_labels(&resolved, &markdown).unwrap();
        let welded = weld_units(&integrated).unwrap();
        for language in ["en", "fr"] {
            assert_eq!(corpus.units()[0].texts()[language].text(), text);
        }
        assert_literal_sites(&welded, literal);
        assert_eq!(snapshot(&root), changed);
        fs::write(&path, &saved).unwrap();
        assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
        assert_eq!(snapshot(&root), original);
        let unknown = source.replace("sec:fixture:target", "sec:fixture:absent");
        fs::write(&path, &unknown).unwrap();
        let changed = snapshot(&root);
        let expected: Vec<_> = [
            ("en", HEADER.len() + 3),
            ("fr", HEADER.len() + literal.len() + 7),
        ]
        .into_iter()
        .map(|(language, start)| {
            diagnostic(
                language,
                UnitSite {
                    file: "units/statements/unit.toml".into(),
                    path: ["unit", "texts", language]
                        .map(|k| PathPart::Key(k.into()))
                        .to_vec(),
                    anchor: SourceAnchor::Bytes(start, start + literal.len()),
                    decoded: Some(29..49),
                    occurrence: None,
                },
                WeldFault::UnresolvedUnitCitation {
                    label: "sec:fixture:absent".into(),
                    span_index: 1,
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::Joint,
                },
            )
        })
        .collect();
        assert_weld(&check_weld_files(&root, &book).unwrap_err(), &expected);
        assert_eq!(snapshot(&root), changed);
        fs::write(&path, &saved).unwrap();
        assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
        assert_eq!(snapshot(&root), original);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn w12_actual_tag_refusals_precede_unknown_citations_at_every_occurrence() {
    use ftoo_manifest::{OutOfSliceCause as O, TagId as T, TagResolutionFault as F};
    let full = ftoo_manifest::seed().unwrap();
    let full_book = ftoo_manifest::addressing(&full).unwrap();
    let mut draft = ftoo_manifest::seed_draft();
    draft.coverage.slice.clear();
    let empty = ftoo_manifest::validate(draft.clone()).unwrap();
    draft
        .nodes
        .retain(|n| n.id != NodeId::LangevinCoefficientThree);
    draft.edges.retain(|e| {
        e.source != NodeId::LangevinCoefficientThree && e.target != NodeId::LangevinCoefficientThree
    });
    let absent = ftoo_manifest::validate(draft).unwrap();
    let empty_book = ftoo_manifest::addressing(&empty).unwrap();
    let absent_book = ftoo_manifest::addressing(&absent).unwrap();
    let root = fixture();
    let path = root.join("units/statements/unit.toml");
    let saved = snapshot(&root);
    let baseline = check_weld_files(&root, &full_book).unwrap();
    for (book, input, fault, display) in [
        (
            &full_book,
            "c:2",
            F::UnknownTag {
                input: "c:2".into(),
            },
            "tag \"c:2\": unknown",
        ),
        (
            &empty_book,
            "c:3",
            F::OutOfSliceTag {
                input: "c:3".into(),
                tag: T::LangevinCoefficientThree,
                target: NodeId::LangevinCoefficientThree,
                cause: O::QuantityOutsideSupport,
            },
            "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause quantity-outside-support",
        ),
        (
            &empty_book,
            "v:3",
            F::OutOfSliceTag {
                input: "v:3".into(),
                tag: T::LangevinCoefficientThreeValue,
                target: NodeId::LangevinCoefficientThreeValue,
                cause: O::ClaimNotSelected,
            },
            "tag \"v:3\": out of slice; label claim:langevin:coefficient-three-value; cause claim-not-selected",
        ),
        (
            &absent_book,
            "c:3",
            F::OutOfSliceTag {
                input: "c:3".into(),
                tag: T::LangevinCoefficientThree,
                target: NodeId::LangevinCoefficientThree,
                cause: O::TargetAbsent,
            },
            "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause target-absent",
        ),
    ] {
        let text = format!(
            "@tag{{{input}}} See ``ignored`` @tag{{{input}}} `sec:fixture:unit` @tag{{{input}}} `missing:label:one`"
        );
        fs::write(&path, statement(&text, &text)).unwrap();
        let changed = snapshot(&root);
        let mut expected = Vec::new();
        for language in ["en", "fr"] {
            for (index, start) in [0, 26, 55].into_iter().enumerate() {
                expected.push(UnitDiagnostic {
                    rule: RuleId::M042,
                    stage: UnitStage::Resolution,
                    unit: Some(UNIT.into()),
                    language: Some(language.into()),
                    site: site(language, &text, &text, Some(start..start + 9), Some(index)),
                    fault: UnitFault::ResolutionFault {
                        cause: fault.clone(),
                    },
                });
            }
        }
        assert_unit(
            &check_weld_files(&root, book).unwrap_err(),
            &expected,
            &vec![display.into(); 6],
        );
        assert_eq!(snapshot(&root), changed);
        fs::write(&path, &saved[&path]).unwrap();
        assert_eq!(check_weld_files(&root, &full_book).unwrap(), baseline);
        assert_eq!(snapshot(&root), saved);
    }
    fs::remove_dir_all(root).unwrap();
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(snapshot(&path));
        } else {
            files.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    files
}
fn site(
    language: &str,
    en: &str,
    fr: &str,
    decoded: Option<std::ops::Range<usize>>,
    occurrence: Option<usize>,
) -> UnitSite {
    let (start, text) = if language == "en" {
        (HEADER.len() + 3, en)
    } else {
        (HEADER.len() + en.len() + 9, fr)
    };
    UnitSite {
        file: "units/statements/unit.toml".into(),
        path: ["unit", "texts", language]
            .map(|k| PathPart::Key(k.into()))
            .to_vec(),
        anchor: SourceAnchor::Bytes(start, start + text.len() + 2),
        decoded,
        occurrence,
    }
}
fn tags(
    language: &str,
    en: &str,
    fr: &str,
    entries: &[(&str, NodeId, usize, usize, usize)],
) -> TagSide {
    TagSide {
        language: language.into(),
        text_site: site(language, en, fr, None, None),
        members: entries
            .iter()
            .map(|(input, target, start, end, index)| TagMember {
                input: (*input).into(),
                target: *target,
                sites: vec![site(language, en, fr, Some(*start..*end), Some(*index))],
            })
            .collect(),
    }
}
fn labels(
    language: &str,
    en: &str,
    fr: &str,
    entries: &[(&str, usize, usize, usize)],
) -> LabelSide {
    LabelSide {
        language: language.into(),
        text_site: site(language, en, fr, None, None),
        members: entries
            .iter()
            .map(|(label, start, end, index)| ftoo_manifest::LabelMember {
                label: (*label).into(),
                sites: vec![LabelUseSite {
                    span_index: *index,
                    site: site(language, en, fr, Some(*start..*end), None),
                }],
            })
            .collect(),
    }
}
fn diagnostic(language: &str, location: UnitSite, fault: WeldFault) -> WeldDiagnostic {
    let citation = matches!(fault, WeldFault::UnresolvedUnitCitation { .. });
    WeldDiagnostic {
        rule: if citation { RuleId::M045 } else { RuleId::M046 },
        stage: if citation {
            WeldStage::UnitCitation
        } else {
            WeldStage::Equality
        },
        unit: Some(UNIT.into()),
        language: Some(language.into()),
        site: LabelSite::Unit {
            unit: UNIT.into(),
            site: location,
        },
        fault,
    }
}
// Expected formatting is assembled from literal fixture fields, never production Display.
fn debug_site(site: &UnitSite) -> String {
    let path = site
        .path
        .iter()
        .map(|p| match p {
            PathPart::Key(k) => format!("Key({k:?})"),
            PathPart::Index(i) => format!("Index({i})"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let SourceAnchor::Bytes(start, end) = site.anchor else {
        panic!("fixture bytes")
    };
    let decoded = site.decoded.as_ref().map_or_else(
        || "None".into(),
        |r| format!("Some({}..{})", r.start, r.end),
    );
    format!(
        "UnitSite {{ file: {:?}, path: [{path}], anchor: Bytes({start}, {end}), decoded: {decoded}, occurrence: {:?} }}",
        site.file, site.occurrence
    )
}
fn display_site(site: &UnitSite) -> String {
    let mut text = format!("{:?} $", site.file);
    for part in &site.path {
        match part {
            PathPart::Key(k) => write!(text, "[{k:?}]").unwrap(),
            PathPart::Index(i) => write!(text, "[{i}]").unwrap(),
        }
    }
    let SourceAnchor::Bytes(start, end) = site.anchor else {
        panic!("fixture bytes")
    };
    write!(text, " bytes {start}..{end}").unwrap();
    if let Some(r) = &site.decoded {
        write!(text, " decoded {}..{}", r.start, r.end).unwrap();
    }
    if let Some(i) = site.occurrence {
        write!(text, " occurrence {i}").unwrap();
    }
    text
}
fn debug_tags(side: &TagSide) -> String {
    let members = side
        .members
        .iter()
        .map(|m| {
            let target = match m.target {
                NodeId::LangevinCoefficientOne => "LangevinCoefficientOne",
                NodeId::LangevinCoefficientOneValue => "LangevinCoefficientOneValue",
                NodeId::LangevinCoefficientThree => "LangevinCoefficientThree",
                NodeId::LangevinCoefficientThreeValue => "LangevinCoefficientThreeValue",
            };
            format!(
                "TagMember {{ input: {:?}, target: {target}, sites: [{}] }}",
                m.input,
                m.sites
                    .iter()
                    .map(debug_site)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "TagSide {{ language: {:?}, text_site: {}, members: [{members}] }}",
        side.language,
        debug_site(&side.text_site)
    )
}
fn debug_labels(side: &LabelSide) -> String {
    let members = side
        .members
        .iter()
        .map(|m| {
            format!(
                "LabelMember {{ label: {:?}, sites: [{}] }}",
                m.label,
                m.sites
                    .iter()
                    .map(|s| format!(
                        "LabelUseSite {{ span_index: {}, site: {} }}",
                        s.span_index,
                        debug_site(&s.site)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "LabelSide {{ language: {:?}, text_site: {}, members: [{members}] }}",
        side.language,
        debug_site(&side.text_site)
    )
}
fn assert_weld(error: &WeldFileError, expected: &[WeldDiagnostic]) {
    let WeldFileError::Weld(errors) = error else {
        panic!("{error}")
    };
    assert_eq!(errors.diagnostics(), expected);
    let mut display = String::new();
    for d in expected {
        let LabelSite::Unit { unit, site } = &d.site else {
            panic!("fixture site")
        };
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
                debug_tags(reference),
                debug_tags(actual)
            ),
            WeldFault::LabelSetMismatch {
                reference,
                actual,
                missing,
                unexpected,
            } => format!(
                "LabelSetMismatch {{ reference: {}, actual: {}, missing: {missing:?}, unexpected: {unexpected:?} }}",
                debug_labels(reference),
                debug_labels(actual)
            ),
        };
        writeln!(
            display,
            "{:?} {:?} unit={:?} language={:?} at unit {unit:?} at {}: {fault}",
            d.rule,
            d.stage,
            d.unit,
            d.language,
            display_site(site)
        )
        .unwrap();
    }
    assert_eq!(error.to_string(), display);
}
fn assert_unit(error: &WeldFileError, expected: &[UnitDiagnostic], fault_texts: &[String]) {
    let WeldFileError::Labels(LabelFileError::Units(UnitLoadError::Admission(errors))) = error
    else {
        panic!("{error}")
    };
    assert_eq!(errors.diagnostics(), expected);
    let mut display = String::new();
    for (d, fault) in expected.iter().zip(fault_texts) {
        writeln!(
            display,
            "{:?} {:?} unit={:?} language={:?} at {}: {fault}",
            d.rule,
            d.stage,
            d.unit,
            d.language,
            display_site(&d.site)
        )
        .unwrap();
    }
    assert_eq!(error.to_string(), display);
}

fn mutation_fault(case: u8, fr: &str) -> WeldFault {
    match case {
        0 | 1 => WeldFault::TagSetMismatch {
            reference: tags(
                "en",
                BASE,
                fr,
                &[("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0)],
            ),
            actual: tags(
                "fr",
                BASE,
                fr,
                if case == 0 {
                    &[]
                } else {
                    &[
                        ("c:1", NodeId::LangevinCoefficientOne, 4, 13, 0),
                        ("v:1", NodeId::LangevinCoefficientOneValue, 40, 49, 1),
                    ]
                },
            ),
            missing: if case == 0 {
                vec!["c:1".into()]
            } else {
                vec![]
            },
            unexpected: if case == 1 {
                vec!["v:1".into()]
            } else {
                vec![]
            },
        },
        2..=4 => WeldFault::LabelSetMismatch {
            reference: labels("en", BASE, fr, &[("sec:fixture:target", 18, 38, 0)]),
            actual: labels(
                "fr",
                BASE,
                fr,
                if case == 3 {
                    &[
                        ("sec:fixture:other", 40, 59, 1),
                        ("sec:fixture:target", 18, 38, 0),
                    ]
                } else {
                    &[]
                },
            ),
            missing: if case == 3 {
                vec![]
            } else {
                vec!["sec:fixture:target".into()]
            },
            unexpected: if case == 3 {
                vec!["sec:fixture:other".into()]
            } else {
                vec![]
            },
        },
        _ => WeldFault::UnresolvedUnitCitation {
            label: "sec:fixture:absent".into(),
            span_index: 0,
            expected: 1,
            actual: 0,
            domain: ReferenceDomain::Joint,
        },
    }
}

#[test]
fn w13_actual_file_set_matrix_restores_every_mutation() {
    let root = fixture();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_weld_files(&root, &book).unwrap();
    assert_eq!(
        baseline,
        WeldCheckSummary {
            units: 1,
            texts: 2,
            tag_sites: 2,
            rust_mints: 4,
            markdown_mints: 119,
            unit_mints: 1,
            markdown_citations: 100,
            rust_citations: 8,
            unit_citation_sites: 2,
            mention_sites: 0,
            language_comparisons: 1
        }
    );
    let saved = snapshot(&root);
    let path = root.join("units/statements/unit.toml");
    for (case, fr) in [
        (0, "See `sec:fixture:target`."),
        (1, "See @tag{c:1} and `sec:fixture:target`. @tag{v:1}"),
        (2, "See @tag{c:1}."),
        (
            3,
            "See @tag{c:1} and `sec:fixture:target`. `sec:fixture:other`",
        ),
        (4, "See @tag{c:1} and ``sec:fixture:target``."),
        (5, "See @tag{c:1} and `sec:fixture:absent`."),
    ] {
        fs::write(&path, statement(BASE, fr)).unwrap();
        let before = snapshot(&root);
        let error = check_weld_files(&root, &book).unwrap_err();
        let fault = mutation_fault(case, fr);
        let location = site(
            "fr",
            BASE,
            fr,
            if case == 5 { Some(18..38) } else { None },
            None,
        );
        assert_weld(&error, &[diagnostic("fr", location, fault)]);
        assert_eq!(snapshot(&root), before);
        fs::write(&path, &saved[&path]).unwrap();
        assert_eq!(snapshot(&root), saved);
        assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn w13_actual_mint_and_unknown_tag_mutations_restore() {
    let root = fixture();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_weld_files(&root, &book).unwrap();
    let saved = snapshot(&root);
    let path = root.join("units/statements/unit.toml");
    for (fr, rule, range, fault, display) in [
        (
            "See @tag{c:1} and · `sec:fixture:target`.",
            RuleId::M045,
            18..41,
            UnitFault::TextMintForbidden {
                span_index: 0,
                label: "sec:fixture:target".into(),
            },
            "TextMintForbidden { span_index: 0, label: \"sec:fixture:target\" }",
        ),
        (
            "See @tag{c:2} and `sec:fixture:target`.",
            RuleId::M042,
            4..13,
            UnitFault::ResolutionFault {
                cause: ftoo_manifest::TagResolutionFault::UnknownTag {
                    input: "c:2".into(),
                },
            },
            "tag \"c:2\": unknown",
        ),
    ] {
        fs::write(&path, statement(BASE, fr)).unwrap();
        let before = snapshot(&root);
        let error = check_weld_files(&root, &book).unwrap_err();
        assert_unit(
            &error,
            &[UnitDiagnostic {
                rule,
                stage: if rule == RuleId::M042 {
                    UnitStage::Resolution
                } else {
                    UnitStage::Content
                },
                unit: Some(UNIT.into()),
                language: Some("fr".into()),
                site: site(
                    "fr",
                    BASE,
                    fr,
                    Some(range),
                    if rule == RuleId::M042 { Some(0) } else { None },
                ),
                fault,
            }],
            &[display.into()],
        );
        assert_eq!(snapshot(&root), before);
        fs::write(&path, &saved[&path]).unwrap();
        assert_eq!(snapshot(&root), saved);
        assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn w13_actual_language_and_markdown_target_mutations_restore() {
    let root = fixture();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_weld_files(&root, &book).unwrap();
    let saved = snapshot(&root);
    let declarations = root.join("units/documents.toml");
    fs::write(&declarations, DECLARATIONS.replace("['en','fr']", "['en']")).unwrap();
    let before = snapshot(&root);
    let mut location = site("fr", BASE, BASE, None, None);
    let key = HEADER.len() + BASE.len() + 6;
    location.anchor = SourceAnchor::Bytes(key, key + 2);
    assert_unit(
        &check_weld_files(&root, &book).unwrap_err(),
        &[UnitDiagnostic {
            rule: RuleId::M040,
            stage: UnitStage::Language,
            unit: Some(UNIT.into()),
            language: Some("fr".into()),
            site: location,
            fault: UnitFault::UndeclaredLanguage {
                document: "explainer".into(),
                required: vec!["en".into()],
            },
        }],
        &["UndeclaredLanguage { document: \"explainer\", required: [\"en\"] }".into()],
    );
    assert_eq!(snapshot(&root), before);
    fs::write(&declarations, &saved[&declarations]).unwrap();
    assert_eq!(snapshot(&root), saved);
    assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
    let md = root.join("docs/weld-fixture.md");
    fs::write(&md, "· `sec:fixture:replacement`\n· `sec:fixture:other`\n").unwrap();
    let before = snapshot(&root);
    let expected: Vec<_> = ["en", "fr"]
        .into_iter()
        .map(|l| {
            diagnostic(
                l,
                site(l, BASE, BASE, Some(18..38), None),
                WeldFault::UnresolvedUnitCitation {
                    label: "sec:fixture:target".into(),
                    span_index: 0,
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::Joint,
                },
            )
        })
        .collect();
    assert_weld(&check_weld_files(&root, &book).unwrap_err(), &expected);
    assert_eq!(snapshot(&root), before);
    fs::write(&md, &saved[&md]).unwrap();
    assert_eq!(snapshot(&root), saved);
    assert_eq!(check_weld_files(&root, &book).unwrap(), baseline);
    fs::remove_dir_all(root).unwrap();
}
