#![allow(clippy::too_many_lines)] // Complete citation records and restoration evidence stay together.
use super::Coverage;
use super::*;
use crate::documents::{input, latex, markdown, registry};
use ftoo_manifest::*;
const ANDREONI: &str = "@article{a:andreoni, author={Andreoni, James}, title={Impure Altruism and Donations to Public Goods: A Theory of Warm-Glow Giving}, journal={The Economic Journal}, volume={100}, number={401}, pages={464--477}, year={1990}, doi={10.2307/2234133}}";

#[test]
fn bibliography_article_shape_and_derived_keys_refuse() {
    let site = input::site("fixture.toml");
    for altered in [
        ANDREONI.replace("Andreoni, James", "Andreoni, James AND Payne, Abigail"),
        ANDREONI.replace("Andreoni, James", "Andreoni, James and Payne, Abigail"),
        ANDREONI.replace("Andreoni, James", "{Andreoni}, James"),
        ANDREONI.replace("year={1990},", ""),
        ANDREONI.replace("year={1990}", "year=someyear"),
        ANDREONI.replace("year={1990}", "year=\"1990\""),
        ANDREONI.replace("year={1990}", "year=1990"),
        ANDREONI.replace("year={1990}", "year={1990}, crossref={other}"),
        ANDREONI.replace("year={1990}", "year={1990}, shorthand={A90}"),
        ANDREONI.replace("year={1990}", "year={1990}, shortauthor={J. Andreoni}"),
        ANDREONI.replace("Warm-Glow", "Warm~Glow"),
        ANDREONI.replace("Warm-Glow", "Warm--Glow"),
        ANDREONI.replace("Warm-Glow", "``Warm-Glow''"),
        ANDREONI.replace("Warm-Glow", "Warm\\-Glow"),
        ANDREONI.replace("Warm-Glow", "Warm'Glow"),
        ANDREONI.replace("Warm-Glow", "Warm\"Glow"),
        ANDREONI.replace("Warm-Glow", "Warm<<Glow"),
        ANDREONI.replace("Warm-Glow", "Warm>>Glow"),
        ANDREONI.replace("Warm-Glow", "Warm,,Glow"),
        ANDREONI.replace("464--477", "464''477"),
        ANDREONI.replace("464--477", "464---477"),
        ANDREONI.replace("{Andreoni, James}", "Andreoni, James"),
        format!("@book{{a:andreoni, author={{Other, A}}, year={{1990}}}}\n{ANDREONI}"),
        format!("@book{{A:ANDREONI, author={{Other, A}}, year={{1990}}}}\n{ANDREONI}"),
        format!("% {ANDREONI}\n{ANDREONI}"),
        format!("@comment{{{ANDREONI}}}\n{ANDREONI}"),
        format!("{ANDREONI}\n@book{{a:other, author=Other, year={{1991}}}}"),
        format!("{ANDREONI}\n@book{{a:other, author={{{{Other}}}}, year={{1991}}}}"),
    ] {
        assert!(
            super::super::bibliography::resolve(&altered, "a:andreoni", &site).is_err(),
            "{altered}"
        );
    }
    for key in super::super::bibliography::DERIVED_KEYS
        .into_iter()
        .filter(|key| *key != "a:sencapability")
    {
        let error = super::super::bibliography::resolve(ANDREONI, key, &site).unwrap_err();
        assert!(format!("{error:?}").contains("entry changed by bibliography derivation"));
    }
    let book = super::super::bibliography::resolve(ANDREONI, "a:sencapability", &site).unwrap_err();
    assert!(format!("{book:?}").contains("one bibliography entry of the admitted kind"));
    let error = super::super::bibliography::resolve(ANDREONI, "a:VMF", &site).unwrap_err();
    assert!(format!("{error:?}").contains("entry changed by bibliography derivation"));
    let collision =
        format!("{ANDREONI}\n@book{{a:other, author={{Andreoni, Another}}, year={{1990}}}}");
    assert!(super::super::bibliography::resolve(&collision, "a:andreoni", &site).is_err());
    let upper = ANDREONI.replace("a:andreoni", "A:ANDREONI");
    assert!(super::super::bibliography::resolve(&upper, "a:andreoni", &site).is_ok());
}

#[test]
fn bibliography_derived_key_refusals_follow_the_paper_derivation() {
    let route = include_str!("../../../../../tools/paper/bibroute.py");
    let bibliography = include_str!("../../../../../documents/paper/references.bib");
    let names = route
        .split_once("names = {")
        .unwrap()
        .1
        .split_once("}\n    for key")
        .unwrap()
        .0;
    let mut changed: BTreeSet<&str> = names
        .split('\'')
        .filter(|part| part.starts_with("a:"))
        .collect();
    for needle in [r"Fran{\c{c}}ois", "Multi-Dimensioned"] {
        let before = &bibliography[..bibliography.find(needle).unwrap()];
        let entry = before.rsplit_once('@').unwrap().1;
        let key = entry.split_once('{').unwrap().1.split_once(',').unwrap().0;
        changed.insert(key);
    }
    assert_eq!(route.matches("bib = bib.replace(").count(), 2);
    assert_eq!(route.matches("re.subn(").count(), 1);
    assert_eq!(
        changed,
        super::super::bibliography::DERIVED_KEYS
            .into_iter()
            .collect()
    );
}

#[test]
fn bibliography_main_document_requires_screen_at_citation_site() {
    let site = input::site("fixture.toml");
    let entry = super::super::bibliography::resolve(ANDREONI, "a:andreoni", &site).unwrap();
    let mut plan = DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage declaration", "Declared slice: (empty)"),
        blocks: vec![Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:citation".into(),
            depth: None,
            quotation: false,
            paragraph: true,
            equation: false,
            econ: None,
            site: site.clone(),
            runs: vec![Inline::BibliographyCitation {
                kind: BibliographyCitationKind::Parenthetical,
                entries: vec![entry],
                label: "(Andreoni 1990)".into(),
                site: site.clone(),
            }],
        }],
        screen: None,
    };
    let error = require_bibliography_screen(&plan).unwrap_err();
    assert!(format!("{error:?}").contains("screen bibliography consumer"));
    assert!(format!("{error:?}").contains("fixture.toml"));
    plan.screen = Some(ScreenPlan {
        gaps: vec![],
        refrains: vec![],
        index: vec![],
        html_gaps: std::collections::BTreeMap::new(),
        end_matter: vec![],
    });
    assert!(require_bibliography_screen(&plan).is_ok());
    plan.screen
        .as_mut()
        .unwrap()
        .index
        .push(super::super::destinations::IndexEntry {
            document: super::super::destinations::Reading::Paper,
            language: "en".into(),
            fragment: "method-self-limit".into(),
            title: "The method’s self-limit".into(),
            locator: "appendix G".into(),
            message: "Not yet available in this rendering.",
        });
    let index = plan.screen.as_ref().unwrap().index.clone();
    let references = super::super::bibliography::entries(&plan);
    plan.screen.as_mut().unwrap().end_matter = vec![
        ScreenEndMatter::Unavailable(index),
        ScreenEndMatter::References(references),
    ];
    assert!(matches!(
        end_matter(&plan).as_slice(),
        [EndMatter::Unavailable(_), EndMatter::References(_)]
    ));
    plan.screen.as_mut().unwrap().index[0].fragment = "bibliography-other".into();
    let ScreenEndMatter::Unavailable(entries) = &mut plan.screen.as_mut().unwrap().end_matter[0]
    else {
        panic!("planned unavailable-content index")
    };
    entries[0].fragment = "bibliography-other".into();
    let error = check_public_anchors(&plan).unwrap_err();
    assert!(format!("{error:?}").contains("DuplicatePublicAnchor"));
}

#[test]
fn bibliography_native_options_match_paper_preamble() {
    let source = include_str!("../../../../../documents/paper/main.tex");
    let end = source.find("]{biblatex}").unwrap();
    let start = source[..end].rfind("\\usepackage[").unwrap() + "\\usepackage[".len();
    let options = &source[start..end];
    let normalized = options.split_whitespace().collect::<String>();
    assert_eq!(normalized, latex::BIBLATEX_OPTIONS);
    let execute = source
        .split_once("\\ExecuteBibliographyOptions{")
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0;
    assert_eq!(
        execute.split_whitespace().collect::<String>(),
        latex::BIBLATEX_EXECUTE_OPTIONS
    );
}

#[test]
fn bibliography_paragraph_emphasis_refuses_doubled_dash() {
    let site = input::site("fixture.toml");
    let block = Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: "sec:fixture:citation".into(),
        depth: None,
        quotation: false,
        paragraph: true,
        equation: false,
        econ: None,
        site: site.clone(),
        runs: vec![Inline::Styled {
            kind: StyleKind::Emphasis,
            children: vec![Inline::Text("a--b".into())],
            child_sites: vec![site.clone()],
            site: site.clone(),
        }],
    };
    let error = super::super::paragraph_fragment_line(&block).unwrap_err();
    assert!(format!("{error:?}").contains("adjacent hyphen or em-dash marks"));
}

#[test]
fn bibliography_source_and_screen_entry_refuse_missing_or_changed_work() {
    let site = input::site("fixture.toml");
    let entry = super::super::bibliography::resolve(ANDREONI, "a:andreoni", &site).unwrap();
    assert_eq!(entry.label, "Andreoni 1990");
    assert_eq!(entry.anchor, "bibliography-andreoni-1990");
    assert!(super::super::bibliography::resolve(ANDREONI, "a:missing", &site).is_err());
    let row = format!(
        "<a id=\"{}\"></a>{}<!-- bibentry {} -->\n",
        entry.anchor,
        markdown::text(&super::super::bibliography::screen_text(&entry)),
        entry.key
    );
    assert!(super::super::bibliography::recover_screen(
        &row,
        std::slice::from_ref(&entry)
    ));
    assert!(!super::super::bibliography::recover_screen(
        "",
        std::slice::from_ref(&entry)
    ));
    assert!(!super::super::bibliography::recover_screen(&row, &[]));
    assert!(!super::super::bibliography::recover_screen(
        &row.replace("Andreoni, James", "Andreoni, John"),
        std::slice::from_ref(&entry)
    ));
    assert!(!super::super::bibliography::recover_screen(
        &row.replace("bibliography-andreoni-1990", "bibliography-andreoni-2000"),
        std::slice::from_ref(&entry)
    ));
    let changed = super::super::bibliography::resolve(
        &ANDREONI.replace("year={1990}", "year={2000}"),
        "a:andreoni",
        &site,
    )
    .unwrap();
    assert_ne!(changed.label, entry.label);
    assert!(!super::super::bibliography::recover_screen(
        &row,
        &[changed]
    ));
}

#[test]
fn bibliography_citation_recovery_preserves_kind_order_and_visible_label() {
    let clean = "(<a href=\"#bibliography-andreoni-1990\">Andreoni 1990</a>; <a href=\"#bibliography-andreoni-1990\">Andreoni 1990</a>)<!-- bibcite a:andreoni,a:other -->";
    let expected = recover_bibliography_markdown(clean).unwrap().0;
    assert!(
        matches!(&expected, Recovered::BibliographyCitation { kind: BibliographyCitationKind::Parenthetical, keys, label, .. } if keys == &["a:andreoni", "a:other"] && label == "(Andreoni 1990; Andreoni 1990)")
    );
    for altered in [
        clean.replace("a:andreoni,a:other", "a:other,a:andreoni"),
        clean.replace("Andreoni 1990</a>", "Andreoni 2000</a>"),
        clean.replace("bibliography-andreoni-1990", "bibliography-andreoni-2000"),
        clean.replacen("(<a", "<a", 1).replacen("</a>)", "</a>", 1),
    ] {
        assert_ne!(recover_bibliography_markdown(&altered).unwrap().0, expected);
    }
    assert_eq!(
        recover_latex(r"\parencite{a:andreoni,a:other}"),
        Some(vec![Recovered::BibliographyCitation {
            kind: BibliographyCitationKind::Parenthetical,
            keys: vec!["a:andreoni".into(), "a:other".into()],
            label: String::new(),
            anchors: vec![]
        }])
    );
    assert!(matches!(recover_latex(r"\textcite{a:andreoni}"),
        Some(runs) if matches!(&runs[0], Recovered::BibliographyCitation { kind: BibliographyCitationKind::Textual, .. })));
}

#[test]
fn bibliography_latex_document_recovers_the_source_command_and_entry() {
    let site = input::site("fixture.toml");
    let entry = super::super::bibliography::resolve(ANDREONI, "a:andreoni", &site).unwrap();
    let plan = DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage declaration", "Declared slice: (empty)"),
        blocks: vec![Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:citation".into(),
            depth: None,
            quotation: false,
            paragraph: true,
            equation: false,
            econ: None,
            site: site.clone(),
            runs: vec![
                Inline::Text("See ".into()),
                Inline::BibliographyCitation {
                    kind: BibliographyCitationKind::Parenthetical,
                    entries: vec![entry.clone()],
                    label: "(Andreoni 1990)".into(),
                    site,
                },
                Inline::Text(".".into()),
            ],
        }],
        screen: None,
    };
    let source = latex::render(&plan);
    assert!(source.starts_with("\\begin{filecontents*}{references.bib}\n"));
    assert!(source.contains(r"See \parencite{a:andreoni}."));
    assert!(source.ends_with("\\printbibliography\n\\end{document}\n"));
    check_emission(&plan, &source, DocumentTarget::Latex).unwrap();
    assert!(
        check_emission(
            &plan,
            &source.replace(r"\parencite{a:andreoni}", r"\textcite{a:andreoni}"),
            DocumentTarget::Latex
        )
        .is_err()
    );
    assert!(
        check_emission(
            &plan,
            &source.replace(r"\parencite{a:andreoni}", r"\parencite{a:unknown}"),
            DocumentTarget::Latex
        )
        .is_err()
    );
    assert!(
        check_emission(
            &plan,
            &source.replace("\\printbibliography\n", ""),
            DocumentTarget::Latex
        )
        .is_err()
    );
    assert!(
        check_emission(
            &plan,
            &source.replace("year={1990}", "year={2000}"),
            DocumentTarget::Latex
        )
        .is_err()
    );
}

#[test]
fn support_required_bibliography_latex_context_typesets() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = super::super::NativeSupport::from_environment(&root)
        .map_err(|error| format!("{error:?}"))?;
    support.set_inherited(
        &std::fs::read(root.join(super::super::inherited_support::PATH))?,
        &std::fs::read(root.join(super::super::inherited_support::TYPED_PATH))?,
    );
    let lock = super::super::native_support::lock(&root).map_err(|error| format!("{error:?}"))?;
    let site = input::site("fixture.toml");
    let entry = super::super::bibliography::resolve(ANDREONI, "a:andreoni", &site)?;
    let plan = DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage declaration", "Declared slice: (empty)"),
        blocks: vec![Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:citation".into(),
            depth: None,
            quotation: false,
            paragraph: true,
            equation: false,
            econ: None,
            site: site.clone(),
            runs: vec![
                Inline::Text("See ".into()),
                Inline::BibliographyCitation {
                    kind: BibliographyCitationKind::Parenthetical,
                    entries: vec![entry],
                    label: "(Andreoni 1990)".into(),
                    site,
                },
                Inline::Text(".".into()),
            ],
        }],
        screen: None,
    };
    let source = latex::render(&plan);
    support.scope(&lock, || {
        super::super::native::consume_plan(
            &support,
            &lock,
            &source,
            "",
            "\\newcommand{\\FtdocContext}[2]{}\n",
            &plan,
        )
    })?;
    let undefined = source.replace(r"\parencite{a:andreoni}", r"\parencite{a:missing}");
    let error = support
        .scope(&lock, || {
            super::super::native::consume_plan(
                &support,
                &lock,
                &undefined,
                "",
                "\\newcommand{\\FtdocContext}[2]{}\n",
                &plan,
            )
        })
        .unwrap_err();
    let DocumentError::Assembly(errors) = error else {
        panic!("undefined citation must be a native assembly refusal");
    };
    assert!(errors.diagnostics().iter().any(|diagnostic| {
        let AssemblyFault::NativeRejected { status, log, .. } = &diagnostic.fault else {
            return false;
        };
        *status == Some(0)
            && String::from_utf8_lossy(log).contains("Citation 'a:missing'")
            && String::from_utf8_lossy(log).contains("undefined")
    }));
    Ok(())
}
const LABEL: &str = "dec:fixture:source";
const UNIT: &str = "sec:fixture:reader";
fn context(
    text: &str,
    ledger: &str,
    check: impl FnOnce(&AddressBook<'_>, &CheckedUnit, &WeldedUnits<'_, '_, '_, '_, '_, '_>, Block),
) {
    let graph = seed().unwrap();
    context_graph(&graph, text, ledger, check);
}
fn context_graph(
    graph: &ValidatedManifest,
    text: &str,
    ledger: &str,
    check: impl FnOnce(&AddressBook<'_>, &CheckedUnit, &WeldedUnits<'_, '_, '_, '_, '_, '_>, Block),
) {
    let site = input::site("fixture.toml");
    let corpus = literal_corpus(text, BTreeMap::new());
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"fixture.md".into(),bytes:"# Decision · `dec:fixture:source`\n# Authority · `dec:fixture:authority`\n# Kernel · `proposal:manifest:kernel-bite`\n# Lifecycle · `proposal:manifest:addressing-lifecycle-bite`\n".as_bytes().to_vec()}]).unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let events = registry::load(&crate::UnitSource {
        file: "fixture-registry.toml".into(),
        bytes: ledger.as_bytes().to_vec(),
    })
    .unwrap();
    let book = addressing(graph)
        .unwrap()
        .with_documentary(&joint, events)
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_joint_labels(&resolved, &joint).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let unit = &corpus.units()[0];
    let block = Block {
        pending_names: BTreeMap::new(),
        displays: BTreeMap::new(),
        unit: UNIT.into(),
        depth: None,
        quotation: false,
        paragraph: false,
        equation: false,
        econ: None,
        site: site.clone(),
        runs: runs(0, unit, "en", &welded, &BTreeMap::new(), "", None).unwrap(),
    };
    check(&book, unit, &welded, block);
}
fn literal_corpus(text: &str, math: BTreeMap<String, LocatedEntryDraft>) -> LanguageCheckedUnits {
    let site = input::site("fixture.toml");
    let value = |s: &str| LocatedString {
        value: s.into(),
        site: site.clone(),
    };
    admit_unit_drafts(
        DocumentDraft {
            documents_site: site.clone(),
            documents: BTreeMap::from([(
                "main".into(),
                DocumentEntryDraft {
                    key_site: site.clone(),
                    table_site: site.clone(),
                    languages_site: site.clone(),
                    required_languages: vec![value("en")],
                },
            )]),
        },
        vec![UnitDraft {
            econ: None,
            id: value(UNIT),
            document: value("main"),
            table_site: site.clone(),
            math_site: site.clone(),
            texts_site: site.clone(),
            documentary_targets: BTreeMap::new(),
            math,
            texts: BTreeMap::from([(
                "en".into(),
                LocatedEntryDraft {
                    key_site: site.clone(),
                    value: value(text),
                },
            )]),
        }],
    )
    .unwrap()
}
fn ledger(handle: &str, label: &str) -> String {
    format!(
        "schema = 1\n[[events]]\nordinal = 1\nchange = \"admit\"\nhandle = {handle:?}\nlabel = {label:?}\nauthority-record = \"dec:fixture:authority\"\nreason = \"Fixture admission.\"\n"
    )
}

#[test]
fn theory_citation_outside_mapping_preserves_mint_and_restores() {
    let label = "claim:langevin:coefficient-three-value";
    let text = format!("See `{label}`.");
    let mut draft = seed_draft();
    draft.coverage.slice = vec![NodeId::LangevinCoefficientOneValue];
    let graph = validate(draft).unwrap();
    context_graph(
        &graph,
        &text,
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site: UnitSite {
                                    decoded: Some(4..44),
                                    ..input::site("fixture.toml")
                                },
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    anchor: SourceAnchor::Unavailable,
                                    ..input::site("crates/ftoo_manifest/src/identity.rs")
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Theory(NodeId::LangevinCoefficientThreeValue),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: vec![ReaderTagCandidate {
                            handle: "v:3".into(),
                            outcome: Err(TagResolutionFault::OutOfSliceTag {
                                input: "v:3".into(),
                                tag: TagId::LangevinCoefficientThreeValue,
                                target: NodeId::LangevinCoefficientThreeValue,
                                cause: OutOfSliceCause::ClaimNotSelected,
                            }),
                        }],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        &text,
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            assert_eq!(
                check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                Ok(())
            );
            assert_eq!(
                check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
                Ok(())
            );
        },
    );
}

#[test]
fn theory_citation_retirement_mapping_keeps_complete_rejected_authority_evidence() {
    // Production projection admission rejects this extra authority act. Its
    // immutable diagnostic supplies an event for mapping, never a checked book.
    let original = include_bytes!("../../../../../generated/manifest.json");
    let mut bytes: serde_json::Value = serde_json::from_slice(original).unwrap();
    bytes["history"].as_array_mut().unwrap().push(serde_json::json!({
        "ordinal":9,"authority-record":"proposal:manifest:kernel-bite","reason":"Fixture retirement.",
        "change":{"kind":"retire-tag","tag":"c:1"},
    }));
    bytes["stores"]["live"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["tag"] != "c:1");
    bytes["stores"]["retired"] =
        serde_json::json!([{"tag":"c:1","cause":{"kind":"tag-retirement","event":9}}]);
    let refused = load_projection(&serde_json::to_vec(&bytes).unwrap()).unwrap_err();
    let ProjectionError::Assertion(ref errors) = refused else {
        panic!("authority refusal: {refused:?}")
    };
    let ProjectionFault::HistoryMismatch {
        actual: Some(value),
        ..
    } = &errors[0].fault
    else {
        panic!("history evidence")
    };
    let ProjectionValue::History(event) = value.as_ref() else {
        panic!("event evidence")
    };
    assert_eq!(
        (
            event.ordinal(),
            event.authority_record(),
            event.reason(),
            event.change()
        ),
        (
            9,
            "proposal:manifest:kernel-bite",
            "Fixture retirement.",
            &HistoryChange::RetireTag {
                tag: TagId::LangevinCoefficientOne
            }
        )
    );
    let expected_projection = ProjectionError::Assertion(vec![ProjectionDiagnostic {
        rule: RuleId::M037,
        path: "$.history[8]".into(),
        position: Some(8),
        fault: ProjectionFault::HistoryMismatch {
            expected: None,
            actual: Some(Box::new(ProjectionValue::History(event.clone()))),
        },
    }]);
    assert_eq!(refused, expected_projection);
    assert_eq!(refused.to_string(), expected_projection.to_string());
    let cause = RetirementCause::TagRetirement(event.clone());
    context(
        "See `def:langevin:coefficient-one`.",
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let Inline::Citation { identity, site, .. } = &block.runs[1] else {
                panic!("citation")
            };
            let origins = vec![
                SourceOrigin {
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: UnitSite {
                        decoded: Some(4..34),
                        ..input::site("fixture.toml")
                    },
                },
                SourceOrigin {
                    unit: None,
                    language: None,
                    target: None,
                    site: UnitSite {
                        anchor: SourceAnchor::Unavailable,
                        ..input::site("crates/ftoo_manifest/src/identity.rs")
                    },
                },
            ];
            assert_eq!(citation_origins(identity, site, UNIT, "en"), origins);
            let candidates = vec![ReaderTagCandidate {
                handle: "c:1".into(),
                outcome: Err(TagResolutionFault::RetiredTag {
                    input: "c:1".into(),
                    tag: TagId::LangevinCoefficientOne,
                    target: NodeId::LangevinCoefficientOne,
                    cause,
                }),
            }];
            let position = ReaderIdentityPosition::Citation {
                unit: UNIT.into(),
                span: 0,
                target: None,
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: position.clone(),
                        origins: origins.clone(),
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Theory(NodeId::LangevinCoefficientOne),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: candidates.clone(),
                    },
                }])
                .unwrap(),
            );
            let actual = reader_failure(
                ReaderIdentityFault {
                    target: AddressTarget::Theory(NodeId::LangevinCoefficientOne),
                    reason: ReaderIdentityReason::NoLiveTag,
                    candidates,
                },
                "main",
                "en",
                Some(UNIT),
                None,
                position,
                origins,
            );
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
        },
    );
    let restored = load_projection(original).unwrap();
    assert_eq!(
        restored.addressing().unwrap().resolve("c:1"),
        Ok(AddressTarget::Theory(NodeId::LangevinCoefficientOne))
    );
}

pub(in crate::documents) fn support_required_literal_build_controls(
    notation: &crate::RenderableNotation<'_, '_, '_>,
) {
    for (text, literal) in [
        ("Literal dec:fixture:source.", true),
        ("Literal ``dec:fixture:source``.", true),
        ("See `dec:fixture:source` and @tag{c:1}.", false),
    ] {
        context(text, &ledger("record", LABEL), |_, unit, welded, _| {
            let corpus = welded.integrated().resolved().corpus();
            let site = input::site("fixture.toml");
            let located = |value: &str| LocatedString {
                value: value.into(),
                site: site.clone(),
            };
            let metadata = BTreeMap::from([(UNIT.into(), site.clone())]);
            let skeletons = check_skeletons(
                corpus,
                &[SkeletonDraft {
                    document: located("main"),
                    sections_site: site.clone(),
                    sections: vec![SkeletonSection {
                        heading: located(UNIT),
                        children: vec![],
                    }],
                }],
                &metadata,
                &metadata,
                &BTreeMap::from([("main".into(), site.clone())]),
            )
            .unwrap();
            let displays = check_displays(&[], &skeletons, welded).unwrap();
            let mut errors = vec![];
            check_reader_literals(unit, "en", &displays, &mut errors);
            if literal {
                let mention = text.contains("``");
                let expected = AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::Source(UnitSite {
                        decoded: mention.then_some(8..30),
                        ..site.clone()
                    }),
                    fault: AssemblyFault::ReaderLabelLiteral {
                        value: LABEL.into(),
                        bytes: if mention { 10..28 } else { 8..26 },
                    },
                };
                assert_eq!(errors, vec![expected.clone()]);
                let expected = DocumentError::Assembly(
                    AssemblyErrors::from_diagnostics(vec![expected]).unwrap(),
                );
                let actual = build(
                    &skeletons,
                    welded,
                    notation,
                    &metadata,
                    &displays,
                    "",
                    None,
                    &BTreeMap::new(),
                    &BTreeSet::new(),
                )
                .err()
                .unwrap();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
            } else {
                assert_eq!(errors, vec![]);
                let plans = build(
                    &skeletons,
                    welded,
                    notation,
                    &metadata,
                    &displays,
                    "",
                    None,
                    &BTreeMap::new(),
                    &BTreeSet::new(),
                )
                .unwrap();
                for plan in plans {
                    assert_eq!(
                        check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                        Ok(())
                    );
                    assert_eq!(
                        check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
                        Ok(())
                    );
                }
            }
        });
    }
}
fn plan(block: Block) -> DocumentPlan {
    DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: Coverage::fixture("Coverage.", "Slice."),
        screen: None,
        blocks: vec![block],
    }
}

#[test]
fn tag_comment_cannot_become_a_citation_even_with_the_right_handle() {
    context(
        "Use @tag{c:1}.",
        "schema = 1\nevents = []\n",
        |_, _, _, block| {
            let plan = plan(block);
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                let source = if target == DocumentTarget::Latex {
                    latex::render(&plan)
                } else {
                    markdown::render(&plan)
                };
                let altered = source.replace(
                    "label def:langevin:coefficient-one",
                    "cite def:langevin:coefficient-one",
                );
                let site = UnitSite {
                    decoded: Some(4..13),
                    occurrence: Some(0),
                    ..input::site("fixture.toml")
                };
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "Use @tag{c:1}.".into(),
                    site: input::site("fixture.toml"),
                };
                let missing = EmissionRecord {
                    kind: EmissionKind::Tag,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: Some(0),
                    value: "def:langevin:coefficient-one".into(),
                    site: site.clone(),
                };
                let unexpected = EmissionRecord {
                    kind: EmissionKind::Citation(Box::new(crate::documents::CitationIdentity {
                        label: "def:langevin:coefficient-one".into(),
                        resolved: None,
                        handle: "c:1".into(),
                        token: if target == DocumentTarget::Latex {
                            "u0-t0".into()
                        } else {
                            String::new()
                        },
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        mint: None,
                    })),
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "def:langevin:coefficient-one".into(),
                    site: input::site("fixture.toml"),
                };
                let mut actual_paragraph = paragraph.clone();
                actual_paragraph.value = "Use `def:langevin:coefficient-one`.".into();
                let marker = if target == DocumentTarget::Latex {
                    "% unit "
                } else {
                    "<!-- unit "
                };
                let expected = crate::documents::failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    AssemblySite::Generated {
                        path: format!(
                            "generated/documents/main/en.{}",
                            if target == DocumentTarget::Latex {
                                "tex"
                            } else {
                                "labels.md"
                            }
                        ),
                        bytes: altered.find(marker).unwrap()..altered.len(),
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: Some(target),
                                site: input::site("fixture.toml"),
                            },
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: Some(target),
                                site,
                            },
                        ],
                    },
                    AssemblyFault::EmissionMismatch {
                        missing: vec![missing],
                        unexpected: vec![unexpected],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph,
                            actual: actual_paragraph,
                        }],
                    },
                );
                let actual = check_emission(&plan, &altered, target).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn recovered_literal_labels_refuse_after_target_decoding_and_restore() {
    context(
        "Literal ordinary.",
        "schema = 1\nevents = []\n",
        |_, _, _, block| {
            let plan = plan(block);
            for (target, injected) in [
                (DocumentTarget::Markdown, "dec:fixture:so&#117;rce"),
                (DocumentTarget::Latex, r"\texttt{dec:fixture:source}"),
            ] {
                let source = if target == DocumentTarget::Latex {
                    latex::render(&plan)
                } else {
                    markdown::render(&plan)
                };
                let altered = source.replace("ordinary", injected);
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: "Literal ordinary.".into(),
                    site: input::site("fixture.toml"),
                };
                let mut actual_paragraph = paragraph.clone();
                actual_paragraph.value = format!("Literal {injected}.");
                let marker = if target == DocumentTarget::Latex {
                    "% unit "
                } else {
                    "<!-- unit "
                };
                let expected = crate::documents::failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    AssemblySite::Generated {
                        path: format!(
                            "generated/documents/main/en.{}",
                            if target == DocumentTarget::Latex {
                                "tex"
                            } else {
                                "labels.md"
                            }
                        ),
                        bytes: altered.find(marker).unwrap()..altered.len(),
                        origins: vec![SourceOrigin {
                            unit: Some(UNIT.into()),
                            language: Some("en".into()),
                            target: Some(target),
                            site: input::site("fixture.toml"),
                        }],
                    },
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph,
                            actual: actual_paragraph,
                        }],
                    },
                );
                let actual = check_emission(&plan, &altered, target).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(actual.to_string(), expected.to_string());
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}
#[test]
fn reader_citations_keep_both_address_families_and_span_indices() {
    context(
        "Literal ``ordinary``. @tag{c:1} and `def:langevin:coefficient-one`; see `dec:fixture:source` and `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, welded, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let identities = block
                .runs
                .iter()
                .filter_map(|run| {
                    if let Inline::Citation { identity, .. } = run {
                        Some((
                            identity.label.as_str(),
                            identity.handle.as_str(),
                            identity.token.as_str(),
                            identity.resolved.clone(),
                            identity.position.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                identities,
                vec![
                    (
                        "def:langevin:coefficient-one",
                        "c:1",
                        "u0-c1",
                        Some(AddressTarget::Theory(NodeId::LangevinCoefficientOne)),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 1,
                            target: None
                        }
                    ),
                    (
                        LABEL,
                        "record",
                        "u0-c2",
                        Some(AddressTarget::Documentary(LABEL.into())),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 2,
                            target: None
                        }
                    ),
                    (
                        LABEL,
                        "record",
                        "u0-c3",
                        Some(AddressTarget::Documentary(LABEL.into())),
                        ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 3,
                            target: None
                        }
                    )
                ]
            );
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                vec!["def:langevin:coefficient-one", LABEL, LABEL]
            );
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}
#[test]
fn reader_citation_missing_live_handle_retains_exact_target_and_origins() {
    context(
        "See `dec:fixture:source`.",
        "schema = 1\nevents = []\n",
        |book, _, _, mut block| {
            let site = UnitSite {
                file: "fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: Some(4..24),
                occurrence: None,
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site,
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    file: "fixture.md".into(),
                                    path: vec![],
                                    anchor: SourceAnchor::Bytes(11, 34),
                                    decoded: None,
                                    occurrence: None,
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Documentary(LABEL.into()),
                        reason: ReaderIdentityReason::MissingTag,
                        candidates: vec![],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        "See `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            assert_eq!(
                check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
                Ok(())
            );
        },
    );
}
#[test]
fn reader_citation_handle_encoding_and_independent_recovery_are_exact() {
    for (handle, encoded) in [
        ("record", "record"),
        ("é_#$&^~", r"\FtdocHandle{233,95,35,36,38,94,126}"),
    ] {
        context(
            "See `dec:fixture:source`.",
            &ledger(handle, LABEL),
            |book, _, _, mut block| {
                block.map_citations(book, "main", "en").unwrap();
                let mut native = String::new();
                latex::prose_inline(&mut native, &block.runs);
                assert_eq!(
                    native,
                    format!("See \\FtdocTag{{{encoded}}}{{u0-c0}}% cite {LABEL}\n.")
                );
                assert_eq!(
                    recover_prose_latex(&native),
                    Some(vec![
                        Recovered::Text("See ".into()),
                        Recovered::Citation {
                            handle: handle.into(),
                            label: LABEL.into(),
                            token: Some("u0-c0".into())
                        },
                        Recovered::Text(".".into())
                    ])
                );
                let plan = plan(block);
                for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                    let source = if target == DocumentTarget::Latex {
                        latex::render(&plan)
                    } else {
                        markdown::render(&plan)
                    };
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            },
        );
    }
    for encoded in [
        r"\FtdocHandle{0233}}",
        r"\FtdocHandle{1114112}}",
        r"\FtdocHandle{233,}}",
        r"\FtdocHandle{}}",
        r"re\allowbreak{}cord}",
    ] {
        assert_eq!(recover_handle(encoded), None);
    }
}

#[test]
fn reader_label_literals_refuse_text_and_mentions_without_changing_the_weld() {
    for (text, bytes, decoded) in [
        ("Literal dec:fixture:source.", 8..26, None),
        ("Literal ``dec:fixture:source``.", 10..28, Some(8..30)),
    ] {
        context(text, "schema = 1\nevents = []\n", |_, unit, welded, _| {
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                Vec::<&str>::new()
            );
            let mut errors = vec![];
            literal_region(
                unit,
                "en",
                None,
                text,
                unit.texts()["en"].site(),
                &assembly_text_spans(&unit.texts()["en"]),
                &[],
                0,
                &mut errors,
            );
            let expected = AssemblyDiagnostic {
                rule: RuleId::M055,
                stage: AssemblyStage::Emission,
                document: Some("main".into()),
                unit: Some(UNIT.into()),
                language: Some("en".into()),
                target: None,
                site: AssemblySite::Source(UnitSite {
                    file: "fixture.toml".into(),
                    path: vec![],
                    anchor: SourceAnchor::WholeFile,
                    decoded,
                    occurrence: None,
                }),
                fault: AssemblyFault::ReaderLabelLiteral {
                    value: LABEL.into(),
                    bytes,
                },
            };
            assert_eq!(errors, vec![expected.clone()]);
            assert_eq!(errors[0].to_string(), expected.to_string());
        });
        context(
            "Literal ordinary.",
            "schema = 1\nevents = []\n",
            |_, unit, _, block| {
                let mut errors = vec![];
                literal_region(
                    unit,
                    "en",
                    None,
                    unit.texts()["en"].text(),
                    unit.texts()["en"].site(),
                    &[],
                    &[],
                    0,
                    &mut errors,
                );
                assert_eq!(errors, vec![]);
                let plan = plan(block);
                for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                    let source = if target == DocumentTarget::Latex {
                        latex::render(&plan)
                    } else {
                        markdown::render(&plan)
                    };
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            },
        );
    }
    assert_eq!(
        reader_label_literals(
            "(dec:fixture:source), other:text and ABCdec:fixture:source and dec:fixture:source_suffix."
        ),
        vec![1..19]
    );
}

#[test]
fn citation_emission_checks_handles_comments_tokens_and_breaks_independently() {
    context(
        "See `dec:fixture:source`.",
        &ledger("record", LABEL),
        |book, _, _, mut block| {
            block.map_citations(book, "main", "en").unwrap();
            let plan = plan(block);
            let authored = UnitSite {
                file: "fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: Some(4..24),
                occurrence: None,
            };
            let identity = crate::documents::CitationIdentity {
                label: LABEL.into(),
                resolved: Some(AddressTarget::Documentary(LABEL.into())),
                handle: "record".into(),
                token: "u0-c0".into(),
                position: ReaderIdentityPosition::Citation {
                    unit: UNIT.into(),
                    span: 0,
                    target: None,
                },
                mint: Some(MintOrigin {
                    kind: MintKind::Markdown,
                    site: LabelSite::Markdown {
                        file: "fixture.md".into(),
                        bytes: 11..34,
                    },
                }),
            };
            for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
                let source = if target == DocumentTarget::Markdown {
                    markdown::render(&plan)
                } else {
                    latex::render(&plan)
                };
                let paragraph = EmissionRecord {
                    kind: EmissionKind::Paragraph,
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: format!("See `{LABEL}`."),
                    site: input::site("fixture.toml"),
                };
                let citation = EmissionRecord {
                    kind: EmissionKind::Citation(Box::new(identity.clone())),
                    unit: UNIT.into(),
                    language: "en".into(),
                    target,
                    occurrence: None,
                    value: LABEL.into(),
                    site: authored.clone(),
                };
                let origins = vec![
                    SourceOrigin {
                        unit: Some(UNIT.into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: input::site("fixture.toml"),
                    },
                    SourceOrigin {
                        unit: Some(UNIT.into()),
                        language: Some("en".into()),
                        target: None,
                        site: authored.clone(),
                    },
                    SourceOrigin {
                        unit: None,
                        language: None,
                        target: None,
                        site: UnitSite {
                            file: "fixture.md".into(),
                            path: vec![],
                            anchor: SourceAnchor::Bytes(11, 34),
                            decoded: None,
                            occurrence: None,
                        },
                    },
                ];
                let use_bytes = if target == DocumentTarget::Markdown {
                    format!("record<!-- cite {LABEL} -->")
                } else {
                    format!("\\FtdocTag{{record}}{{u0-c0}}% cite {LABEL}\n")
                };
                let mut cases = vec![];
                let other_label = "dec:fixture:other";
                let mut wrong_citation = citation.clone();
                wrong_citation.value = other_label.into();
                if let EmissionKind::Citation(ref mut identity) = wrong_citation.kind {
                    identity.label = other_label.into();
                    identity.resolved = None;
                    identity.mint = None;
                }
                let mut wrong_paragraph = paragraph.clone();
                wrong_paragraph.value = format!("See `{other_label}`.");
                cases.push((
                    source.replace(&format!("cite {LABEL}"), &format!("cite {other_label}")),
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![wrong_citation],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: wrong_paragraph,
                        }],
                    },
                ));
                if target == DocumentTarget::Markdown {
                    let mut actual = paragraph.clone();
                    actual.value = format!("See other<!-- cite {LABEL} -->.");
                    cases.push((
                        source.replace("record<!--", "other<!--"),
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                } else {
                    let old = format!("\\texttt{{{LABEL}}}% cite {LABEL}\n");
                    let mut actual = paragraph.clone();
                    actual.value = format!("See {old}.");
                    cases.push((
                        source.replace(&use_bytes, &old),
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                    let plans = std::slice::from_ref(&plan);
                    let runtime = crate::documents::runtime::render(book, plans).unwrap();
                    let row = runtime
                        .lines()
                        .find(|line| line.starts_with("\\prop_gput:Nxx \\g_ftdoc_sites_prop"))
                        .unwrap()
                        .to_owned()
                        + "\n";
                    let begin = runtime.find(&row).unwrap();
                    let record = EmissionRecord {
                        kind: EmissionKind::Tag,
                        unit: "main/en/u0-c0".into(),
                        language: "sites".into(),
                        target,
                        occurrence: None,
                        value: UNIT.into(),
                        site: UnitSite {
                            path: vec![
                                PathPart::Key("sites".into()),
                                PathPart::Key("main/en/u0-c0".into()),
                            ],
                            ..input::site("generated/addressing.tex")
                        },
                    };
                    for duplicate in [false, true] {
                        let altered = runtime.replace(
                            &row,
                            &if duplicate {
                                row.repeat(2)
                            } else {
                                String::new()
                            },
                        );
                        let expected = crate::documents::failure(
                            RuleId::M055,
                            AssemblyStage::Emission,
                            AssemblySite::Generated {
                                path: "generated/addressing.tex".into(),
                                bytes: if duplicate {
                                    begin + row.len()..begin + 2 * row.len()
                                } else {
                                    begin..begin
                                },
                                origins: origins.clone(),
                            },
                            AssemblyFault::EmissionMismatch {
                                missing: if duplicate {
                                    vec![]
                                } else {
                                    vec![record.clone()]
                                },
                                unexpected: if duplicate {
                                    vec![record.clone()]
                                } else {
                                    vec![]
                                },
                                changed: vec![],
                            },
                        );
                        let actual =
                            crate::documents::runtime::check(book, plans, &altered).unwrap_err();
                        assert_eq!(actual, expected);
                        assert_eq!(actual.to_string(), expected.to_string());
                        assert_eq!(
                            crate::documents::runtime::check(book, plans, &runtime),
                            Ok(())
                        );
                    }
                }
                if target == DocumentTarget::Latex {
                    for (from, to, changed_handle) in
                        [("{record}", "{other}", true), ("{u0-c0}", "{u0-c1}", false)]
                    {
                        let altered = source.replace(from, to);
                        let mut actual = citation.clone();
                        let EmissionKind::Citation(ref mut found) = actual.kind else {
                            unreachable!()
                        };
                        if changed_handle {
                            found.handle = "other".into();
                        } else {
                            found.token = "u0-c1".into();
                        }
                        cases.push((
                            altered,
                            AssemblyFault::EmissionMismatch {
                                missing: vec![citation.clone()],
                                unexpected: vec![actual],
                                changed: vec![],
                            },
                        ));
                    }
                }
                let repeated = source.replace(&use_bytes, &use_bytes.repeat(2));
                let mut repeated_paragraph = paragraph.clone();
                repeated_paragraph.value = format!("See `{LABEL}``{LABEL}`.");
                cases.push((
                    repeated,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![citation.clone()],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: repeated_paragraph,
                        }],
                    },
                ));
                let dropped = source.replace(&use_bytes, "");
                let mut dropped_paragraph = paragraph.clone();
                dropped_paragraph.value = "See .".into();
                cases.push((
                    dropped,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: dropped_paragraph,
                        }],
                    },
                ));
                let wrong_comment =
                    source.replace("cite dec:fixture:source", "label dec:fixture:source");
                let mut as_tag = citation.clone();
                as_tag.kind = EmissionKind::Tag;
                // A recovered tag has no planned authored-tag site to borrow.
                as_tag.site = input::site("fixture.toml");
                let mut tag_paragraph = paragraph.clone();
                tag_paragraph.value = "See @tag{record}.".into();
                cases.push((
                    wrong_comment,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![citation.clone()],
                        unexpected: vec![as_tag],
                        changed: vec![crate::documents::EmissionChange {
                            expected: paragraph.clone(),
                            actual: tag_paragraph,
                        }],
                    },
                ));
                if target == DocumentTarget::Latex {
                    let altered = source.replace("{record}", r"{re\allowbreak{}cord}");
                    let mut actual = paragraph.clone();
                    actual.value = format!(
                        "See \\FtdocTag{{re\\allowbreak{{}}cord}}{{u0-c0}}% cite {LABEL}\n."
                    );
                    cases.push((
                        altered,
                        AssemblyFault::EmissionMismatch {
                            missing: vec![citation.clone()],
                            unexpected: vec![],
                            changed: vec![crate::documents::EmissionChange {
                                expected: paragraph.clone(),
                                actual,
                            }],
                        },
                    ));
                }
                for (altered, fault) in cases {
                    let marker = if target == DocumentTarget::Markdown {
                        "<!-- unit "
                    } else {
                        "% unit "
                    };
                    let expected = crate::documents::failure(
                        RuleId::M055,
                        AssemblyStage::Emission,
                        AssemblySite::Generated {
                            path: format!(
                                "generated/documents/main/en.{}",
                                if target == DocumentTarget::Markdown {
                                    "labels.md"
                                } else {
                                    "tex"
                                }
                            ),
                            bytes: altered.find(marker).unwrap()..altered.len(),
                            origins: origins.clone(),
                        },
                        fault,
                    );
                    let actual = check_emission(&plan, &altered, target).unwrap_err();
                    assert_eq!(actual, expected, "{altered}");
                    assert_eq!(actual.to_string(), expected.to_string());
                    assert_eq!(check_emission(&plan, &source, target), Ok(()));
                }
            }
        },
    );
}

#[test]
fn unit_metadata_citations_keep_their_own_mint_and_live_handle() {
    context(
        "See `sec:fixture:reader`.",
        &ledger("self", UNIT),
        |book, _, welded, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let Inline::Citation {
                identity,
                site,
                text,
            } = &block.runs[1]
            else {
                panic!("citation")
            };
            assert_eq!(
                identity,
                &crate::documents::CitationIdentity {
                    label: UNIT.into(),
                    resolved: Some(AddressTarget::Documentary(UNIT.into())),
                    handle: "self".into(),
                    token: "u0-c0".into(),
                    position: ReaderIdentityPosition::Citation {
                        unit: UNIT.into(),
                        span: 0,
                        target: None
                    },
                    mint: Some(MintOrigin {
                        kind: MintKind::UnitMetadata,
                        site: LabelSite::Unit {
                            unit: UNIT.into(),
                            site: UnitSite {
                                file: "fixture.toml".into(),
                                path: vec![
                                    PathPart::Key("unit".into()),
                                    PathPart::Key("id".into())
                                ],
                                anchor: SourceAnchor::Unavailable,
                                decoded: None,
                                occurrence: None
                            }
                        }
                    })
                }
            );
            assert_eq!(text, UNIT);
            assert_eq!(
                site,
                &UnitSite {
                    decoded: Some(4..24),
                    ..input::site("fixture.toml")
                }
            );
            assert_eq!(
                welded.units()[0].languages()[0]
                    .citations()
                    .iter()
                    .map(ftoo_manifest::ResolvedUnitCitation::label)
                    .collect::<Vec<_>>(),
                vec![UNIT]
            );
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn retired_documentary_citation_keeps_the_complete_event_and_restores() {
    let live = ledger("record", LABEL);
    let retired = format!(
        "{live}[[events]]\nordinal = 2\nchange = \"retire\"\nhandle = \"record\"\nauthority-record = \"dec:fixture:authority\"\nreason = \"Fixture retirement.\"\n"
    );
    context(
        "See `dec:fixture:source`.",
        &retired,
        |book, _, _, mut block| {
            let event = DocumentaryRetirement {
                ordinal: 2,
                authority_record: "dec:fixture:authority".into(),
                reason: "Fixture retirement.".into(),
                site: UnitSite {
                    file: "fixture-registry.toml".into(),
                    path: vec![PathPart::Key("events".into()), PathPart::Index(1)],
                    anchor: SourceAnchor::Bytes(live.len(), live.len() + "[[events]]".len()),
                    decoded: None,
                    occurrence: None,
                },
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("main".into()),
                    unit: Some(UNIT.into()),
                    language: Some("en".into()),
                    target: None,
                    site: AssemblySite::ReaderIdentity {
                        position: ReaderIdentityPosition::Citation {
                            unit: UNIT.into(),
                            span: 0,
                            target: None,
                        },
                        origins: vec![
                            SourceOrigin {
                                unit: Some(UNIT.into()),
                                language: Some("en".into()),
                                target: None,
                                site: UnitSite {
                                    decoded: Some(4..24),
                                    ..input::site("fixture.toml")
                                },
                            },
                            SourceOrigin {
                                unit: None,
                                language: None,
                                target: None,
                                site: UnitSite {
                                    file: "fixture.md".into(),
                                    path: vec![],
                                    anchor: SourceAnchor::Bytes(11, 34),
                                    decoded: None,
                                    occurrence: None,
                                },
                            },
                        ],
                    },
                    fault: AssemblyFault::ReaderIdentity {
                        target: AddressTarget::Documentary(LABEL.into()),
                        reason: ReaderIdentityReason::NoLiveTag,
                        candidates: vec![ReaderTagCandidate {
                            handle: "record".into(),
                            outcome: Err(TagResolutionFault::RetiredDocumentaryTag {
                                input: "record".into(),
                                label: LABEL.into(),
                                event,
                            }),
                        }],
                    },
                }])
                .unwrap(),
            );
            let actual = block.map_citations(book, "main", "en").unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), expected.to_string());
        },
    );
    context(
        "See `dec:fixture:source`.",
        &live,
        |book, _, _, mut block| {
            assert_eq!(block.map_citations(book, "main", "en"), Ok(()));
            let plan = plan(block);
            for (target, source) in [
                (DocumentTarget::Markdown, markdown::render(&plan)),
                (DocumentTarget::Latex, latex::render(&plan)),
            ] {
                assert_eq!(check_emission(&plan, &source, target), Ok(()));
            }
        },
    );
}

#[test]
fn display_citation_sites_count_mentions_across_regions_and_languages() {
    let graph = seed().unwrap();
    let site = input::site("display-fixture.toml");
    let located = |s: &str| LocatedString {
        value: s.into(),
        site: site.clone(),
    };
    let languages = || {
        ["en", "fr"]
            .into_iter()
            .map(|language| {
                (
                    language.into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("Caption."),
                    },
                )
            })
            .collect()
    };
    let corpus = admit_unit_drafts(
        DocumentDraft {
            documents_site: site.clone(),
            documents: BTreeMap::from([
                (
                    "main".into(),
                    DocumentEntryDraft {
                        key_site: site.clone(),
                        table_site: site.clone(),
                        languages_site: site.clone(),
                        required_languages: vec![located("en")],
                    },
                ),
                (
                    "fixture".into(),
                    DocumentEntryDraft {
                        key_site: site.clone(),
                        table_site: site.clone(),
                        languages_site: site.clone(),
                        required_languages: vec![located("en"), located("fr")],
                    },
                ),
            ]),
        },
        ["sec:fixture:heading", UNIT]
            .into_iter()
            .map(|id| UnitDraft {
                econ: None,
                id: located(id),
                document: located("fixture"),
                table_site: site.clone(),
                math_site: site.clone(),
                texts_site: site.clone(),
                documentary_targets: BTreeMap::new(),
                math: BTreeMap::new(),
                texts: languages(),
            })
            .chain(std::iter::once(UnitDraft {
                econ: None,
                id: located("sec:main:heading"),
                document: located("main"),
                table_site: site.clone(),
                math_site: site.clone(),
                texts_site: site.clone(),
                documentary_targets: BTreeMap::new(),
                math: BTreeMap::new(),
                texts: BTreeMap::from([(
                    "en".into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("Heading."),
                    },
                )]),
            }))
            .collect(),
    )
    .unwrap();
    let markdown=check_markdown_sources(vec![MarkdownSource {file:"fixture.md".into(),bytes:b"# Decision \xc2\xb7 `dec:fixture:source`\n# Authority \xc2\xb7 `dec:fixture:authority`\n# Kernel \xc2\xb7 `proposal:manifest:kernel-bite`\n# Lifecycle \xc2\xb7 `proposal:manifest:addressing-lifecycle-bite`\n".to_vec()}]).unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let events = registry::load(&crate::UnitSource {
        file: "fixture-registry.toml".into(),
        bytes: ledger("record", LABEL).into_bytes(),
    })
    .unwrap();
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, events)
        .unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_joint_labels(&resolved, &joint).unwrap();
    let welded = weld_units(&integrated).unwrap();
    let metadata = corpus
        .units()
        .iter()
        .map(|u| (u.label().as_str().into(), site.clone()))
        .collect();
    let skeletons = check_skeletons(
        &corpus,
        &[
            SkeletonDraft {
                document: located("main"),
                sections_site: site.clone(),
                sections: vec![SkeletonSection {
                    heading: located("sec:main:heading"),
                    children: vec![],
                }],
            },
            SkeletonDraft {
                document: located("fixture"),
                sections_site: site.clone(),
                sections: vec![SkeletonSection {
                    heading: located("sec:fixture:heading"),
                    children: vec![SkeletonChild::Unit(located(UNIT))],
                }],
            },
        ],
        &metadata,
        &metadata,
        &BTreeMap::from([
            ("fixture".into(), site.clone()),
            ("main".into(), site.clone()),
        ]),
    )
    .unwrap();
    let drafts=[DisplayDraft {unit:located(UNIT),kind:DisplayKind::Table,bodies:[(DocumentTarget::Markdown,"| ``ordinary`` | `dec:fixture:source` |\n| --- | --- |\n| `dec:fixture:source` | ``tail`` |"),(DocumentTarget::Latex,r"\begin{tabular}{ll} ``ordinary`` & `dec:fixture:source` \\ `dec:fixture:source` & ``tail`` \\ \end{tabular}")].into_iter().map(|(target,text)|(target,DisplayTargetDraft {site:site.clone(),languages:["en","fr"].into_iter().map(|language|(language.into(),located(text))).collect()})).collect()}];
    let displays = check_displays(&drafts, &skeletons, &welded).unwrap();
    for language in ["en", "fr"] {
        let mut errors = vec![];
        check_reader_literals(&corpus.units()[1], language, &displays, &mut errors);
        assert_eq!(errors, vec![]);
    }
    let mut altered_drafts = drafts.clone();
    for body in altered_drafts[0].bodies.values_mut() {
        for source in body.languages.values_mut() {
            source.value = source.value.replace("``ordinary``", "dec:fixture:source");
        }
    }
    let altered = check_displays(&altered_drafts, &skeletons, &welded).unwrap();
    for language in ["en", "fr"] {
        let mut actual = vec![];
        check_reader_literals(&corpus.units()[1], language, &altered, &mut actual);
        let expected: Vec<_> = [DocumentTarget::Markdown, DocumentTarget::Latex]
            .into_iter()
            .map(|target| {
                let start = if target == DocumentTarget::Latex {
                    20
                } else {
                    2
                };
                AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Emission,
                    document: Some("fixture".into()),
                    unit: Some(UNIT.into()),
                    language: Some(language.into()),
                    target: Some(target),
                    site: AssemblySite::Source(UnitSite {
                        path: vec![
                            PathPart::Key("regions".into()),
                            PathPart::Index(0),
                            PathPart::Index(0),
                        ],
                        ..site.clone()
                    }),
                    fault: AssemblyFault::ReaderLabelLiteral {
                        value: LABEL.into(),
                        bytes: start..start + 18,
                    },
                }
            })
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.iter().map(ToString::to_string).collect::<Vec<_>>(),
            expected.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
        let reread = check_displays(&drafts, &skeletons, &welded).unwrap();
        let mut restored = vec![];
        check_reader_literals(&corpus.units()[1], language, &reread, &mut restored);
        assert_eq!(restored, vec![]);
    }

    let unit = &corpus.units()[1];
    for language in ["en", "fr"] {
        let mut block = Block {
            unit: UNIT.into(),
            depth: None,
            quotation: false,
            paragraph: false,
            equation: false,
            econ: None,
            site: site.clone(),
            runs: vec![Inline::Text("Caption.".into())],
            pending_names: BTreeMap::new(),
            displays: display_plans(1, unit, language, &welded, &BTreeMap::new(), &displays),
        };
        assert_eq!(block.map_citations(&book, "fixture", language), Ok(()));
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let name = if target == DocumentTarget::Markdown {
                "markdown"
            } else {
                "latex"
            };
            let found = block.displays[&target]
                .regions
                .iter()
                .flat_map(|r| &r.runs)
                .filter_map(|r| {
                    if let Inline::Citation { identity, .. } = r {
                        Some((
                            identity.token.clone(),
                            identity.position.clone(),
                            identity.label.clone(),
                            identity.handle.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                found,
                [1, 2].map(|span| (
                    format!("u1-d{name}-{language}-c{span}"),
                    ReaderIdentityPosition::Citation {
                        unit: UNIT.into(),
                        span,
                        target: Some(target)
                    },
                    LABEL.into(),
                    "record".into()
                ))
            );
        }
        let mut plan = plan(block);
        plan.document = "fixture".into();
        plan.language = language.into();
        for (target, source) in [
            (DocumentTarget::Markdown, markdown::render(&plan)),
            (DocumentTarget::Latex, latex::render(&plan)),
        ] {
            assert_eq!(check_emission(&plan, &source, target), Ok(()));
        }
    }
}

#[test]
fn literal_checker_protects_admitted_citation_tag_and_math_ranges() {
    context(
        "Plain text.",
        "schema = 1\nevents = []\n",
        |_, _, welded, _| {
            let site = input::site("fixture.toml");
            let located = |value: &str| LocatedString {
                value: value.into(),
                site: site.clone(),
            };
            let metadata = BTreeMap::from([(UNIT.into(), site.clone())]);
            let skeletons = check_skeletons(
                welded.integrated().resolved().corpus(),
                &[SkeletonDraft {
                    document: located("main"),
                    sections_site: site.clone(),
                    sections: vec![SkeletonSection {
                        heading: located(UNIT),
                        children: vec![],
                    }],
                }],
                &metadata,
                &metadata,
                &BTreeMap::from([("main".into(), site.clone())]),
            )
            .unwrap();
            let displays = check_displays(&[], &skeletons, welded).unwrap();
            let text = "See `dec:fixture:source` @tag{dec:fixture:source} @math{value}.";
            let corpus = literal_corpus(
                text,
                BTreeMap::from([(
                    "value".into(),
                    LocatedEntryDraft {
                        key_site: site.clone(),
                        value: located("x"),
                    },
                )]),
            );
            let unit = &corpus.units()[0];
            assert_eq!(
                assembly_text_spans(&unit.texts()["en"])
                    .into_iter()
                    .map(|(label, site, citation)| (label, site.decoded, citation))
                    .collect::<Vec<_>>(),
                vec![(LABEL.into(), Some(4..24), true)]
            );
            assert_eq!(
                unit.texts()["en"]
                    .tag_occurrences()
                    .iter()
                    .map(|o| (o.input(), o.site().decoded.clone()))
                    .collect::<Vec<_>>(),
                vec![(LABEL, Some(25..49))]
            );
            assert_eq!(
                unit.texts()["en"]
                    .occurrences()
                    .iter()
                    .map(|o| (o.name(), o.site().decoded.clone()))
                    .collect::<Vec<_>>(),
                vec![("value", Some(50..62))]
            );
            let mut errors = vec![];
            check_reader_literals(unit, "en", &displays, &mut errors);
            assert_eq!(errors, vec![]);
        },
    );
}
#[test]
fn sen_book_uses_only_the_compared_parenthetical_derivation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let stock = std::fs::read_to_string(root.join("documents/paper/references.bib")).unwrap();
    let header = "@book{a:sencapability,\n";
    assert_eq!(stock.matches(header).count(), 1);
    let derived = stock.replacen(
        header,
        "@book{a:sencapability,\n  shortauthor = {{A. Sen}},\n",
        1,
    );
    let site = ftoo_manifest::UnitSite {
        file: "units/statements/coda-flourishing-pole.toml".into(),
        path: vec![
            ftoo_manifest::PathPart::Key("unit".into()),
            ftoo_manifest::PathPart::Key("texts".into()),
            ftoo_manifest::PathPart::Key("en".into()),
        ],
        anchor: ftoo_manifest::SourceAnchor::Unavailable,
        decoded: None,
        occurrence: None,
    };
    let parenthetical = ftoo_manifest::BibliographyCitationKind::Parenthetical;
    let entry = super::super::bibliography::resolve_with_derived(
        &stock,
        &derived,
        "a:sencapability",
        &site,
        parenthetical,
    )
    .unwrap();
    assert_eq!(entry.label, "A. Sen 1999");
    assert_eq!(
        super::super::bibliography::screen_text(&entry),
        "Sen, Amartya, 1999. Development as Freedom. Oxford: Oxford University Press."
    );
    for (changed, fault) in [
        (
            derived.replacen("year = {1999}", "year = {2000}", 1),
            "derived book row differs",
        ),
        (
            derived.replacen("{{A. Sen}}", "{{a. Sen}}", 1),
            "derived shortauthor shape",
        ),
        (
            derived.replacen("{{A. Sen}}", "{{A. Other}}", 1),
            "derived shortauthor shape",
        ),
    ] {
        assert!(
            super::super::bibliography::resolve_with_derived(
                &stock,
                &changed,
                "a:sencapability",
                &site,
                parenthetical
            )
            .unwrap_err()
            .to_string()
            .contains(fault)
        );
    }
    assert!(
        super::super::bibliography::resolve_with_derived(
            &stock,
            &derived,
            "a:sencapability",
            &site,
            ftoo_manifest::BibliographyCitationKind::Textual
        )
        .unwrap_err()
        .to_string()
        .contains("textual book citation")
    );
    assert!(
        super::super::bibliography::resolve_with_derived(
            &stock,
            &derived,
            "a:senidentity",
            &site,
            parenthetical
        )
        .unwrap_err()
        .to_string()
        .contains("entry changed by bibliography derivation")
    );
}
