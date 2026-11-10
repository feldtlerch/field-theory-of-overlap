use super::*;

#[test]
fn w07_span_grammar_exact_faults_and_first_decisive_ranges() {
    use LabelSpanReason::*;
    for (token, reason, fragment) in [
        ("`", Unclosed, "`"),
        ("``", Unclosed, "``"),
        ("````", DelimiterWidth, "````"),
        ("```a:b:c```", DelimiterWidth, "```"),
        ("`a:b:c``", MismatchedDelimiter, "`a:b:c``"),
        ("``a`later``", MismatchedDelimiter, "``a`"),
        ("`a:b:c```", MismatchedDelimiter, "`a:b:c```"),
        ("`a:b:c``d:e:f`", MismatchedDelimiter, "`a:b:c``"),
        ("``outer `a:b:c` inner``", MismatchedDelimiter, "``outer `"),
        ("`` ``", EmptyBody, "`` ``"),
        ("``\u{a0}``", EmptyBody, "``\u{a0}``"),
        ("`A:b:c`", InvalidLabel, "`A:b:c`"),
        ("`a:b`", InvalidLabel, "`a:b`"),
        ("`a:b:c:d`", InvalidLabel, "`a:b:c:d`"),
        ("` a:b:c`", InvalidLabel, "` a:b:c`"),
        ("` `", InvalidLabel, "` `"),
        ("`a:b:c` `unterminated", Unclosed, "`unterminated"),
    ] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = format!("See {token}");
        let error = admit_unit_drafts(documents, units).unwrap_err();
        let (start, index) = if token.starts_with("`a:b:c` ") {
            (12, 1)
        } else {
            (4, 0)
        };
        let mut location = site("statement.toml", &["unit", "texts", "en"]);
        location.decoded = Some(start..start + fragment.len());
        let fault = UnitFault::MalformedLabelSpan {
            span_index: index,
            reason,
            fragment: fragment.into(),
        };
        assert_eq!(
            error.diagnostics(),
            [expected(
                RuleId::M041,
                UnitStage::Content,
                Some("en"),
                location,
                fault
            )]
        );
        assert_eq!(
            error.to_string(),
            format!(
                "M041 Content unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"statement.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 10..90 decoded {start}..{}: MalformedLabelSpan {{ span_index: {index}, reason: {reason:?}, fragment: {fragment:?} }}\n",
                start + fragment.len()
            )
        );
    }
}

#[test]
fn w07_every_forbidden_span_character_and_opaque_mention_payload() {
    for c in [
        '$', '\\', '<', '>', '\n', '\r', '\t', '\0', '\u{7f}', '\u{85}', '\u{2028}', '\u{2029}',
    ] {
        for delimiter in ["`", "``"] {
            let (documents, mut units) = fixtures();
            units[0].texts.get_mut("en").unwrap().value.value =
                format!("See {delimiter}x{c}later{delimiter}.");
            let fragment = format!("{delimiter}x{c}");
            let end = 4 + fragment.len();
            let mut location = site("statement.toml", &["unit", "texts", "en"]);
            location.decoded = Some(4..end);
            let error = admit_unit_drafts(documents, units).unwrap_err();
            assert_eq!(
                error.diagnostics(),
                [expected(
                    RuleId::M041,
                    UnitStage::Content,
                    Some("en"),
                    location,
                    UnitFault::MalformedLabelSpan {
                        span_index: 0,
                        reason: LabelSpanReason::ForbiddenCharacter,
                        fragment: fragment.clone()
                    }
                )]
            );
            assert_eq!(
                error.to_string(),
                format!(
                    "M041 Content unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"statement.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 10..90 decoded 4..{end}: MalformedLabelSpan {{ span_index: 0, reason: ForbiddenCharacter, fragment: {fragment:?} }}\n"
                )
            );
        }
    }
    for text in [
        "See ``@tag{unknown} @math{unknown} é``.",
        "See `a:b:c`.",
        "See (`a:b:c`).",
        "See ·`a:b:c`, ·  `a:b:c`, and ordinary a:b:c.",
        "See @@tag{c:1} and ``unknown:label:name``.",
    ] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = text.into();
        let checked = admit_unit_drafts(documents, units).unwrap();
        assert_eq!(checked.units()[0].texts()["en"].text(), text);
        assert_eq!(checked.units()[0].texts()["en"].tag_occurrences(), []);
        assert_eq!(checked.units()[0].texts()["en"].occurrences(), []);
    }
}

#[test]
fn w08_text_mints_are_forbidden_and_spans_do_not_supply_prose() {
    for label in [
        "sec:coefficients:first-value",
        "def:langevin:coefficient-one",
    ] {
        let (documents, mut units) = fixtures();
        let text = format!("· `{label}`");
        units[0].texts.get_mut("en").unwrap().value.value = text.clone();
        let error = admit_unit_drafts(documents, units).unwrap_err();
        let mut location = site("statement.toml", &["unit", "texts", "en"]);
        location.decoded = Some(0..text.len());
        assert_eq!(
            error.diagnostics(),
            [expected(
                RuleId::M045,
                UnitStage::Content,
                Some("en"),
                location,
                UnitFault::TextMintForbidden {
                    span_index: 0,
                    label: label.into()
                }
            )]
        );
        assert_eq!(
            error.to_string(),
            format!(
                "M045 Content unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"statement.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 10..90 decoded 0..{}: TextMintForbidden {{ span_index: 0, label: {label:?} }}\n",
                text.len()
            )
        );
    }
    for text in ["`a:b:c`", "``mention``", "@tag{c:1} `a:b:c` ``mention``"] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = text.into();
        let error = admit_unit_drafts(documents, units).unwrap_err();
        assert_eq!(
            error.diagnostics(),
            [expected(
                RuleId::M040,
                UnitStage::Language,
                Some("en"),
                site("statement.toml", &["unit", "texts", "en"]),
                UnitFault::BlankText
            )]
        );
        assert_eq!(
            error.to_string(),
            "M040 Language unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"statement.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 10..90: BlankText\n"
        );
    }
}

fn site(file: &str, path: &[&str]) -> UnitSite {
    UnitSite {
        file: file.to_owned(),
        path: path
            .iter()
            .map(|k| PathPart::Key((*k).to_owned()))
            .collect(),
        anchor: SourceAnchor::Bytes(10, 90),
        decoded: None,
        occurrence: None,
    }
}
fn located(value: &str, path: &[&str]) -> LocatedString {
    LocatedString {
        value: value.to_owned(),
        site: site("statement.toml", path),
    }
}
fn entry(value: &str, path: &[&str]) -> LocatedEntryDraft {
    LocatedEntryDraft {
        key_site: site("statement.toml", path),
        value: located(value, path),
    }
}
fn fixtures() -> (DocumentDraft, Vec<UnitDraft>) {
    let document = DocumentEntryDraft {
        key_site: site("documents.toml", &["documents", "main"]),
        table_site: site("documents.toml", &["documents", "main"]),
        languages_site: site(
            "documents.toml",
            &["documents", "main", "required-languages"],
        ),
        required_languages: vec![LocatedString {
            value: "en".to_owned(),
            site: UnitSite {
                path: vec![
                    PathPart::Key("documents".to_owned()),
                    PathPart::Key("main".to_owned()),
                    PathPart::Key("required-languages".to_owned()),
                    PathPart::Index(0),
                ],
                ..site("documents.toml", &[])
            },
        }],
    };
    (
        DocumentDraft {
            documents_site: site("documents.toml", &["documents"]),
            documents: BTreeMap::from([("main".to_owned(), document)]),
        },
        vec![UnitDraft {
            id: located("sec:coefficients:first-value", &["unit", "id"]),
            document: located("main", &["unit", "document"]),
            table_site: site("statement.toml", &["unit"]),
            math_site: site("statement.toml", &["unit", "math"]),
            texts_site: site("statement.toml", &["unit", "texts"]),
            math: BTreeMap::new(),
            texts: BTreeMap::from([(
                "en".to_owned(),
                entry("The coefficient.", &["unit", "texts", "en"]),
            )]),
        }],
    )
}
fn expected(
    rule: RuleId,
    stage: UnitStage,
    language: Option<&str>,
    location: UnitSite,
    fault: UnitFault,
) -> UnitDiagnostic {
    UnitDiagnostic {
        rule,
        stage,
        unit: Some("sec:coefficients:first-value".to_owned()),
        language: language.map(str::to_owned),
        site: location,
        fault,
    }
}
fn refusal(documents: DocumentDraft, units: Vec<UnitDraft>, expected: Vec<UnitDiagnostic>) {
    let actual = admit_unit_drafts(documents, units).unwrap_err();
    assert_eq!(actual, UnitErrors::from_diagnostics(expected).unwrap());
}
#[test]
fn u07_known_tag_acceptance_and_literal_escape() {
    let (documents, mut units) = fixtures();
    units[0].texts.get_mut("en").unwrap().value.value = "See @tag{c:1}.".to_owned();
    let corpus = admit_unit_drafts(documents.clone(), units.clone()).unwrap();
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let mut location = site("statement.toml", &["unit", "texts", "en"]);
    location.decoded = Some(4..13);
    location.occurrence = Some(0);
    assert_eq!(resolved.tags()[0].occurrence().input(), "c:1");
    assert_eq!(resolved.tags()[0].occurrence().site(), &location);
    assert_eq!(
        resolved.tags()[0].target(),
        crate::NodeId::LangevinCoefficientOne
    );
    for text in [
        "See the coefficient.",
        "See @@tag{c:1}.",
        "@@",
        "Ordinary *punctuation*, é.",
    ] {
        units[0].texts.get_mut("en").unwrap().value.value = text.to_owned();
        let checked = admit_unit_drafts(documents.clone(), units.clone()).unwrap();
        assert_eq!(checked.units()[0].texts()["en"].occurrences(), []);
        assert_eq!(checked.units()[0].texts()["en"].tag_occurrences(), []);
    }
}
#[test]
fn u07_insertion_fault_matrix_and_unicode_ranges() {
    for (text, reason, fragment, range) in [
        ("See @.", InsertionReason::UnknownForm, "@.", 4..6),
        ("See @", InsertionReason::Unclosed, "@", 4..5),
        ("See @mat", InsertionReason::Unclosed, "@mat", 4..8),
        ("See @math{x", InsertionReason::Unclosed, "@math{x", 4..11),
        (
            "See @math{x{y}.",
            InsertionReason::NestedBrace,
            "@math{x{",
            4..12,
        ),
        (
            "See @math{}.",
            InsertionReason::BadMathName,
            "@math{}",
            4..11,
        ),
        (
            "See @math{X}.",
            InsertionReason::BadMathName,
            "@math{X",
            4..11,
        ),
        (
            "See @math{x-}.",
            InsertionReason::BadMathName,
            "@math{x-}",
            4..13,
        ),
        ("é @bad{c:1}", InsertionReason::UnknownForm, "@b", 3..5),
    ] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = text.to_owned();
        let mut location = site("statement.toml", &["unit", "texts", "en"]);
        location.decoded = Some(range);
        location.occurrence = Some(0);
        refusal(
            documents,
            units,
            vec![expected(
                RuleId::M041,
                UnitStage::Content,
                Some("en"),
                location,
                UnitFault::MalformedInsertion {
                    reason,
                    fragment: fragment.to_owned(),
                },
            )],
        );
    }
    for c in ['$', '\\', '<', '>'].into_iter().chain(
        (0..=0x9f)
            .filter_map(char::from_u32)
            .filter(|c| c.is_control() && !matches!(c, '\t' | '\n')),
    ) {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = format!("é {c}.");
        let mut location = site("statement.toml", &["unit", "texts", "en"]);
        location.decoded = Some(3..3 + c.len_utf8());
        refusal(
            documents,
            units,
            vec![expected(
                RuleId::M041,
                UnitStage::Content,
                Some("en"),
                location,
                UnitFault::UnsupportedTextCharacter {
                    value: c.to_string(),
                },
            )],
        );
    }
}
#[test]
fn u05_invalid_metadata_names_and_unknown_document() {
    for id in [
        "", "a:b", "a:b:c:d", ":b:c", "a::c", "a:b:", "A:b:c", "a:b:c_", " a:b:c", "a:b:é",
    ] {
        let (documents, mut units) = fixtures();
        units[0].id.value = id.to_owned();
        let mut diagnostic = expected(
            RuleId::M039,
            UnitStage::Metadata,
            None,
            site("statement.toml", &["unit", "id"]),
            UnitFault::InvalidName {
                domain: NameDomain::UnitLabel,
                value: id.to_owned(),
            },
        );
        diagnostic.unit = Some(id.to_owned());
        refusal(documents, units, vec![diagnostic]);
    }
    for name in [
        "", "Main", "1main", "main_", "main-", "main--x", "-main", "é",
    ] {
        let (documents, mut units) = fixtures();
        units[0].document.value = name.to_owned();
        refusal(
            documents,
            units,
            vec![expected(
                RuleId::M039,
                UnitStage::Metadata,
                None,
                site("statement.toml", &["unit", "document"]),
                UnitFault::InvalidName {
                    domain: NameDomain::DocumentName,
                    value: name.to_owned(),
                },
            )],
        );
    }
    let (documents, mut units) = fixtures();
    units[0].document.value = "other".to_owned();
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M039,
            UnitStage::Metadata,
            None,
            site("statement.toml", &["unit", "document"]),
            UnitFault::UnknownDocument {
                name: "other".to_owned(),
                available: vec!["main".to_owned()],
            },
        )],
    );
}
#[test]
fn u05_duplicate_ids_name_all_sites_in_file_order() {
    let (documents, mut units) = fixtures();
    let mut second = units[0].clone();
    second.id.site.file = "another.toml".to_owned();
    let sites = vec![second.id.site.clone(), units[0].id.site.clone()];
    units.push(second);
    let diagnostics = sites
        .iter()
        .map(|s| {
            expected(
                RuleId::M039,
                UnitStage::Metadata,
                None,
                s.clone(),
                UnitFault::DuplicateUnit {
                    id: "sec:coefficients:first-value".to_owned(),
                    sites: sites.clone(),
                },
            )
        })
        .collect();
    refusal(documents, units, diagnostics);
}
#[test]
fn u06_invalid_language_tokens_at_both_roles() {
    for language in [
        "",
        "e",
        "engl",
        "EN",
        "pt_BR",
        "en-",
        "en-x",
        "en-123456789",
    ] {
        let (mut documents, mut units) = fixtures();
        let requirement = &mut documents
            .documents
            .get_mut("main")
            .unwrap()
            .required_languages[0];
        requirement.value = language.to_owned();
        let requirement_site = requirement.site.clone();
        units[0].texts = BTreeMap::from([(
            language.to_owned(),
            entry("Text.", &["unit", "texts", language]),
        )]);
        let mut declaration = expected(
            RuleId::M040,
            UnitStage::Metadata,
            Some(language),
            requirement_site,
            UnitFault::InvalidLanguage {
                value: language.to_owned(),
            },
        );
        declaration.unit = None;
        refusal(
            documents,
            units,
            vec![
                declaration,
                expected(
                    RuleId::M040,
                    UnitStage::Metadata,
                    Some(language),
                    site("statement.toml", &["unit", "texts", language]),
                    UnitFault::InvalidLanguage {
                        value: language.to_owned(),
                    },
                ),
            ],
        );
    }
}
#[test]
fn u06_main_empty_missing_and_non_english_contracts() {
    let (mut documents, units) = fixtures();
    documents.documents.clear();
    let mut missing = expected(
        RuleId::M040,
        UnitStage::Metadata,
        None,
        site("documents.toml", &["documents"]).parent(),
        UnitFault::MissingMain,
    );
    missing.unit = None;
    refusal(
        documents,
        units,
        vec![
            expected(
                RuleId::M039,
                UnitStage::Metadata,
                None,
                site("statement.toml", &["unit", "document"]),
                UnitFault::UnknownDocument {
                    name: "main".to_owned(),
                    available: vec![],
                },
            ),
            missing,
        ],
    );
    for languages in [vec![], vec!["fr"], vec!["en", "fr"]] {
        let (mut documents, units) = fixtures();
        documents
            .documents
            .get_mut("main")
            .unwrap()
            .required_languages = languages
            .iter()
            .map(|s| LocatedString {
                value: (*s).to_owned(),
                site: site(
                    "documents.toml",
                    &["documents", "main", "required-languages"],
                ),
            })
            .collect();
        let fault = if languages.is_empty() {
            UnitFault::EmptyRequiredLanguages {
                document: "main".to_owned(),
            }
        } else {
            UnitFault::MainLanguages {
                actual: languages.iter().map(|s| (*s).to_owned()).collect(),
            }
        };
        let mut diagnostic = expected(
            RuleId::M040,
            UnitStage::Metadata,
            None,
            site(
                "documents.toml",
                &["documents", "main", "required-languages"],
            ),
            fault,
        );
        diagnostic.unit = None;
        refusal(documents, units, vec![diagnostic]);
    }
}
#[test]
fn u06_missing_extra_blank_and_two_unit_language_change() {
    let (documents, mut units) = fixtures();
    units[0].texts.clear();
    let required_at = documents.documents["main"].required_languages[0]
        .site
        .clone();
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M040,
            UnitStage::Language,
            Some("en"),
            site("statement.toml", &["unit", "texts", "en"]).parent(),
            UnitFault::MissingLanguage {
                document: "main".to_owned(),
                required_at,
            },
        )],
    );
    let (documents, mut units) = fixtures();
    units[0]
        .texts
        .insert("fr".to_owned(), entry("Texte.", &["unit", "texts", "fr"]));
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M040,
            UnitStage::Language,
            Some("fr"),
            site("statement.toml", &["unit", "texts", "fr"]),
            UnitFault::UndeclaredLanguage {
                document: "main".to_owned(),
                required: vec!["en".to_owned()],
            },
        )],
    );
    for text in ["", " \n\t\u{2003}"] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = text.to_owned();
        refusal(
            documents,
            units,
            vec![expected(
                RuleId::M040,
                UnitStage::Language,
                Some("en"),
                site("statement.toml", &["unit", "texts", "en"]),
                UnitFault::BlankText,
            )],
        );
    }
    let (mut documents, mut units) = fixtures();
    let mut contract = documents.documents["main"].clone();
    let requirement = LocatedString {
        value: "fr".to_owned(),
        site: site(
            "documents.toml",
            &["documents", "explainer", "required-languages"],
        ),
    };
    contract.required_languages.push(requirement.clone());
    documents.documents.insert("explainer".to_owned(), contract);
    units[0].document.value = "explainer".to_owned();
    let mut second = units[0].clone();
    second.id.value = "sec:coefficients:second".to_owned();
    second.texts_site.file = "second.toml".to_owned();
    units.push(second);
    let diagnostics = units
        .iter()
        .map(|u| UnitDiagnostic {
            rule: RuleId::M040,
            stage: UnitStage::Language,
            unit: Some(u.id.value.clone()),
            language: Some("fr".to_owned()),
            site: u.texts_site.parent().child("fr"),
            fault: UnitFault::MissingLanguage {
                document: "explainer".to_owned(),
                required_at: requirement.site.clone(),
            },
        })
        .collect();
    refusal(documents.clone(), units.clone(), diagnostics);
    for unit in &mut units {
        unit.texts
            .insert("fr".to_owned(), entry("Texte.", &["unit", "texts", "fr"]));
    }
    assert_eq!(
        admit_unit_drafts(documents, units).unwrap().units().len(),
        2
    );
}
#[test]
fn u08_math_faults_and_usage_suppression() {
    for (name, text, fault) in [
        (
            "Bad",
            "x",
            UnitFault::InvalidMathName {
                value: "Bad".to_owned(),
            },
        ),
        (
            "value",
            " ",
            UnitFault::BlankMath {
                name: "value".to_owned(),
            },
        ),
    ] {
        let (documents, mut units) = fixtures();
        units[0]
            .math
            .insert(name.to_owned(), entry(text, &["unit", "math", name]));
        let first = expected(
            RuleId::M041,
            UnitStage::Content,
            None,
            site("statement.toml", &["unit", "math", name]),
            fault,
        );
        let unused = expected(
            RuleId::M041,
            UnitStage::Content,
            None,
            site("statement.toml", &["unit", "math", name]),
            UnitFault::UnusedMath {
                name: name.to_owned(),
            },
        );
        refusal(documents, units, vec![first, unused]);
    }
    let (documents, mut units) = fixtures();
    units[0]
        .math
        .insert("value".to_owned(), entry("x", &["unit", "math", "value"]));
    refusal(
        documents.clone(),
        units.clone(),
        vec![expected(
            RuleId::M041,
            UnitStage::Content,
            None,
            site("statement.toml", &["unit", "math", "value"]),
            UnitFault::UnusedMath {
                name: "value".to_owned(),
            },
        )],
    );
    units[0].texts.get_mut("en").unwrap().value.value = "See @bad{c:1}.".to_owned();
    let mut location = site("statement.toml", &["unit", "texts", "en"]);
    location.decoded = Some(4..6);
    location.occurrence = Some(0);
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M041,
            UnitStage::Content,
            Some("en"),
            location,
            UnitFault::MalformedInsertion {
                reason: InsertionReason::UnknownForm,
                fragment: "@b".to_owned(),
            },
        )],
    );
    let (documents, mut units) = fixtures();
    units[0].texts.get_mut("en").unwrap().value.value = "See @math{value}.".to_owned();
    let mut location = site("statement.toml", &["unit", "texts", "en"]);
    location.decoded = Some(4..16);
    location.occurrence = Some(0);
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M041,
            UnitStage::Content,
            Some("en"),
            location,
            UnitFault::UnknownMath {
                name: "value".to_owned(),
                available: vec![],
            },
        )],
    );
}
#[test]
fn u08_all_forbidden_math_tokens_and_verbatim_positive() {
    let tokens = [
        "$", "`", "@", "\\(", "\\)", "\\[", "\\]", "\\begin", "\\end",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(
        (0..=0x9f)
            .filter_map(char::from_u32)
            .filter(|c| c.is_control() && !matches!(c, '\t' | '\n'))
            .map(|c| c.to_string()),
    );
    for token in tokens {
        let (documents, mut units) = fixtures();
        units[0].math.insert(
            "value".to_owned(),
            entry(&format!("x{token}"), &["unit", "math", "value"]),
        );
        units[0].texts.get_mut("en").unwrap().value.value = "See @math{value}.".to_owned();
        let mut location = site("statement.toml", &["unit", "math", "value"]);
        location.decoded = Some(1..1 + token.len());
        refusal(
            documents,
            units,
            vec![expected(
                RuleId::M041,
                UnitStage::Content,
                None,
                location,
                UnitFault::UnsupportedMathSyntax {
                    name: "value".to_owned(),
                    token,
                },
            )],
        );
    }
    let (documents, mut units) = fixtures();
    units[0].math.insert(
        "value".to_owned(),
        entry("c_1 = \\frac{1}{3}", &["unit", "math", "value"]),
    );
    units[0].texts.get_mut("en").unwrap().value.value = "é @math{value}\n@math{value}".to_owned();
    let checked = admit_unit_drafts(documents.clone(), units.clone()).unwrap();
    assert_eq!(
        checked.units()[0].math()["value"].value,
        "c_1 = \\frac{1}{3}"
    );
    let occurrences = checked.units()[0].texts()["en"].occurrences();
    assert_eq!(
        occurrences
            .iter()
            .map(|o| (o.name(), o.site().decoded.clone(), o.site().occurrence))
            .collect::<Vec<_>>(),
        vec![
            ("value", Some(3..15), Some(0)),
            ("value", Some(16..28), Some(1))
        ]
    );
    units[0].texts.get_mut("en").unwrap().value.value = "@math{value}".to_owned();
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M040,
            UnitStage::Language,
            Some("en"),
            site("statement.toml", &["unit", "texts", "en"]),
            UnitFault::BlankText,
        )],
    );
}
#[test]
fn register_final_unit_statements() {
    let expected = [
        (
            RuleId::M039,
            "Unit metadata carries a well-formed label unique within the admitted unit corpus and references a declared document language contract; document names obey the admitted grammar.",
            "Statement-unit metadata and document declaration names in one corpus admission.",
        ),
        (
            RuleId::M040,
            "Documents declare nonempty distinct language sets, a main document is declared and requires English alone, and each unit has exactly one nonblank authored text per required language and no text for an undeclared language; language tokens obey the admitted grammar.",
            "Document language declarations and all associated statement-unit texts.",
        ),
        (
            RuleId::M041,
            "Statement texts use the admitted insertion syntax and reference only their unit's shared nonblank TeX math entries, each of which is used; ordinary prose and math entries use only the admitted characters and fragment syntax, and math names obey the local-name grammar.",
            "Every authored statement text, shared math entry and local math insertion.",
        ),
    ];
    assert_eq!(
        crate::RULES[38..41]
            .iter()
            .map(|r| (r.number, r.statement, r.scope))
            .collect::<Vec<_>>(),
        expected
    );
    for rule in &crate::RULES[38..41] {
        assert_eq!(
            (rule.state, rule.deferred_reason),
            (crate::RuleState::Enforced, None)
        );
    }
}

#[test]
fn u06_duplicate_requirement_sites_and_invalid_document_key() {
    let (mut documents, units) = fixtures();
    let main = documents.documents.get_mut("main").unwrap();
    let mut second = main.required_languages[0].clone();
    second.site.path.pop();
    second.site.path.push(PathPart::Index(1));
    main.required_languages.push(second);
    let sites: Vec<_> = main
        .required_languages
        .iter()
        .map(|l| l.site.clone())
        .collect();
    let expected = sites
        .iter()
        .map(|site| UnitDiagnostic {
            rule: RuleId::M040,
            stage: UnitStage::Metadata,
            unit: None,
            language: Some("en".to_owned()),
            site: site.clone(),
            fault: UnitFault::DuplicateRequiredLanguage {
                document: "main".to_owned(),
                language: "en".to_owned(),
                sites: sites.clone(),
            },
        })
        .collect();
    refusal(documents, units, expected);
    let (mut documents, units) = fixtures();
    let mut other = documents.documents["main"].clone();
    other.key_site = site("documents.toml", &["documents", "Bad"]);
    documents.documents.insert("Bad".to_owned(), other);
    refusal(
        documents,
        units,
        vec![UnitDiagnostic {
            rule: RuleId::M039,
            stage: UnitStage::Metadata,
            unit: None,
            language: None,
            site: site("documents.toml", &["documents", "Bad"]),
            fault: UnitFault::InvalidName {
                domain: NameDomain::DocumentName,
                value: "Bad".to_owned(),
            },
        }],
    );
}
#[test]
fn u08_bilingual_shared_math_different_reference_sets_and_each_missing_slot() {
    let (mut documents, mut units) = fixtures();
    let mut contract = documents.documents["main"].clone();
    contract.required_languages.push(LocatedString {
        value: "fr".to_owned(),
        site: site(
            "documents.toml",
            &["documents", "explainer", "required-languages"],
        ),
    });
    documents
        .documents
        .insert("explainer".to_owned(), contract.clone());
    units[0].document.value = "explainer".to_owned();
    units[0].texts.get_mut("en").unwrap().value.value = "See @math{one} and @math{two}.".to_owned();
    units[0].texts.insert(
        "fr".to_owned(),
        entry("Voir @math{one}.", &["unit", "texts", "fr"]),
    );
    for (name, tex) in [("one", "x"), ("two", "y")] {
        units[0]
            .math
            .insert(name.to_owned(), entry(tex, &["unit", "math", name]));
    }
    let corpus = admit_unit_drafts(documents.clone(), units.clone()).unwrap();
    assert_eq!(corpus.units()[0].texts()["en"].occurrences().len(), 2);
    assert_eq!(corpus.units()[0].texts()["fr"].occurrences().len(), 1);
    // Keep both math entries used when independently deleting either language.
    units[0].texts.get_mut("fr").unwrap().value.value = "Voir @math{one} et @math{two}.".to_owned();
    for language in ["en", "fr"] {
        let mut missing = units.clone();
        missing[0].texts.remove(language);
        let required_at = contract
            .required_languages
            .iter()
            .find(|l| l.value == language)
            .unwrap()
            .site
            .clone();
        refusal(
            documents.clone(),
            missing,
            vec![expected(
                RuleId::M040,
                UnitStage::Language,
                Some(language),
                site("statement.toml", &["unit", "texts", language]).parent(),
                UnitFault::MissingLanguage {
                    document: "explainer".to_owned(),
                    required_at,
                },
            )],
        );
    }
}
#[test]
fn diagnostic_displays_have_literal_payloads_and_anchor_forms() {
    for (fault, text) in [
        (UnitFault::InvalidUtf8, "InvalidUtf8"),
        (UnitFault::Bom, "Bom"),
        (
            UnitFault::TomlSyntax {
                message: "duplicate\nkey".to_owned(),
            },
            "TomlSyntax { message: \"duplicate\\nkey\" }",
        ),
        (
            UnitFault::NonIntegerNumber {
                token: "+nan".to_owned(),
            },
            "NonIntegerNumber { token: \"+nan\" }",
        ),
        (
            UnitFault::SchemaValue {
                expected: "1".to_owned(),
                actual: "+1".to_owned(),
            },
            "SchemaValue { expected: \"1\", actual: \"+1\" }",
        ),
        (
            UnitFault::UnknownField {
                name: "kind".to_owned(),
                expected: vec!["id".to_owned()],
            },
            "UnknownField { name: \"kind\", expected: [\"id\"] }",
        ),
        (
            UnitFault::MissingField {
                name: "math".to_owned(),
            },
            "MissingField { name: \"math\" }",
        ),
        (
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Boolean,
            },
            "WrongType { expected: String, actual: Boolean }",
        ),
        (
            UnitFault::InvalidName {
                domain: NameDomain::UnitLabel,
                value: "x".to_owned(),
            },
            "InvalidName { domain: UnitLabel, value: \"x\" }",
        ),
        (
            UnitFault::DuplicateUnit {
                id: "a:b:c".to_owned(),
                sites: vec![],
            },
            "DuplicateUnit { id: \"a:b:c\", sites: [] }",
        ),
        (
            UnitFault::UnknownDocument {
                name: "x".to_owned(),
                available: vec!["main".to_owned()],
            },
            "UnknownDocument { name: \"x\", available: [\"main\"] }",
        ),
    ] {
        assert_eq!(fault.to_string(), text);
    }
}
#[test]
fn semantic_fault_displays_have_literal_payloads() {
    for (fault, text) in [
        (UnitFault::MissingMain, "MissingMain"),
        (
            UnitFault::InvalidLanguage {
                value: "EN".to_owned(),
            },
            "InvalidLanguage { value: \"EN\" }",
        ),
        (
            UnitFault::EmptyRequiredLanguages {
                document: "main".to_owned(),
            },
            "EmptyRequiredLanguages { document: \"main\" }",
        ),
        (
            UnitFault::DuplicateRequiredLanguage {
                document: "main".to_owned(),
                language: "en".to_owned(),
                sites: vec![],
            },
            "DuplicateRequiredLanguage { document: \"main\", language: \"en\", sites: [] }",
        ),
        (
            UnitFault::MainLanguages {
                actual: vec!["fr".to_owned()],
            },
            "MainLanguages { actual: [\"fr\"] }",
        ),
        (
            UnitFault::UndeclaredLanguage {
                document: "main".to_owned(),
                required: vec!["en".to_owned()],
            },
            "UndeclaredLanguage { document: \"main\", required: [\"en\"] }",
        ),
        (UnitFault::BlankText, "BlankText"),
        (
            UnitFault::UnsupportedTextCharacter {
                value: "$".to_owned(),
            },
            "UnsupportedTextCharacter { value: \"$\" }",
        ),
        (
            UnitFault::MalformedInsertion {
                reason: InsertionReason::UnknownForm,
                fragment: "@t".to_owned(),
            },
            "MalformedInsertion { reason: UnknownForm, fragment: \"@t\" }",
        ),
        (
            UnitFault::InvalidMathName {
                value: "X".to_owned(),
            },
            "InvalidMathName { value: \"X\" }",
        ),
        (
            UnitFault::BlankMath {
                name: "x".to_owned(),
            },
            "BlankMath { name: \"x\" }",
        ),
        (
            UnitFault::UnsupportedMathSyntax {
                name: "x".to_owned(),
                token: "\\begin".to_owned(),
            },
            "UnsupportedMathSyntax { name: \"x\", token: \"\\\\begin\" }",
        ),
        (
            UnitFault::UnknownMath {
                name: "x".to_owned(),
                available: vec!["y".to_owned()],
            },
            "UnknownMath { name: \"x\", available: [\"y\"] }",
        ),
        (
            UnitFault::UnusedMath {
                name: "x".to_owned(),
            },
            "UnusedMath { name: \"x\" }",
        ),
    ] {
        assert_eq!(fault.to_string(), text);
    }
}
#[test]
fn source_anchor_and_error_collection_displays() {
    let requirement = UnitSite {
        file: "d.toml".to_owned(),
        path: vec![PathPart::Index(2)],
        anchor: SourceAnchor::Bytes(3, 4),
        decoded: None,
        occurrence: None,
    };
    assert_eq!(
        UnitFault::MissingLanguage {
            document: "main".to_owned(),
            required_at: requirement
        }
        .to_string(),
        "MissingLanguage { document: \"main\", required_at: UnitSite { file: \"d.toml\", path: [Index(2)], anchor: Bytes(3, 4), decoded: None, occurrence: None } }"
    );
    for (anchor, rendered) in [
        (SourceAnchor::Bytes(1, 2), "bytes 1..2"),
        (SourceAnchor::Parent(1, 2), "parent 1..2"),
        (SourceAnchor::WholeFile, "whole-file"),
        (SourceAnchor::Unavailable, "unavailable"),
    ] {
        let site = UnitSite {
            file: "é\".toml".to_owned(),
            path: vec![PathPart::Key("a\"".to_owned()), PathPart::Index(2)],
            anchor,
            decoded: Some(3..5),
            occurrence: Some(0),
        };
        assert_eq!(
            site.to_string(),
            format!("\"é\\\".toml\" $[\"a\\\"\"][2] {rendered} decoded 3..5 occurrence 0")
        );
    }
    let diagnostic = UnitDiagnostic {
        rule: RuleId::M001,
        stage: UnitStage::Encoding,
        unit: None,
        language: None,
        site: UnitSite {
            file: "a".to_owned(),
            path: vec![],
            anchor: SourceAnchor::Bytes(0, 3),
            decoded: None,
            occurrence: None,
        },
        fault: UnitFault::Bom,
    };
    assert_eq!(
        UnitErrors::from_diagnostics(vec![diagnostic])
            .unwrap()
            .to_string(),
        "M001 Encoding unit=None language=None at \"a\" $ bytes 0..3: Bom\n"
    );
    assert_eq!(UnitErrors::from_diagnostics(vec![]), None);
}

#[test]
fn u06_empty_explainer_and_u07_later_occurrence_refusal() {
    let (mut documents, units) = fixtures();
    let mut explainer = documents.documents["main"].clone();
    explainer.languages_site = site(
        "documents.toml",
        &["documents", "explainer", "required-languages"],
    );
    explainer.required_languages.clear();
    documents
        .documents
        .insert("explainer".to_owned(), explainer);
    refusal(
        documents,
        units,
        vec![UnitDiagnostic {
            rule: RuleId::M040,
            stage: UnitStage::Metadata,
            unit: None,
            language: None,
            site: site(
                "documents.toml",
                &["documents", "explainer", "required-languages"],
            ),
            fault: UnitFault::EmptyRequiredLanguages {
                document: "explainer".to_owned(),
            },
        }],
    );
    let (documents, mut units) = fixtures();
    units[0]
        .math
        .insert("x".to_owned(), entry("x", &["unit", "math", "x"]));
    units[0].texts.get_mut("en").unwrap().value.value = "See @math{x} @bad{c:1}.".to_owned();
    let mut location = site("statement.toml", &["unit", "texts", "en"]);
    location.decoded = Some(13..15);
    location.occurrence = Some(1);
    refusal(
        documents,
        units,
        vec![expected(
            RuleId::M041,
            UnitStage::Content,
            Some("en"),
            location,
            UnitFault::MalformedInsertion {
                reason: InsertionReason::UnknownForm,
                fragment: "@b".to_owned(),
            },
        )],
    );
}

fn bilingual(en: &str, fr: &str) -> LanguageCheckedUnits {
    let (mut documents, mut units) = fixtures();
    let mut explainer = documents.documents["main"].clone();
    explainer.required_languages.push(LocatedString {
        value: "fr".into(),
        site: site(
            "documents.toml",
            &["documents", "explainer", "required-languages"],
        ),
    });
    documents.documents.insert("explainer".into(), explainer);
    units[0].document.value = "explainer".into();
    units[0].texts = [("en", en), ("fr", fr)]
        .into_iter()
        .map(|(language, text)| (language.into(), entry(text, &["unit", "texts", language])))
        .collect();
    admit_unit_drafts(documents, units).unwrap()
}
fn positions(input: &str) -> Vec<(String, Vec<std::ops::Range<usize>>)> {
    let size = input.len() + 6;
    vec![
        (
            format!("@tag{{{input}}} begins."),
            std::iter::once(0..size).collect(),
        ),
        (
            format!("See @tag{{{input}}} here."),
            std::iter::once(4..4 + size).collect(),
        ),
        (
            format!("See @tag{{{input}}}"),
            std::iter::once(4..4 + size).collect(),
        ),
        (
            format!("See @tag{{{input}}} and @tag{{{input}}}."),
            vec![4..4 + size, 9 + size..9 + size * 2],
        ),
    ]
}
fn tag_site(language: &str, range: std::ops::Range<usize>, index: usize) -> UnitSite {
    UnitSite {
        decoded: Some(range),
        occurrence: Some(index),
        ..site("statement.toml", &["unit", "texts", language])
    }
}
fn resolution_fault(
    language: &str,
    range: std::ops::Range<usize>,
    index: usize,
    cause: crate::TagResolutionFault,
) -> UnitDiagnostic {
    expected(
        RuleId::M042,
        UnitStage::Resolution,
        Some(language),
        tag_site(language, range, index),
        UnitFault::ResolutionFault { cause },
    )
}
fn resolution_matrix(
    book: &AddressBook<'_>,
    input: &str,
    cause: &crate::TagResolutionFault,
    message: &str,
) {
    use std::fmt::Write;
    let restored = crate::seed().unwrap();
    let restored_book = crate::addressing(&restored).unwrap();
    for (text, ranges) in positions(input) {
        let corpus = bilingual(&text, &text);
        let mut faults = Vec::new();
        let mut display = String::new();
        for language in ["en", "fr"] {
            for (index, range) in ranges.iter().enumerate() {
                faults.push(resolution_fault(
                    language,
                    range.clone(),
                    index,
                    cause.clone(),
                ));
                writeln!(display,"M042 Resolution unit=Some(\"sec:coefficients:first-value\") language=Some({language:?}) at \"statement.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes 10..90 decoded {}..{} occurrence {index}: {message}",range.start,range.end).unwrap();
            }
        }
        let error = resolve_unit_tags(&corpus, book).unwrap_err();
        assert_eq!(error, UnitErrors::from_diagnostics(faults).unwrap());
        assert_eq!(error.to_string(), display);
        assert_eq!(error.diagnostics()[0].fault.to_string(), message);
        if let UnitFault::ResolutionFault { cause } = &error.diagnostics()[0].fault {
            assert_eq!(cause.to_string(), message);
        } else {
            panic!("resolution fault required");
        }
        // Restore the live spelling independently for every positional fixture.
        let live = text.replace(&format!("@tag{{{input}}}"), "@tag{c:1}");
        let control = bilingual(&live, &live);
        assert_eq!(
            resolve_unit_tags(&control, &restored_book)
                .unwrap()
                .tags()
                .len(),
            ranges.len() * 2
        );
    }
}
#[test]
fn u10_all_live_tags_every_position_language_and_bound_context() {
    let graph = crate::seed().unwrap();
    let before = crate::Snapshot::capture(&graph);
    let statuses = crate::NodeId::ALL.map(|id| graph.status(id));
    let book = crate::addressing(&graph).unwrap();
    for (input, target) in [
        ("c:1", NodeId::LangevinCoefficientOne),
        ("c:3", NodeId::LangevinCoefficientThree),
        ("v:1", NodeId::LangevinCoefficientOneValue),
        ("v:3", NodeId::LangevinCoefficientThreeValue),
    ] {
        for (text, ranges) in positions(input) {
            let corpus = bilingual(&text, &text);
            let resolved = resolve_unit_tags(&corpus, &book).unwrap();
            assert!(std::ptr::eq(resolved.book(), &raw const book));
            assert!(std::ptr::eq(resolved.corpus(), &raw const corpus));
            let expected: Vec<_> = ["en", "fr"]
                .into_iter()
                .flat_map(|language| {
                    ranges.iter().enumerate().map(move |(index, range)| {
                        (
                            "sec:coefficients:first-value",
                            language,
                            input,
                            tag_site(language, range.clone(), index),
                            target,
                        )
                    })
                })
                .collect();
            let actual: Vec<_> = resolved
                .tags()
                .iter()
                .map(|tag| {
                    (
                        tag.unit().as_str(),
                        tag.language(),
                        tag.occurrence().input(),
                        tag.occurrence().site().clone(),
                        tag.target(),
                    )
                })
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(graph.node(target).unwrap().id, target);
            assert_eq!(graph.status(target), statuses[target as usize]);
        }
    }
    assert_eq!(crate::Snapshot::capture(&graph), before);
    assert_eq!(crate::NodeId::ALL.map(|id| graph.status(id)), statuses);
}
#[test]
fn u11_unknown_payloads_every_position_language_and_literal_display() {
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    for input in [
        "c:01",
        "C:1",
        " c:1",
        "c:1 ",
        "",
        "def:langevin:coefficient-one",
        "x\u{0}y",
        "x\r\ny",
        "x\\y",
        "`$<>",
    ] {
        resolution_matrix(
            &book,
            input,
            &crate::TagResolutionFault::UnknownTag {
                input: input.into(),
            },
            &format!("tag {input:?}: unknown"),
        );
    }
}
fn unit_slice(ids: &[NodeId]) -> crate::ValidatedManifest {
    let mut draft = crate::seed_draft();
    draft.coverage.slice = ids.to_vec();
    crate::validate(draft).unwrap()
}
fn absent_three() -> crate::ValidatedManifest {
    let mut draft = crate::seed_draft();
    draft
        .nodes
        .retain(|node| node.id != NodeId::LangevinCoefficientThree);
    draft.edges.retain(|edge| {
        edge.source != NodeId::LangevinCoefficientThree
            && edge.target != NodeId::LangevinCoefficientThree
    });
    draft.coverage.slice.clear();
    crate::validate(draft).unwrap()
}
#[test]
fn u12_out_of_slice_every_position_language_and_cause() {
    use crate::{OutOfSliceCause as O, TagId as T};
    let one = unit_slice(&[NodeId::LangevinCoefficientOneValue]);
    let empty = unit_slice(&[]);
    let absent = absent_three();
    let quantity = [
        (
            "c:1",
            T::LangevinCoefficientOne,
            NodeId::LangevinCoefficientOne,
            O::QuantityOutsideSupport,
            "def:langevin:coefficient-one",
            "quantity-outside-support",
        ),
        (
            "c:3",
            T::LangevinCoefficientThree,
            NodeId::LangevinCoefficientThree,
            O::QuantityOutsideSupport,
            "def:langevin:coefficient-three",
            "quantity-outside-support",
        ),
    ];
    let claims = [
        (
            "v:1",
            T::LangevinCoefficientOneValue,
            NodeId::LangevinCoefficientOneValue,
            O::ClaimNotSelected,
            "claim:langevin:coefficient-one-value",
            "claim-not-selected",
        ),
        (
            "v:3",
            T::LangevinCoefficientThreeValue,
            NodeId::LangevinCoefficientThreeValue,
            O::ClaimNotSelected,
            "claim:langevin:coefficient-three-value",
            "claim-not-selected",
        ),
    ];
    for (graph, cases) in [
        (&one, vec![quantity[1], claims[1]]),
        (&empty, vec![quantity[0], quantity[1], claims[0], claims[1]]),
        (
            &absent,
            vec![(
                "c:3",
                T::LangevinCoefficientThree,
                NodeId::LangevinCoefficientThree,
                O::TargetAbsent,
                "def:langevin:coefficient-three",
                "target-absent",
            )],
        ),
    ] {
        let book = crate::addressing(graph).unwrap();
        let before = crate::Snapshot::capture(graph);
        for (input, tag, target, cause, label, reason) in cases {
            resolution_matrix(
                &book,
                input,
                &crate::TagResolutionFault::OutOfSliceTag {
                    input: input.into(),
                    tag,
                    target,
                    cause,
                },
                &format!("tag {input:?}: out of slice; label {label}; cause {reason}"),
            );
            let full = crate::seed().unwrap();
            let restored = crate::addressing(&full).unwrap();
            let corpus = bilingual(
                &format!("See @tag{{{input}}}."),
                &format!("See @tag{{{input}}}."),
            );
            assert_eq!(
                resolve_unit_tags(&corpus, &restored).unwrap().tags()[0].target(),
                target
            );
        }
        assert_eq!(crate::Snapshot::capture(graph), before);
    }
}
fn retirement_event(ordinal: u32, change: crate::HistoryChange) -> crate::HistoryEvent {
    crate::HistoryEvent {
        ordinal,
        authority_record: "rec:units:retirement".into(),
        reason: "Withdrawn for correction.".into(),
        change,
    }
}
#[test]
fn u13_private_retirement_every_position_language_and_earliest_event() {
    use crate::{HistoryChange as H, RetirementCause as R, TagId as T};
    let full = crate::seed().unwrap();
    let one = unit_slice(&[NodeId::LangevinCoefficientOneValue]);
    let absent = absent_three();
    let tag_event = retirement_event(
        9,
        H::RetireTag {
            tag: T::LangevinCoefficientThree,
        },
    );
    let identity_event = retirement_event(
        9,
        H::RetireIdentity {
            identity: NodeId::LangevinCoefficientThree,
        },
    );
    for (graph, events, cause) in [
        (
            &full,
            vec![tag_event.clone()],
            R::TagRetirement(tag_event.clone()),
        ),
        (
            &one,
            vec![tag_event.clone()],
            R::TagRetirement(tag_event.clone()),
        ),
        (
            &absent,
            vec![tag_event.clone()],
            R::TagRetirement(tag_event.clone()),
        ),
        (
            &absent,
            vec![identity_event.clone()],
            R::IdentityRetirement(identity_event),
        ),
        (
            &absent,
            vec![
                tag_event.clone(),
                retirement_event(
                    10,
                    H::RetireIdentity {
                        identity: NodeId::LangevinCoefficientThree,
                    },
                ),
            ],
            R::TagRetirement(tag_event),
        ),
    ] {
        let mut history = crate::tag_history::seed_history();
        history.extend(events.clone());
        let book = crate::tag_validation::construct(
            graph,
            crate::TagId::ALL.map(crate::TagId::descriptor).to_vec(),
            history,
        )
        .unwrap();
        assert_eq!(book.history().skip(8).cloned().collect::<Vec<_>>(), events);
        let before = crate::Snapshot::capture(graph);
        resolution_matrix(
            &book,
            "c:3",
            &crate::TagResolutionFault::RetiredTag {
                input: "c:3".into(),
                tag: T::LangevinCoefficientThree,
                target: NodeId::LangevinCoefficientThree,
                cause,
            },
            "tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority rec:units:retirement; reason \"Withdrawn for correction.\"",
        );
        assert_eq!(crate::Snapshot::capture(graph), before);
        let restored = crate::addressing(&full).unwrap();
        let corpus = bilingual("See @tag{c:3}.", "See @tag{c:3}.");
        assert_eq!(
            resolve_unit_tags(&corpus, &restored).unwrap().tags()[0].target(),
            NodeId::LangevinCoefficientThree
        );
    }
}
#[test]
fn u14_different_live_tag_sets_pass_without_weld() {
    let graph = crate::seed().unwrap();
    let book = crate::addressing(&graph).unwrap();
    let corpus = bilingual("See @tag{c:1}.", "Voir @tag{v:3} et @tag{c:3}.");
    let result = resolve_unit_tags(&corpus, &book).unwrap();
    assert_eq!(
        result
            .tags()
            .iter()
            .map(|tag| (tag.language(), tag.target()))
            .collect::<Vec<_>>(),
        vec![
            ("en", NodeId::LangevinCoefficientOne),
            ("fr", NodeId::LangevinCoefficientThreeValue),
            ("fr", NodeId::LangevinCoefficientThree)
        ]
    );
}

#[test]
fn u07_tag_boundaries_empty_payload_and_mixed_occurrence_indices() {
    for (text, reason, fragment, range) in [
        ("See @taX{c:1}.", InsertionReason::UnknownForm, "@taX", 4..8),
        ("See @ta", InsertionReason::Unclosed, "@ta", 4..7),
        ("See @tag{c:1", InsertionReason::Unclosed, "@tag{c:1", 4..12),
        (
            "See @tag{x{y}.",
            InsertionReason::NestedBrace,
            "@tag{x{",
            4..11,
        ),
    ] {
        let (documents, mut units) = fixtures();
        units[0].texts.get_mut("en").unwrap().value.value = text.into();
        let fault = expected(
            RuleId::M041,
            UnitStage::Content,
            Some("en"),
            tag_site("en", range.clone(), 0),
            UnitFault::MalformedInsertion {
                reason,
                fragment: fragment.into(),
            },
        );
        let error = admit_unit_drafts(documents, units).unwrap_err();
        assert_eq!(error, UnitErrors::from_diagnostics(vec![fault]).unwrap());
        assert_eq!(
            error.to_string(),
            format!(
                "M041 Content unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"statement.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 10..90 decoded {}..{} occurrence 0: MalformedInsertion {{ reason: {reason:?}, fragment: {fragment:?} }}\n",
                range.start, range.end
            )
        );
    }
    let corpus = bilingual("See @tag{}.", "See @tag{}.");
    assert_eq!(
        corpus.units()[0].texts()["en"].tag_occurrences()[0].input(),
        ""
    );
    assert_eq!(
        corpus.units()[0].texts()["en"].tag_occurrences()[0].site(),
        &tag_site("en", 4..10, 0)
    );
    let (documents, mut units) = fixtures();
    units[0].texts.get_mut("en").unwrap().value.value = " @tag{c:1} \n".into();
    refusal(
        documents.clone(),
        units.clone(),
        vec![expected(
            RuleId::M040,
            UnitStage::Language,
            Some("en"),
            site("statement.toml", &["unit", "texts", "en"]),
            UnitFault::BlankText,
        )],
    );
    units[0]
        .math
        .insert("x".into(), entry("x", &["unit", "math", "x"]));
    units[0].texts.get_mut("en").unwrap().value.value = "é @tag{c:1}\n@math{x} @tag{v:1}.".into();
    let corpus = admit_unit_drafts(documents, units).unwrap();
    let text = &corpus.units()[0].texts()["en"];
    assert_eq!(
        text.tag_occurrences()
            .iter()
            .map(|o| (o.input(), o.site().clone()))
            .collect::<Vec<_>>(),
        vec![
            ("c:1", tag_site("en", 3..12, 0)),
            ("v:1", tag_site("en", 22..31, 2))
        ]
    );
    assert_eq!(text.occurrences()[0].site(), &tag_site("en", 13..21, 1));
}
#[test]
fn u14_complete_ordered_mixed_refusals_keep_repetitions_and_status() {
    use crate::{HistoryChange as H, OutOfSliceCause as O, RetirementCause as R, TagId as T};
    use std::fmt::Write;
    let graph = unit_slice(&[NodeId::LangevinCoefficientOneValue]);
    let before = crate::Snapshot::capture(&graph);
    let statuses = NodeId::ALL.map(|id| graph.status(id));
    let event = retirement_event(
        9,
        H::RetireTag {
            tag: T::LangevinCoefficientThree,
        },
    );
    let mut history = crate::tag_history::seed_history();
    history.push(event.clone());
    let book =
        crate::tag_validation::construct(&graph, T::ALL.map(T::descriptor).to_vec(), history)
            .unwrap();
    let mut corpus = bilingual(
        "See @tag{c:1} @tag{bad} @tag{c:3} @tag{v:3} @tag{bad}.",
        "See @tag{c:1} @tag{bad} @tag{c:3} @tag{v:3} @tag{bad}.",
    );
    // Deliberately reverse the unit input order: semantic diagnostics sort by ID.
    let mut second = corpus.units[0].clone();
    second.label = UnitLabel("sec:coefficients:second".into());
    corpus.units.insert(0, second);
    let mut expected_faults = Vec::new();
    let mut display = String::new();
    for unit in ["sec:coefficients:first-value", "sec:coefficients:second"] {
        for language in ["en", "fr"] {
            for (index, range, cause, message) in [
                (
                    1,
                    14..23,
                    crate::TagResolutionFault::UnknownTag {
                        input: "bad".into(),
                    },
                    "tag \"bad\": unknown",
                ),
                (
                    2,
                    24..33,
                    crate::TagResolutionFault::RetiredTag {
                        input: "c:3".into(),
                        tag: T::LangevinCoefficientThree,
                        target: NodeId::LangevinCoefficientThree,
                        cause: R::TagRetirement(event.clone()),
                    },
                    "tag \"c:3\": retired; label def:langevin:coefficient-three; event 9; authority rec:units:retirement; reason \"Withdrawn for correction.\"",
                ),
                (
                    3,
                    34..43,
                    crate::TagResolutionFault::OutOfSliceTag {
                        input: "v:3".into(),
                        tag: T::LangevinCoefficientThreeValue,
                        target: NodeId::LangevinCoefficientThreeValue,
                        cause: O::ClaimNotSelected,
                    },
                    "tag \"v:3\": out of slice; label claim:langevin:coefficient-three-value; cause claim-not-selected",
                ),
                (
                    4,
                    44..53,
                    crate::TagResolutionFault::UnknownTag {
                        input: "bad".into(),
                    },
                    "tag \"bad\": unknown",
                ),
            ] {
                let mut fault = resolution_fault(language, range.clone(), index, cause);
                fault.unit = Some(unit.into());
                expected_faults.push(fault);
                writeln!(display,"M042 Resolution unit=Some({unit:?}) language=Some({language:?}) at \"statement.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes 10..90 decoded {}..{} occurrence {index}: {message}",range.start,range.end).unwrap();
            }
        }
    }
    let errors = resolve_unit_tags(&corpus, &book).unwrap_err();
    assert_eq!(
        errors,
        UnitErrors::from_diagnostics(expected_faults).unwrap()
    );
    assert_eq!(errors.to_string(), display);
    assert_eq!(crate::Snapshot::capture(&graph), before);
    assert_eq!(NodeId::ALL.map(|id| graph.status(id)), statuses);
    let full = crate::seed().unwrap();
    let restored = crate::addressing(&full).unwrap();
    let control = bilingual("See @tag{c:1} @tag{c:3} @tag{v:3}.", "See @tag{v:1}.");
    assert_eq!(
        resolve_unit_tags(&control, &restored).unwrap().tags().len(),
        4
    );
}
#[test]
fn register_m042_literal_statement_scope_and_state() {
    let rule = &crate::RULES[41];
    assert_eq!(rule.number, RuleId::M042);
    assert_eq!(
        rule.statement,
        "Every tag-facing statement-text occurrence resolves through the supplied AddressBook's live resolver or refuses admission with its unit, language and site."
    );
    assert_eq!(
        rule.scope,
        "All tag insertions in every admitted language text."
    );
    assert_eq!(rule.state, crate::RuleState::Enforced);
    assert_eq!(rule.deferred_reason, None);
}
