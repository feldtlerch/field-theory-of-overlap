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
    assert_eq!(RULES[..38], previous[..38]);
    for index in 38..42 {
        let actual = &RULES[index];
        let prior = &previous[index];
        assert_eq!(
            (actual.number, actual.state, actual.deferred_reason),
            (prior.number, prior.state, prior.deferred_reason)
        );
    }
    assert_eq!(RULES[38].scope, previous[38].scope);
    assert_eq!(RULES[42..46], previous[42..46]);
    for index in 46..52 {
        let actual = &RULES[index];
        let prior = &previous[index];
        assert_eq!(
            (actual.number, actual.state, actual.deferred_reason),
            (prior.number, prior.state, prior.deferred_reason)
        );
        if index < 48 {
            assert_eq!(actual.scope, prior.scope);
        }
    }
    assert_eq!(
        RULES[52..57]
            .iter()
            .map(|r| (r.number, r.statement, r.scope, r.state, r.deferred_reason))
            .collect::<Vec<_>>(),
        [
            (
                RuleId::M053,
                "Every shared math source parses completely into the closed notation AST within the assembly math bounds, resolves every command with its declared syntax, and passes both rendering consumers with all authored use sites retained. Text bodies use only the bounded decoded plain-text grammar at their source ranges.",
                "All math entries of every welded unit consumed by assembly.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M054,
                "Every declared document has one ordered skeleton, every skeleton names a declared document, and every admitted unit occurs exactly once in its associated document's skeleton within the assembly bounds. A quotation child names an existing unit owned by another document with text in every language the quoting document requires, and adds no membership. A paragraph child is an owning prose position, not a heading, refrain or quotation. An econ child is one owning prose position with checked econ metadata; metadata and role must agree. An econ quotation is not admitted without its own consumer. An equation child is an owning unnumbered display position whose text selects one shared mathematical expression and a terminal full stop.",
                "The complete skeleton and admitted unit corpora, including heading positions.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M055,
                "Markdown and LaTeX emission preserves the checked block and occurrence inventory, uses the matching notation and addressing context, refuses every unavailable runtime tag at its site, and generated documents contribute no Markdown mint or documentary citation. A checked see-reference to an existing paper section, subsection or appendix heading retains its source label, checked ordinal, owning document and fragment in the shared plan; Markdown emission forms its public link, an unavailable target has one end-index entry in its owning reader document, and a changed link, locator or index entry refuses emission. The plan derives heading and admitted corpus result ordinals from captured master inclusion order and admitted source counter events; missing or ambiguous heading or result ownership and unsupported outline events refuse. Each paper pair compares the plan ordinal of each heading or keyed corpus result with the typesetter counter and the PDF destination. Bibliography citations resolve ordered keys against the stock paper bibliography where their entries are unchanged by the paper bibliography derivation, with the changed-key set pinned to that derivation, except for the compared parenthetical a:sencapability book admission. That admission consumes the Python producer's pinned derived bytes and compares its one shortauthor insertion with the stock row; other derived entries remain refused. TeX recovery preserves command form and ordered keys, Markdown and HTML recovery preserve printed labels and destinations, and each screen has one source-derived entry at the planned References position for each cited key; a citation without a screen, a missing, extra, reordered or changed citation link or entry, or a changed References position refuses. Each cited bibliography key matches exactly one article entry ignoring key case, except that a:sencapability matches exactly one book in both stock and derived inputs. Native textual use of that book refuses until independently admitted. An article admits only braced author, title, journal, volume, number, pages, year and doi fields, one literal author with an ASCII-letter surname, a four-digit year, and no TeX markup or font ligature spellings except a double hyphen in pages normalized to an en dash. The admitted book has exactly braced literal author, title, publisher, address and four-digit year fields; its derived row differs only by one nested-brace shortauthor insertion. Its decoded shortauthor has one or more ASCII uppercase-initial/period/space groups followed by the stock author's ASCII-letter surname. Other fields, markup, kinds, derivation changes or shapes refuse. A bibliography containing a percent sign or an @comment command refuses. For both admitted articles and the admitted book, every other regular stock entry must yield a braced author and four-digit year for the whole-bibliography surname-and-year collision check, or the citation refuses; equal surname and year refuse. Styled runs preserve ordered boundaries and child identities through TeX and Markdown recovery; HTML recovery compares decoded prose, ordered style boundaries, bibliography citation kinds, link labels and destinations, ordered end-matter entries, coverage and gaps against checked plan fields, and embedded font bytes, face declarations and licence texts against pinned loaded faces and retained licence files; HTML emits screen documents with whole pinned font files and bibliography citations linked to planned References entries; a paragraph, refrain or equation whose content has no HTML emitter becomes a plan-recorded, unlinked notice at that block position, recovered by position and wording, and a withheld quotation shows the notice without a link; an emitted quotation requires an emitted owner destination and refuses at the quotation site otherwise; release checks refuse HTML block gaps; missing screen plans and unrealizable coverage refuse. Structured native display bodies and heading content require an admitted emitter and recovery for their closed kinds and contexts; otherwise they refuse. Corpus mathematics may use a bounded observed construct-and-context MathML profile under ADR 053; unsupported mathematics remains a plan-recorded notice and release gap under M065. Existing exact-source admissions remain valid. The complete explainer flow is emitted in HTML only as the observed exact-source unnumbered MathML display with its terminal stop; other mathematical blocks without an admitted display profile remain plan-recorded gaps. Mathematical presentation derives from each admitted run's strict expanded source; recovery compares its source annotation with that run's planned source followed by the planned terminal stop for an unnumbered equation, its visible tree with the prepared presentation for that occurrence, and its face declaration and embedded bytes with the pinned mathematics face; an adapter fault, unsupported output for an admitted source, a missing or mismatched mathematics face file, or a mismatch of a recovered mathematical occurrence refuses at the mathematical use site. The stylesheet line, face declaration, and font and licence inventories refuse at the coverage site. Markdown emits styles only in observed faithful contexts and refuses unsupported kinds or contexts, a punctuation inner edge whose outer neighbour is present and is neither ASCII whitespace nor ASCII punctuation, and direct mathematics, style, tag or citation adjacency at the style source site. Generated coverage and pending object names require a unique live tag. Every reader-document documentary citation requires a unique live tag for its cited label and retains that label in its cite comment; literal semantic-label tokens are refused in reader text. The only paper-key exceptions are the exact planned emitted spelling spans of: (1) checked paper tag runs; (2) visible auxiliary handle placements; (3) econ header and footer runs; (4) checked corpus result-header keys; in every case their printed spelling equals their own source key. The same bytes elsewhere in text, titles, hrefs or ids remain refused. Markdown mathematics is emitted only in admitted observed contexts, with a body that satisfies the delimiter and citation boundary, and refuses an unobserved context, forbidden body character or direct tag/citation, mathematics or code-mention adjacency at the mathematical use site before sizing or emission. An unnumbered equation uses checked display-mode output, a double-dollar Markdown envelope with the dimensioned thick-space spelling corresponding to the source thick-space command, and recovery of its complete expression and terminal stop. A fragment target emits each non-heading position in its document as one TeX macro line: a plain-text refrain or quotation, or a paragraph containing admitted text, checked paper tags and visible auxiliary handle placements, checked see references, bibliography citations, plain-text emphasis runs, checked virtue references and mathematics invoking one of the compared inherited commands covered by the native sweep (comovfield, willvirtue or gensym), or an unnumbered equation containing one checked display expression and its terminal full stop; readout remains refused until its mathematics-alphabet context is decided; other bodies refuse. An econ position instead emits one complete econ environment on one physical line, generated from checked econ metadata and its admitted prose body. The econ form preserves the inherited environment's counter event, target mint, title reference, departure validation, five-field concordance insertion and closing footer; arbitrary environments and options refuse. Econ option values are emitted in braces; braces, brackets, backslash, percent, hash and line breaks in decoded option values refuse. A bounded corpus-slot is a distinct composition mode: one unique source marker is replaced by the canonical multiline body of a checked coarse unit. Its heading form contains source paragraph bodies with decoded text, strong and emphasis runs and named en-space intent, and unchanged uses of existing native paragraph units; in the explainer an unnumbered section heading of the admitted form may precede them. Each source paragraph body begins after a blank line or, as a heading-led prose tail, after an unchanged section or subsection and its own label and optional section grade; it ends before a blank line or a section or subsection. A run-in heading or printed section grade remains outside a heading-led prose tail as the unchanged prefix of the same TeX paragraph, and the emitted body begins with that tail's prose itself. A paragraph-only slot's replaced interval contains no heading, label, mint, checking effect or layout event; those around it remain outside and unchanged. A comment or a display, list, float, label or other command boundary does not by itself establish a paragraph boundary. Other continuations, nested bodies and paragraphs with layout events require their own admission. Its result form contains one checked registered insight, remark or definition header, its actual optional grade and admitted grade gap, and ordered independently admitted plain paragraph children. Complete source accounting, exactly-once existing inclusions, preserved mints and checking effects, and emission and recovery for every selected kind are required; other kinds and unresolved children refuse. Recovery compares the ordered children, decoded text, style and spacing intent, identities and targets with the checked plan. Paper retains en-space intent; both screens spell it as one ASCII space. This mode does not widen any fragment body grammar. Fragment paragraph target spellings preserve decoded characters and refuse merging hyphen/em-dash runs in plain text and emphasis or text requiring braced, backslash or multiline output. Corpus-slot paragraphs preserve the same decoded characters and distinctions through their admitted run grammar. For paper-context paragraph tags, an unstarred @tag occurrence must have bare print output; a located result refuses except for the unchanged sec:log:andreoni-joint/J:occ occurrence expressly retained by `rec:adr:048-reference-forms`. The exception is pinned by unit bytes, marker and occurrence index and remains located. Located-print admission for new conversions requires the separately admitted @tag-located form and occurrence-level print-mode evidence. That evidence joins the occurrence's shipped identity row to its own locator row at the fragment site and applies the existing auxiliary/PDF destination check. Reference-tag branches remain plain unlinked handles on screen; print mode is checked against shipped identity and locator evidence. Each declared fragment or corpus slot replaces exactly one unique matching marker line in its declared source; malformed, repeated, undeclared or absent markers, role disagreements and authored duplicates of a converted body refuse. Each child-use line in a corpus-slot artifact passes preparation's existing paragraph-role checks unchanged, including bindings and auxiliary-key uniqueness; the slot's paragraph bodies join the authored-duplicate check. An econ inclusion replaces its complete environment and all six authored options; any remaining authored copy or second target mint refuses. Preparation admits auxiliary handle commands only in the checked fragment body, and header mints only in a checked corpus result header, and refuses duplicate auxiliary keys across fragments and corpus headers and retained authored definitions of those keys in the declared paper sources. The final auxiliary evidence contains exactly one label definition and a matching PDF destination for each auxiliary key. Every generated artifact path is unique. An econ body admits only text and one parenthetical bibliography citation; other inline forms and a second citation refuse at their source site. A selected target or role without an emitter refuses; Markdown quotations require the checked owner destination, and LaTeX quotations remain refused. A virtue reference run retains its word, paper semantic binding, paper anchor, owning occurrence and reference mode through all three renderings and recovery; its screen link resolves through the existing availability inventory. Pair evidence compares its expected word and mode with the inherited command's shipped evidence and checks the auxiliary and PDF destination. Missing, extra, reordered or changed occurrences, a wrong owner, a native first-use occurrence in the reference-only admission, or a mismatched recovered screen link or word refuse.",
                "Every selected document target, its declared fragment and bounded corpus source slots and generated notation and addressing support, and every paper pair build.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M056,
                "Every generated LaTeX document and every derived notation sweep case except the readout inherited definition pass the pinned native document context with checked dependency integrity. Readout remains outside that native sweep until its mathematics-alphabet context is decided. Inherited paper commands retain checked source comparison and their screen forms retain the strict KaTeX sweep.",
                "Prepared LaTeX documents, their actual preamble and support files, including the compared inherited support files, and every derived notation sweep case except the readout inherited definition.",
                RuleState::Enforced,
                None
            ),
            (
                RuleId::M057,
                "Document generation and check share one preparation and writer, and the complete generated document and support inventory equals its regenerated bytes. That inventory includes generated/notation-status.json and generated/native-paper-support.tex.",
                "Generated Markdown, LaTeX and HTML documents, notation.tex, notation-status.json, native-paper-support.tex and addressing.tex.",
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
        (79, 68, 5, 6)
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
        (
            r"\mathrm{S}",
            0..7,
            MathSyntaxReason::ExpectedAtom,
            r"\mathrm",
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
        parse_assembly_math(&catalogue, "xω").unwrap_err(),
        (
            1..3,
            AssemblyInputFault::MathSyntax {
                reason: MathSyntaxReason::UnexpectedToken,
                token: "ω".into(),
                expected: vec!["end of expression".into()],
            }
        )
    );
    assert!(parse_assembly_math(&catalogue, "ω").is_ok());
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
