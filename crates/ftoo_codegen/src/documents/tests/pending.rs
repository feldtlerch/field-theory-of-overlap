use super::*;

#[test]
fn pending_document_records_merge_symbols_and_alias_reasons_in_catalogue_order() {
    use super::super::{PendingExplanation, PendingOccurrence};
    use ftoo_manifest::{DefinitionId, NodeId, NotationOrigin, PendingReason};
    let one_reason = PendingReason::FormulationProposed {
        object: NodeId::LangevinCoefficientOne,
    };
    let three_reason = PendingReason::FormulationProposed {
        object: NodeId::LangevinCoefficientThree,
    };
    let alias_reason = PendingReason::BackingUnbound {
        object: NodeId::LangevinCoefficientOne,
    };
    let origin = |id| NotationOrigin {
        definition: Some(id),
        path: vec![],
    };
    // A typed document-layer fixture: an alias's unbound own backing and its
    // target's proposed formulation are distinct reasons in the combined list.
    let entry = |id, order, body, reasons| {
        crate::PendingEntry::fixture(id, order, body, reasons, vec![origin(id)])
    };
    let one = entry(
        DefinitionId::CoefficientOne,
        0,
        r"{c}_{1}",
        vec![one_reason.clone()],
    );
    let three = entry(
        DefinitionId::CoefficientThree,
        1,
        r"{c}_{3}",
        vec![three_reason.clone()],
    );
    let alias = entry(
        DefinitionId::FirstCoefficient,
        4,
        r"{c}_{1}",
        vec![alias_reason.clone(), one_reason.clone()],
    );
    let site = |name| input::site(name);
    let run = |name: &'static str, pending| plan::Inline::Math {
        definition_site: super::super::input::site("fixture-math.toml"),
        key: name.into(),
        markdown: "x".into(),
        latex: "x".into(),
        pending,
        site: site(name),
    };
    let mut block = plan::Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: "sec:fixture:pending".into(),
        depth: None,
        site: site("block"),
        runs: vec![
            run("alias-first", vec![alias.clone(), one.clone()]),
            run("three", vec![three.clone()]),
            run("alias-second", vec![one.clone(), alias]),
        ],
    };
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let name = if target == DocumentTarget::Latex {
            "native-cell"
        } else {
            "markdown-cell"
        };
        block.displays.insert(
            target,
            plan::DisplayBody {
                source: "fixture cell".into(),
                layout: ftoo_manifest::DisplayLayout::Quote {
                    rows: vec![Some(0)],
                },
                regions: vec![plan::Block {
                    pending_names: BTreeMap::new(),
                    displays: BTreeMap::new(),
                    unit: block.unit.clone(),
                    depth: None,
                    site: site(name),
                    runs: vec![run(name, vec![three.clone(), one.clone()])],
                }],
                site: site(name),
            },
        );
    }
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    block.map_pending(&book, 0, "fixture", "en").unwrap();
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![block],
    };
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let cell = if target == DocumentTarget::Latex {
            "native-cell"
        } else {
            "markdown-cell"
        };
        let expected: Vec<_> = [
            (DefinitionId::CoefficientOne, r"{c}_{1}", vec![one_reason.clone()], vec!["alias-first", "alias-second", cell], "{c}_{1}: retained formulation def:langevin:coefficient-one"),
            (DefinitionId::CoefficientThree, r"{c}_{3}", vec![three_reason.clone()], vec!["three", cell], "{c}_{3}: retained formulation def:langevin:coefficient-three"),
            (DefinitionId::FirstCoefficient, r"{c}_{1}", vec![alias_reason.clone(), one_reason.clone()], vec!["alias-first", "alias-second"], "{c}_{1}: backing object def:langevin:coefficient-one is unbound; retained formulation def:langevin:coefficient-one"),
        ].into_iter().map(|(definition, body, reasons, occurrences, value)| EmissionRecord {
            kind: EmissionKind::PendingExplanation(Box::new(PendingExplanation {
                definition, body_macro: body.into(), body_unicode: body.into(),
                names: reasons.iter().enumerate().map(|(reason, value)| {
                    let object = match value { PendingReason::BackingUnbound { object } | PendingReason::BackingAbsent { object } | PendingReason::FormulationProposed { object } | PendingReason::ClaimDeferred { object, .. } => *object };
                    let (handle, label) = if object == NodeId::LangevinCoefficientOne { ("c:1", "def:langevin:coefficient-one") } else { ("c:3", "def:langevin:coefficient-three") };
                    let order = match definition { DefinitionId::CoefficientOne => 0, DefinitionId::CoefficientThree => 1, DefinitionId::FirstCoefficient => 4, _ => unreachable!() };
                    super::super::PendingReaderName { object, handle: handle.into(), label: label.into(), token: format!("u0-p{order}-r{reason}") }
                }).collect(), reasons,
                occurrences: occurrences.into_iter().map(|name| PendingOccurrence {
                    definition: SourceOrigin { unit: Some("sec:fixture:pending".into()), language: Some("en".into()), target: None, site: super::super::input::site("fixture-math.toml") },
                    source: SourceOrigin { unit: Some("sec:fixture:pending".into()), language: Some("en".into()), target: Some(target), site: site(name) },
                    notation: vec![origin(definition)],
                }).collect(),
            })), unit: "sec:fixture:pending".into(), language: "en".into(), target,
            occurrence: None, value: value.into(), site: site("block"),
        }).collect();
        let records: Vec<_> = plan
            .records(target)
            .into_iter()
            .filter(|record| matches!(record.kind, EmissionKind::PendingExplanation(_)))
            .collect();
        assert_eq!(records, expected);
        let source = if target == DocumentTarget::Latex {
            latex::render(&plan)
        } else {
            markdown::render(&plan)
        };
        assert_eq!(plan::check_emission(&plan, &source, target), Ok(()));
        assert_eq!(source.matches("Pending notation: ").count(), 1);
        let tag = |handle, label, token| {
            if target == DocumentTarget::Latex {
                format!("\\FtdocTag{{{handle}}}{{{token}}}% label {label}\n")
            } else {
                format!("{handle}<!-- label {label} -->")
            }
        };
        let first = tag("c:1", "def:langevin:coefficient-one", "u0-p0-r0");
        let third = tag("c:3", "def:langevin:coefficient-three", "u0-p1-r0");
        let r0 = tag("c:1", "def:langevin:coefficient-one", "u0-p4-r0");
        let r1 = tag("c:1", "def:langevin:coefficient-one", "u0-p4-r1");
        let (one_body, three_body, space) = if target == DocumentTarget::Latex {
            (r"\({c}_{1}\)", r"\({c}_{3}\)", r"\space{}")
        } else {
            ("${c}_{1}$", "${c}_{3}$", " ")
        };
        let prefix = format!(
            "Pending notation: {one_body}: retained formulation {first}. {three_body}: retained formulation {third}. {one_body}: "
        );
        let alias = format!("backing object {r0}{space}is unbound; retained formulation {r1}.");
        let paragraph = format!("{prefix}{alias}");
        assert_eq!(source.matches(&paragraph).count(), 1);
        let mut mutations = vec![
            paragraph.replace("; ", ", "),
            paragraph.replace(". ", "; "),
            format!("{prefix}retained formulation {r1}; backing object {r0}{space}is unbound."),
            format!("{prefix}backing object {r0}{space}is unbound; retained formulation ."),
            format!("{prefix}backing object {r0}{space}is unbound; retained formulation {r1}{r1}."),
        ];
        if target == DocumentTarget::Latex {
            mutations.extend([
                paragraph
                    .replace("u0-p4-r0", "SWAP")
                    .replace("u0-p4-r1", "u0-p4-r0")
                    .replace("SWAP", "u0-p4-r1"),
                paragraph.replace("u0-p4-r1", "u0-p4-r9"),
                paragraph.replace("u0-p4", "u0-p5"),
                paragraph.replace("u0-p1", "u0-p9"),
            ]);
        }
        let directory = Directory::new();
        let path = directory.0.join("pending.txt");
        for replacement in mutations {
            assert_ne!(replacement, paragraph);
            let changed_source = source.replace(&paragraph, &replacement);
            fs::write(&path, &changed_source).unwrap();
            let reread = fs::read_to_string(&path).unwrap();
            assert_eq!(reread, changed_source);
            let mut changed = expected[0].clone();
            changed.value = replacement;
            let start = reread
                .find(if target == DocumentTarget::Latex {
                    "% unit "
                } else {
                    "<!-- unit "
                })
                .unwrap();
            let fault = AssemblyFault::EmissionMismatch {
                missing: expected[1..].to_vec(),
                unexpected: vec![],
                changed: vec![EmissionChange {
                    expected: expected[0].clone(),
                    actual: changed,
                }],
            };
            let expected_fault = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: None,
                    unit: None,
                    language: None,
                    target: None,
                    site: AssemblySite::Generated {
                        path: if target == DocumentTarget::Latex {
                            "generated/documents/fixture/en.tex"
                        } else {
                            "generated/documents/fixture/en.md"
                        }
                        .into(),
                        bytes: start..reread.len(),
                        origins: plan.origins(target),
                    },
                    fault,
                }])
                .unwrap(),
            );
            let actual = plan::check_emission(&plan, &reread, target).unwrap_err();
            assert_eq!(actual, expected_fault);
            assert_eq!(actual.to_string(), expected_fault.to_string());
            fs::write(&path, &source).unwrap();
            let restored = fs::read_to_string(&path).unwrap();
            assert_eq!(restored, source);
            assert_eq!(plan::check_emission(&plan, &restored, target), Ok(()));
        }
    }
}

pub(super) fn support_required_pending_paragraphs_are_complete_label_neutral_and_independently_consumed(
    book: &ftoo_manifest::AddressBook<'_>,
    notation: &crate::RenderableNotation<'_, '_, '_>,
) -> Result<(), DocumentError> {
    let directory = Directory::new();
    let emitted_path = directory.0.join("pending.txt");
    let expression = notation.check_expression(ftoo_manifest::ExprDraft::Call {
        callee: ftoo_manifest::Callee::Definition(ftoo_manifest::DefinitionId::CoefficientOne),
        arguments: vec![],
    })?;
    let markdown = expression.render(ftoo_manifest::RenderTarget::Unicode)?;
    let latex = expression.render(ftoo_manifest::RenderTarget::Macro)?;
    let site = input::site("units/statements/pending.toml");
    let math = || plan::Inline::Math {
        definition_site: super::super::input::site("fixture-math.toml"),
        key: "value".into(),
        markdown: markdown.source().into(),
        latex: latex.source().into(),
        pending: markdown.pending_entries().to_vec(),
        site: site.clone(),
    };
    for shape in 0..3 {
        let mut block = plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:pending".into(),
            depth: (shape == 0).then_some(1),
            site: site.clone(),
            runs: vec![math(), plan::Inline::Text(" and ".into()), math()],
        };
        if shape == 2 {
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                block.displays.insert(
                    target,
                    plan::DisplayBody {
                        source: "@math{value}".into(),
                        layout: ftoo_manifest::DisplayLayout::Quote {
                            rows: vec![Some(0)],
                        },
                        regions: vec![plan::Block {
                            pending_names: BTreeMap::new(),
                            displays: BTreeMap::new(),
                            unit: block.unit.clone(),
                            depth: None,
                            site: site.clone(),
                            runs: vec![math()],
                        }],
                        site: site.clone(),
                    },
                );
            }
        }
        block.map_pending(book, 0, "fixture", "en")?;
        let plan = plan::DocumentPlan {
            document: "fixture".into(),
            language: "en".into(),
            coverage: plan::Coverage::fixture("Coverage.", "Slice."),
            blocks: vec![block],
        };
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let (source, paragraph, expected_size) = match target {
                DocumentTarget::Markdown => (
                    super::super::markdown::render(&plan),
                    "Pending notation: ${c}_{1}$: retained formulation c:1<!-- label def:langevin:coefficient-one -->.",
                    super::super::markdown::size(&plan),
                ),
                DocumentTarget::Latex => (
                    super::super::latex::render(&plan),
                    "Pending notation: \\({c}_{1}\\): retained formulation \\FtdocTag{c:1}{u0-p0-r0}% label def:langevin:coefficient-one\n.",
                    super::super::latex::size(&plan),
                ),
            };
            assert_eq!(source.len(), expected_size);
            assert_eq!(source.matches(paragraph).count(), 1);
            assert_eq!(plan::check_emission(&plan, &source, target), Ok(()));
            if target == DocumentTarget::Markdown {
                let checked =
                    ftoo_manifest::check_markdown_sources(vec![ftoo_manifest::MarkdownSource {
                        file: "generated/documents/fixture/en.md".into(),
                        bytes: source.as_bytes().to_vec(),
                    }])
                    .unwrap();
                assert_eq!(checked.mints(), []);
                assert_eq!(checked.citations(), []);
            }
            let records = plan.records(target);
            let expected = records.last().unwrap().clone();
            let EmissionKind::PendingExplanation(entry) = &expected.kind else {
                panic!("pending explanation record")
            };
            assert_eq!(
                entry.definition,
                ftoo_manifest::DefinitionId::CoefficientOne
            );
            assert_eq!(entry.body_macro, r"{c}_{1}");
            assert_eq!(entry.body_unicode, r"{c}_{1}");
            assert_eq!(
                entry.reasons,
                vec![ftoo_manifest::PendingReason::FormulationProposed {
                    object: ftoo_manifest::NodeId::LangevinCoefficientOne
                }]
            );
            assert_eq!(entry.occurrences.len(), if shape == 2 { 3 } else { 2 });
            for occurrence in &entry.occurrences {
                assert_eq!(
                    occurrence.source,
                    SourceOrigin {
                        unit: Some("sec:fixture:pending".into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: site.clone()
                    }
                );
                assert_eq!(occurrence.notation, markdown.pending_entries()[0].origins());
            }
            let mut replacements = vec![
                paragraph.replace("retained formulation", "retainedformulation"),
                paragraph.replace("retained formulation ", ""),
                paragraph.replace("{c}_{1}", "{c}_{3}"),
            ];
            replacements.push(paragraph.replace(
                "def:langevin:coefficient-one",
                "def:langevin:coefficient-three",
            ));
            replacements.push(paragraph.replace("c:1", "c:3"));
            if target == DocumentTarget::Latex {
                replacements.push(paragraph.replace("u0-p0-r0", "u0-p0-r9"));
                replacements.push(paragraph.replace("retained", r"re\allowbreak{}tained"));
            }
            for replacement in replacements {
                assert_ne!(replacement, paragraph);
                let mutated = source.replace(paragraph, &replacement);
                fs::write(&emitted_path, &mutated).unwrap();
                let mutated = fs::read_to_string(&emitted_path).unwrap();
                let mut actual = expected.clone();
                actual.value = replacement.clone();
                if let EmissionKind::PendingExplanation(entry) = &mut actual.kind {
                    if replacement.contains("label def:langevin:coefficient-three") {
                        entry.names[0].label = "def:langevin:coefficient-three".into();
                    }
                    if target == DocumentTarget::Latex && replacement.contains("u0-p0-r9") {
                        entry.names[0].token = "u0-p0-r9".into();
                    }
                    if target == DocumentTarget::Latex && replacement.contains("{c:3}") {
                        entry.names[0].handle = "c:3".into();
                    }
                }
                let range = mutated
                    .find(if target == DocumentTarget::Latex {
                        "% unit "
                    } else {
                        "<!-- unit "
                    })
                    .unwrap()..mutated.len();
                let fault = if actual.kind == expected.kind {
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![],
                        changed: vec![EmissionChange {
                            expected: expected.clone(),
                            actual,
                        }],
                    }
                } else {
                    AssemblyFault::EmissionMismatch {
                        missing: vec![expected.clone()],
                        unexpected: vec![actual],
                        changed: vec![],
                    }
                };
                let expected_fault = super::super::failure(
                    RuleId::M055,
                    ftoo_manifest::AssemblyStage::Emission,
                    ftoo_manifest::AssemblySite::Generated {
                        path: format!(
                            "generated/documents/fixture/en.{}",
                            if target == DocumentTarget::Latex {
                                "tex"
                            } else {
                                "md"
                            }
                        ),
                        bytes: range,
                        origins: plan.origins(target),
                    },
                    fault,
                );
                let actual = plan::check_emission(&plan, &mutated, target).unwrap_err();
                assert_eq!(actual, expected_fault);
                assert_eq!(actual.to_string(), expected_fault.to_string());
                fs::write(&emitted_path, &source).unwrap();
                let restored = fs::read_to_string(&emitted_path).unwrap();
                assert_eq!(restored, source);
                assert_eq!(plan::check_emission(&plan, &restored, target), Ok(()));
            }
        }
    }
    Ok(())
}
