use super::*;
use ftoo_manifest::{PathPart, RuleId, ShapeKind, SourceAnchor, UnitFault, UnitSite, UnitStage};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
const DECLARATION: &str = "schema = 1\n[documents.main]\nrequired-languages = ['en']\n";
const STATEMENT: &str = "schema = 1\n[unit]\nid = 'sec:coefficients:first-value'\ndocument = 'main'\nmath = {}\n[unit.texts]\nen = 'The coefficient.'\n";
fn source(file: &str, text: &str) -> UnitSource {
    UnitSource {
        file: file.to_owned(),
        bytes: text.as_bytes().to_vec(),
    }
}
fn load(declaration: &str, statement: &str) -> Result<LanguageCheckedUnits, UnitLoadError> {
    load_unit_sources(
        source("units/documents.toml", declaration),
        vec![source("units/statements/one.toml", statement)],
    )
}
fn location(file: &str, path: &[&str], anchor: SourceAnchor) -> UnitSite {
    UnitSite {
        file: file.to_owned(),
        path: path
            .iter()
            .map(|k| PathPart::Key((*k).to_owned()))
            .collect(),
        anchor,
        decoded: None,
        occurrence: None,
    }
}
fn span(text: &str, token: &str) -> SourceAnchor {
    let start = text.find(token).unwrap();
    SourceAnchor::Bytes(start, start + token.len())
}
fn diagnostic(
    rule: RuleId,
    stage: UnitStage,
    unit: Option<&str>,
    language: Option<&str>,
    site: UnitSite,
    fault: UnitFault,
) -> UnitDiagnostic {
    UnitDiagnostic {
        rule,
        stage,
        unit: unit.map(str::to_owned),
        language: language.map(str::to_owned),
        site,
        fault,
    }
}
fn errors(expected: Vec<UnitDiagnostic>) -> UnitLoadError {
    UnitLoadError::Admission(UnitErrors::from_diagnostics(expected).unwrap())
}
fn assert_fault(declaration: &str, statement: &str, expected: UnitDiagnostic) {
    assert_eq!(load(declaration, statement), Err(errors(vec![expected])));
}
#[test]
fn u04_shape_context_four_discriminating_cases() {
    for (original, replacement, path, unit, language) in [
        (
            "'The coefficient.'",
            "true",
            vec!["unit", "texts", "en"],
            Some("sec:coefficients:first-value"),
            Some("en"),
        ),
        (
            "{}",
            "true",
            vec!["unit", "math"],
            Some("sec:coefficients:first-value"),
            None,
        ),
        (
            "'sec:coefficients:first-value'",
            "true",
            vec!["unit", "id"],
            None,
            None,
        ),
    ] {
        let text = STATEMENT.replace(original, replacement);
        let expected_kind = if path.last() == Some(&"math") {
            ShapeKind::Table
        } else {
            ShapeKind::String
        };
        let expected = diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            unit,
            language,
            location("units/statements/one.toml", &path, span(&text, "true")),
            UnitFault::WrongType {
                expected: expected_kind,
                actual: ShapeKind::Boolean,
            },
        );
        assert_fault(DECLARATION, &text, expected.clone());
        let display = match path.last().copied().unwrap() {
            "en" => {
                "M004 Shape unit=Some(\"sec:coefficients:first-value\") language=Some(\"en\") at \"units/statements/one.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 100..104: WrongType { expected: String, actual: Boolean }"
            }
            "math" => {
                "M004 Shape unit=Some(\"sec:coefficients:first-value\") language=None at \"units/statements/one.toml\" $[\"unit\"][\"math\"] bytes 79..83: WrongType { expected: Table, actual: Boolean }"
            }
            "id" => {
                "M004 Shape unit=None language=None at \"units/statements/one.toml\" $[\"unit\"][\"id\"] bytes 23..27: WrongType { expected: String, actual: Boolean }"
            }
            _ => unreachable!(),
        };
        assert_eq!(expected.to_string(), display);
    }
    let declaration = DECLARATION.replace("['en']", "true");
    let expected = diagnostic(
        RuleId::M004,
        UnitStage::Shape,
        None,
        None,
        location(
            "units/documents.toml",
            &["documents", "main", "required-languages"],
            span(&declaration, "true"),
        ),
        UnitFault::WrongType {
            expected: ShapeKind::Array,
            actual: ShapeKind::Boolean,
        },
    );
    assert_fault(&declaration, STATEMENT, expected.clone());
    assert_eq!(
        expected.to_string(),
        "M004 Shape unit=None language=None at \"units/documents.toml\" $[\"documents\"][\"main\"][\"required-languages\"] bytes 49..53: WrongType { expected: Array, actual: Boolean }"
    );
}
#[test]
fn u07_byte_known_tag_acceptance_and_restoration() {
    let text = STATEMENT.replace("The coefficient.", "See @tag{c:1}.");
    let corpus = load(DECLARATION, &text).unwrap();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let resolved = ftoo_manifest::resolve_unit_tags(&corpus, &book).unwrap();
    let mut site = location(
        "units/statements/one.toml",
        &["unit", "texts", "en"],
        span(&text, "'See @tag{c:1}.'"),
    );
    site.decoded = Some(4..13);
    site.occurrence = Some(0);
    assert_eq!(
        resolved.tags()[0].target(),
        ftoo_manifest::NodeId::LangevinCoefficientOne
    );
    assert_eq!(resolved.tags()[0].occurrence().input(), "c:1");
    assert_eq!(resolved.tags()[0].occurrence().site(), &site);
    assert_eq!(
        site.to_string(),
        "\"units/statements/one.toml\" $[\"unit\"][\"texts\"][\"en\"] bytes 100..116 decoded 4..13 occurrence 0"
    );
    assert_eq!(
        load(DECLARATION, STATEMENT).unwrap().units()[0].texts()["en"].text(),
        "The coefficient."
    );
}

#[test]
fn u01_encoding_both_roles_and_global_precedence() {
    for declaration_role in [false, true] {
        for (bytes, anchor, fault) in [
            (
                vec![0xff],
                SourceAnchor::Bytes(0, 1),
                UnitFault::InvalidUtf8,
            ),
            (
                vec![b'a', 0xe2, 0x82],
                SourceAnchor::Bytes(1, 3),
                UnitFault::InvalidUtf8,
            ),
            (
                vec![0xef, 0xbb, 0xbf],
                SourceAnchor::Bytes(0, 3),
                UnitFault::Bom,
            ),
        ] {
            let mut declaration = source("units/documents.toml", DECLARATION);
            let mut statement = source("units/statements/one.toml", STATEMENT);
            let file = if declaration_role {
                declaration.bytes = bytes;
                &declaration.file
            } else {
                statement.bytes = bytes;
                &statement.file
            };
            let expected = diagnostic(
                RuleId::M001,
                UnitStage::Encoding,
                None,
                None,
                location(file, &[], anchor),
                fault,
            );
            assert_eq!(
                load_unit_sources(declaration, vec![statement]),
                Err(errors(vec![expected]))
            );
        }
    }
    let invalid = UnitSource {
        file: "z.toml".to_owned(),
        bytes: vec![0xff],
    };
    assert_eq!(
        load_unit_sources(source("a.toml", "schema = true"), vec![invalid]),
        Err(errors(vec![diagnostic(
            RuleId::M001,
            UnitStage::Encoding,
            None,
            None,
            location("z.toml", &[], SourceAnchor::Bytes(0, 1)),
            UnitFault::InvalidUtf8
        )]))
    );
}
#[test]
fn u01_collection_exact_wrappers() {
    for name in ["", "/a", "a//b", "./a", "a/../b", "a\\b", "a/"] {
        assert_eq!(
            load_unit_sources(
                source(name, DECLARATION),
                vec![source("one.toml", STATEMENT)]
            ),
            Err(UnitLoadError::Input(UnitInputFault::InvalidSourceName {
                value: name.to_owned()
            }))
        );
    }
    let expected = UnitLoadError::Input(UnitInputFault::DuplicateSource {
        file: "a.toml".to_owned(),
        positions: vec![0, 1, 2],
    });
    assert_eq!(
        load_unit_sources(
            source("a.toml", DECLARATION),
            vec![source("a.toml", STATEMENT), source("a.toml", STATEMENT)]
        ),
        Err(expected.clone())
    );
    assert_eq!(
        expected.to_string(),
        "Input(DuplicateSource { file: \"a.toml\", positions: [0, 1, 2] })"
    );
    assert_eq!(
        load_unit_sources(source("documents.toml", DECLARATION), vec![]),
        Err(UnitLoadError::Input(UnitInputFault::EmptyCorpus {
            directory: "units/statements".to_owned()
        }))
    );
}
#[test]
fn u03_numeric_tokens_and_schema_kinds() {
    for token in ["1.0", "1e0", "+inf", "-inf", "inf", "nan", "+nan", "-nan"] {
        for unknown in [false, true] {
            let text = if unknown {
                format!("{STATEMENT}\n[unknown]\nx = {token}\n")
            } else {
                STATEMENT.replacen("schema = 1", &format!("schema = {token}"), 1)
            };
            let path = if unknown {
                vec!["unknown", "x"]
            } else {
                vec!["schema"]
            };
            assert_fault(
                DECLARATION,
                &text,
                diagnostic(
                    RuleId::M003,
                    UnitStage::Numeric,
                    None,
                    None,
                    location("units/statements/one.toml", &path, span(&text, token)),
                    UnitFault::NonIntegerNumber {
                        token: token.to_owned(),
                    },
                ),
            );
        }
    }
    for token in ["+1", "0x1", "0o1", "0b1", "1_0", "-0", "2"] {
        let text = STATEMENT.replacen("schema = 1", &format!("schema = {token}"), 1);
        assert_fault(
            DECLARATION,
            &text,
            diagnostic(
                RuleId::M003,
                UnitStage::Shape,
                Some("sec:coefficients:first-value"),
                None,
                location("units/statements/one.toml", &["schema"], span(&text, token)),
                UnitFault::SchemaValue {
                    expected: "1".to_owned(),
                    actual: token.to_owned(),
                },
            ),
        );
    }
    for (token, actual) in [
        ("'1'", ShapeKind::String),
        ("true", ShapeKind::Boolean),
        ("1979-05-27", ShapeKind::Datetime),
        ("[]", ShapeKind::Array),
        ("{}", ShapeKind::Table),
    ] {
        let text = STATEMENT.replacen("schema = 1", &format!("schema = {token}"), 1);
        assert_fault(
            DECLARATION,
            &text,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                Some("sec:coefficients:first-value"),
                None,
                location("units/statements/one.toml", &["schema"], span(&text, token)),
                UnitFault::WrongType {
                    expected: ShapeKind::Integer,
                    actual,
                },
            ),
        );
    }
}
#[test]
fn u04_scalar_and_table_type_matrix() {
    for (token, actual) in [
        ("true", ShapeKind::Boolean),
        ("1979-05-27", ShapeKind::Datetime),
        ("123", ShapeKind::Integer),
        ("[]", ShapeKind::Array),
        ("{}", ShapeKind::Table),
        ("'text'", ShapeKind::String),
    ] {
        for (original, path, expected) in [
            (
                "'sec:coefficients:first-value'",
                vec!["unit", "id"],
                ShapeKind::String,
            ),
            ("'main'", vec!["unit", "document"], ShapeKind::String),
            ("{}", vec!["unit", "math"], ShapeKind::Table),
            (
                "'The coefficient.'",
                vec!["unit", "texts", "en"],
                ShapeKind::String,
            ),
        ] {
            if expected == actual {
                continue;
            }
            let text = STATEMENT.replace(original, token);
            let unit = if path.last() == Some(&"id") {
                None
            } else {
                Some("sec:coefficients:first-value")
            };
            let language = (path.last() == Some(&"en")).then_some("en");
            let start = STATEMENT.find(original).unwrap();
            assert_fault(
                DECLARATION,
                &text,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    unit,
                    language,
                    location(
                        "units/statements/one.toml",
                        &path,
                        SourceAnchor::Bytes(start, start + token.len()),
                    ),
                    UnitFault::WrongType { expected, actual },
                ),
            );
        }
    }
}
#[test]
fn u04_equivalent_representations_and_four_string_forms() {
    for statement in [
        "schema=1\nunit={ id='sec:coefficients:first-value', document='main', math={}, texts={en='The coefficient.'} }",
        "schema=1\nunit.id='sec:coefficients:first-value'\nunit.document='main'\nunit.math={}\nunit.texts.en='The coefficient.'",
        "\"schema\"=1\n[\"unit\"]\n\"id\"='sec:coefficients:first-value'\n\"document\"='main'\n\"math\"={}\n[unit.\"texts\"]\n\"en\"='The coefficient.'",
    ] {
        assert_eq!(
            load(DECLARATION, statement).unwrap().units()[0].texts()["en"].text(),
            "The coefficient."
        );
    }
    for (literal, decoded) in [
        ("'The coefficient.'", "The coefficient."),
        ("\"The coefficient.\"", "The coefficient."),
        ("'''\nThe coefficient.'''", "The coefficient."),
        ("\"\"\"\nThe coefficient.\"\"\"", "The coefficient."),
        ("\"The \\u0063oefficient.\"", "The coefficient."),
        ("\"\"\"The \\\n  coefficient.\"\"\"", "The coefficient."),
        ("'''\r\nThe coefficient.'''", "The coefficient."),
    ] {
        let text = STATEMENT.replace("'The coefficient.'", literal);
        let checked = load(DECLARATION, &text).unwrap();
        let text_value = &checked.units()[0].texts()["en"];
        assert_eq!(text_value.text(), decoded);
        assert_eq!(
            text_value.site(),
            &location(
                "units/statements/one.toml",
                &["unit", "texts", "en"],
                span(&text, literal)
            )
        );
    }
}
#[test]
fn u02_duplicate_and_syntax_oracles() {
    let mut actual_errors = Vec::new();
    let mut expected_errors = Vec::new();
    for (text, message, range) in [
        (
            "schema=1\nschema=1\n",
            "duplicate key `schema` in document root",
            9..10,
        ),
        (
            "schema=1\n\"sche\\u006da\"=1\n",
            "duplicate key `schema` in document root",
            9..10,
        ),
        (
            "[unit]\nid='x'\nid='y'\n",
            "duplicate key `id` in table `unit`",
            14..15,
        ),
        (
            "[unit.texts]\nen='x'\nen='y'\n",
            "duplicate key `en` in table `unit.texts`",
            20..21,
        ),
        (
            "[unit]\n[unit]\n",
            "invalid table header\nduplicate key `\"unit\"` in document root",
            7..8,
        ),
        (
            "[unit.texts]\n[unit.texts]\n",
            "invalid table header\nduplicate key `\"texts\"` in table `unit`",
            13..14,
        ),
        ("x={a=1,a=2}\n", "duplicate key `a`", 3..4),
        ("x=[{a=1,a=2}]\n", "duplicate key `a`", 4..5),
        ("schema=1\ntrailing", "expected `.`, `=`", 17..17),
        ("{} {}", "invalid key", 0..1),
        ("x='unfinished", "invalid literal string", 13..13),
        (
            "x=\"\\q\"",
            "invalid escape sequence\nexpected `b`, `f`, `n`, `r`, `t`, `u`, `U`, `\\`, `\"`",
            5..6,
        ),
        (
            "x=9223372036854775808",
            "number too large to fit in target type",
            2..3,
        ),
    ] {
        actual_errors.push(load(DECLARATION, text));
        expected_errors.push(Err(errors(vec![diagnostic(
            RuleId::M002,
            UnitStage::Syntax,
            None,
            None,
            location(
                "units/statements/one.toml",
                &[],
                SourceAnchor::Bytes(range.start, range.end),
            ),
            UnitFault::TomlSyntax {
                message: message.to_owned(),
            },
        )])));
    }
    assert_eq!(actual_errors, expected_errors);
}
static NEXT: AtomicU64 = AtomicU64::new(0);
const INLINE: &str = "schema=1\nunit={id='sec:coefficients:first-value',document='main',math={},texts={en='Text.'}}\n";
fn inline_parent(text: &str) -> SourceAnchor {
    SourceAnchor::Parent(text.find('{').unwrap(), text.rfind('}').unwrap() + 1)
}
#[test]
fn u04_all_missing_unit_fields_and_unknown_key_order() {
    for (field, fragment) in [
        ("id", "id='sec:coefficients:first-value',"),
        ("document", "document='main',"),
        ("math", "math={},"),
        ("texts", ",texts={en='Text.'}"),
    ] {
        let text = INLINE.replace(fragment, "");
        assert_fault(
            DECLARATION,
            &text,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                (field != "id").then_some("sec:coefficients:first-value"),
                None,
                location("units/statements/one.toml", &["unit"], inline_parent(&text)),
                UnitFault::MissingField {
                    name: field.to_owned(),
                },
            ),
        );
    }
    for field in ["schema", "unit"] {
        let text = if field == "schema" {
            INLINE.replace("schema=1\n", "")
        } else {
            "schema=1\n".to_owned()
        };
        assert_fault(
            DECLARATION,
            &text,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                (field == "schema").then_some("sec:coefficients:first-value"),
                None,
                location("units/statements/one.toml", &[], SourceAnchor::WholeFile),
                UnitFault::MissingField {
                    name: field.to_owned(),
                },
            ),
        );
    }
    for field in [
        "kind",
        "status",
        "target",
        "renderer",
        "skeleton",
        "heading",
        "display",
        "prose-mint",
    ] {
        let text = INLINE.replace("math={},", &format!("math={{}},{field}=true,"));
        assert_fault(
            DECLARATION,
            &text,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                Some("sec:coefficients:first-value"),
                None,
                location(
                    "units/statements/one.toml",
                    &["unit", field],
                    span(&text, field),
                ),
                UnitFault::UnknownField {
                    name: field.to_owned(),
                    expected: ["id", "document", "math", "texts"]
                        .map(str::to_owned)
                        .to_vec(),
                },
            ),
        );
    }
    let text = INLINE.replace("id='sec:coefficients:first-value',", "z=1,a=2,");
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            None,
            None,
            location(
                "units/statements/one.toml",
                &["unit", "z"],
                span(&text, "z"),
            ),
            UnitFault::UnknownField {
                name: "z".to_owned(),
                expected: ["id", "document", "math", "texts"]
                    .map(str::to_owned)
                    .to_vec(),
            },
        ),
    );
}
#[test]
fn u04_declaration_missing_extra_fields() {
    for (text, path, anchor, name) in [
        (
            "documents={main={required-languages=['en']}}",
            vec![],
            SourceAnchor::WholeFile,
            "schema",
        ),
        ("schema=1", vec![], SourceAnchor::WholeFile, "documents"),
        (
            "schema=1\ndocuments={main={}}",
            vec!["documents", "main"],
            SourceAnchor::Parent(25, 27),
            "required-languages",
        ),
    ] {
        assert_fault(
            text,
            STATEMENT,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                None,
                None,
                location("units/documents.toml", &path, anchor),
                UnitFault::MissingField {
                    name: name.to_owned(),
                },
            ),
        );
    }
    for (text, path, field, keys) in [
        (
            "schema=1\nextra=1\ndocuments={main={required-languages=['en']}}",
            vec!["extra"],
            "extra",
            vec!["schema", "documents"],
        ),
        (
            "schema=1\ndocuments={main={required-languages=['en'],extra=1}}",
            vec!["documents", "main", "extra"],
            "extra",
            vec!["required-languages"],
        ),
    ] {
        assert_fault(
            text,
            STATEMENT,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                None,
                None,
                location("units/documents.toml", &path, span(text, field)),
                UnitFault::UnknownField {
                    name: field.to_owned(),
                    expected: keys.into_iter().map(str::to_owned).collect(),
                },
            ),
        );
    }
}
#[test]
fn u04_dynamic_tables_arrays_and_every_wrong_kind() {
    for (token, actual) in [
        ("true", ShapeKind::Boolean),
        ("1979-05-27", ShapeKind::Datetime),
        ("123", ShapeKind::Integer),
        ("[]", ShapeKind::Array),
        ("{}", ShapeKind::Table),
        ("'text'", ShapeKind::String),
    ] {
        wrong_dynamic_tables(token, actual);
        if actual != ShapeKind::String {
            let text = INLINE.replace("math={}", &format!("math={{value={token}}}"));
            let start = text.find("value=").unwrap() + 6;
            assert_fault(
                DECLARATION,
                &text,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    Some("sec:coefficients:first-value"),
                    None,
                    location(
                        "units/statements/one.toml",
                        &["unit", "math", "value"],
                        SourceAnchor::Bytes(start, start + token.len()),
                    ),
                    UnitFault::WrongType {
                        expected: ShapeKind::String,
                        actual,
                    },
                ),
            );
            let text = DECLARATION.replace("'en'", token);
            let start = DECLARATION.find("'en'").unwrap();
            let mut site = location(
                "units/documents.toml",
                &["documents", "main", "required-languages"],
                SourceAnchor::Bytes(start, start + token.len()),
            );
            site.path.push(PathPart::Index(0));
            assert_fault(
                &text,
                STATEMENT,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    None,
                    None,
                    site,
                    UnitFault::WrongType {
                        expected: ShapeKind::String,
                        actual,
                    },
                ),
            );
        }
        if actual != ShapeKind::Array {
            let text = DECLARATION.replace("['en']", token);
            let start = DECLARATION.find("['en']").unwrap();
            assert_fault(
                &text,
                STATEMENT,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    None,
                    None,
                    location(
                        "units/documents.toml",
                        &["documents", "main", "required-languages"],
                        SourceAnchor::Bytes(start, start + token.len()),
                    ),
                    UnitFault::WrongType {
                        expected: ShapeKind::Array,
                        actual,
                    },
                ),
            );
        }
    }
}
#[test]
fn u02_more_duplicates_and_numeric_phase_precedence() {
    for (text, message, offset) in [
        (
            "[unit.texts]\nen='x'\n\"\\u0065n\"='y'",
            "duplicate key `en` in table `unit.texts`",
            20,
        ),
        (
            "unit.id='x'\n[unit]\nid='y'",
            "invalid table header\nduplicate key `\"unit\"` in document root",
            12,
        ),
        (
            "[unknown]\nx=1\nx=2",
            "duplicate key `x` in table `unknown`",
            14,
        ),
        (
            "schema=1.0\nschema=1",
            "duplicate key `schema` in document root",
            11,
        ),
    ] {
        assert_fault(
            DECLARATION,
            text,
            diagnostic(
                RuleId::M002,
                UnitStage::Syntax,
                None,
                None,
                location(
                    "units/statements/one.toml",
                    &[],
                    SourceAnchor::Bytes(offset, offset + 1),
                ),
                UnitFault::TomlSyntax {
                    message: message.to_owned(),
                },
            ),
        );
    }
    let text = format!("{INLINE}\nunknown=[{{x=-nan}},1e0]");
    let mut site = location(
        "units/statements/one.toml",
        &["unknown"],
        span(&text, "-nan"),
    );
    site.path
        .extend([PathPart::Index(0), PathPart::Key("x".to_owned())]);
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M003,
            UnitStage::Numeric,
            None,
            None,
            site,
            UnitFault::NonIntegerNumber {
                token: "-nan".to_owned(),
            },
        ),
    );
}
#[test]
fn u09_empty_literals_implicit_tables_and_decoded_unicode() {
    let text = STATEMENT.replace("'The coefficient.'", "''");
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M040,
            UnitStage::Language,
            Some("sec:coefficients:first-value"),
            Some("en"),
            location(
                "units/statements/one.toml",
                &["unit", "texts", "en"],
                span(&text, "''"),
            ),
            UnitFault::BlankText,
        ),
    );
    let text = STATEMENT.replace("'The coefficient.'", "\"\\u00e9 @tag{c:1}\"");
    let mut site = location(
        "units/statements/one.toml",
        &["unit", "texts", "en"],
        span(&text, "\"\\u00e9 @tag{c:1}\""),
    );
    site.decoded = Some(3..12);
    site.occurrence = Some(0);
    let corpus = load(DECLARATION, &text).unwrap();
    assert_eq!(
        corpus.units()[0].texts()["en"].tag_occurrences()[0].site(),
        &site
    );
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    assert_eq!(
        ftoo_manifest::resolve_unit_tags(&corpus, &book)
            .unwrap()
            .tags()[0]
            .target(),
        ftoo_manifest::NodeId::LangevinCoefficientOne
    );
    let text = "schema=1\nunit.texts.en='Text.'";
    assert_fault(
        DECLARATION,
        text,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            None,
            None,
            location(
                "units/statements/one.toml",
                &["unit"],
                SourceAnchor::WholeFile,
            ),
            UnitFault::MissingField {
                name: "id".to_owned(),
            },
        ),
    );
}
#[test]
fn u01_injected_io_failures_and_fresh_restoration() {
    let fixture = Fixture::new();
    for (operation, relative) in [
        (UnitIoOperation::ReadDeclarations, "units/documents.toml"),
        (UnitIoOperation::ListStatements, "units/statements"),
        (UnitIoOperation::InspectEntry, "units/statements/one.toml"),
        (UnitIoOperation::ReadStatement, "units/statements/one.toml"),
    ] {
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::NotFound,
            io::ErrorKind::Interrupted,
        ] {
            let actual = files::collect_with(&fixture.root, |current, _| {
                if current == operation {
                    Err(io::Error::from(kind))
                } else {
                    Ok(())
                }
            });
            let file = fixture.root.join(relative);
            let expected = UnitLoadError::Io {
                operation,
                file: file.clone(),
                kind,
            };
            assert_eq!(actual, Err(expected.clone()));
            assert_eq!(
                expected.to_string(),
                format!("Io {{ operation: {operation:?}, file: {file:?}, kind: {kind:?} }}")
            );
            assert_eq!(
                fs::read(fixture.root.join("units/documents.toml")).unwrap(),
                DECLARATION.as_bytes()
            );
            assert_eq!(
                fs::read(fixture.root.join("units/statements/one.toml")).unwrap(),
                STATEMENT.as_bytes()
            );
            fixture.restore();
        }
    }
}
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ftoo-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("units/statements")).unwrap();
        fs::write(root.join("units/documents.toml"), DECLARATION).unwrap();
        fs::write(root.join("units/statements/one.toml"), STATEMENT).unwrap();
        Self { root }
    }
    fn restore(&self) {
        assert_eq!(checked_files(&self.root).unwrap().tag_sites, 0);
        assert_eq!(
            load_unit_files(&self.root).unwrap().units()[0].texts()["en"].text(),
            "The coefficient."
        );
    }
    fn tamper(&self, text: &str, diagnostics: Vec<UnitDiagnostic>) {
        let path = self.root.join("units/statements/one.toml");
        fs::write(&path, text).unwrap();
        let expected = errors(diagnostics);
        assert_eq!(checked_files(&self.root), Err(expected.clone()));
        assert_eq!(
            checked_files(&self.root).unwrap_err().to_string(),
            expected.to_string()
        );
        assert_eq!(fs::read(&path).unwrap(), text.as_bytes());
        assert_eq!(
            fs::read(self.root.join("units/documents.toml")).unwrap(),
            DECLARATION.as_bytes()
        );
        fs::write(path, STATEMENT).unwrap();
        self.restore();
    }
}

#[test]
fn u15_actual_missing_language_unknown_field_duplicate_and_float() {
    let fixture = Fixture::new();
    fixture.restore();
    let text = INLINE.replace("{en='Text.'}", "{}");
    let start = text.find("texts=").unwrap() + 6;
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M040,
            UnitStage::Language,
            Some("sec:coefficients:first-value"),
            Some("en"),
            location(
                "units/statements/one.toml",
                &["unit", "texts", "en"],
                SourceAnchor::Parent(start, start + 2),
            ),
            UnitFault::MissingLanguage {
                document: "main".to_owned(),
                required_at: UnitSite {
                    path: vec![
                        PathPart::Key("documents".to_owned()),
                        PathPart::Key("main".to_owned()),
                        PathPart::Key("required-languages".to_owned()),
                        PathPart::Index(0),
                    ],
                    ..location("units/documents.toml", &[], span(DECLARATION, "'en'"))
                },
            },
        )],
    );
    let text = format!("{INLINE}extra=1\n");
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            Some("sec:coefficients:first-value"),
            None,
            location(
                "units/statements/one.toml",
                &["extra"],
                span(&text, "extra"),
            ),
            UnitFault::UnknownField {
                name: "extra".to_owned(),
                expected: vec!["schema".to_owned(), "unit".to_owned()],
            },
        )],
    );
    let text = INLINE.replace("en='Text.'", "en='Text.',\"\\u0065n\"='Again.'");
    let start = text.find("en=").unwrap();
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M002,
            UnitStage::Syntax,
            None,
            None,
            location(
                "units/statements/one.toml",
                &[],
                SourceAnchor::Bytes(start, start + 1),
            ),
            UnitFault::TomlSyntax {
                message: "duplicate key `en`".to_owned(),
            },
        )],
    );
    let text = format!("{INLINE}unknown=[{{x=1.5}}]");
    let mut site = location(
        "units/statements/one.toml",
        &["unknown"],
        span(&text, "1.5"),
    );
    site.path
        .extend([PathPart::Index(0), PathPart::Key("x".to_owned())]);
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M003,
            UnitStage::Numeric,
            None,
            None,
            site,
            UnitFault::NonIntegerNumber {
                token: "1.5".to_owned(),
            },
        )],
    );
}
#[test]
fn u15_actual_document_and_math_mutations() {
    let fixture = Fixture::new();
    fixture.restore();
    let text = INLINE.replace("'main'", "'absent'");
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M039,
            UnitStage::Metadata,
            Some("sec:coefficients:first-value"),
            None,
            location(
                "units/statements/one.toml",
                &["unit", "document"],
                span(&text, "'absent'"),
            ),
            UnitFault::UnknownDocument {
                name: "absent".to_owned(),
                available: vec!["main".to_owned()],
            },
        )],
    );
    let text = INLINE.replace("'Text.'", "'See @math{missing}.'");
    let mut site = location(
        "units/statements/one.toml",
        &["unit", "texts", "en"],
        span(&text, "'See @math{missing}.'"),
    );
    site.decoded = Some(4..18);
    site.occurrence = Some(0);
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M041,
            UnitStage::Content,
            Some("sec:coefficients:first-value"),
            Some("en"),
            site,
            UnitFault::UnknownMath {
                name: "missing".to_owned(),
                available: vec![],
            },
        )],
    );
    let text = INLINE.replace("math={}", "math={value='x'}");
    let start = text.find("value=").unwrap();
    fixture.tamper(
        &text,
        vec![diagnostic(
            RuleId::M041,
            UnitStage::Content,
            Some("sec:coefficients:first-value"),
            None,
            location(
                "units/statements/one.toml",
                &["unit", "math", "value"],
                SourceAnchor::Bytes(start, start + 5),
            ),
            UnitFault::UnusedMath {
                name: "value".to_owned(),
            },
        )],
    );
}
#[test]
fn u15_actual_duplicate_unit_and_two_missing_translations() {
    let fixture = Fixture::new();
    fixture.restore();
    let second = fixture.root.join("units/statements/two.toml");
    fs::write(&second, STATEMENT).unwrap();
    let sites: Vec<_> = ["one", "two"]
        .map(|name| {
            location(
                &format!("units/statements/{name}.toml"),
                &["unit", "id"],
                span(STATEMENT, "'sec:coefficients:first-value'"),
            )
        })
        .to_vec();
    let expected = errors(
        sites
            .iter()
            .map(|site| {
                diagnostic(
                    RuleId::M039,
                    UnitStage::Metadata,
                    Some("sec:coefficients:first-value"),
                    None,
                    site.clone(),
                    UnitFault::DuplicateUnit {
                        id: "sec:coefficients:first-value".to_owned(),
                        sites: sites.clone(),
                    },
                )
            })
            .collect(),
    );
    assert_eq!(checked_files(&fixture.root), Err(expected.clone()));
    assert_eq!(
        checked_files(&fixture.root).unwrap_err().to_string(),
        expected.to_string()
    );
    assert_eq!(fs::read(&second).unwrap(), STATEMENT.as_bytes());
    fs::remove_file(&second).unwrap();
    fixture.restore();
    let declaration =
        format!("{DECLARATION}[documents.explainer]\nrequired-languages=['en','fr']\n");
    let first_text = INLINE.replace("'main'", "'explainer'");
    let second_text = first_text.replace("first-value", "value-context");
    fs::write(fixture.root.join("units/documents.toml"), &declaration).unwrap();
    fs::write(fixture.root.join("units/statements/one.toml"), &first_text).unwrap();
    fs::write(&second, &second_text).unwrap();
    let mut required_at = location(
        "units/documents.toml",
        &["documents", "explainer", "required-languages"],
        span(&declaration, "'fr'"),
    );
    required_at.path.push(PathPart::Index(1));
    let diagnostics = [
        ("one", "sec:coefficients:first-value", &first_text),
        ("two", "sec:coefficients:value-context", &second_text),
    ]
    .into_iter()
    .map(|(name, id, text)| {
        let start = text.find("{en=").unwrap();
        let end = text[start..].find('}').unwrap() + start + 1;
        diagnostic(
            RuleId::M040,
            UnitStage::Language,
            Some(id),
            Some("fr"),
            location(
                &format!("units/statements/{name}.toml"),
                &["unit", "texts", "fr"],
                SourceAnchor::Parent(start, end),
            ),
            UnitFault::MissingLanguage {
                document: "explainer".to_owned(),
                required_at: required_at.clone(),
            },
        )
    })
    .collect();
    let expected = errors(diagnostics);
    assert_eq!(checked_files(&fixture.root), Err(expected.clone()));
    assert_eq!(
        checked_files(&fixture.root).unwrap_err().to_string(),
        expected.to_string()
    );
    assert_eq!(fs::read(&second).unwrap(), second_text.as_bytes());
    assert_eq!(
        fs::read(fixture.root.join("units/statements/one.toml")).unwrap(),
        first_text.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.root.join("units/documents.toml")).unwrap(),
        declaration.as_bytes()
    );
    fs::write(fixture.root.join("units/documents.toml"), DECLARATION).unwrap();
    fs::write(fixture.root.join("units/statements/one.toml"), STATEMENT).unwrap();
    fs::remove_file(second).unwrap();
    fixture.restore();
}
#[test]
fn u02_bounded_recursion_and_quote_aware_control() {
    let text = format!("x={}0{}", "[".repeat(80), "]".repeat(80));
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M002,
            UnitStage::Syntax,
            None,
            None,
            location(
                "units/statements/one.toml",
                &[],
                SourceAnchor::Bytes(81, 82),
            ),
            UnitFault::TomlSyntax {
                message: "recursion limit exceeded".to_owned(),
            },
        ),
    );
    let text = format!("x={}0{}", "[".repeat(79), "]".repeat(79));
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            None,
            None,
            location(
                "units/statements/one.toml",
                &["x"],
                SourceAnchor::Bytes(0, 1),
            ),
            UnitFault::UnknownField {
                name: "x".to_owned(),
                expected: vec!["schema".to_owned(), "unit".to_owned()],
            },
        ),
    );
    let prose = format!("Brackets {}{}.", "[".repeat(200), "]".repeat(200));
    assert_eq!(
        load(DECLARATION, &STATEMENT.replace("The coefficient.", &prose))
            .unwrap()
            .units()[0]
            .texts()["en"]
            .text(),
        prose
    );
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn u01_actual_removals_entries_and_restoration() {
    let fixture = Fixture::new();
    fixture.restore();
    for (relative, operation) in [
        ("units/documents.toml", UnitIoOperation::ReadDeclarations),
        ("units/statements", UnitIoOperation::ListStatements),
    ] {
        let path = fixture.root.join(relative);
        let saved = fixture.root.join("saved");
        fs::rename(&path, &saved).unwrap();
        let expected = UnitLoadError::Io {
            operation,
            file: path.clone(),
            kind: io::ErrorKind::NotFound,
        };
        assert_eq!(checked_files(&fixture.root), Err(expected.clone()));
        assert_eq!(
            expected.to_string(),
            format!("Io {{ operation: {operation:?}, file: {path:?}, kind: NotFound }}")
        );
        fs::rename(saved, path).unwrap();
        fixture.restore();
    }
    let statement = fixture.root.join("units/statements/one.toml");
    fs::remove_file(&statement).unwrap();
    assert_eq!(
        checked_files(&fixture.root),
        Err(UnitLoadError::Input(UnitInputFault::EmptyCorpus {
            directory: "units/statements".to_owned()
        }))
    );
    fs::write(statement, STATEMENT).unwrap();
    fixture.restore();
    for (name, kind) in [
        ("extra.txt", EntryKind::NonTomlFile),
        ("extra.toml", EntryKind::Directory),
        ("linked.toml", EntryKind::Symlink),
    ] {
        let path = fixture.root.join("units/statements").join(name);
        match kind {
            EntryKind::Directory => fs::create_dir(&path).unwrap(),
            EntryKind::Symlink => std::os::unix::fs::symlink("one.toml", &path).unwrap(),
            _ => fs::write(&path, "ignored").unwrap(),
        }
        assert_eq!(
            checked_files(&fixture.root),
            Err(UnitLoadError::Input(UnitInputFault::UnexpectedEntry {
                file: path.clone(),
                kind
            }))
        );
        if kind == EntryKind::Directory {
            fs::remove_dir(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
        fixture.restore();
    }
}
#[test]
fn u15_actual_file_tag_and_shape_tampering_restores_every_time() {
    let fixture = Fixture::new();
    fixture.restore();
    for (replacement, fault, stage, rule, decoded, occurrence) in [
        (
            "true",
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Boolean,
            },
            UnitStage::Shape,
            RuleId::M004,
            None,
            None,
        ),
        (
            "'See @tag{unknown}.'",
            UnitFault::ResolutionFault {
                cause: ftoo_manifest::TagResolutionFault::UnknownTag {
                    input: "unknown".to_owned(),
                },
            },
            UnitStage::Resolution,
            RuleId::M042,
            Some(4..17),
            Some(0),
        ),
        (
            "{latex='text'}",
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Table,
            },
            UnitStage::Shape,
            RuleId::M004,
            None,
            None,
        ),
    ] {
        let text = STATEMENT.replace("'The coefficient.'", replacement);
        let path = fixture.root.join("units/statements/one.toml");
        fs::write(&path, &text).unwrap();
        let mut location = location(
            "units/statements/one.toml",
            &["unit", "texts", "en"],
            span(&text, replacement),
        );
        location.decoded = decoded;
        location.occurrence = occurrence;
        let expected = errors(vec![diagnostic(
            rule,
            stage,
            Some("sec:coefficients:first-value"),
            Some("en"),
            location,
            fault,
        )]);
        assert_eq!(checked_files(&fixture.root), Err(expected.clone()));
        assert_eq!(
            checked_files(&fixture.root).unwrap_err().to_string(),
            expected.to_string()
        );
        assert_eq!(fs::read(&path).unwrap(), text.as_bytes());
        assert_eq!(
            fs::read(fixture.root.join("units/documents.toml")).unwrap(),
            DECLARATION.as_bytes()
        );
        fs::write(&path, STATEMENT).unwrap();
        fixture.restore();
    }
}

fn wrong_dynamic_tables(token: &str, actual: ShapeKind) {
    if actual != ShapeKind::Table {
        for (prefix, path) in [
            ("schema=1\ndocuments=", vec!["documents"]),
            ("schema=1\ndocuments={main=", vec!["documents", "main"]),
        ] {
            let text = format!("{prefix}{token}{}", if path.len() == 2 { "}" } else { "" });
            assert_fault(
                &text,
                STATEMENT,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    None,
                    None,
                    location("units/documents.toml", &path, span(&text, token)),
                    UnitFault::WrongType {
                        expected: ShapeKind::Table,
                        actual,
                    },
                ),
            );
        }
        for (original, path) in [
            (
                "{id='sec:coefficients:first-value',document='main',math={},texts={en='Text.'}}",
                vec!["unit"],
            ),
            ("{en='Text.'}", vec!["unit", "texts"]),
        ] {
            let text = INLINE.replace(original, token);
            let start = INLINE.find(original).unwrap();
            assert_fault(
                DECLARATION,
                &text,
                diagnostic(
                    RuleId::M004,
                    UnitStage::Shape,
                    (path.len() > 1).then_some("sec:coefficients:first-value"),
                    None,
                    location(
                        "units/statements/one.toml",
                        &path,
                        SourceAnchor::Bytes(start, start + token.len()),
                    ),
                    UnitFault::WrongType {
                        expected: ShapeKind::Table,
                        actual,
                    },
                ),
            );
        }
    }
}

#[test]
fn u01_non_utf8_names_special_entries_and_displays() {
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    fixture.restore();
    let bad = fixture
        .root
        .join("units/statements")
        .join(std::ffi::OsString::from_vec(vec![
            0xff, b'.', b't', b'o', b'm', b'l',
        ]));
    fs::write(&bad, "data").unwrap();
    let expected = UnitLoadError::Input(UnitInputFault::UnexpectedEntry {
        file: bad.clone(),
        kind: EntryKind::NonTomlFile,
    });
    assert_eq!(checked_files(&fixture.root), Err(expected.clone()));
    assert_eq!(
        expected.to_string(),
        format!("Input(UnexpectedEntry {{ file: {bad:?}, kind: NonTomlFile }})")
    );
    fs::remove_file(bad).unwrap();
    fixture.restore();
    let socket = fixture.root.join("units/statements/socket.toml");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert_eq!(
        checked_files(&fixture.root),
        Err(UnitLoadError::Input(UnitInputFault::UnexpectedEntry {
            file: socket.clone(),
            kind: EntryKind::Other
        }))
    );
    drop(listener);
    fs::remove_file(socket).unwrap();
    fixture.restore();
    for (fault, text) in [
        (
            UnitInputFault::InvalidSourceName {
                value: "../a".to_owned(),
            },
            "InvalidSourceName { value: \"../a\" }",
        ),
        (
            UnitInputFault::DuplicateSource {
                file: "a".to_owned(),
                positions: vec![0, 2],
            },
            "DuplicateSource { file: \"a\", positions: [0, 2] }",
        ),
        (
            UnitInputFault::EmptyCorpus {
                directory: "units/statements".to_owned(),
            },
            "EmptyCorpus { directory: \"units/statements\" }",
        ),
        (
            UnitInputFault::UnexpectedEntry {
                file: PathBuf::from("a"),
                kind: EntryKind::Directory,
            },
            "UnexpectedEntry { file: \"a\", kind: Directory }",
        ),
    ] {
        assert_eq!(fault.to_string(), text);
    }
}
#[test]
fn u01_discovered_statement_disappears_before_read() {
    let fixture = Fixture::new();
    fixture.restore();
    let path = fixture.root.join("units/statements/one.toml");
    let actual = files::collect_with(&fixture.root, |operation, file| {
        if operation == UnitIoOperation::ReadStatement {
            fs::remove_file(file)?;
        }
        Ok(())
    });
    assert_eq!(
        actual,
        Err(UnitLoadError::Io {
            operation: UnitIoOperation::ReadStatement,
            file: path.clone(),
            kind: io::ErrorKind::NotFound
        })
    );
    fs::write(path, STATEMENT).unwrap();
    fixture.restore();
}
#[test]
fn u04_array_of_tables_cannot_replace_strings_or_language_members() {
    let text = STATEMENT.replace("en = 'The coefficient.'\n", "[[unit.texts.en]]\n");
    let start = text.find("[[unit.texts.en]]").unwrap();
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            Some("sec:coefficients:first-value"),
            Some("en"),
            location(
                "units/statements/one.toml",
                &["unit", "texts", "en"],
                SourceAnchor::Bytes(start, text.len() - 1),
            ),
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Array,
            },
        ),
    );
    let text = "schema=1\n[[documents.main.required-languages]]\n";
    let mut site = location(
        "units/documents.toml",
        &["documents", "main", "required-languages"],
        SourceAnchor::Bytes(9, text.len() - 1),
    );
    site.path.push(PathPart::Index(0));
    assert_fault(
        text,
        STATEMENT,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            None,
            None,
            site,
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Table,
            },
        ),
    );
}
#[test]
fn u09_global_phase_order_and_reordered_source_names() {
    let declaration = source("d.toml", DECLARATION);
    let a = UnitSource {
        file: "a.toml".to_owned(),
        bytes: vec![0xff],
    };
    let z = UnitSource {
        file: "z.toml".to_owned(),
        bytes: vec![0xef, 0xbb, 0xbf],
    };
    let expected = errors(vec![
        diagnostic(
            RuleId::M001,
            UnitStage::Encoding,
            None,
            None,
            location("a.toml", &[], SourceAnchor::Bytes(0, 1)),
            UnitFault::InvalidUtf8,
        ),
        diagnostic(
            RuleId::M001,
            UnitStage::Encoding,
            None,
            None,
            location("z.toml", &[], SourceAnchor::Bytes(0, 3)),
            UnitFault::Bom,
        ),
    ]);
    for sources in [vec![a.clone(), z.clone()], vec![z, a]] {
        assert_eq!(
            load_unit_sources(declaration.clone(), sources),
            Err(expected.clone())
        );
    }
    let syntax = diagnostic(
        RuleId::M002,
        UnitStage::Syntax,
        None,
        None,
        location("z.toml", &[], SourceAnchor::Bytes(0, 1)),
        UnitFault::TomlSyntax {
            message: "invalid key".to_owned(),
        },
    );
    assert_eq!(
        load_unit_sources(
            declaration.clone(),
            vec![source("a.toml", "schema=1.0"), source("z.toml", "{}")]
        ),
        Err(errors(vec![syntax]))
    );
    let a = source("a.toml", &STATEMENT.replace("first-value", "value-context"));
    let z = source("z.toml", STATEMENT);
    for sources in [vec![a.clone(), z.clone()], vec![z, a]] {
        let corpus = load_unit_sources(declaration.clone(), sources).unwrap();
        assert_eq!(
            corpus
                .units()
                .iter()
                .map(|u| u.label().as_str())
                .collect::<Vec<_>>(),
            [
                "sec:coefficients:first-value",
                "sec:coefficients:value-context"
            ]
        );
    }
}
#[test]
fn u09_multiline_math_locations_keep_raw_and_decoded_coordinates_distinct() {
    for literal in [
        "'é See @math{value}.'",
        "\"\\u00e9 See @math{value}.\"",
        "'''\né See @math{value}.'''",
        "\"\"\"\r\né See @math{value}.\"\"\"",
        "\"\"\"é See \\\n  @math{value}.\"\"\"",
    ] {
        let text = INLINE
            .replace("math={}", "math={value='x'}")
            .replace("'Text.'", literal);
        let corpus = load(DECLARATION, &text).unwrap();
        let mut site = location(
            "units/statements/one.toml",
            &["unit", "texts", "en"],
            span(&text, literal),
        );
        site.decoded = Some(7..19);
        site.occurrence = Some(0);
        assert_eq!(
            corpus.units()[0].texts()["en"].text(),
            "é See @math{value}."
        );
        assert_eq!(
            corpus.units()[0].texts()["en"].occurrences()[0].site(),
            &site
        );
    }
    let text = STATEMENT
        .replace("sec:coefficients:first-value", "bad")
        .replace("'The coefficient.'", "false");
    assert_fault(
        DECLARATION,
        &text,
        diagnostic(
            RuleId::M004,
            UnitStage::Shape,
            Some("bad"),
            Some("en"),
            location(
                "units/statements/one.toml",
                &["unit", "texts", "en"],
                span(&text, "false"),
            ),
            UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Boolean,
            },
        ),
    );
}

#[test]
fn u03_declaration_schema_and_float_looking_strings() {
    for (token, actual) in [
        ("'1'", ShapeKind::String),
        ("true", ShapeKind::Boolean),
        ("1979-05-27", ShapeKind::Datetime),
        ("[]", ShapeKind::Array),
        ("{}", ShapeKind::Table),
    ] {
        let text = DECLARATION.replacen("schema = 1", &format!("schema = {token}"), 1);
        assert_fault(
            &text,
            STATEMENT,
            diagnostic(
                RuleId::M004,
                UnitStage::Shape,
                None,
                None,
                location("units/documents.toml", &["schema"], span(&text, token)),
                UnitFault::WrongType {
                    expected: ShapeKind::Integer,
                    actual,
                },
            ),
        );
    }
    for token in ["+1", "0x1", "0o1", "0b1", "1_0", "-0", "2"] {
        let text = DECLARATION.replacen("schema = 1", &format!("schema = {token}"), 1);
        assert_fault(
            &text,
            STATEMENT,
            diagnostic(
                RuleId::M003,
                UnitStage::Shape,
                None,
                None,
                location("units/documents.toml", &["schema"], span(&text, token)),
                UnitFault::SchemaValue {
                    expected: "1".to_owned(),
                    actual: token.to_owned(),
                },
            ),
        );
    }
    for token in ["1.0", "1e0", "+inf", "-inf", "nan", "+nan", "-nan"] {
        let text = DECLARATION.replacen("schema = 1", &format!("schema = {token}"), 1);
        assert_fault(
            &text,
            STATEMENT,
            diagnostic(
                RuleId::M003,
                UnitStage::Numeric,
                None,
                None,
                location("units/documents.toml", &["schema"], span(&text, token)),
                UnitFault::NonIntegerNumber {
                    token: token.to_owned(),
                },
            ),
        );
    }
    let text = INLINE
        .replace("math={}", "math={value='1e0'}")
        .replace("Text.", "1.0 is prose; see @math{value}.");
    let corpus = load(DECLARATION, &text).unwrap();
    assert_eq!(corpus.units()[0].math()["value"].value, "1e0");
    assert_eq!(
        corpus.units()[0].texts()["en"].text(),
        "1.0 is prose; see @math{value}."
    );
}

fn checked_files(root: &Path) -> Result<UnitCheckSummary, UnitLoadError> {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    check_unit_files(root, &book)
}

#[test]
fn u15_actual_tag_sites_unknown_and_different_books_restore_bytes() {
    use ftoo_manifest::{NodeId as N, OutOfSliceCause as O, TagResolutionFault as F};
    use std::fmt::Write;
    let fixture = Fixture::new();
    fixture.restore();
    let declaration = "schema=1\n[documents.main]\nrequired-languages=['en']\n[documents.explainer]\nrequired-languages=['en','fr']\n";
    let declaration_path = fixture.root.join("units/documents.toml");
    let statement_path = fixture.root.join("units/statements/one.toml");
    let graph = ftoo_manifest::seed().unwrap();
    let full = ftoo_manifest::addressing(&graph).unwrap();
    let mut one_draft = ftoo_manifest::seed_draft();
    one_draft.coverage.slice = vec![N::LangevinCoefficientOneValue];
    let one = ftoo_manifest::validate(one_draft).unwrap();
    let one_book = ftoo_manifest::addressing(&one).unwrap();
    let mut empty_draft = ftoo_manifest::seed_draft();
    empty_draft.coverage.slice.clear();
    let empty = ftoo_manifest::validate(empty_draft).unwrap();
    let empty_book = ftoo_manifest::addressing(&empty).unwrap();
    for (input, tag, target, label, reason) in file_tag_cases() {
        for (text, starts) in [
            (format!("@tag{{{input}}} begins."), vec![0]),
            (format!("See @tag{{{input}}} here."), vec![4]),
            (format!("See @tag{{{input}}}"), vec![4]),
            (
                format!("See @tag{{{input}}} and @tag{{{input}}}."),
                vec![4, 18],
            ),
        ] {
            let statement = format!(
                "schema=1\n[unit]\nid='sec:coefficients:first-value'\ndocument='explainer'\nmath={{}}\n[unit.texts]\nen='{text}'\nfr='{text}'\n"
            );
            fs::write(&declaration_path, declaration).unwrap();
            fs::write(&statement_path, &statement).unwrap();
            let summary = UnitCheckSummary {
                documents: 2,
                units: 1,
                texts: 2,
                tag_sites: starts.len() * 2,
            };
            assert_eq!(check_unit_files(&fixture.root, &full).unwrap(), summary);
            mutate_tag_sites(
                &fixture,
                declaration,
                &statement,
                &text,
                &starts,
                &full,
                summary,
            );
            for book in [&one_book, &empty_book] {
                if std::ptr::eq(book, &raw const one_book) && matches!(input, "c:1" | "v:1") {
                    assert_eq!(check_unit_files(&fixture.root, book).unwrap(), summary);
                    continue;
                }
                let mut faults = Vec::new();
                let mut display = String::new();
                for language in ["en", "fr"] {
                    let raw_start = statement.find(&format!("{language}=")).unwrap() + 3;
                    for (index, start) in starts.iter().copied().enumerate() {
                        let mut site = location(
                            "units/statements/one.toml",
                            &["unit", "texts", language],
                            SourceAnchor::Bytes(raw_start, raw_start + text.len() + 2),
                        );
                        site.decoded = Some(start..start + 9);
                        site.occurrence = Some(index);
                        faults.push(diagnostic(
                            RuleId::M042,
                            UnitStage::Resolution,
                            Some("sec:coefficients:first-value"),
                            Some(language),
                            site,
                            UnitFault::ResolutionFault {
                                cause: F::OutOfSliceTag {
                                    input: input.into(),
                                    tag,
                                    target,
                                    cause: reason,
                                },
                            },
                        ));
                        let cause = match reason {
                            O::QuantityOutsideSupport => "quantity-outside-support",
                            O::ClaimNotSelected => "claim-not-selected",
                            O::TargetAbsent => unreachable!(),
                        };
                        writeln!(display,"M042 Resolution unit=Some(\"sec:coefficients:first-value\") language=Some({language:?}) at \"units/statements/one.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes {raw_start}..{} decoded {start}..{} occurrence {index}: tag {input:?}: out of slice; label {label}; cause {cause}",raw_start+text.len()+2,start+9).unwrap();
                    }
                }
                let actual = check_unit_files(&fixture.root, book).unwrap_err();
                assert_eq!(actual, errors(faults));
                assert_eq!(actual.to_string(), display);
                assert_eq!(fs::read(&statement_path).unwrap(), statement.as_bytes());
                assert_eq!(fs::read(&declaration_path).unwrap(), declaration.as_bytes());
                assert_eq!(check_unit_files(&fixture.root, &full).unwrap(), summary);
            }
            fs::write(&declaration_path, DECLARATION).unwrap();
            fs::write(&statement_path, STATEMENT).unwrap();
            fixture.restore();
        }
    }
}
#[test]
fn u09_tag_raw_literals_decoded_unicode_crlf_and_continuation() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    for literal in [
        "'é See @tag{c:1}.'",
        "\"\\u00e9 See @tag{c:1}.\"",
        "'''\né See @tag{c:1}.'''",
        "\"\"\"\r\né See @tag{c:1}.\"\"\"",
        "\"\"\"é See \\\n  @tag{c:1}.\"\"\"",
    ] {
        let source = INLINE.replace("'Text.'", literal);
        let corpus = load(DECLARATION, &source).unwrap();
        let resolved = ftoo_manifest::resolve_unit_tags(&corpus, &book).unwrap();
        let mut site = location(
            "units/statements/one.toml",
            &["unit", "texts", "en"],
            span(&source, literal),
        );
        site.decoded = Some(7..16);
        site.occurrence = Some(0);
        assert_eq!(resolved.tags()[0].occurrence().site(), &site);
        assert_eq!(
            resolved.tags()[0].target(),
            ftoo_manifest::NodeId::LangevinCoefficientOne
        );
        assert_eq!(corpus.units()[0].texts()["en"].text(), "é See @tag{c:1}.");
    }
}

fn mutate_tag_sites(
    fixture: &Fixture,
    declaration: &str,
    statement: &str,
    text: &str,
    starts: &[usize],
    full: &ftoo_manifest::AddressBook<'_>,
    summary: UnitCheckSummary,
) {
    use ftoo_manifest::TagResolutionFault as F;
    let statement_path = fixture.root.join("units/statements/one.toml");
    let declaration_path = fixture.root.join("units/documents.toml");
    for language in ["en", "fr"] {
        for (index, start) in starts.iter().copied().enumerate() {
            let mut changed = text.to_owned();
            changed.replace_range(start + 5..start + 8, "bad");
            let mutated = statement.replace(
                &format!("{language}='{text}'"),
                &format!("{language}='{changed}'"),
            );
            fs::write(&statement_path, &mutated).unwrap();
            let raw_start = mutated.find(&format!("{language}=")).unwrap() + 3;
            let mut site = location(
                "units/statements/one.toml",
                &["unit", "texts", language],
                SourceAnchor::Bytes(raw_start, raw_start + changed.len() + 2),
            );
            site.decoded = Some(start..start + 9);
            site.occurrence = Some(index);
            let expected = errors(vec![diagnostic(
                RuleId::M042,
                UnitStage::Resolution,
                Some("sec:coefficients:first-value"),
                Some(language),
                site,
                UnitFault::ResolutionFault {
                    cause: F::UnknownTag {
                        input: "bad".into(),
                    },
                },
            )]);
            let actual = check_unit_files(&fixture.root, full).unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(
                actual.to_string(),
                format!(
                    "M042 Resolution unit=Some(\"sec:coefficients:first-value\") language=Some({language:?}) at \"units/statements/one.toml\" $[\"unit\"][\"texts\"][{language:?}] bytes {raw_start}..{} decoded {start}..{} occurrence {index}: tag \"bad\": unknown\n",
                    raw_start + changed.len() + 2,
                    start + 9
                )
            );
            assert_eq!(fs::read(&statement_path).unwrap(), mutated.as_bytes());
            assert_eq!(fs::read(&declaration_path).unwrap(), declaration.as_bytes());
            fs::write(&statement_path, statement).unwrap();
            assert_eq!(check_unit_files(&fixture.root, full).unwrap(), summary);
        }
    }
}

type FileTagCase = (
    &'static str,
    ftoo_manifest::TagId,
    ftoo_manifest::NodeId,
    &'static str,
    ftoo_manifest::OutOfSliceCause,
);
fn file_tag_cases() -> [FileTagCase; 4] {
    use ftoo_manifest::{NodeId as N, OutOfSliceCause as O, TagId as T};
    [
        (
            "c:1",
            T::LangevinCoefficientOne,
            N::LangevinCoefficientOne,
            "def:langevin:coefficient-one",
            O::QuantityOutsideSupport,
        ),
        (
            "c:3",
            T::LangevinCoefficientThree,
            N::LangevinCoefficientThree,
            "def:langevin:coefficient-three",
            O::QuantityOutsideSupport,
        ),
        (
            "v:1",
            T::LangevinCoefficientOneValue,
            N::LangevinCoefficientOneValue,
            "claim:langevin:coefficient-one-value",
            O::ClaimNotSelected,
        ),
        (
            "v:3",
            T::LangevinCoefficientThreeValue,
            N::LangevinCoefficientThreeValue,
            "claim:langevin:coefficient-three-value",
            O::ClaimNotSelected,
        ),
    ]
}
