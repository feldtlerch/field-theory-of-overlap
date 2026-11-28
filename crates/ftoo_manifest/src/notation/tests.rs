// Keep independent complete fault payloads and Display oracles beside each fixture.
#![allow(clippy::too_many_lines)]
use super::*;
use crate::{RuleId, addressing, seed};
use catalogue::{call, literal, primitive};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn expression_site(fields: &[&str]) -> NotationSite {
    NotationSite::Expression {
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
    }
}
fn registry_site(id: DefinitionId, index: usize, fields: &[&str]) -> NotationSite {
    NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
        entry: RegistryEntry::Definition(id),
        index,
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
        source_range: None,
    }
}
fn primitive_site(id: PrimitiveId, index: usize, fields: &[&str]) -> NotationSite {
    NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/primitives.rs".into(),
        entry: RegistryEntry::Primitive(id),
        index,
        path: fields
            .iter()
            .map(|s| NotationPathPart::Field((*s).into()))
            .collect(),
        source_range: None,
    }
}
fn diag(
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) -> NotationDiagnostic {
    NotationDiagnostic {
        rule,
        stage,
        site,
        fault,
    }
}
fn exact(error: &NotationErrors, expected: &[NotationDiagnostic], display: &str) {
    assert_eq!(error.diagnostics(), expected);
    assert_eq!(error.to_string(), display);
    let lines: Vec<_> = display.lines().collect();
    for (diagnostic, line) in error.diagnostics().iter().zip(lines) {
        assert_eq!(diagnostic.to_string(), line);
    }
}
fn body_mut(d: &mut DefinitionSpec) -> &mut ExprDraft {
    match &mut d.kind {
        DefinitionKind::Canonical { body, .. }
        | DefinitionKind::Constructor { body }
        | DefinitionKind::Inherited { body } => body,
        DefinitionKind::Alias { .. } => panic!("fixture needs a body"),
    }
}
fn backing_mut(d: &mut DefinitionSpec) -> &mut Option<NodeId> {
    match &mut d.kind {
        DefinitionKind::Canonical { backing, .. } | DefinitionKind::Alias { backing, .. } => {
            backing
        }
        DefinitionKind::Constructor { .. } | DefinitionKind::Inherited { .. } => {
            panic!("fixture needs binding")
        }
    }
}

#[test]
fn inherited_coda_definitions_are_checked_and_unbound() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let notation = notation(&book)?;
    for id in [
        DefinitionId::Comovfield,
        DefinitionId::Readout,
        DefinitionId::Willvirtue,
        DefinitionId::Gensym,
        DefinitionId::Alignrate,
        DefinitionId::Willnet,
        DefinitionId::Willdecay,
    ] {
        let definition = notation.definition(id);
        assert!(matches!(
            definition.spec().kind(),
            DefinitionKind::Inherited { .. }
        ));
        assert_eq!(definition.spec().signature().arity, 0);
        assert_eq!(definition.command(), format!("\\{}", id.token()));
        assert_eq!(definition.status().unwrap().tier(), SymbolTier::Pending);
        assert_eq!(
            definition.status().unwrap().reasons(),
            &[PendingReason::InheritedUnbound { definition: id }]
        );
        check_expression(&notation, call(id, vec![]))?;
    }
    Ok(())
}

#[test]
fn document_text_bounds_and_catalogue_refusal() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let runs = |count: usize, width: usize| {
        (0..count)
            .map(|index| {
                let start = index * (width + 1);
                TextRun {
                    text: "a".repeat(width),
                    source: start..start + width,
                    decoded: start..start + width,
                }
            })
            .collect()
    };
    for (count, width) in [(1, 256), (16, 256), (64, 1)] {
        check_document_expression(
            &catalogue,
            ExprDraft::Text {
                runs: runs(count, width),
            },
        )?;
    }
    let total_error = check_document_expression(
        &catalogue,
        ExprDraft::Text {
            runs: runs(17, 256),
        },
    )
    .unwrap_err();
    assert!(total_error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::NotationLimit {
            resource: NotationResource::ExpressionTextBytes,
            limit: 4096,
            actual: 4352,
        }
    )));
    let run_error =
        check_document_expression(&catalogue, ExprDraft::Text { runs: runs(65, 1) }).unwrap_err();
    assert_eq!(run_error.diagnostics().len(), 1);
    assert!(matches!(
        run_error.diagnostics()[0].fault,
        NotationFault::InvalidTextRuns { count: 65 }
    ));
    for value in ["a--b", "a,,b", "a  b"] {
        let error = check_document_expression(
            &catalogue,
            ExprDraft::Text {
                runs: vec![TextRun {
                    text: value.into(),
                    source: 0..value.len(),
                    decoded: 0..value.len(),
                }],
            },
        )
        .unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| matches!(d.fault, NotationFault::InvalidText { .. }))
        );
    }
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[0]) = ExprDraft::Text { runs: runs(1, 1) };
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|d| matches!(d.fault, NotationFault::TextOutsideDocument))
    );
    Ok(())
}

#[test]
fn document_text_cross_run_separators_and_pending_refuse() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let split = |first: &str, second: &str| ExprDraft::Text {
        runs: vec![
            TextRun {
                text: first.into(),
                source: 0..first.len(),
                decoded: 0..first.len(),
            },
            TextRun {
                text: second.into(),
                source: first.len() + 1..first.len() + 1 + second.len(),
                decoded: first.len() + 1..first.len() + 1 + second.len(),
            },
        ],
    };
    for (first, second, pair) in [("a-", "-b", "--"), ("a,", ",b", ",,"), ("a ", " b", "  ")] {
        let error = check_document_expression(&catalogue, split(first, second)).unwrap_err();
        assert_eq!(error.diagnostics().len(), 1);
        assert_eq!(
            error.diagnostics()[0].fault,
            NotationFault::InvalidText { value: pair.into() }
        );
        assert_eq!(
            error.diagnostics()[0].site,
            NotationSite::Expression {
                path: vec![
                    NotationPathPart::Field("runs".into()),
                    NotationPathPart::Argument(1)
                ]
            }
        );
    }
    let error = check_document_expression(
        &catalogue,
        primitive(
            PrimitiveId::Underbrace,
            vec![literal("a"), split("pend", "ing")],
        ),
    )
    .unwrap_err();
    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(
        error.diagnostics()[0].fault,
        NotationFault::PendingAnnotation
    );
    assert_eq!(
        error.diagnostics()[0].site,
        NotationSite::Expression {
            path: vec![
                NotationPathPart::Field("arguments".into()),
                NotationPathPart::Argument(1),
            ]
        }
    );
    Ok(())
}

#[test]
fn inherited_source_and_body_changes_refuse() {
    let source = include_str!("../../../../documents/paper/data/ftoo_symbols.tex");
    let original = "\\ensuremath{S}";
    assert!(catalogue::source_matches(source, original));
    assert!(!catalogue::source_matches(source, "\\ensuremath{X}"));
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[7]) = literal("X");
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Comovfield
        }
    )));
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[8]) = call(DefinitionId::Comovfield, vec![]);
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Readout
        }
    )));
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[10]) = primitive(
        PrimitiveId::Superscript,
        vec![literal("A"), primitive(PrimitiveId::UprightS, vec![])],
    );
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Gensym
        }
    )));
}

#[test]
fn inherited_source_bytes_and_declaration_sites_refuse_changes() {
    let symbols = include_str!("../../../../documents/paper/data/ftoo_symbols.tex");
    let historical = include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex");
    let registers = include_str!("../../../../documents/paper/data/ftoo_registers.tex");
    let definitions = catalogue::catalogue();
    for (source, before, after, id) in [
        (
            symbols,
            "\\ensuremath{S}",
            "\\ensuremath{X}",
            DefinitionId::Comovfield,
        ),
        (
            historical,
            "\\symcal{M}",
            "\\symcal{N}",
            DefinitionId::Readout,
        ),
        (
            historical,
            "\\mathrm{vir}",
            "\\mathrm{via}",
            DefinitionId::Willvirtue,
        ),
        (
            registers,
            "variance=operator",
            "variance=operatpr",
            DefinitionId::Gensym,
        ),
        (
            historical,
            "\\ensuremath{\\dot{\\sigma}}",
            "\\ensuremath{\\dot{\\alpha}}",
            DefinitionId::Alignrate,
        ),
        (
            historical,
            "\\ensuremath{\\omega\\sb{\\mathrm{net}}}",
            "\\ensuremath{\\omega\\sb{\\mathrm{new}}}",
            DefinitionId::Willnet,
        ),
        (
            historical,
            "\\ensuremath{\\gamma\\sigma}",
            "\\ensuremath{\\gamma\\alpha}",
            DefinitionId::Willdecay,
        ),
        (
            registers,
            "\\RegisterVariance{\\comovfield}{operator}",
            "\\RegisterVariance{\\comovfield}{operatpr}",
            DefinitionId::Comovfield,
        ),
    ] {
        let changed = source.replacen(before, after, 1);
        assert_ne!(changed, source);
        let sources = catalogue::InheritedSources {
            symbols: if std::ptr::eq(source, symbols) {
                &changed
            } else {
                symbols
            },
            historical: if std::ptr::eq(source, historical) {
                &changed
            } else {
                historical
            },
            registers: if std::ptr::eq(source, registers) {
                &changed
            } else {
                registers
            },
        };
        let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
        assert!(error.diagnostics().iter().any(
            |d| matches!(d.fault, NotationFault::SourceMismatch { definition } if definition == id)
        ));
    }
    let duplicate = format!("{historical}\\RenewDocumentCommand{{\\comovfield}}{{}}{{X}}\n");
    let sources = catalogue::InheritedSources {
        symbols,
        historical: &duplicate,
        registers,
    };
    let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Comovfield
        }
    )));
    for declaration in [
        r"\renewcommand*{\comovfield}{X}",
        r"\renewcommand\comovfield{X}",
        r"\DeclareDocumentCommand\comovfield{}{X}",
        r"\ProvideDocumentCommand\comovfield{}{X}",
        r"\NewExpandableDocumentCommand\comovfield{}{X}",
        r"\RenewExpandableDocumentCommand\comovfield{}{X}",
        r"\DeclareExpandableDocumentCommand\comovfield{}{X}",
        r"\ProvideExpandableDocumentCommand\comovfield{}{X}",
        r"\DeclareRobustCommand\comovfield{X}",
        r"\NewCommandCopy\comovfield\otherfield",
        r"\RenewCommandCopy\comovfield\otherfield",
        r"\DeclareCommandCopy\comovfield\otherfield",
        r"\cs_set:Npn \comovfield {X}",
        r"\expandafter\def\csname comovfield\endcsname{X}",
    ] {
        let duplicate = format!("{historical}{declaration}\n");
        let sources = catalogue::InheritedSources {
            symbols,
            historical: &duplicate,
            registers,
        };
        let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
        assert!(error.diagnostics().iter().any(|d| matches!(
            d.fault,
            NotationFault::SourceMismatch {
                definition: DefinitionId::Comovfield
            }
        )));
    }
    let duplicate = format!("{historical}\\RenewDocumentCommand\\comovfield{{}}{{X}}\n");
    let sources = catalogue::InheritedSources {
        symbols,
        historical: &duplicate,
        registers,
    };
    let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Comovfield
        }
    )));
    let duplicate = format!("{registers}\\def\\comovfield{{X}}\n");
    let sources = catalogue::InheritedSources {
        symbols,
        historical,
        registers: &duplicate,
    };
    let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Comovfield
        }
    )));
    for (line, id) in [
        (
            r"\RegisterVariance{\comovfield}{vector}",
            DefinitionId::Comovfield,
        ),
        (r"\RegisterVariance{\gensym}{scalar}", DefinitionId::Gensym),
        (
            r"\RegisterVariance{\readout}{scalar}",
            DefinitionId::Readout,
        ),
    ] {
        let changed = format!("{registers}{line}\n");
        let sources = catalogue::InheritedSources {
            symbols,
            historical,
            registers: &changed,
        };
        let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
        assert!(error.diagnostics().iter().any(
            |d| matches!(d.fault, NotationFault::SourceMismatch { definition } if definition == id)
        ));
    }
    let mentions = format!(
        "{historical}\\let\\saved\\comovfield\n\\NewCommandCopy\\saved\\comovfield\n\\csname comovfield\\endcsname\n% \\renewcommand\\comovfield{{X}}\n\\renewcommand\\comovfield_aux:n{{X}}\n"
    );
    let sources = catalogue::InheritedSources {
        symbols,
        historical: &mentions,
        registers,
    };
    assert_eq!(catalogue::inherited_source(&definitions, &sources), Ok(()));
}

#[test]
fn decoded_single_symbol_declarations_use_the_document_grammar() {
    let definitions = catalogue::catalogue();
    let primitives = primitives::catalogue();
    let rows = catalogue::inherited_decoded_declarations();
    assert_eq!(rows.len(), 33);
    for (id, source) in &rows {
        assert!(source.contains(&format!("\\{}", id.token())));
        assert!(
            definitions.iter().any(|entry| entry.id() == *id
                && matches!(entry.kind(), DefinitionKind::Inherited { .. }))
        );
    }
    let source = include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex");
    let (_, body) = inherited::decode(source, "will", &primitives, &definitions).unwrap();
    assert_eq!(body, primitive(PrimitiveId::Omega, vec![]));
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let checked = notation(&book).unwrap();
    let document = crate::assembly::parse_assembly_math(&checked, r"\omega").unwrap();
    let construction =
        crate::assembly::parse_catalogue_body(&primitives, &definitions, r"\omega").unwrap();
    assert_eq!(document.draft(), construction.draft());
    assert_eq!(document.ranges(), construction.ranges());
    assert_eq!(
        crate::assembly::parse_catalogue_body(&primitives, &definitions, r"\unknown")
            .unwrap_err()
            .0,
        0..8
    );
    let front = include_str!("../../../../documents/paper/parts/front.tex");
    let context = "an edge between them: a rate $\\will$ opposing the reward gradient";
    assert!(front.contains(context));
    let source = context.split('$').nth(1).unwrap();
    assert_eq!(
        crate::assembly::parse_assembly_math(&checked, source)
            .unwrap()
            .draft(),
        &call(DefinitionId::Will, vec![])
    );
}

#[test]
fn decoded_symbol_wrapper_signature_and_body_refuse_independently() {
    let source = include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex");
    let primitives = primitives::catalogue();
    let definitions = catalogue::catalogue();
    for (changed, fault) in [
        (
            source.replacen("{ \\will }\n  {  }", "{ \\will }\n  { m }", 1),
            inherited::DecodeFault::Signature,
        ),
        (
            source.replacen(
                "\\ensuremath{\\omega}\n  }",
                "\\ensuremath{\\omega+}\n  }",
                1,
            ),
            inherited::DecodeFault::Body,
        ),
        (
            source.replacen(
                "\\ensuremath{\\omega}\n  }",
                "\\ensuremath{\\unknown}\n  }",
                1,
            ),
            inherited::DecodeFault::Body,
        ),
    ] {
        assert_ne!(changed, source);
        assert_eq!(
            inherited::decode(&changed, "will", &primitives, &definitions),
            Err(fault)
        );
        let sources = catalogue::InheritedSources {
            symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
            historical: &changed,
            registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
        };
        let expected = inherited::declaration_range(&changed, "will").unwrap();
        let errors = catalogue::inherited_source(&definitions, &sources).unwrap_err();
        assert!(errors.diagnostics().iter().any(|diagnostic| {
            diagnostic.rule == RuleId::M078
                && matches!(
                    &diagnostic.site,
                    NotationSite::Registry {
                        file,
                        source_range: Some(range),
                        ..
                    } if file == "documents/paper/data/ftoo_symbols_historical.tex" && range == &expected
                )
        }));
    }
    let changed = source.replacen("\\ensuremath{\\omega}\n  }", "\\ensuremath{A}\n  }", 1);
    assert_eq!(
        inherited::decode(&changed, "will", &primitives, &definitions)
            .unwrap()
            .1,
        literal("A")
    );
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: &changed,
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let errors = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(errors.diagnostics().iter().any(|d| d.rule == RuleId::M078
        && matches!(
            d.fault,
            NotationFault::SourceMismatch {
                definition: DefinitionId::Will
            }
        )));
}

#[test]
fn later_declarations_and_property_effects_refuse_at_m078() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    for (path, input, id) in [
        (
            "ftoo_typed_tools.tex",
            r"\prop_gput:Nen \g__ftoo_qty_var_prop { \tl_to_str:n {\will} } { scalar }",
            DefinitionId::Will,
        ),
        (
            "ftoo_typed_tools.tex",
            r"\prop_gput:Nnn \g__ftoo_qty_var_prop { \tl_to_str:n {\will} } { scalar }",
            DefinitionId::Will,
        ),
        (
            "econ_membrane.tex",
            r"\RenewDocumentCommand{\horizon}{}{X}",
            DefinitionId::Horizon,
        ),
        (
            "reader-hooks.tex",
            r"\cs_set:Npx \will {X}",
            DefinitionId::Will,
        ),
        ("front.tex", r"\cs_undefine:N \will", DefinitionId::Will),
        (
            "front.tex",
            r"\RenewDocumentCommand{\phi}{}{X}",
            DefinitionId::Bendfn,
        ),
        (
            "front.tex",
            r"\prop_gput:Nne \g__ftoo_qty_var_prop {#1} {scalar}",
            DefinitionId::Will,
        ),
        ("front.tex", r"\input{later-symbols}", DefinitionId::Will),
    ] {
        let mut inputs = catalogue::OTHER_INPUTS.to_vec();
        let index = inputs.iter().position(|(file, _)| *file == path).unwrap();
        let changed = format!("{}\n{input}\n", inputs[index].1);
        let at = inputs[index].1.len() + 1;
        inputs[index].1 = &changed;
        let errors =
            catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
        assert!(errors.diagnostics().iter().any(|d| d.rule == RuleId::M078
            && matches!(d.fault, NotationFault::SourceMismatch { definition } if definition == id)
            && (if input.starts_with("\\input") {
                matches!(&d.site, NotationSite::Registry { file, .. }
                    if file == catalogue::FILE)
            } else {
                matches!(&d.site, NotationSite::Registry { file, source_range: Some(range), .. }
                    if file == path && range.start == at)
            })), "{path}: {input}: {:?}", errors.diagnostics());
    }
    let reordered = include_str!("../../../../documents/paper/main.tex").replace(
        "\\input{data/ftoo_symbols.tex}\n\\input{data/ftoo_symbols_historical.tex}",
        "\\input{data/ftoo_symbols_historical.tex}\n\\input{data/ftoo_symbols.tex}",
    );
    assert_ne!(
        reordered,
        include_str!("../../../../documents/paper/main.tex")
    );
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let main_index = inputs
        .iter()
        .position(|(file, _)| *file == "main.tex")
        .unwrap();
    inputs[main_index].1 = &reordered;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(errors.diagnostics().iter().any(|d| d.rule == RuleId::M078
        && matches!(
            d.fault,
            NotationFault::SourceMismatch {
                definition: DefinitionId::Will
            }
        )));
    let omitted = include_str!("../../../../documents/paper/main.tex")
        .replace("\\input{data/ftoo_symbols_historical.tex}\n", "");
    inputs[main_index].1 = &omitted;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(errors.diagnostics().iter().any(|d| d.rule == RuleId::M078
        && matches!(
            d.fault,
            NotationFault::SourceMismatch {
                definition: DefinitionId::Will
            }
        )));
    let appended = format!(
        "{}\n\\input{{later-symbols}}\n",
        include_str!("../../../../documents/paper/main.tex")
    );
    inputs[main_index].1 = &appended;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(errors.diagnostics().iter().any(|d| d.rule == RuleId::M078
        && matches!(
            d.fault,
            NotationFault::SourceMismatch {
                definition: DefinitionId::Will
            }
        )));
}

#[test]
fn a_paper_macro_may_use_defines_with_a_parameter() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "partB.tex")
        .unwrap();
    let changed = format!(
        "{}\n\\newcommand\\dyndefines[1]{{\\defines{{#1}}\\DisplayClass{{dynamics}}}}\n",
        inputs[index].1
    );
    inputs[index].1 = &changed;
    catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap();
}

#[test]
fn longer_registration_names_and_string_mentions_are_not_registrations() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "partB.tex")
        .unwrap();
    let changed = format!(
        "{}\n\\newcommand\\RegisterVarianceTable[1]{{#1}}\n\\newcommand\\DeclareMetricPairs[1]{{#1}}\n\\string\\RegisterVariance\n\\string\\DeclareMetricPair\n",
        inputs[index].1
    );
    inputs[index].1 = &changed;
    catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap();
}

#[test]
fn ordinary_expandafter_append_still_passes() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "partB.tex")
        .unwrap();
    let changed = format!(
        "{}\n\\expandafter\\def\\expandafter\\foo\\expandafter{{X}}\n",
        inputs[index].1
    );
    inputs[index].1 = &changed;
    catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap();
}

fn named_source_effect_refuses(addition: &str, command: &str, located: bool) {
    let definitions = catalogue::catalogue();
    let symbols = include_str!("../../../../documents/paper/data/ftoo_symbols.tex");
    let historical = include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex");
    let original = include_str!("../../../../documents/paper/data/ftoo_registers.tex");
    let changed = format!("{original}\n{addition}\n");
    let at = original.len() + 1;
    let sources = catalogue::InheritedSources {
        symbols,
        historical,
        registers: &changed,
    };
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, catalogue::OTHER_INPUTS)
            .unwrap_err();
    assert!(errors.diagnostics().iter().any(|diagnostic| {
        diagnostic.rule == RuleId::M078
            && matches!(diagnostic.fault, NotationFault::SourceMismatch { .. })
            && (if located {
                matches!(&diagnostic.site, NotationSite::Registry { file, source_range: Some(range), .. }
                    if file == "ftoo_registers.tex" && range.start == at)
            } else {
                matches!(&diagnostic.site, NotationSite::Registry { file, .. }
                    if file == catalogue::FILE)
            })
    }), "{command}: {:?}", errors.diagnostics());
}

#[test]
fn named_source_dynamic_target_refuses() {
    named_source_effect_refuses(r"\def#1{X}", r"\def", true);
}

#[test]
fn named_source_store_operation_refuses() {
    named_source_effect_refuses(
        r"\prop_gclear:N \g__ftoo_qty_var_prop",
        r"\prop_gclear:N",
        true,
    );
}

#[test]
fn named_source_load_command_refuses() {
    named_source_effect_refuses(r"\input{extra}", r"\input", false);
}

#[test]
fn spaced_and_computed_definition_targets_refuse_at_their_commands() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    for (path, addition, command, id) in [
        (
            "reader-hooks.tex",
            r"\cs_gset:cpn { will } {X}",
            r"\cs_gset:cpn",
            DefinitionId::Will,
        ),
        (
            "macros_overlap.tex",
            r"\cs_set_eq:cN { phi } \varphi",
            r"\cs_set_eq:cN",
            DefinitionId::Bendfn,
        ),
        (
            "front.tex",
            r"\RenewDocumentCommand { #1 }{}{X}",
            r"\RenewDocumentCommand",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\RenewDocumentCommand*{#1}{}{X}",
            r"\RenewDocumentCommand",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\expandafter\def\csname #1\endcsname{X}",
            r"\def",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\cs_gset:cpn {wi#1}{X}",
            r"\cs_gset:cpn",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\expandafter\def\csname wi#1\endcsname{X}",
            r"\def",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\expandafter\def\csname wi\empty ll\endcsname{X}",
            r"\def",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\cs_gset:cpn {wi\empty ll}{X}",
            r"\cs_gset:cpn",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\expandafter\def\csname wi\csname empty\endcsname#1\endcsname{X}",
            r"\def",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\expandafter\RegisterVariance\expandafter{\csname will\endcsname}{vector}",
            r"\RegisterVariance",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\expandafter\renewcommand\expandafter{\csname will\endcsname}{X}",
            r"\renewcommand",
            DefinitionId::Will,
        ),
        (
            "partB.tex",
            r"\expandafter\def\expandafter\will\expandafter{\will\,}",
            r"\def",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\exp_args:Nc \RenewDocumentCommand {will}{}{X}",
            r"\RenewDocumentCommand",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\RegisterVariance{#1}{vector}",
            r"\RegisterVariance",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\RegisterVariance{will#1}{vector}",
            r"\RegisterVariance",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\exp_args:Nc \RegisterVariance {will}{vector}",
            r"\RegisterVariance",
            DefinitionId::Will,
        ),
        (
            "front.tex",
            r"\DeclareMetricPair{#1}",
            r"\DeclareMetricPair",
            DefinitionId::Will,
        ),
    ] {
        let mut inputs = catalogue::OTHER_INPUTS.to_vec();
        let index = inputs.iter().position(|(file, _)| *file == path).unwrap();
        let at = inputs[index].1.len() + 1 + addition.find(command).unwrap();
        let changed = format!("{}\n{addition}\n", inputs[index].1);
        inputs[index].1 = &changed;
        let errors =
            catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
        assert!(errors.diagnostics().iter().any(|diagnostic| {
            diagnostic.rule == RuleId::M078
                && matches!(diagnostic.fault, NotationFault::SourceMismatch { definition } if definition == id)
                && matches!(&diagnostic.site, NotationSite::Registry { file, source_range: Some(range), .. }
                    if file == path && range.start == at)
        }), "{path}: {addition}: {:?}", errors.diagnostics());
    }
}

#[test]
fn prepared_hooks_and_quantity_operations_refuse_at_their_sites() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    for (path, addition, id) in [
        (
            "reader-hooks.tex",
            "\n\\RenewDocumentCommand\\will{}{X}\n",
            DefinitionId::Will,
        ),
        (
            "macros_overlap.tex",
            "\n\\cs_set_eq:NN \\phi \\varphi\n",
            DefinitionId::Bendfn,
        ),
        (
            "macros_overlap.tex",
            "\n\\cs_gset:cpn {phi} {X}\n",
            DefinitionId::Bendfn,
        ),
        (
            "ftoo_typed_tools.tex",
            "\n\\prop_gremove:Nn \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\comovfield} }\n",
            DefinitionId::Comovfield,
        ),
        (
            "ftoo_typed_tools.tex",
            "\n\\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {\\will} } { scalar }\n",
            DefinitionId::Will,
        ),
        (
            "ftoo_typed_tools.tex",
            "\n\\prop_gremove:Nn\\g__ftoo_qty_var_prop { \\tl_to_str:n {\\comovfield} }\n",
            DefinitionId::Comovfield,
        ),
    ] {
        let mut inputs = catalogue::OTHER_INPUTS.to_vec();
        let index = inputs.iter().position(|(file, _)| *file == path).unwrap();
        let at = inputs[index].1.len() + 1;
        let changed = format!("{}{}", inputs[index].1, addition);
        inputs[index].1 = &changed;
        let errors =
            catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
        assert!(errors.diagnostics().iter().any(|diagnostic| {
            diagnostic.rule == RuleId::M078
                && matches!(diagnostic.fault, NotationFault::SourceMismatch { definition } if definition == id)
                && matches!(&diagnostic.site, NotationSite::Registry { file, source_range: Some(range), .. }
                    if file == path && range.start == at)
        }), "{path}: {addition}");
        if addition.starts_with("\n\\prop_gput") {
            assert!(errors.diagnostics().iter().any(|diagnostic| {
                diagnostic.rule == RuleId::M078
                    && matches!(diagnostic.fault, NotationFault::SourceMismatch { definition: DefinitionId::Will })
                    && matches!(&diagnostic.site, NotationSite::Registry { file, source_range: Some(_), .. }
                        if file == "documents/paper/data/ftoo_symbols_historical.tex")
            }), "{:?}", errors.diagnostics());
        }
    }
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "ftoo_typed_tools.tex")
        .unwrap();
    let removed = "    \\prop_gput:Nen \\g__ftoo_qty_var_prop { \\tl_to_str:n {#1} } {#2}\n";
    let changed = inputs[index].1.replacen(removed, "", 1);
    assert_ne!(changed, inputs[index].1);
    inputs[index].1 = &changed;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(
        errors.diagnostics().iter().any(|diagnostic| {
            diagnostic.rule == RuleId::M078
                && matches!(&diagnostic.site, NotationSite::Registry { file, .. }
                if file == "ftoo_typed_tools.tex")
        }),
        "{:?}",
        errors.diagnostics()
    );
}

#[test]
fn quantity_definition_template_is_compared_as_one_site() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "ftoo_typed_tools.tex")
        .unwrap();
    let changed = inputs[index].1.replacen(
        "{ \\exp_not:V \\l__ftoo_q_log_tl \\exp_not:n {#3} }",
        "{ \\exp_not:V \\l__ftoo_q_log_tl X }",
        1,
    );
    assert_ne!(changed, inputs[index].1);
    let at = changed
        .find("\\exp_args:NNee \\NewDocumentCommand #1")
        .unwrap();
    inputs[index].1 = &changed;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(errors.diagnostics().iter().any(|diagnostic| {
        diagnostic.rule == RuleId::M078
            && matches!(&diagnostic.site, NotationSite::Registry { file, source_range: Some(range), .. }
                if file == "ftoo_typed_tools.tex" && range.start == at)
    }));
}

#[test]
fn typed_redeclaration_keeps_its_checking_calls() {
    let definitions = catalogue::catalogue();
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut inputs = catalogue::OTHER_INPUTS.to_vec();
    let index = inputs
        .iter()
        .position(|(file, _)| *file == "ftoo_typed_tools.tex")
        .unwrap();
    let original = "\\__ftoo_var_assert:nn {#1} {vector}";
    let changed = inputs[index]
        .1
        .replacen(original, "\\__ftoo_var_assert:nn {#1} {scalar}", 1);
    assert_ne!(changed, inputs[index].1);
    inputs[index].1 = &changed;
    let errors =
        catalogue::inherited_source_with_inputs(&definitions, &sources, &inputs).unwrap_err();
    assert!(
        errors.diagnostics().iter().any(|diagnostic| {
            diagnostic.rule == RuleId::M078
                && matches!(&diagnostic.site, NotationSite::Registry { file, .. }
                if file == "ftoo_typed_tools.tex")
        }),
        "{:?}",
        errors.diagnostics()
    );
}

#[test]
fn inherited_source_rows_and_kinds_are_bijective() {
    let sources = catalogue::InheritedSources {
        symbols: include_str!("../../../../documents/paper/data/ftoo_symbols.tex"),
        historical: include_str!("../../../../documents/paper/data/ftoo_symbols_historical.tex"),
        registers: include_str!("../../../../documents/paper/data/ftoo_registers.tex"),
    };
    let mut definitions = catalogue::catalogue();
    definitions[5].kind = DefinitionKind::Inherited { body: literal("x") };
    let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Dimension
        }
    )));
    let mut definitions = catalogue::catalogue();
    definitions[7].kind = DefinitionKind::Constructor { body: literal("S") };
    let error = catalogue::inherited_source(&definitions, &sources).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::SourceMismatch {
            definition: DefinitionId::Comovfield
        }
    )));
}

#[test]
fn inherited_body_cannot_depend_on_object_bound_definition() {
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[7]) = call(DefinitionId::CoefficientOne, vec![]);
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::DefinitionShape {
            reason: DefinitionShapeReason::InheritedDependency {
                target: DefinitionId::CoefficientOne
            },
            ..
        }
    )));
}

#[test]
fn inherited_cycle_and_wrong_arity_refuse() -> TestResult {
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[7]) = call(DefinitionId::Comovfield, vec![]);
    let edges = catalogue::dependencies(&definitions);
    let cycle = catalogue::cycles(&edges).unwrap_err();
    assert!(
        cycle
            .diagnostics()
            .iter()
            .any(|d| matches!(d.fault, NotationFault::DefinitionCycle { .. }))
    );
    let graph = seed()?;
    let book = addressing(&graph)?;
    let notation = notation(&book)?;
    let arity =
        check_expression(&notation, call(DefinitionId::Gensym, vec![literal("x")])).unwrap_err();
    assert!(arity.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::CallArity {
            expected: 0,
            actual: 1,
            ..
        }
    )));
    Ok(())
}

#[test]
fn inherited_reason_propagates_and_canonical_body_collision_refuses() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[2]) = primitive(
        PrimitiveId::Add,
        vec![call(DefinitionId::Comovfield, vec![]), literal("x")],
    );
    let notation = construct(&book, primitives::catalogue(), definitions)?;
    assert!(
        notation
            .definition(DefinitionId::CoefficientOneValue)
            .status()
            .unwrap()
            .reasons()
            .contains(&PendingReason::InheritedUnbound {
                definition: DefinitionId::Comovfield
            })
    );
    assert!(
        notation
            .definition(DefinitionId::CoefficientOneValue)
            .status()
            .unwrap()
            .reasons()
            .iter()
            .any(|reason| !matches!(reason, PendingReason::InheritedUnbound { .. }))
    );
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[0]) = literal("S");
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::DefinitionShape {
            reason: DefinitionShapeReason::InheritedBodyConflict { .. },
            ..
        }
    )));
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[0]) = call(DefinitionId::Comovfield, vec![]);
    let error = catalogue::admit(&primitives::catalogue(), &definitions).unwrap_err();
    assert!(error.diagnostics().iter().any(|d| matches!(
        d.fault,
        NotationFault::DefinitionShape {
            reason: DefinitionShapeReason::InheritedBodyConflict { .. },
            ..
        }
    )));
    Ok(())
}

#[test]
fn inherited_source_rule_is_pinned() {
    let rule = crate::RULES
        .iter()
        .find(|r| r.number == RuleId::M078)
        .unwrap();
    assert_eq!(
        rule.statement,
        "Each inherited source declaration has the admitted signature, decoded visual body and complete dependency closure, with named quantity, metric and registered-variance interfaces unchanged. A source/expression pairing is reviewed individually or established by an admitted closed declaration decoder. Each inherited definition is compared at its effective declaration in the captured load order, with its complete source and dependency closure and every intervening definition, removal and redeclaration in a recognised definition form accounted for. A typed-tooling redeclaration is compared as an explicitly admitted source case, including its checking calls, or refuses; an earlier body alone cannot satisfy the comparison. Registered-variance declaration sites for each inherited name across that effective closure equal its recorded count."
    );
    assert_eq!(
        rule.scope,
        "The three named source files and the explicitly admitted typed-tooling/predecessor inputs that determine an inherited name's effective declaration, bytes, signature, dependencies and checking interface. A dynamic-target definition site in a recognised form other than the admitted template, a quantity-store operation outside the admitted typed-tooling sites, and a changed sequence of recognised load commands refuse."
    );
}

#[test]
fn n01_primitive_inventory_and_signatures() -> TestResult {
    use PrimitiveId::{
        Add, Alpha, Beta, Calligraphic, Dimension, Dot, Equal, Fraction, Gamma, Juxtapose,
        Multiply, Negate, Omega, Parentheses, Sigma, Subscript, Subtract, Superscript, TightJoin,
        Times, Underbrace, UprightNet, UprightS, UprightVir,
    };
    let graph = seed()?;
    let book = addressing(&graph)?;
    let expected = [
        (Alpha, "alpha", 0),
        (Beta, "beta", 0),
        (Fraction, "fraction", 2),
        (Subscript, "subscript", 2),
        (Superscript, "superscript", 2),
        (Add, "add", 2),
        (Equal, "equal", 2),
        (Negate, "negate", 1),
        (Multiply, "multiply", 2),
        (Juxtapose, "juxtapose", 2),
        (Parentheses, "parentheses", 1),
        (Dimension, "dimension", 1),
        (Omega, "omega", 0),
        (Calligraphic, "calligraphic", 1),
        (UprightVir, "upright-vir", 0),
        (UprightS, "upright-s", 0),
        (Dot, "dot", 1),
        (Sigma, "sigma", 0),
        (Gamma, "gamma", 0),
        (Times, "times", 2),
        (Subtract, "subtract", 2),
        (TightJoin, "tight-join", 2),
        (Underbrace, "underbrace", 2),
        (UprightNet, "upright-net", 0),
        (PrimitiveId::CapitalOmega, "capital-omega", 0),
        (PrimitiveId::CapitalXi, "capital-xi", 0),
        (PrimitiveId::Chi, "chi", 0),
        (PrimitiveId::Eta, "eta", 0),
        (PrimitiveId::Lambda, "lambda", 0),
        (PrimitiveId::Mu, "mu", 0),
        (PrimitiveId::Phi, "phi", 0),
        (PrimitiveId::Tau, "tau", 0),
        (PrimitiveId::Theta, "theta", 0),
        (PrimitiveId::VariantEpsilon, "variant-epsilon", 0),
        (PrimitiveId::Zeta, "zeta", 0),
        (PrimitiveId::Nabla, "nabla", 0),
        (PrimitiveId::LessEqual, "less-equal", 2),
    ];
    let n = notation(&book)?;
    assert_eq!(
        n.primitives()
            .iter()
            .map(|p| (p.id(), p.token(), p.signature().arity))
            .collect::<Vec<_>>(),
        expected
    );
    for (id, _, arity) in expected {
        assert_eq!(id.operation().arity(), arity);
        assert_eq!(
            Signature { arity }.parameters().collect::<Vec<_>>(),
            vec![MathSort::MathExpr; usize::from(arity)]
        );
        for actual in [
            usize::from(arity).checked_sub(1),
            Some(usize::from(arity) + 1),
        ]
        .into_iter()
        .flatten()
        {
            let error =
                check_expression(&n, primitive(id, vec![literal("x"); actual])).unwrap_err();
            exact(
                &error,
                &[diag(
                    RuleId::M047,
                    NotationStage::Ast,
                    expression_site(&["arguments"]),
                    NotationFault::CallArity {
                        callee: Callee::Primitive(id),
                        expected: arity,
                        actual,
                    },
                )],
                &format!(
                    "M047 Ast at Expression {{ path: [Field(\"arguments\")] }}: CallArity {{ callee: Primitive({id:?}), expected: {arity}, actual: {actual} }}\n"
                ),
            );
            check_expression(&n, primitive(id, vec![literal("x"); usize::from(arity)]))?;
        }
    }
    let mut p = primitives::catalogue();
    p.remove(0);
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 36, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Primitive(Alpha),
                expected: 1,
                actual: 0,
                positions: vec![],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 36, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 0, positions: [] }\n",
    );
    let mut p = primitives::catalogue();
    let mut duplicate = p[0].clone();
    duplicate.token = "other";
    p.push(duplicate);
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 0, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Primitive(Alpha),
                expected: 1,
                actual: 2,
                positions: vec![0, 37],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 2, positions: [0, 37] }\n",
    );
    let mut p = primitives::catalogue();
    p[1].token = "alpha";
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Alpha, 0, &["token"]),
            NotationFault::DuplicateToken {
                token: "alpha".into(),
                entries: vec![
                    RegistryEntry::Primitive(Alpha),
                    RegistryEntry::Primitive(Beta),
                ],
                positions: vec![0, 1],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"token\")] }: DuplicateToken { token: \"alpha\", entries: [Primitive(Alpha), Primitive(Beta)], positions: [0, 1] }\n",
    );
    let mut p = primitives::catalogue();
    p[2].signature.arity = 1;
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            primitive_site(Fraction, 2, &["signature", "arity"]),
            NotationFault::PrimitiveSignature {
                primitive: Fraction,
                expected: 2,
                actual: 1,
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Fraction), index: 2, path: [Field(\"signature\"), Field(\"arity\")] }: PrimitiveSignature { primitive: Fraction, expected: 2, actual: 1 }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n02_literal_grammar_exact_offsets() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    for value in ["a", "Z", "1", "45", "000"] {
        check_expression(&n, literal(value))?;
    }
    for (value, reason, byte, scalar) in [
        ("", LiteralReason::Empty, 0, None),
        ("ab", LiteralReason::MultipleLetters, 1, Some("b")),
        ("a1", LiteralReason::ForbiddenScalar, 1, Some("1")),
        ("1a", LiteralReason::ForbiddenScalar, 1, Some("a")),
        ("αx", LiteralReason::ForbiddenScalar, 0, Some("α")),
        (
            "a\u{301}",
            LiteralReason::ForbiddenScalar,
            1,
            Some("\u{301}"),
        ),
        ("1.2", LiteralReason::ForbiddenScalar, 1, Some(".")),
    ] {
        let scalar = scalar.map(str::to_string);
        exact(
            &check_expression(&n, literal(value)).unwrap_err(),
            &[diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["value"]),
                NotationFault::InvalidLiteral {
                    value: value.into(),
                    reason,
                    byte,
                    scalar: scalar.clone(),
                },
            )],
            &format!(
                "M047 Ast at Expression {{ path: [Field(\"value\")] }}: InvalidLiteral {{ value: {value:?}, reason: {reason:?}, byte: {byte}, scalar: {scalar:?} }}\n"
            ),
        );
        check_expression(&n, literal("x"))?;
    }
    for scalar in [
        "\\", "{", "}", "_", "^", "#", "%", "$", "&", "<", ">", "+", "-", ".", " ", "\t", "\n",
        "\r", "\0", "\u{7f}", "\u{a0}", "\u{2003}", "\u{85}", "β", "é", "\u{301}",
    ] {
        exact(
            &check_expression(&n, literal(scalar)).unwrap_err(),
            &[diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["value"]),
                NotationFault::InvalidLiteral {
                    value: scalar.into(),
                    reason: LiteralReason::ForbiddenScalar,
                    byte: 0,
                    scalar: Some(scalar.into()),
                },
            )],
            &format!(
                "M047 Ast at Expression {{ path: [Field(\"value\")] }}: InvalidLiteral {{ value: {scalar:?}, reason: ForbiddenScalar, byte: 0, scalar: Some({scalar:?}) }}\n"
            ),
        );
        check_expression(&n, literal("0"))?;
    }
    check_expression(&n, literal(&"0".repeat(256)))?;
    exact(
        &check_expression(&n, literal(&"0".repeat(257))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            expression_site(&["value"]),
            NotationFault::NotationLimit {
                resource: NotationResource::LiteralBytes,
                limit: 256,
                actual: 257,
            },
        )],
        "M047 Ast at Expression { path: [Field(\"value\")] }: NotationLimit { resource: LiteralBytes, limit: 256, actual: 257 }\n",
    );
    Ok(())
}

#[test]
fn n03_stem_grammar_collision_and_category_separation() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree};
    let graph = seed()?;
    let book = addressing(&graph)?;
    for value in [
        "",
        "x",
        "X",
        "XML",
        "Coefficient1",
        "Coefficient_One",
        "Élan",
        "Bad\\Name",
        "Bad{Name}",
    ] {
        let mut d = catalogue::catalogue();
        d[0].stem = value;
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::Catalogue,
                registry_site(CoefficientOne, 0, &["stem"]),
                NotationFault::InvalidStem {
                    value: value.into(),
                },
            )],
            &format!(
                "M048 Catalogue at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"stem\")] }}: InvalidStem {{ value: {value:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    d[1].stem = "CoefficientOne";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["stem"]),
            NotationFault::CommandCollision {
                command: "\\FtooQCoefficientOne".into(),
                definitions: vec![CoefficientOne, CoefficientThree],
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"stem\")] }: CommandCollision { command: \"\\\\FtooQCoefficientOne\", definitions: [CoefficientOne, CoefficientThree] }\n",
    );
    let mut d = catalogue::catalogue();
    d[4].stem = "CoefficientOne";
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(DefinitionId::FirstCoefficient).command(),
        "\\FtooACoefficientOne"
    );
    assert_eq!(
        n.definition(CoefficientOne).command(),
        "\\FtooQCoefficientOne"
    );
    let mut d = catalogue::catalogue();
    d.swap(0, 6);
    d[6].stem = "Bad1";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 6, &["stem"]),
            NotationFault::InvalidStem {
                value: "Bad1".into(),
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 6, path: [Field(\"stem\")] }: InvalidStem { value: \"Bad1\" }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_definition_shapes() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    for (i, id, arity, reason) in [
        (
            0,
            DefinitionId::CoefficientOne,
            1,
            DefinitionShapeReason::CanonicalMustBeNullary,
        ),
        (
            4,
            DefinitionId::FirstCoefficient,
            1,
            DefinitionShapeReason::AliasMustBeNullary,
        ),
        (
            5,
            DefinitionId::Dimension,
            0,
            DefinitionShapeReason::ConstructorArity,
        ),
        (
            5,
            DefinitionId::Dimension,
            10,
            DefinitionShapeReason::ConstructorArity,
        ),
    ] {
        let mut d = catalogue::catalogue();
        d[i].signature.arity = arity;
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::Catalogue,
                registry_site(id, i, &["signature", "arity"]),
                NotationFault::DefinitionShape {
                    definition: id,
                    reason: reason.clone(),
                },
            )],
            &format!(
                "M048 Catalogue at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition({id:?}), index: {i}, path: [Field(\"signature\"), Field(\"arity\")] }}: DefinitionShape {{ definition: {id:?}, reason: {reason:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[0]) = Some(NodeId::LangevinCoefficientThree);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Catalogue,
            registry_site(DefinitionId::CoefficientOne, 0, &["backing"]),
            NotationFault::DefinitionShape {
                definition: DefinitionId::CoefficientOne,
                reason: DefinitionShapeReason::BackingObjectMismatch {
                    expected: NodeId::LangevinCoefficientOne,
                    actual: NodeId::LangevinCoefficientThree,
                },
            },
        )],
        "M048 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"backing\")] }: DefinitionShape { definition: CoefficientOne, reason: BackingObjectMismatch { expected: LangevinCoefficientOne, actual: LangevinCoefficientThree } }\n",
    );
    Ok(())
}

#[test]
fn n03_alias_targets_missing_entries_and_canonical_coverage() -> TestResult {
    use DefinitionId::{CoefficientOne, Dimension, FirstCoefficient};
    use NodeId::{LangevinCoefficientOne as One, LangevinCoefficientThree as Three};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    d.remove(0);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 46, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Definition(CoefficientOne),
                expected: 1,
                actual: 0,
                positions: vec![],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 46, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Definition(CoefficientOne), expected: 1, actual: 0, positions: [] }\n",
    );
    for (target, reason) in [
        (Dimension, AliasTargetReason::ConstructorTarget),
        (DefinitionId::Comovfield, AliasTargetReason::InheritedTarget),
        (
            DefinitionId::CoefficientThree,
            AliasTargetReason::ObjectMismatch {
                expected: One,
                actual: Three,
            },
        ),
    ] {
        let mut d = catalogue::catalogue();
        d[4].kind = DefinitionKind::Alias {
            object: One,
            backing: Some(One),
            target,
        };
        exact(
            &construct(&book, primitives::catalogue(), d).unwrap_err(),
            &[diag(
                RuleId::M048,
                NotationStage::References,
                registry_site(FirstCoefficient, 4, &["target"]),
                NotationFault::AliasTarget {
                    alias: FirstCoefficient,
                    target,
                    reason: reason.clone(),
                },
            )],
            &format!(
                "M048 References at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }}: AliasTarget {{ alias: FirstCoefficient, target: {target:?}, reason: {reason:?} }}\n"
            ),
        );
        notation(&book)?;
    }
    let mut d = catalogue::catalogue();
    d[4].kind = DefinitionKind::Canonical {
        object: One,
        backing: Some(One),
        body: literal("x"),
    };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::References,
            registry_site(CoefficientOne, 0, &["object"]),
            NotationFault::CanonicalMultiplicity {
                object: One,
                expected: 1,
                actual: 2,
                definitions: vec![CoefficientOne, FirstCoefficient],
                positions: vec![0, 4],
            },
        )],
        "M048 References at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"object\")] }: CanonicalMultiplicity { object: LangevinCoefficientOne, expected: 1, actual: 2, definitions: [CoefficientOne, FirstCoefficient], positions: [0, 4] }\n",
    );
    let mut d = catalogue::catalogue();
    d[0].kind = DefinitionKind::Alias {
        object: One,
        backing: Some(One),
        target: FirstCoefficient,
    };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::References,
            NotationSite::Catalogue {
                file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
            },
            NotationFault::CanonicalMultiplicity {
                object: One,
                expected: 1,
                actual: 0,
                definitions: vec![],
                positions: vec![],
            },
        )],
        "M048 References at Catalogue { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\" }: CanonicalMultiplicity { object: LangevinCoefficientOne, expected: 1, actual: 0, definitions: [], positions: [] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_cycles_have_concrete_closed_paths() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree, Dimension, FirstCoefficient, Product};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(CoefficientOne, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, CoefficientOne],
                sites: vec![s],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(CoefficientThree, vec![]);
    *body_mut(&mut d[1]) = call(CoefficientOne, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    let t = registry_site(CoefficientThree, 1, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, CoefficientThree, CoefficientOne],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, CoefficientThree, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientThree), index: 1, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = call(FirstCoefficient, vec![]);
    let s = registry_site(CoefficientOne, 0, &["body", "callee"]);
    let t = registry_site(FirstCoefficient, 4, &["target"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![CoefficientOne, FirstCoefficient, CoefficientOne],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [CoefficientOne, FirstCoefficient, CoefficientOne], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }] }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = call(Product, vec![ExprDraft::Slot { index: 0 }, literal("x")]);
    *body_mut(&mut d[6]) = call(
        Dimension,
        vec![primitive(
            PrimitiveId::Multiply,
            vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 1 }],
        )],
    );
    let s = registry_site(Dimension, 5, &["body", "callee"]);
    let t = registry_site(Product, 6, &["body", "callee"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            s.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![Dimension, Product, Dimension],
                sites: vec![s, t],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Dimension), index: 5, path: [Field(\"body\"), Field(\"callee\")] }: DefinitionCycle { chain: [Dimension, Product, Dimension], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Dimension), index: 5, path: [Field(\"body\"), Field(\"callee\")] }, Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), Field(\"callee\")] }] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n04_slots_and_all_arguments_are_checked() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    exact(
        &check_expression(&n, ExprDraft::Slot { index: 0 }).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            expression_site(&["index"]),
            NotationFault::SlotOutOfScope { index: 0, arity: 0 },
        )],
        "M047 Ast at Expression { path: [Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[0]) = ExprDraft::Slot { index: 0 };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            registry_site(DefinitionId::CoefficientOne, 0, &["body", "index"]),
            NotationFault::SlotOutOfScope { index: 0, arity: 0 },
        )],
        "M047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"body\"), Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[6]) = ExprDraft::Slot { index: 2 };
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body"]),
                NotationFault::UnusedSlot { index: 0, arity: 2 },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body"]),
                NotationFault::UnusedSlot { index: 1, arity: 2 },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                registry_site(DefinitionId::Product, 6, &["body", "index"]),
                NotationFault::SlotOutOfScope { index: 2, arity: 2 },
            ),
        ],
        "M047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\")] }: UnusedSlot { index: 0, arity: 2 }\nM047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\")] }: UnusedSlot { index: 1, arity: 2 }\nM047 Ast at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), Field(\"index\")] }: SlotOutOfScope { index: 2, arity: 2 }\n",
    );
    let mut d = catalogue::catalogue();
    d[5].signature.arity = 9;
    *body_mut(&mut d[5]) = (0..9)
        .map(|index| ExprDraft::Slot { index })
        .reduce(|a, b| primitive(PrimitiveId::Add, vec![a, b]))
        .unwrap();
    let n9 = construct(&book, primitives::catalogue(), d)?;
    check_expression(&n9, call(DefinitionId::Dimension, vec![literal("x"); 9]))?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    *body_mut(&mut d[6]) = primitive(
        PrimitiveId::Multiply,
        vec![
            call(DefinitionId::Dimension, vec![ExprDraft::Slot { index: 1 }]),
            ExprDraft::Slot { index: 0 },
        ],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    let e = check_expression(
        &n,
        call(
            DefinitionId::Product,
            vec![
                call(DefinitionId::CoefficientOne, vec![]),
                call(DefinitionId::CoefficientThree, vec![]),
            ],
        ),
    )?;
    assert_eq!(
        e.uses()
            .iter()
            .map(SymbolUse::definition)
            .collect::<Vec<_>>(),
        [
            DefinitionId::CoefficientOne,
            DefinitionId::CoefficientThree,
            DefinitionId::CoefficientThree
        ]
    );
    Ok(())
}

fn retained_draft() -> crate::ManifestDraft {
    let mut draft = crate::seed_draft();
    for node in &mut draft.nodes {
        node.maturity = crate::Maturity::Retained;
    }
    draft
}
#[test]
fn n05_every_own_binding_reason_from_real_graph() -> TestResult {
    use crate::{Awaiting, Blocker, CapabilityId, ClaimAudit, NodePayload};
    use NodeId::{LangevinCoefficientOne as One, LangevinCoefficientOneValue as Value};
    let graph = crate::validate(retained_draft())?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    assert_eq!(
        n.quantity_symbol(One)?.status().unwrap().tier(),
        SymbolTier::Authorised
    );
    assert_eq!(n.quantity_symbol(One)?.status().unwrap().reasons(), []);
    assert_eq!(
        n.object_symbol(Value)?.status().unwrap().reasons(),
        [PendingReason::ClaimDeferred {
            object: Value,
            blocker: Blocker::AuditRunRequired
        }]
    );
    for awaits in [
        Awaiting::AuditBinding,
        Awaiting::Capability(CapabilityId::ExactRationalLinearEquivalence),
        Awaiting::Capability(CapabilityId::SymbolicTrigonometry),
        Awaiting::Capability(CapabilityId::TrigonometricIntegration),
    ] {
        let mut draft = retained_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        draft
            .budgets
            .retain(|b| b.producer.descriptor().claim != Value);
        for node in &mut draft.nodes {
            if node.id == Value
                && let NodePayload::Claim { audit, .. } = &mut node.payload
            {
                *audit = ClaimAudit::Awaiting(awaits);
            }
        }
        let graph = crate::validate(draft)?;
        let book = addressing(&graph)?;
        let n = notation(&book)?;
        let reason = PendingReason::ClaimDeferred {
            object: Value,
            blocker: Blocker::Awaiting(awaits),
        };
        assert_eq!(
            n.object_symbol(Value)?.status().unwrap().tier(),
            SymbolTier::Pending
        );
        assert_eq!(
            n.object_symbol(Value)?.status().unwrap().reasons(),
            [reason]
        );
    }
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[0]) = None;
    let n = construct(&book, primitives::catalogue(), d)?;
    let status = n.quantity_symbol(One)?.status().unwrap();
    assert_eq!(status.tier(), SymbolTier::Pending);
    assert_eq!(
        status.reasons(),
        [PendingReason::BackingUnbound { object: One }]
    );
    assert_eq!(
        status.reasons()[0].to_string(),
        "backing object def:langevin:coefficient-one is unbound"
    );
    let mut draft = retained_draft();
    draft.coverage.slice.clear();
    draft.coverage.closed_world.audit = false;
    draft.nodes.retain(|node| node.id != One);
    draft.edges.retain(|e| e.source != One && e.target != One);
    let graph = crate::validate(draft)?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    assert_eq!(
        n.definition(DefinitionId::CoefficientOne)
            .status()
            .unwrap()
            .reasons(),
        [PendingReason::BackingAbsent { object: One }]
    );
    assert_eq!(
        n.definition(DefinitionId::CoefficientOne)
            .status()
            .unwrap()
            .reasons()[0]
            .to_string(),
        "backing object def:langevin:coefficient-one is absent"
    );
    assert_eq!(
        n.object_symbol(One).unwrap_err(),
        SymbolLookupFault::AbsentObject { object: One }
    );
    assert_eq!(
        n.object_symbol(One).unwrap_err().to_string(),
        "AbsentObject { object: LangevinCoefficientOne }"
    );
    assert_eq!(
        n.quantity_symbol(One).unwrap_err(),
        SymbolLookupFault::AbsentObject { object: One }
    );
    Ok(())
}

#[test]
fn n06_recursive_alias_minima_and_transitive_reason_union() -> TestResult {
    use DefinitionId::{CoefficientOne, Dimension, FirstCoefficient};
    use NodeId::LangevinCoefficientOne as One;
    for (proposed, missing, expected) in [
        (false, false, vec![]),
        (
            false,
            true,
            vec![PendingReason::BackingUnbound { object: One }],
        ),
        (
            true,
            false,
            vec![PendingReason::FormulationProposed { object: One }],
        ),
        (
            true,
            true,
            vec![
                PendingReason::BackingUnbound { object: One },
                PendingReason::FormulationProposed { object: One },
            ],
        ),
    ] {
        let graph = crate::validate(if proposed {
            crate::seed_draft()
        } else {
            retained_draft()
        })?;
        let book = addressing(&graph)?;
        let mut d = catalogue::catalogue();
        if missing {
            *backing_mut(&mut d[4]) = None;
        }
        let n = construct(&book, primitives::catalogue(), d)?;
        let status = n.definition(FirstCoefficient).status().unwrap();
        assert_eq!(status.reasons(), expected);
        assert_eq!(
            status.tier(),
            if expected.is_empty() {
                SymbolTier::Authorised
            } else {
                SymbolTier::Pending
            }
        );
        assert_eq!(n.quantity_symbol(One)?.spec().id(), CoefficientOne);
    }
    let graph = crate::validate(retained_draft())?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *backing_mut(&mut d[4]) = None;
    d[5].signature.arity = 0;
    d[5].kind = DefinitionKind::Alias {
        object: One,
        backing: Some(One),
        target: FirstCoefficient,
    };
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(CoefficientOne).status().unwrap().tier(),
        SymbolTier::Authorised
    );
    for id in [FirstCoefficient, Dimension] {
        assert_eq!(
            n.definition(id).status().unwrap().tier(),
            SymbolTier::Pending
        );
        assert_eq!(
            n.definition(id).status().unwrap().reasons(),
            [PendingReason::BackingUnbound { object: One }]
        );
    }
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[1]) = call(CoefficientOne, vec![]);
    let n = construct(&book, primitives::catalogue(), d)?;
    assert_eq!(
        n.definition(DefinitionId::CoefficientThreeValue)
            .status()
            .unwrap()
            .reasons(),
        [
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientThreeValue
            },
            PendingReason::FormulationProposed { object: One },
            PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientThree
            },
            PendingReason::ClaimDeferred {
                object: NodeId::LangevinCoefficientThreeValue,
                blocker: crate::Blocker::AuditRunRequired
            },
        ]
    );
    Ok(())
}

#[test]
fn n07_addressing_independence_and_snapshot_preservation() -> TestResult {
    use crate::{OutOfSliceCause, TagId, TagResolutionFault};
    use NodeId::{
        LangevinCoefficientOne as One, LangevinCoefficientOneValue as Value,
        LangevinCoefficientThree as Three,
    };
    let mut snapshots = vec![];
    for slice in [
        vec![Value, NodeId::LangevinCoefficientThreeValue],
        vec![Value],
        vec![],
    ] {
        let mut draft = retained_draft();
        draft.coverage.slice = slice.clone();
        let graph = crate::validate(draft)?;
        let before = crate::projection_bytes(&graph)?;
        let book = addressing(&graph)?;
        let history: Vec<_> = book.history().cloned().collect();
        let n = notation(&book)?;
        snapshots.push(
            n.definitions()
                .iter()
                .map(|d| d.status().cloned())
                .collect::<Vec<_>>(),
        );
        if slice.is_empty() {
            let error = book.resolve("c:1").unwrap_err();
            assert_eq!(
                error,
                TagResolutionFault::OutOfSliceTag {
                    input: "c:1".into(),
                    tag: TagId::LangevinCoefficientOne,
                    target: One,
                    cause: OutOfSliceCause::QuantityOutsideSupport
                }
            );
            assert_eq!(
                error.to_string(),
                "tag \"c:1\": out of slice; label def:langevin:coefficient-one; cause quantity-outside-support"
            );
        } else {
            assert_eq!(book.resolve("c:1")?, crate::AddressTarget::Theory(One));
        }
        if slice.len() == 1 {
            let error = book.resolve("c:3").unwrap_err();
            assert_eq!(
                error,
                TagResolutionFault::OutOfSliceTag {
                    input: "c:3".into(),
                    tag: TagId::LangevinCoefficientThree,
                    target: Three,
                    cause: OutOfSliceCause::QuantityOutsideSupport
                }
            );
            assert_eq!(
                error.to_string(),
                "tag \"c:3\": out of slice; label def:langevin:coefficient-three; cause quantity-outside-support"
            );
        }
        assert_eq!(
            n.quantity_symbol(Value).unwrap_err(),
            SymbolLookupFault::NotQuantity {
                object: Value,
                actual: NodeKind::Claim
            }
        );
        assert_eq!(
            n.quantity_symbol(Value).unwrap_err().to_string(),
            "NotQuantity { object: LangevinCoefficientOneValue, actual: Claim }"
        );
        for object in NodeId::ALL {
            check_expression(&n, call(n.object_symbol(object)?.spec().id(), vec![]))?;
        }
        assert_eq!(book.history().cloned().collect::<Vec<_>>(), history);
        assert_eq!(crate::projection_bytes(&graph)?, before);
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
    Ok(())
}

fn chain(count: usize, mut expr: ExprDraft) -> ExprDraft {
    for _ in 0..count {
        expr = primitive(PrimitiveId::Parentheses, vec![expr]);
    }
    expr
}
fn full_binary(levels: usize) -> ExprDraft {
    if levels == 0 {
        literal("x")
    } else {
        primitive(
            PrimitiveId::Add,
            vec![full_binary(levels - 1), full_binary(levels - 1)],
        )
    }
}
fn argument_path(indices: &[usize]) -> Vec<NotationPathPart> {
    indices
        .iter()
        .flat_map(|i| {
            [
                NotationPathPart::Field("arguments".into()),
                NotationPathPart::Argument(*i),
            ]
        })
        .collect()
}
#[test]
fn n02_depth_and_node_boundaries() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    check_expression(&n, chain(63, literal("x")))?;
    let path = argument_path(&[0; 64]);
    let expected_path = std::iter::repeat_n("Field(\"arguments\"), Argument(0)", 64)
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &check_expression(&n, chain(64, literal("x"))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::Depth,
                limit: 64,
                actual: 65,
            },
        )],
        &format!(
            "M047 Ast at Expression {{ path: [{expected_path}] }}: NotationLimit {{ resource: Depth, limit: 64, actual: 65 }}\n"
        ),
    );
    check_expression(&n, chain(1, full_binary(11)))?;
    let mut indices = vec![0, 0];
    indices.extend([1; 11]);
    let path = argument_path(&indices);
    let expected_path = [
        vec!["Field(\"arguments\"), Argument(0)"; 2],
        vec!["Field(\"arguments\"), Argument(1)"; 11],
    ]
    .concat()
    .join(", ");
    exact(
        &check_expression(&n, chain(2, full_binary(11))).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Ast,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::AstNodes,
                limit: 4096,
                actual: 4097,
            },
        )],
        &format!(
            "M047 Ast at Expression {{ path: [{expected_path}] }}: NotationLimit {{ resource: AstNodes, limit: 4096, actual: 4097 }}\n"
        ),
    );
    check_expression(&n, literal("x"))?;
    Ok(())
}

#[test]
fn document_text_run_cannot_step_past_node_bound() -> TestResult {
    fn balanced_literals(leaves: usize) -> ExprDraft {
        if leaves == 1 {
            literal("a")
        } else {
            primitive(
                PrimitiveId::Add,
                vec![
                    balanced_literals(leaves / 2),
                    balanced_literals(leaves - leaves / 2),
                ],
            )
        }
    }
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let expression = primitive(
        PrimitiveId::Add,
        vec![
            balanced_literals(2_047),
            primitive(
                PrimitiveId::Equal,
                vec![
                    ExprDraft::Text {
                        runs: vec![TextRun {
                            text: "a".into(),
                            source: 0..1,
                            decoded: 0..1,
                        }],
                    },
                    primitive(PrimitiveId::Negate, vec![literal("a"), literal("b")]),
                ],
            ),
        ],
    );
    let error = check_document_expression(&catalogue, expression).unwrap_err();
    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(
        error.diagnostics()[0].fault,
        NotationFault::NotationLimit {
            resource: NotationResource::AstNodes,
            limit: 4_096,
            actual: 4_097,
        }
    );
    assert_eq!(
        error.diagnostics()[0].site,
        NotationSite::Expression {
            path: argument_path(&[1, 0]),
        }
    );
    Ok(())
}
#[test]
fn document_text_runs_keep_source_and_decoded_order() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    for (first, source, decoded) in [
        ("a", vec![2..3, 0..1], vec![0..1, 2..3]),
        ("ab", vec![0..2, 1..2], vec![0..2, 2..3]),
        ("a", vec![0..1, 2..3], vec![2..3, 0..1]),
    ] {
        let expression = ExprDraft::Text {
            runs: vec![
                TextRun {
                    text: first.into(),
                    source: source[0].clone(),
                    decoded: decoded[0].clone(),
                },
                TextRun {
                    text: "b".into(),
                    source: source[1].clone(),
                    decoded: decoded[1].clone(),
                },
            ],
        };
        let error = check_document_expression(&catalogue, expression).unwrap_err();
        assert!(error.diagnostics().iter().any(|diagnostic| diagnostic.fault
            == NotationFault::InvalidText { value: "b".into() }
            && diagnostic.site
                == NotationSite::Expression {
                    path: vec![
                        NotationPathPart::Field("runs".into()),
                        NotationPathPart::Argument(1)
                    ]
                }));
    }
    Ok(())
}
// Independent visit arithmetic: a duplicating invocation visits its call, the
// binary primitive and two slots, plus two visits of its argument tree.
fn expanded_size(size: usize) -> ExprDraft {
    if size == 1 {
        literal("x")
    } else if size >= 6 && size.is_multiple_of(2) {
        call(DefinitionId::Dimension, vec![expanded_size((size - 4) / 2)])
    } else {
        chain(1, expanded_size(size - 1))
    }
}
#[test]
fn n02_substitution_counts_repeated_occurrences_at_exact_limit() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    check_expression(&n, expanded_size(65_536))?;
    let error = check_expression(&n, chain(1, expanded_size(65_536))).unwrap_err();
    // Each argument remains at its authored site, even on the final duplicate.
    let mut size: usize = 65_536;
    let mut indices = vec![0];
    while size != 1 {
        indices.push(0);
        size = if size >= 6 && size.is_multiple_of(2) {
            (size - 4) / 2
        } else {
            size - 1
        };
    }
    let path = argument_path(&indices);
    let display_path = std::iter::repeat_n("Field(\"arguments\"), Argument(0)", indices.len())
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &error,
        &[diag(
            RuleId::M047,
            NotationStage::Dependencies,
            NotationSite::Expression { path },
            NotationFault::NotationLimit {
                resource: NotationResource::ExpansionNodes,
                limit: 65_536,
                actual: 65_537,
            },
        )],
        &format!(
            "M047 Dependencies at Expression {{ path: [{display_path}] }}: NotationLimit {{ resource: ExpansionNodes, limit: 65536, actual: 65537 }}\n"
        ),
    );
    check_expression(&n, expanded_size(65_536))?;
    Ok(())
}

#[test]
fn n16_exact_appended_register() {
    use crate::{RULES, RuleState};
    let rules = &RULES[..49];
    assert_eq!(
        rules[46..]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M047,
                "Notation expressions use only the closed registry, exact signatures, bounded literal grammar and scoped constructor parameters, and every formal is used, within declared notation limits. Inherited signatures and decoded bodies obey the same grammar and limits, with no checking-effect fields. Mathematics-text nodes contain bounded decoded plain runs in submitted document expressions; text nodes in catalogue bodies and tag runs refuse. An authored underbrace annotation whose decoded text, after trimming ASCII spaces, is pending refuses to preserve the derived status marker's distinction.",
                "Every primitive descriptor, definition body and submitted notation expression.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M048,
                "Definitions have their kind's signature shape and any backing equals their object; stems obey the reserved alphabetic grammar and generated names are unique; every permanent object has exactly one canonical symbol; aliases preserve that object and terminate at its canonical symbol; all definition dependencies are acyclic. An inherited definition has a checked signature and decoded body but no theory object or backing; an identical fully expanded body with an object-bound canonical symbol or a dependency on a non-inherited definition in its own body refuses admission.",
                "The complete notation definition catalogue, including absent objects and unused definitions.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M049,
                "Symbol tiers are derived from the supplied graph, an accepted retained audit context where supplied, and transitive symbol uses; an alias takes the lesser of its own binding tier and its target tier, retaining every pending reason. An inherited definition without a theory object is pending with a missing-binding reason, which every definition using it retains; generated notation-status evidence lists every inherited definition and its reason.",
                "Base checked notation catalogues, accepted retained notation views and all exposed symbol-status values, including inherited missing bindings, transitive uses and generated/notation-status.json.",
                RuleState::Structural,
                None
            ),
        ]
    );
    assert_eq!(
        (
            rules.len(),
            rules
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            rules
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            rules.iter().filter(|r| r.deferred_reason.is_some()).count()
        ),
        (49, 40, 4, 5)
    );
    assert_eq!(
        (
            RULES[..46].len(),
            RULES[..46]
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            RULES[..46]
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            RULES[..46]
                .iter()
                .filter(|r| r.deferred_reason.is_some())
                .count()
        ),
        (46, 38, 3, 5)
    );
}

#[test]
fn n16_renderer_register_has_no_existing_transitions() {
    use crate::{RULES, RuleState};
    assert_eq!(
        RULES[49..52]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M050,
                "Generated notation declarations refuse collisions in the consuming typesetting context. Temporary inherited declarations used by the sweep and expression validation refuse collisions in that same checked KaTeX context.",
                "Every emitted notation declaration and temporary inherited declaration used in the checked KaTeX context.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M051,
                "Both notation forms derive from the same checked AST with explicit token boundaries and within the output limit; full Unicode expansion contains no project commands and every returned expression passes the pinned KaTeX grammar. For inherited definitions instead, the paper form invokes the existing checked source declaration by name, whose body is source-compared with the admitted AST, and the strict screen form derives from that AST; pending marks and visible pending explanations are omitted only when every pending reason is an inherited missing binding.",
                "Generated declaration bodies, macro invocations and fully expanded notation expressions, including named inherited paper invocations and their checked source-comparison exception.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M052,
                "A generated sweep typesets every emitted notation command with its declared arguments and checks its fully expanded counterpart. The KaTeX sweep includes temporary declarations and fully expanded strict screen forms for inherited definitions.",
                "The complete emitted command inventory, including aliases, constructors, pending symbols and temporary inherited declarations in KaTeX.",
                RuleState::Enforced,
                None
            ),
        ]
    );
    assert_eq!(
        (
            RULES.len(),
            RULES
                .iter()
                .filter(|r| r.state == RuleState::Enforced)
                .count(),
            RULES
                .iter()
                .filter(|r| r.state == RuleState::Structural)
                .count(),
            RULES.iter().filter(|r| r.deferred_reason.is_some()).count()
        ),
        (76, 65, 5, 6)
    );
}

#[test]
fn n01_definition_multiplicity_tokens_and_every_call_arity() -> TestResult {
    use DefinitionId::{CoefficientOne, CoefficientThree};
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    for (id, arity) in [
        (CoefficientOne, 0),
        (CoefficientThree, 0),
        (DefinitionId::CoefficientOneValue, 0),
        (DefinitionId::CoefficientThreeValue, 0),
        (DefinitionId::FirstCoefficient, 0),
        (DefinitionId::Dimension, 1),
        (DefinitionId::Product, 2),
    ] {
        for actual in [
            usize::from(arity).checked_sub(1),
            Some(usize::from(arity) + 1),
        ]
        .into_iter()
        .flatten()
        {
            exact(
                &check_expression(&n, call(id, vec![literal("x"); actual])).unwrap_err(),
                &[diag(
                    RuleId::M047,
                    NotationStage::Ast,
                    expression_site(&["arguments"]),
                    NotationFault::CallArity {
                        callee: Callee::Definition(id),
                        expected: arity,
                        actual,
                    },
                )],
                &format!(
                    "M047 Ast at Expression {{ path: [Field(\"arguments\")] }}: CallArity {{ callee: Definition({id:?}), expected: {arity}, actual: {actual} }}\n"
                ),
            );
            check_expression(&n, call(id, vec![literal("x"); usize::from(arity)]))?;
        }
    }
    let mut d = catalogue::catalogue();
    let mut copy = d[0].clone();
    copy.stem = "Another";
    copy.token = "another";
    d.push(copy);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["id"]),
            NotationFault::CatalogMultiplicity {
                entry: RegistryEntry::Definition(CoefficientOne),
                expected: 1,
                actual: 2,
                positions: vec![0, 47],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"id\")] }: CatalogMultiplicity { entry: Definition(CoefficientOne), expected: 1, actual: 2, positions: [0, 47] }\n",
    );
    let mut d = catalogue::catalogue();
    d[1].token = "coefficient-one";
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Catalogue,
            registry_site(CoefficientOne, 0, &["token"]),
            NotationFault::DuplicateToken {
                token: "coefficient-one".into(),
                entries: vec![
                    RegistryEntry::Definition(CoefficientOne),
                    RegistryEntry::Definition(CoefficientThree),
                ],
                positions: vec![0, 1],
            },
        )],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(CoefficientOne), index: 0, path: [Field(\"token\")] }: DuplicateToken { token: \"coefficient-one\", entries: [Definition(CoefficientOne), Definition(CoefficientThree)], positions: [0, 1] }\n",
    );
    let mut p = primitives::catalogue();
    p[1].operation = PrimitiveOperation::Alpha;
    exact(
        &construct(&book, p, catalogue::catalogue()).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Catalogue,
                primitive_site(PrimitiveId::Alpha, 0, &["operation"]),
                NotationFault::CatalogMultiplicity {
                    entry: RegistryEntry::Primitive(PrimitiveId::Alpha),
                    expected: 1,
                    actual: 2,
                    positions: vec![0, 1],
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Catalogue,
                primitive_site(PrimitiveId::Beta, 37, &["operation"]),
                NotationFault::CatalogMultiplicity {
                    entry: RegistryEntry::Primitive(PrimitiveId::Beta),
                    expected: 1,
                    actual: 0,
                    positions: vec![],
                },
            ),
        ],
        "M047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Alpha), index: 0, path: [Field(\"operation\")] }: CatalogMultiplicity { entry: Primitive(Alpha), expected: 1, actual: 2, positions: [0, 1] }\nM047 Catalogue at Registry { file: \"crates/ftoo_manifest/src/notation/primitives.rs\", entry: Primitive(Beta), index: 37, path: [Field(\"operation\")] }: CatalogMultiplicity { entry: Primitive(Beta), expected: 1, actual: 0, positions: [] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n03_all_argument_errors_and_alias_self_cycle() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    let expression = primitive(
        PrimitiveId::Fraction,
        vec![literal("α"), ExprDraft::Slot { index: 0 }, literal("x")],
    );
    let mut first = argument_path(&[0]);
    first.push(NotationPathPart::Field("value".into()));
    let mut second = argument_path(&[1]);
    second.push(NotationPathPart::Field("index".into()));
    exact(
        &check_expression(&n, expression).unwrap_err(),
        &[
            diag(
                RuleId::M047,
                NotationStage::Ast,
                expression_site(&["arguments"]),
                NotationFault::CallArity {
                    callee: Callee::Primitive(PrimitiveId::Fraction),
                    expected: 2,
                    actual: 3,
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                NotationSite::Expression { path: first },
                NotationFault::InvalidLiteral {
                    value: "α".into(),
                    reason: LiteralReason::ForbiddenScalar,
                    byte: 0,
                    scalar: Some("α".into()),
                },
            ),
            diag(
                RuleId::M047,
                NotationStage::Ast,
                NotationSite::Expression { path: second },
                NotationFault::SlotOutOfScope { index: 0, arity: 0 },
            ),
        ],
        "M047 Ast at Expression { path: [Field(\"arguments\")] }: CallArity { callee: Primitive(Fraction), expected: 2, actual: 3 }\nM047 Ast at Expression { path: [Field(\"arguments\"), Argument(0), Field(\"value\")] }: InvalidLiteral { value: \"α\", reason: ForbiddenScalar, byte: 0, scalar: Some(\"α\") }\nM047 Ast at Expression { path: [Field(\"arguments\"), Argument(1), Field(\"index\")] }: SlotOutOfScope { index: 0, arity: 0 }\n",
    );
    let mut d = catalogue::catalogue();
    if let DefinitionKind::Alias { target, .. } = &mut d[4].kind {
        *target = DefinitionId::FirstCoefficient;
    }
    let site = registry_site(DefinitionId::FirstCoefficient, 4, &["target"]);
    exact(
        &construct(&book, primitives::catalogue(), d).unwrap_err(),
        &[diag(
            RuleId::M048,
            NotationStage::Dependencies,
            site.clone(),
            NotationFault::DefinitionCycle {
                chain: vec![
                    DefinitionId::FirstCoefficient,
                    DefinitionId::FirstCoefficient,
                ],
                sites: vec![site],
            },
        )],
        "M048 Dependencies at Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }: DefinitionCycle { chain: [FirstCoefficient, FirstCoefficient], sites: [Registry { file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(FirstCoefficient), index: 4, path: [Field(\"target\")] }] }\n",
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn n05_seed_exact_statuses_and_catalogue_bodies() -> TestResult {
    use DefinitionId::{
        Alignrate, CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue,
        Comovfield, Dimension, FirstCoefficient, Gensym, Product, Readout, Willdecay, Willnet,
        Willvirtue,
    };
    use NodeId::{
        LangevinCoefficientOne as One, LangevinCoefficientOneValue as OneValue,
        LangevinCoefficientThree as Three, LangevinCoefficientThreeValue as ThreeValue,
    };
    let graph = seed()?;
    let book = addressing(&graph)?;
    let n = notation(&book)?;
    let expected = [
        (
            CoefficientOne,
            "coefficient-one",
            "CoefficientOne",
            "\\FtooQCoefficientOne",
            0,
            DefinitionKind::Canonical {
                object: One,
                backing: Some(One),
                body: primitive(PrimitiveId::Subscript, vec![literal("c"), literal("1")]),
            },
        ),
        (
            CoefficientThree,
            "coefficient-three",
            "CoefficientThree",
            "\\FtooQCoefficientThree",
            0,
            DefinitionKind::Canonical {
                object: Three,
                backing: Some(Three),
                body: primitive(PrimitiveId::Subscript, vec![literal("c"), literal("3")]),
            },
        ),
        (
            CoefficientOneValue,
            "coefficient-one-value",
            "CoefficientOneValue",
            "\\FtooOCoefficientOneValue",
            0,
            DefinitionKind::Canonical {
                object: OneValue,
                backing: Some(OneValue),
                body: primitive(
                    PrimitiveId::Equal,
                    vec![
                        call(CoefficientOne, vec![]),
                        primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
                    ],
                ),
            },
        ),
        (
            CoefficientThreeValue,
            "coefficient-three-value",
            "CoefficientThreeValue",
            "\\FtooOCoefficientThreeValue",
            0,
            DefinitionKind::Canonical {
                object: ThreeValue,
                backing: Some(ThreeValue),
                body: primitive(
                    PrimitiveId::Equal,
                    vec![
                        call(CoefficientThree, vec![]),
                        primitive(
                            PrimitiveId::Negate,
                            vec![primitive(
                                PrimitiveId::Fraction,
                                vec![literal("1"), literal("45")],
                            )],
                        ),
                    ],
                ),
            },
        ),
        (
            FirstCoefficient,
            "first-coefficient",
            "FirstCoefficient",
            "\\FtooAFirstCoefficient",
            0,
            DefinitionKind::Alias {
                object: One,
                backing: Some(One),
                target: CoefficientOne,
            },
        ),
        (
            Dimension,
            "dimension",
            "Dimension",
            "\\FtooNDimension",
            1,
            DefinitionKind::Constructor {
                body: primitive(PrimitiveId::Dimension, vec![ExprDraft::Slot { index: 0 }]),
            },
        ),
        (
            Product,
            "product",
            "Product",
            "\\FtooNProduct",
            2,
            DefinitionKind::Constructor {
                body: primitive(
                    PrimitiveId::Multiply,
                    vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 1 }],
                ),
            },
        ),
        (
            Comovfield,
            "comovfield",
            "Comovfield",
            "\\comovfield",
            0,
            DefinitionKind::Inherited { body: literal("S") },
        ),
        (
            Readout,
            "readout",
            "Readout",
            "\\readout",
            0,
            DefinitionKind::Inherited {
                body: primitive(PrimitiveId::Calligraphic, vec![literal("M")]),
            },
        ),
        (
            Willvirtue,
            "willvirtue",
            "Willvirtue",
            "\\willvirtue",
            0,
            DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Superscript,
                    vec![
                        primitive(PrimitiveId::Omega, vec![]),
                        primitive(PrimitiveId::UprightVir, vec![]),
                    ],
                ),
            },
        ),
        (
            Gensym,
            "gensym",
            "Gensym",
            "\\gensym",
            0,
            DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Subscript,
                    vec![literal("A"), primitive(PrimitiveId::UprightS, vec![])],
                ),
            },
        ),
        (
            Alignrate,
            "alignrate",
            "Alignrate",
            "\\alignrate",
            0,
            DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Dot,
                    vec![primitive(PrimitiveId::Sigma, vec![])],
                ),
            },
        ),
        (
            Willnet,
            "willnet",
            "Willnet",
            "\\willnet",
            0,
            DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::Subscript,
                    vec![
                        primitive(PrimitiveId::Omega, vec![]),
                        primitive(PrimitiveId::UprightNet, vec![]),
                    ],
                ),
            },
        ),
        (
            Willdecay,
            "willdecay",
            "Willdecay",
            "\\willdecay",
            0,
            DefinitionKind::Inherited {
                body: primitive(
                    PrimitiveId::TightJoin,
                    vec![
                        primitive(PrimitiveId::Gamma, vec![]),
                        primitive(PrimitiveId::Sigma, vec![]),
                    ],
                ),
            },
        ),
    ];
    assert_eq!(
        n.definitions()
            .iter()
            .map(|d| (
                d.spec().id(),
                d.spec().token(),
                d.spec().stem(),
                d.command(),
                d.spec().signature().arity,
                d.spec().kind().clone()
            ))
            .take(expected.len())
            .collect::<Vec<_>>(),
        expected
    );
    for (id, objects, claim) in [
        (CoefficientOne, vec![One], None),
        (CoefficientThree, vec![Three], None),
        (CoefficientOneValue, vec![OneValue, One], Some(OneValue)),
        (
            CoefficientThreeValue,
            vec![ThreeValue, Three],
            Some(ThreeValue),
        ),
        (FirstCoefficient, vec![One], None),
    ] {
        let mut reasons: Vec<_> = objects
            .into_iter()
            .map(|object| PendingReason::FormulationProposed { object })
            .collect();
        if let Some(object) = claim {
            reasons.push(PendingReason::ClaimDeferred {
                object,
                blocker: crate::Blocker::AuditRunRequired,
            });
        }
        assert_eq!(n.definition(id).status().unwrap().reasons(), reasons);
        assert_eq!(
            n.definition(id).status().unwrap().tier(),
            SymbolTier::Pending
        );
    }
    assert_eq!(n.definition(Dimension).status(), None);
    assert_eq!(n.definition(Product).status(), None);
    Ok(())
}

#[test]
fn n04_repeated_substitution_keeps_each_origin_chain() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut d = catalogue::catalogue();
    *body_mut(&mut d[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let n = construct(&book, primitives::catalogue(), d)?;
    let e = check_expression(
        &n,
        call(
            DefinitionId::Dimension,
            vec![call(DefinitionId::CoefficientOne, vec![])],
        ),
    )?;
    let expected: Vec<_> = [0, 1]
        .into_iter()
        .map(|index| {
            vec![
                NotationOrigin {
                    definition: None,
                    path: vec![],
                },
                NotationOrigin {
                    definition: Some(DefinitionId::Dimension),
                    path: vec![
                        NotationPathPart::Field("body".into()),
                        NotationPathPart::Field("arguments".into()),
                        NotationPathPart::Argument(index),
                    ],
                },
                NotationOrigin {
                    definition: None,
                    path: vec![
                        NotationPathPart::Field("arguments".into()),
                        NotationPathPart::Argument(0),
                    ],
                },
            ]
        })
        .collect();
    assert_eq!(
        e.uses()
            .iter()
            .map(|u| u.origins().to_vec())
            .collect::<Vec<_>>(),
        expected
    );
    for usage in e.uses() {
        assert_eq!(usage.definition(), DefinitionId::CoefficientOne);
        assert_eq!(
            usage.status().reasons(),
            [PendingReason::FormulationProposed {
                object: NodeId::LangevinCoefficientOne
            }]
        );
    }
    Ok(())
}

#[test]
fn n02_constructor_body_expansion_boundary() -> TestResult {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let mut definitions = catalogue::catalogue();
    *body_mut(&mut definitions[5]) = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let body = primitive(
        PrimitiveId::Add,
        vec![
            ExprDraft::Slot { index: 0 },
            primitive(
                PrimitiveId::Add,
                vec![ExprDraft::Slot { index: 1 }, expanded_size(65_530)],
            ),
        ],
    );
    *body_mut(&mut definitions[6]) = body.clone();
    construct(&book, primitives::catalogue(), definitions.clone())?;
    *body_mut(&mut definitions[6]) = chain(1, body);
    let mut indices = vec![0, 1, 1];
    let mut size: usize = 65_530;
    while size != 1 {
        indices.push(0);
        size = if size >= 6 && size.is_multiple_of(2) {
            (size - 4) / 2
        } else {
            size - 1
        };
    }
    let mut path = vec![NotationPathPart::Field("body".into())];
    path.extend(argument_path(&indices));
    let location = NotationSite::Registry {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
        entry: RegistryEntry::Definition(DefinitionId::Product),
        index: 6,
        path,
        source_range: None,
    };
    let display_path = indices
        .iter()
        .map(|i| format!("Field(\"arguments\"), Argument({i})"))
        .collect::<Vec<_>>()
        .join(", ");
    exact(
        &construct(&book, primitives::catalogue(), definitions).unwrap_err(),
        &[diag(
            RuleId::M047,
            NotationStage::Dependencies,
            location,
            NotationFault::NotationLimit {
                resource: NotationResource::ExpansionNodes,
                limit: 65_536,
                actual: 65_537,
            },
        )],
        &format!(
            "M047 Dependencies at Registry {{ file: \"crates/ftoo_manifest/src/notation/catalogue.rs\", entry: Definition(Product), index: 6, path: [Field(\"body\"), {display_path}] }}: NotationLimit {{ resource: ExpansionNodes, limit: 65536, actual: 65537 }}\n"
        ),
    );
    notation(&book)?;
    Ok(())
}

#[test]
fn complete_seed_consumer() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let n = notation(&book).unwrap();
    assert_eq!(n.definitions().len(), 47);
    assert_eq!(n.primitives().len(), 37);
    assert_eq!(
        n.definitions()
            .iter()
            .filter(|d| d.status().is_some())
            .count(),
        45
    );
    for object in NodeId::ALL {
        assert_eq!(
            n.object_symbol(object).unwrap().status().unwrap().tier(),
            SymbolTier::Pending
        );
    }
    for d in n.definitions() {
        let args = (0..d.spec().signature().arity)
            .map(|_| literal("x"))
            .collect();
        check_expression(&n, call(d.spec().id(), args)).unwrap();
    }
    for p in n.primitives() {
        let args = (0..p.signature().arity).map(|_| literal("x")).collect();
        check_expression(&n, primitive(p.id(), args)).unwrap();
    }
}

#[test]
fn exact_fraction_arity_and_display() {
    let graph = seed().unwrap();
    let book = addressing(&graph).unwrap();
    let n = notation(&book).unwrap();
    let error =
        check_expression(&n, primitive(PrimitiveId::Fraction, vec![literal("1")])).unwrap_err();
    assert_eq!(
        error.diagnostics(),
        &[NotationDiagnostic {
            rule: RuleId::M047,
            stage: NotationStage::Ast,
            site: NotationSite::Expression {
                path: vec![NotationPathPart::Field("arguments".into())]
            },
            fault: NotationFault::CallArity {
                callee: Callee::Primitive(PrimitiveId::Fraction),
                expected: 2,
                actual: 1
            },
        }]
    );
    assert_eq!(
        error.to_string(),
        "M047 Ast at Expression { path: [Field(\"arguments\")] }: CallArity { callee: Primitive(Fraction), expected: 2, actual: 1 }\n"
    );
    check_expression(
        &n,
        primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
    )
    .unwrap();
}

#[test]
fn pending_text_parts_keep_every_template_and_blocker_spelling() {
    use crate::{Awaiting, Blocker, CapabilityId, NodeId, PendingReason};
    let object = NodeId::LangevinCoefficientOne;
    let mut cases = vec![
        (
            PendingReason::BackingUnbound { object },
            "backing object ",
            " is unbound",
        ),
        (
            PendingReason::BackingAbsent { object },
            "backing object ",
            " is absent",
        ),
        (
            PendingReason::FormulationProposed { object },
            "retained formulation ",
            "",
        ),
    ];
    for (blocker, after) in [
        (Blocker::AuditRunRequired, " awaits audit-run-required"),
        (
            Blocker::Awaiting(Awaiting::AuditBinding),
            " awaits audit-binding",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::ExactRationalLinearEquivalence,
            )),
            " awaits capability exact-rational-linear-equivalence",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(
                CapabilityId::FormalTruncatedPowerSeries,
            )),
            " awaits capability formal-truncated-power-series",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::SymbolicTrigonometry)),
            " awaits capability symbolic-trigonometry",
        ),
        (
            Blocker::Awaiting(Awaiting::Capability(CapabilityId::TrigonometricIntegration)),
            " awaits capability trigonometric-integration",
        ),
    ] {
        cases.push((
            PendingReason::ClaimDeferred { object, blocker },
            "claim evidence ",
            after,
        ));
    }
    for (reason, before, after) in cases {
        assert_eq!(
            reason.text_parts(),
            vec![
                crate::ReasonPart::Text(before.into()),
                crate::ReasonPart::Object {
                    role: crate::ReasonRole::Subject,
                    id: object
                },
                crate::ReasonPart::Text(after.into())
            ]
        );
        assert_eq!(
            reason.to_string(),
            format!("{before}def:langevin:coefficient-one{after}")
        );
    }
}
