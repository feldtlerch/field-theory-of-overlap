use crate::*;
#[test]
fn native_lifecycle_fixture_tables_come_from_complete_valid_books()
-> Result<(), Box<dyn std::error::Error>> {
    use NodeId::{LangevinCoefficientOneValue as OneValue, LangevinCoefficientThree as Three};
    for name in [
        "tag-retirement",
        "identity-retirement",
        "target-absent",
        "selected-one",
        "empty",
    ] {
        let mut draft = seed_draft();
        if matches!(name, "identity-retirement" | "target-absent") {
            draft.nodes.retain(|node| node.id != Three);
            draft
                .edges
                .retain(|edge| edge.source != Three && edge.target != Three);
            draft.coverage.slice.clear();
        } else if name == "selected-one" {
            draft.coverage.slice = vec![OneValue];
        } else if name == "empty" {
            draft.coverage.slice.clear();
        }
        let graph = validate(draft)?;
        let mut history = crate::tag_history::seed_history();
        if matches!(name, "tag-retirement" | "identity-retirement") {
            history.push(HistoryEvent {
                ordinal: 9,
                change: if name == "tag-retirement" {
                    HistoryChange::RetireTag {
                        tag: TagId::LangevinCoefficientThree,
                    }
                } else {
                    HistoryChange::RetireIdentity { identity: Three }
                },
                authority_record: "rec:test:retirement".into(),
                reason: "Withdrawn for correction.".into(),
            });
        }
        let book = crate::tag_validation::construct(
            &graph,
            TagId::ALL.map(TagId::descriptor).into(),
            history,
        )?;
        let mut actual = Vec::new();
        for tag in book.live() {
            let key = tag.descriptor().spelling();
            actual.push(("live", key, book.resolve(key)?.label().to_owned()));
        }
        for (store, tags) in [
            (
                "retired",
                book.retired().map(RetiredEntry::tag).collect::<Vec<_>>(),
            ),
            (
                "outside",
                book.out_of_slice()
                    .map(|entry| entry.tag())
                    .collect::<Vec<_>>(),
            ),
        ] {
            for tag in tags {
                let key = tag.descriptor().spelling();
                actual.push((store, key, book.resolve(key).unwrap_err().to_string()));
            }
        }
        actual.sort_by_key(|(store, key, _)| {
            (
                match *store {
                    "live" => 0,
                    "retired" => 1,
                    _ => 2,
                },
                *key,
            )
        });
        let expected = include_str!("../../tests/fixtures/assembly/lifecycle.txt")
            .lines()
            .filter_map(|line| {
                let fields = line.splitn(4, '\t').collect::<Vec<_>>();
                (fields[0] == name).then(|| (fields[1], fields[2], fields[3].to_owned()))
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{name}");
        assert_eq!(
            book.resolve("c:01"),
            Err(TagResolutionFault::UnknownTag {
                input: "c:01".into()
            })
        );
        assert_eq!(
            book.resolve("c:01").unwrap_err().to_string(),
            "tag \"c:01\": unknown"
        );
    }
    Ok(())
}
#[test]
fn math_token_limit_preserves_a_complete_boundary_ast() -> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let atom = format!("{}x{}", "{".repeat(64), "}".repeat(64));
    let source = format!("-{}", atom.repeat(127));
    assert_eq!(source.len(), 16_384);
    let mut expected = call(PrimitiveId::Negate, vec![literal("x")]);
    for _ in 1..127 {
        expected = call(PrimitiveId::Juxtapose, vec![expected, literal("x")]);
    }
    let parsed = parse_assembly_math(&catalogue, &source).map_err(|e| format!("{e:?}"))?;
    assert_eq!(parsed.draft(), &expected);
    assert_eq!(parsed.commands(), []);
    assert_eq!(
        parse_assembly_math(&catalogue, &format!("{source}+")).unwrap_err(),
        (
            16_384..16_385,
            AssemblyInputFault::Limit {
                resource: AssemblyResource::MathTokens,
                limit: 16_384,
                actual: 16_385
            }
        )
    );
    Ok(())
}
#[test]
fn math_node_limit_has_a_balanced_complete_boundary_ast() -> Result<(), Box<dyn std::error::Error>>
{
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let mut level = (0..2048)
        .map(|_| ("x".to_owned(), literal("x")))
        .collect::<Vec<_>>();
    while level.len() > 1 {
        let mut children = level.into_iter();
        let mut next = Vec::new();
        while let Some((left_source, left)) = children.next() {
            let (right_source, right) = children.next().unwrap();
            next.push((
                format!("{{{left_source}+{right_source}}}"),
                call(PrimitiveId::Add, vec![left, right]),
            ));
        }
        level = next;
    }
    let (body, tree) = level.pop().unwrap();
    let source = format!("-{body}");
    let expected = call(PrimitiveId::Negate, vec![tree]);
    let parsed = parse_assembly_math(&catalogue, &source).map_err(|e| format!("{e:?}"))?;
    assert_eq!(parsed.draft(), &expected);
    assert_eq!(parsed.commands(), []);
    let start = source.len() + 1;
    assert_eq!(
        parse_assembly_math(&catalogue, &format!("{source}+x")).unwrap_err(),
        (
            start..start + 1,
            AssemblyInputFault::Limit {
                resource: AssemblyResource::MathNodes,
                limit: 4096,
                actual: 4097
            }
        )
    );
    Ok(())
}
#[test]
fn every_generated_signature_has_exact_command_and_argument_ranges()
-> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    for (source, id, command_end, arguments, argument_ranges) in [
        (
            r"\FtooQCoefficientOne{}",
            DefinitionId::CoefficientOne,
            20,
            Vec::new(),
            Vec::new(),
        ),
        (
            r"\FtooQCoefficientThree",
            DefinitionId::CoefficientThree,
            22,
            Vec::new(),
            Vec::new(),
        ),
        (
            r"\FtooOCoefficientOneValue",
            DefinitionId::CoefficientOneValue,
            25,
            Vec::new(),
            Vec::new(),
        ),
        (
            r"\FtooOCoefficientThreeValue{}",
            DefinitionId::CoefficientThreeValue,
            27,
            Vec::new(),
            Vec::new(),
        ),
        (
            r"\FtooAFirstCoefficient",
            DefinitionId::FirstCoefficient,
            22,
            Vec::new(),
            Vec::new(),
        ),
        (
            r"\FtooNDimension{x}",
            DefinitionId::Dimension,
            15,
            vec![literal("x")],
            std::iter::once(15..18).collect(),
        ),
        (
            r"\FtooNProduct{x}{y}",
            DefinitionId::Product,
            13,
            vec![literal("x"), literal("y")],
            vec![13..16, 16..19],
        ),
    ] {
        let parsed = parse_assembly_math(&catalogue, source).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            parsed.draft(),
            &ExprDraft::Call {
                callee: Callee::Definition(id),
                arguments
            }
        );
        assert_eq!(
            parsed.commands(),
            [MathCommandUse {
                definition: id,
                bytes: 0..command_end,
                arguments: argument_ranges
            }]
        );
    }
    let extra =
        parse_assembly_math(&catalogue, r"\FtooNDimension{x}{y}").map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        extra.draft(),
        &call(
            PrimitiveId::Juxtapose,
            vec![
                ExprDraft::Call {
                    callee: Callee::Definition(DefinitionId::Dimension),
                    arguments: vec![literal("x")]
                },
                literal("y")
            ]
        )
    );
    assert_eq!(
        parse_assembly_math(&catalogue, r"\FtooQCoefficientOneExtra{x}").unwrap_err(),
        (
            0..25,
            AssemblyInputFault::UnknownMathCommand {
                command: r"\FtooQCoefficientOneExtra".into()
            }
        )
    );
    Ok(())
}

#[test]
fn math_byte_and_group_limits_have_exact_boundary_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let source = format!("x{}", " ".repeat(262_143));
    assert_eq!(
        parse_assembly_math(&catalogue, &source)
            .map_err(|e| format!("{e:?}"))?
            .draft(),
        &literal("x")
    );
    assert_eq!(
        parse_assembly_math(&catalogue, &format!("{source} ")).unwrap_err(),
        (
            262_144..262_145,
            AssemblyInputFault::Limit {
                resource: AssemblyResource::MathBytes,
                limit: 262_144,
                actual: 262_145
            }
        )
    );
    let source = format!("{}x{}", "{".repeat(64), "}".repeat(64));
    assert_eq!(
        parse_assembly_math(&catalogue, &source)
            .map_err(|e| format!("{e:?}"))?
            .draft(),
        &literal("x")
    );
    let source = format!("{}x{}", "{".repeat(65), "}".repeat(65));
    assert_eq!(
        parse_assembly_math(&catalogue, &source).unwrap_err(),
        (
            64..65,
            AssemblyInputFault::Limit {
                resource: AssemblyResource::MathDepth,
                limit: 64,
                actual: 65
            }
        )
    );
    Ok(())
}
#[test]
fn assembly_register_preserves_prior_rules_and_reader_identity_amendments() {
    let previous: [RuleSpec; 52] = include!("../../tests/fixtures/assembly/rules_before.rs");
    assert_eq!(RULES[..48], previous[..48]);
    assert_eq!(RULES[49..52], previous[49..52]);
    assert_eq!(
        RULES[52..57]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M053,
                "Every shared math source parses completely into the closed notation AST within the assembly math bounds, resolves every command with its declared syntax, and passes both rendering consumers with all authored use sites retained.",
                "All math entries of every welded unit consumed by assembly.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M054,
                "Every declared document has one ordered skeleton, every skeleton names a declared document, and every admitted unit occurs exactly once in its associated document's skeleton within the assembly bounds.",
                "The complete skeleton and admitted unit corpora, including heading positions.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M055,
                "Document emission preserves the checked block and occurrence inventory, uses the matching notation and addressing context, refuses every unavailable runtime tag at its site, and generated documents contribute no Markdown mint or citation. Generated coverage and pending object names require a unique live tag. Every reader-document citation requires a unique live tag for its cited label and retains that label in its cite comment; literal semantic-label tokens are refused in reader text.",
                "Both document targets and their generated notation and addressing support.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M056,
                "Every generated LaTeX document and the complete derived notation sweep pass the pinned native document context with checked dependency integrity.",
                "Prepared LaTeX documents, their actual preamble and support files, and every native sweep case.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M057,
                "Document generation and check share one preparation and writer, and the complete generated document and support inventory equals its regenerated bytes.",
                "Generated Markdown and LaTeX documents, notation.tex and addressing.tex.",
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
        (62, 53, 4, 5)
    );
}
fn literal(value: &str) -> ExprDraft {
    ExprDraft::Literal {
        value: value.into(),
    }
}
fn call(id: PrimitiveId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Primitive(id),
        arguments,
    }
}
#[test]
fn parser_precedence_scripts_groups_and_unicode() -> Result<(), Box<dyn std::error::Error>> {
    use PrimitiveId::{
        Add, Alpha, Beta, Dimension, Equal, Fraction, Juxtapose, Multiply, Negate, Parentheses,
        Subscript, Superscript,
    };
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let cases = [
        ("x", literal("x")),
        ("12", literal("12")),
        (
            "x-y",
            call(Add, vec![literal("x"), call(Negate, vec![literal("y")])]),
        ),
        (
            "x+yz",
            call(
                Add,
                vec![
                    literal("x"),
                    call(Juxtapose, vec![literal("y"), literal("z")]),
                ],
            ),
        ),
        ("x_12", call(Subscript, vec![literal("x"), literal("12")])),
        (
            "x_1y",
            call(
                Juxtapose,
                vec![
                    call(Subscript, vec![literal("x"), literal("1")]),
                    literal("y"),
                ],
            ),
        ),
        (
            "x^2_1",
            call(
                Superscript,
                vec![
                    call(Subscript, vec![literal("x"), literal("1")]),
                    literal("2"),
                ],
            ),
        ),
        (
            "x_1^2",
            call(
                Superscript,
                vec![
                    call(Subscript, vec![literal("x"), literal("1")]),
                    literal("2"),
                ],
            ),
        ),
        (
            "(x=y)",
            call(
                Parentheses,
                vec![call(Equal, vec![literal("x"), literal("y")])],
            ),
        ),
        ("[x]", call(Dimension, vec![literal("x")])),
        ("{x}", literal("x")),
        (
            "α⋅β",
            call(Multiply, vec![call(Alpha, vec![]), call(Beta, vec![])]),
        ),
        ("−x", call(Negate, vec![literal("x")])),
        (
            r"\alpha{}\,\beta{}",
            call(Juxtapose, vec![call(Alpha, vec![]), call(Beta, vec![])]),
        ),
        (
            r"\frac{1}{3}",
            call(Fraction, vec![literal("1"), literal("3")]),
        ),
    ];
    for (source, expected) in cases {
        let parsed = parse_assembly_math(&catalogue, source).map_err(|e| format!("{e:?}"))?;
        assert_eq!(parsed.draft(), &expected, "{source}");
    }
    Ok(())
}
#[test]
fn parser_refusals_retain_full_first_decisive_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    for (source, bytes, reason, token, expected) in [
        (
            "",
            0..0,
            MathSyntaxReason::ExpectedAtom,
            "",
            vec!["letter", "digits", "command", "{", "(", "["],
        ),
        ("{}", 1..2, MathSyntaxReason::EmptyGroup, "}", vec!["atom"]),
        ("(x", 2..2, MathSyntaxReason::UnclosedGroup, "", vec![")"]),
        (
            "(x]",
            2..3,
            MathSyntaxReason::MismatchedCloser,
            "]",
            vec![")"],
        ),
        (
            "x=y=z",
            3..4,
            MathSyntaxReason::ChainedEquality,
            "=",
            vec!["end of expression"],
        ),
        (
            "x_1_2",
            3..4,
            MathSyntaxReason::RepeatedSubscript,
            "_",
            vec!["atom", "operator", "end of expression"],
        ),
        (
            "x^1^2",
            3..4,
            MathSyntaxReason::RepeatedSuperscript,
            "^",
            vec!["atom", "operator", "end of expression"],
        ),
        (
            "+x",
            0..1,
            MathSyntaxReason::ExpectedAtom,
            "+",
            vec!["letter", "digits", "command", "{", "(", "["],
        ),
    ] {
        assert_eq!(
            parse_assembly_math(&catalogue, source).unwrap_err(),
            (
                bytes,
                AssemblyInputFault::MathSyntax {
                    reason,
                    token: token.into(),
                    expected: expected.into_iter().map(str::to_owned).collect()
                }
            )
        );
    }
    assert_eq!(
        parse_assembly_math(&catalogue, r"\frac{1}x").unwrap_err(),
        (
            8..9,
            AssemblyInputFault::MathArguments {
                command: r"\frac".into(),
                expected: 2,
                actual: 1,
                argument: 1
            }
        )
    );
    Ok(())
}
#[test]
fn unknown_command_is_one_complete_word() -> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    assert_eq!(
        parse_assembly_math(&catalogue, r"\NoSuch{x}").unwrap_err(),
        (
            0..7,
            AssemblyInputFault::UnknownMathCommand {
                command: r"\NoSuch".into()
            }
        )
    );
    Ok(())
}
#[test]
fn canonical_equation_retains_command_site() -> Result<(), Box<dyn std::error::Error>> {
    let graph = seed()?;
    let book = addressing(&graph)?;
    let catalogue = notation(&book)?;
    let parsed = parse_assembly_math(&catalogue, r"\FtooQCoefficientOne{} = \frac{1}{3}")
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        parsed.commands(),
        [MathCommandUse {
            definition: DefinitionId::CoefficientOne,
            bytes: 0..20,
            arguments: Vec::new()
        }]
    );
    assert_eq!(
        parsed.draft(),
        &call(
            PrimitiveId::Equal,
            vec![
                ExprDraft::Call {
                    callee: Callee::Definition(DefinitionId::CoefficientOne),
                    arguments: Vec::new()
                },
                call(PrimitiveId::Fraction, vec![literal("1"), literal("3")])
            ]
        )
    );
    assert_eq!(
        parsed.ranges(),
        [
            MathSourceRange {
                path: Vec::new(),
                bytes: 0..36
            },
            MathSourceRange {
                path: vec![0],
                bytes: 0..22
            },
            MathSourceRange {
                path: vec![1],
                bytes: 25..36
            },
            MathSourceRange {
                path: vec![1, 0],
                bytes: 30..33
            },
            MathSourceRange {
                path: vec![1, 1],
                bytes: 33..36
            },
        ]
    );
    let checked = check_expression(&catalogue, parsed.into_draft())?;
    assert_eq!(
        checked
            .uses()
            .iter()
            .map(SymbolUse::definition)
            .collect::<Vec<_>>(),
        [DefinitionId::CoefficientOne]
    );
    Ok(())
}
