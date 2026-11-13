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
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![plan::Block {
            pending_names: BTreeMap::new(),
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
fn extra_break_markers_and_ragged_groups_refuse_with_exact_faults_and_reread() {
    let directory = Directory::new();
    let path = directory.0.join("extra.tex");
    for shape in 0..5 {
        let mut plan = fixture(false, (shape == 1).then_some(1), "");
        plan.blocks[0].runs = if shape == 2 {
            vec![plan::Inline::Mention {
                text: "word".into(),
                site: site(),
            }]
        } else {
            vec![plan::Inline::Text("word".into())]
        };
        let correct = latex::render(&plan);
        let mutated = if shape == 4 {
            correct.replace("word", r"{\raggedright word\par}")
        } else if shape == 3 {
            correct.replace("Coverage.", r"Cover\allowbreak{}age.")
        } else {
            correct.replace("word", r"wo\allowbreak{}rd")
        };
        let records = plan.records(DocumentTarget::Latex);
        let (range, fault) = if shape == 3 {
            let expected = EmissionRecord {
                kind: EmissionKind::CoverageDeclaration,
                unit: String::new(),
                language: "en".into(),
                target: DocumentTarget::Latex,
                occurrence: None,
                value: "Coverage.".into(),
                site: plan.coverage.site.clone(),
            };
            let mut actual = expected.clone();
            actual.value = r"Cover\allowbreak{}age.".into();
            (
                mutated.find("Cover").unwrap()..mutated.find("Slice.").unwrap() + 6,
                AssemblyFault::EmissionMismatch {
                    missing: vec![],
                    unexpected: vec![],
                    changed: vec![EmissionChange { expected, actual }],
                },
            )
        } else {
            let mut actual = records[0].clone();
            actual.value = if shape == 4 {
                r"{\raggedright word\par}"
            } else if shape == 2 {
                r"\texttt{wo\allowbreak{}rd}"
            } else if shape == 1 {
                r"\section*{wo\allowbreak{}rd}"
            } else {
                r"wo\allowbreak{}rd"
            }
            .into();
            (
                mutated.find("% unit ").unwrap()..mutated.len(),
                AssemblyFault::EmissionMismatch {
                    missing: records[1..].to_vec(),
                    unexpected: vec![],
                    changed: vec![EmissionChange {
                        expected: records[0].clone(),
                        actual,
                    }],
                },
            )
        };
        let expected = failure(
            RuleId::M055,
            AssemblyStage::Emission,
            AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: range,
                origins: plan.origins(DocumentTarget::Latex),
            },
            fault,
        );
        fs::write(&path, &mutated).unwrap();
        let reread = fs::read_to_string(&path).unwrap();
        let actual = plan::check_emission(&plan, &reread, DocumentTarget::Latex).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        fs::write(&path, &correct).unwrap();
        let restored = fs::read_to_string(&path).unwrap();
        assert_eq!(restored, correct);
        assert_eq!(
            plan::check_emission(&plan, &restored, DocumentTarget::Latex),
            Ok(())
        );
    }
}

#[test]
fn citation_identity_breaks_refuse_missing_and_misplaced_markers_and_reread_restoration() {
    let directory = Directory::new();
    let path = directory.0.join("identity.tex");
    {
        let plan = fixture(true, None, " word");
        let correct = latex::render(&plan);
        assert_eq!(latex::size(&plan), correct.len());
        let marker = r"dec:\allowbreak{}fixture:\allowbreak{}source";
        assert!(correct.contains(marker));
        for replacement in [
            r"dec:fixture:\allowbreak{}source",
            r"de\allowbreak{}c:fixture:\allowbreak{}source",
        ] {
            let mutated = correct.replacen(marker, replacement, 1);
            fs::write(&path, &mutated).unwrap();
            let records = plan.records(DocumentTarget::Latex);
            let (range, fault) = {
                let mut actual = records[0].clone();
                actual.value =
                    format!("\\texttt{{{replacement}}}% cite dec:fixture:source\n\\space{{}}word");
                (
                    mutated.find("% unit ").unwrap()..mutated.len(),
                    AssemblyFault::EmissionMismatch {
                        missing: records[1..].to_vec(),
                        unexpected: vec![],
                        changed: vec![EmissionChange {
                            expected: records[0].clone(),
                            actual,
                        }],
                    },
                )
            };
            let expected = failure(
                RuleId::M055,
                AssemblyStage::Emission,
                AssemblySite::Generated {
                    path: "generated/documents/fixture/en.tex".into(),
                    bytes: range,
                    origins: plan.origins(DocumentTarget::Latex),
                },
                fault,
            );
            let actual = plan::check_emission(
                &plan,
                &fs::read_to_string(&path).unwrap(),
                DocumentTarget::Latex,
            )
            .unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
            fs::write(&path, &correct).unwrap();
            assert_eq!(
                plan::check_emission(
                    &plan,
                    &fs::read_to_string(&path).unwrap(),
                    DocumentTarget::Latex
                ),
                Ok(())
            );
        }
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
                "\\texttt{dec:\\allowbreak{}fixture:\\allowbreak{}source}% cite dec:fixture:source\n"
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
            r"\texttt{dec:\allowbreak{}fixture:\allowbreak{}source}"
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
                    definition_site: super::super::input::site("fixture-math.toml"),
                    key: "x".into(),
                    markdown: "x".into(),
                    latex: "x".into(),
                    pending: vec![],
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
