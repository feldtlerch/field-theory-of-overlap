use super::*;

fn site() -> UnitSite {
    UnitSite {
        file: "units/statements/spacing.toml".into(),
        path: vec![],
        anchor: SourceAnchor::WholeFile,
        decoded: None,
        occurrence: None,
    }
}
fn insertion(citation: bool) -> plan::Inline {
    if citation {
        plan::Inline::Citation {
            text: "dec:fixture:source".into(),
            site: site(),
        }
    } else {
        plan::Inline::Tag {
            handle: "c:1".into(),
            label: "def:langevin:coefficient-one".into(),
            token: "u0-t0".into(),
            site: site(),
        }
    }
}
fn fixture(citation: bool, depth: Option<usize>, suffix: &str) -> plan::DocumentPlan {
    plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: ["Coverage.".into(), "Slice.".into()],
        blocks: vec![plan::Block {
            displays: BTreeMap::new(),
            unit: "sec:fixture:spacing".into(),
            depth,
            site: site(),
            runs: vec![
                insertion(citation),
                plan::Inline::Text(String::new()),
                plan::Inline::Text(suffix.into()),
            ],
        }],
    }
}

#[test]
fn token_spaces_refuse_old_comment_indentation_with_exact_m055_and_restore() {
    for citation in [false, true] {
        for depth in [None, Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)] {
            for suffix in [" word", " ."] {
                let plan = fixture(citation, depth, suffix);
                let correct = latex::render(&plan);
                assert_eq!(
                    plan::check_emission(&plan, &correct, DocumentTarget::Latex),
                    Ok(())
                );
                let canonical = if citation {
                    "`dec:fixture:source`"
                } else {
                    "@tag{c:1}"
                };
                let expected_record = EmissionRecord {
                    kind: if depth.is_some() {
                        EmissionKind::Heading
                    } else {
                        EmissionKind::Paragraph
                    },
                    unit: "sec:fixture:spacing".into(),
                    language: "en".into(),
                    target: DocumentTarget::Latex,
                    occurrence: None,
                    value: format!("{canonical}{suffix}"),
                    site: site(),
                };
                for (indentation, recovered_space) in [
                    (" ", ""),
                    ("\t  ", ""),
                    ("", ""),
                    (r"\space{}\space{}", "  "),
                ] {
                    // The first variant is the exact pre-fix comment continuation.
                    let old = correct.replace(r"\space{}", indentation);
                    let start = old.find("% unit ").unwrap();
                    let mut actual_record = expected_record.clone();
                    actual_record.value = format!("{canonical}{recovered_space}{}", &suffix[1..]);
                    let fault = AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![],
                        changed: vec![EmissionChange {
                            expected: expected_record.clone(),
                            actual: actual_record,
                        }],
                    };
                    let location = AssemblySite::Generated {
                        path: "generated/documents/fixture/en.tex".into(),
                        bytes: start..old.len(),
                        origins: vec![
                            SourceOrigin {
                                unit: Some("sec:fixture:spacing".into()),
                                language: Some("en".into()),
                                target: Some(DocumentTarget::Latex),
                                site: site()
                            };
                            2
                        ],
                    };
                    let expected = failure(
                        RuleId::M055,
                        AssemblyStage::Emission,
                        location.clone(),
                        fault.clone(),
                    );
                    let actual =
                        plan::check_emission(&plan, &old, DocumentTarget::Latex).unwrap_err();
                    assert_eq!(actual, expected);
                    assert_eq!(
                        actual.to_string(),
                        format!(
                            "M055 Emission document=None unit=None language=None target=None at {location:?}: {fault:?}\n"
                        )
                    );
                    assert_eq!(
                        plan::check_emission(&plan, &correct, DocumentTarget::Latex),
                        Ok(())
                    );
                }
            }
            for suffix in [".", ""] {
                let plan = fixture(citation, depth, suffix);
                let correct = latex::render(&plan);
                assert_eq!(
                    plan::check_emission(&plan, &correct, DocumentTarget::Latex),
                    Ok(())
                );
                assert!(!correct.contains(r"\space{}"));
            }
        }
    }
}

#[test]
fn token_spaces_check_caption_cell_and_picture_regions_independently() {
    use ftoo_manifest::{DisplayLayout, PictureCommand};
    for citation in [false, true] {
        for picture in [false, true] {
            let mut plan = fixture(citation, None, " word");
            let region = fixture(citation, None, " word").blocks.remove(0);
            let (layout, before, after) = if picture {
                (
                    DisplayLayout::Picture {
                        width: 120,
                        height: 20,
                        commands: vec![PictureCommand::Label {
                            x: 0,
                            y: 0,
                            region: 0,
                        }],
                    },
                    "\\begingroup\\setlength{\\unitlength}{1pt}\n\\begin{picture}(120,20)\n\\put(0,0){",
                    "}\n\\end{picture}\n\\endgroup",
                )
            } else {
                (
                    DisplayLayout::Table {
                        columns: vec!["l".into()],
                        rows: vec![vec![0]],
                        rules: vec![],
                    },
                    "\\begin{center}\n\\begin{tabular}{l}\n",
                    " \\\\\n\\end{tabular}\n\\end{center}",
                )
            };
            plan.blocks[0].displays.insert(
                DocumentTarget::Latex,
                plan::DisplayBody {
                    source: "authored body".into(),
                    layout,
                    regions: vec![region],
                    site: site(),
                },
            );
            let correct = latex::render(&plan);
            assert_eq!(
                plan::check_emission(&plan, &correct, DocumentTarget::Latex),
                Ok(())
            );
            let insertion = if citation {
                "\\texttt{dec:fixture:source}% cite dec:fixture:source\n"
            } else {
                "\\FtdocTag{c:1}{u0-t0}% label def:langevin:coefficient-one\n"
            };
            for caption in [true, false] {
                let at = if caption {
                    correct.find(r"\space{}")
                } else {
                    correct.rfind(r"\space{}")
                }
                .unwrap();
                let mut old = correct.clone();
                old.replace_range(at..at + r"\space{}".len(), " ");
                let canonical = if citation {
                    "`dec:fixture:source`"
                } else {
                    "@tag{c:1}"
                };
                let expected_record = EmissionRecord {
                    kind: if caption {
                        EmissionKind::Caption
                    } else {
                        EmissionKind::Body
                    },
                    unit: "sec:fixture:spacing".into(),
                    language: "en".into(),
                    target: DocumentTarget::Latex,
                    occurrence: None,
                    value: if caption {
                        format!("{canonical} word")
                    } else {
                        "authored body".into()
                    },
                    site: site(),
                };
                let mut actual_record = expected_record.clone();
                actual_record.value = if caption {
                    format!("{canonical}word")
                } else {
                    format!("{before}{insertion} word{after}")
                };
                let location = AssemblySite::Generated {
                    path: "generated/documents/fixture/en.tex".into(),
                    bytes: old.find("% unit ").unwrap()..old.len(),
                    origins: vec![
                        SourceOrigin {
                            unit: Some("sec:fixture:spacing".into()),
                            language: Some("en".into()),
                            target: Some(DocumentTarget::Latex),
                            site: site()
                        };
                        4
                    ],
                };
                let fault = AssemblyFault::EmissionMismatch {
                    missing: vec![],
                    unexpected: vec![],
                    changed: vec![EmissionChange {
                        expected: expected_record,
                        actual: actual_record,
                    }],
                };
                let expected = failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    location.clone(),
                    fault.clone(),
                );
                let actual = plan::check_emission(&plan, &old, DocumentTarget::Latex).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(
                    actual.to_string(),
                    format!(
                        "M055 Emission document=None unit=None language=None target=None at {location:?}: {fault:?}\n"
                    )
                );
                assert_eq!(
                    plan::check_emission(&plan, &correct, DocumentTarget::Latex),
                    Ok(())
                );
            }
        }
    }
}

#[test]
fn support_required_native_comment_glue_matches_authored_controls()
-> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
    let lock = native_support::lock(&root).map_err(|e| format!("{e:?}"))?;
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let mut source = latex::start("fixture", "en");
    let mut count = 0;
    for citation in [false, true] {
        let control = if citation {
            r"\texttt{dec:fixture:source}"
        } else {
            r"\FtdocTag{c:1}{u0-t0}"
        };
        for (suffix, expected) in [
            (" word", r"\space{}word"),
            (".", "."),
            (" .", r"\space{}."),
            (" ", r"\space{}\(x\)"),
        ] {
            let mut runs = vec![insertion(citation), plan::Inline::Text(suffix.into())];
            if suffix == " " {
                runs.push(plan::Inline::Math {
                    key: "x".into(),
                    markdown: "x".into(),
                    latex: "x".into(),
                    site: site(),
                });
            }
            let mut emitted = String::new();
            latex::inline(&mut emitted, &runs);
            for (open, close) in [
                ("\\section*{", "}\n"),
                ("", "\n\n"),
                ("\\begin{tabular}{l}\n", " \\\\\n\\end{tabular}\n"),
            ] {
                source.push_str(open);
                writeln!(source, "\\setbox0=\\hbox{{{emitted}}}\\setbox1=\\hbox{{{control}{expected}}}\\ifdim\\wd0=\\wd1\\else\\errmessage{{spacing fixture {count}}}\\fi\\typeout{{SPACE-CONTROL:{count}:OK}}\\box0").unwrap();
                source.push_str(close);
                count += 1;
            }
        }
    }
    // The old shape compiles, but its horizontal list lacks the control's glue.
    source.push_str("\\setbox0=\\hbox{\\FtdocTag{c:1}{u0-t0}% label def:langevin:coefficient-one\n word}\\setbox1=\\hbox{\\FtdocTag{c:1}{u0-t0}\\space{}word}\\ifdim\\wd0=\\wd1\\errmessage{old spacing unexpectedly retained}\\fi\n\\end{document}\n");
    let mut evidence = String::new();
    let result = native::consume_with(
        &support,
        &lock,
        &source,
        "",
        &addressing,
        NativeCase::Document {
            document: "fixture".into(),
            language: "en".into(),
        },
        vec![],
        |_| {},
        |scratch, _| {
            evidence = std::fs::read_to_string(scratch.join("output/document.log")).unwrap();
        },
    );
    assert_eq!(count, 24);
    assert_eq!(result, Ok(()));
    assert_eq!(support.closing_hash(), Some(Ok(())));
    assert_eq!(
        evidence
            .lines()
            .filter(|line| line.starts_with("SPACE-CONTROL:"))
            .collect::<Vec<_>>(),
        (0..24)
            .map(|index| format!("SPACE-CONTROL:{index}:OK"))
            .collect::<Vec<_>>()
    );
    Ok(())
}
