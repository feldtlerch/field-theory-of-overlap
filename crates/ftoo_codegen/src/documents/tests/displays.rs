use super::*;
use ftoo_manifest::{
    DisplayKind, PathPart, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault, UnitStage,
};

const SKELETON: &str = "schema=1\n[skeleton]\ndocument='main'\nsections=[{heading='sec:fixture:heading',children=[]}]\n";
const DISPLAY: &str = "schema=1\n[display]\nunit='sec:fixture:caption'\nkind='figure'\n[display.bodies.markdown]\nen='> A'\n[display.bodies.latex]\nen='\\begin{picture}(1,1)\\put(0,0){A}\\end{picture}'\n";
fn source(file: &str, text: &str) -> crate::UnitSource {
    crate::UnitSource {
        file: file.into(),
        bytes: text.as_bytes().into(),
    }
}
fn sources(
    text: &str,
) -> Result<
    (
        Vec<ftoo_manifest::SkeletonDraft>,
        Vec<ftoo_manifest::DisplayDraft>,
    ),
    DocumentError,
> {
    input::assembly_sources(
        &[source("units/skeletons/main.toml", SKELETON)],
        &[source("units/displays/a.toml", text)],
    )
}
fn boundary(site: UnitSite, rule: RuleId, stage: UnitStage, fault: UnitFault) -> DocumentError {
    DocumentError::Boundary(
        UnitErrors::from_diagnostics(vec![UnitDiagnostic {
            rule,
            stage,
            unit: None,
            language: None,
            site,
            fault,
        }])
        .unwrap(),
    )
}
fn location(path: &[&str], anchor: SourceAnchor) -> UnitSite {
    UnitSite {
        file: "units/displays/a.toml".into(),
        path: path.iter().map(|p| PathPart::Key((*p).into())).collect(),
        anchor,
        decoded: None,
        occurrence: None,
    }
}
#[test]
fn companion_schema_tokens_and_quoted_key_spelling_are_exact() {
    let original = sources(DISPLAY).unwrap();
    for token in ["0", "2", "+1", "0x1", "0o1", "0b1", "1_0"] {
        let text = DISPLAY.replacen("schema=1", &format!("schema={token}"), 1);
        let expected = boundary(
            location(&["schema"], SourceAnchor::Bytes(7, 7 + token.len())),
            RuleId::M003,
            UnitStage::Shape,
            UnitFault::SchemaValue {
                expected: "1".into(),
                actual: token.into(),
            },
        );
        let actual = sources(&text).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(sources(DISPLAY).unwrap().1, original.1);
    }
    for (token, kind) in [
        ("'1'", ShapeKind::String),
        ("true", ShapeKind::Boolean),
        ("[]", ShapeKind::Array),
        ("{}", ShapeKind::Table),
    ] {
        let text = DISPLAY.replacen("schema=1", &format!("schema={token}"), 1);
        let expected = boundary(
            location(&["schema"], SourceAnchor::Bytes(7, 7 + token.len())),
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::WrongType {
                expected: ShapeKind::Integer,
                actual: kind,
            },
        );
        assert_eq!(sources(&text).unwrap_err(), expected);
        assert_eq!(sources(DISPLAY).unwrap().1, original.1);
    }
    let quoted = sources(&DISPLAY.replacen("schema=1", "\"schema\"=1", 1)).unwrap();
    assert_eq!(quoted.1[0].unit.value, original.1[0].unit.value);
    assert_eq!(quoted.1[0].kind, DisplayKind::Figure);
}
#[test]
fn companion_source_phases_suppress_later_skeleton_and_display_faults() {
    for (bytes, fault, anchor) in [
        (vec![255], UnitFault::InvalidUtf8, SourceAnchor::Bytes(0, 1)),
        (
            vec![239, 187, 191],
            UnitFault::Bom,
            SourceAnchor::Bytes(0, 3),
        ),
    ] {
        let actual = input::assembly_sources(
            &[source("units/skeletons/main.toml", "schema=1\nextra=0")],
            &[crate::UnitSource {
                file: "units/displays/a.toml".into(),
                bytes,
            }],
        )
        .unwrap_err();
        assert_eq!(
            actual,
            boundary(
                location(&[], anchor),
                RuleId::M001,
                UnitStage::Encoding,
                fault
            )
        );
        assert_eq!(
            sources(DISPLAY).unwrap().1[0].unit.value,
            "sec:fixture:caption"
        );
    }
    let text = "schema=1\nschema=1\n";
    let expected = boundary(
        location(&[], SourceAnchor::Bytes(9, 10)),
        RuleId::M002,
        UnitStage::Syntax,
        UnitFault::TomlSyntax {
            message: "duplicate key `schema` in document root".into(),
        },
    );
    assert_eq!(sources(text).unwrap_err(), expected);
    for token in ["1.0", "1e0", "+inf", "-inf", "nan"] {
        let text = format!("schema=1\nextra=[{token}]\n");
        let mut site = location(&["extra"], SourceAnchor::Bytes(16, 16 + token.len()));
        site.path.push(PathPart::Index(0));
        let expected = boundary(
            site,
            RuleId::M003,
            UnitStage::Numeric,
            UnitFault::NonIntegerNumber {
                token: token.into(),
            },
        );
        assert_eq!(sources(&text).unwrap_err(), expected);
    }
}
#[test]
fn companion_shapes_are_closed_at_every_fixed_object() {
    for (text, path, name, expected) in [
        (
            "schema=1\nextra=0\n",
            vec![],
            "extra",
            vec!["schema", "display"],
        ),
        (
            "schema=1\n[display]\nextra=0\n",
            vec!["display"],
            "extra",
            vec!["unit", "kind", "bodies"],
        ),
        (
            "schema=1\n[display]\nunit='sec:fixture:caption'\nkind='table'\n[display.bodies]\nhtml={}\n",
            vec!["display", "bodies"],
            "html",
            vec!["markdown", "latex"],
        ),
    ] {
        let offset = text.find(&format!("{name}=")).unwrap();
        let mut path = path;
        path.push(name);
        let expected = boundary(
            location(&path, SourceAnchor::Bytes(offset, offset + name.len())),
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::UnknownField {
                name: name.into(),
                expected: expected.into_iter().map(str::to_owned).collect(),
            },
        );
        let actual = sources(text).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(sources(DISPLAY).unwrap().1.len(), 1);
    }
    for (before, after, path, expected_kind) in [
        (
            "unit='sec:fixture:caption'",
            "unit=1",
            vec!["display", "unit"],
            ShapeKind::String,
        ),
        (
            "kind='figure'",
            "kind=1",
            vec!["display", "kind"],
            ShapeKind::String,
        ),
        (
            "en='> A'",
            "en=1",
            vec!["display", "bodies", "markdown", "en"],
            ShapeKind::String,
        ),
    ] {
        let text = DISPLAY.replace(before, after);
        let offset = text.find(after).unwrap() + after.len() - 1;
        let expected = boundary(
            location(&path, SourceAnchor::Bytes(offset, offset + 1)),
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::WrongType {
                expected: expected_kind,
                actual: ShapeKind::Integer,
            },
        );
        assert_eq!(sources(&text).unwrap_err(), expected);
        assert_eq!(sources(DISPLAY).unwrap().1.len(), 1);
    }
    let text = DISPLAY.replace("kind='figure'", "kind='image'");
    let start = text.find("'image'").unwrap();
    let mut site = location(&["display", "kind"], SourceAnchor::Bytes(start, start + 7));
    site.decoded = Some(0..5);
    assert_eq!(
        sources(&text).unwrap_err(),
        failure(
            RuleId::M004,
            AssemblyStage::Shape,
            AssemblySite::Source(site),
            AssemblyFault::UnknownToken {
                value: "image".into(),
                expected: vec!["table".into(), "figure".into()]
            }
        )
    );
}

#[test]
fn companion_missing_and_wrong_typed_objects_keep_exact_parent_and_value_sites() {
    for (text, path, name) in [
        ("schema=1\n", vec![], "display"),
        ("display={}\n", vec![], "schema"),
        ("schema=1\ndisplay={}\n", vec!["display"], "unit"),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption'}\n",
            vec!["display"],
            "kind",
        ),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure'}\n",
            vec!["display"],
            "bodies",
        ),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure',bodies={}}\n",
            vec!["display", "bodies"],
            "markdown",
        ),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure',bodies={markdown={}}}\n",
            vec!["display", "bodies"],
            "latex",
        ),
    ] {
        let anchor = path.last().map_or(SourceAnchor::WholeFile, |name| {
            SourceAnchor::Parent(
                text.find(&format!("{name}=")).unwrap() + name.len() + 1,
                text.len() - path.len(),
            )
        });
        let expected = boundary(
            location(&path, anchor),
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::MissingField { name: name.into() },
        );
        let actual = sources(text).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(sources(DISPLAY).unwrap().1.len(), 1);
    }
    for (text, path, marker) in [
        ("schema=1\ndisplay=2\n", vec!["display"], "display="),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure',bodies=2}\n",
            vec!["display", "bodies"],
            "bodies=",
        ),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure',bodies={markdown=2,latex={}}}\n",
            vec!["display", "bodies", "markdown"],
            "markdown=",
        ),
        (
            "schema=1\ndisplay={unit='sec:fixture:caption',kind='figure',bodies={markdown={},latex=2}}\n",
            vec!["display", "bodies", "latex"],
            "latex=",
        ),
    ] {
        let start = text.find(marker).unwrap() + marker.len();
        let expected = boundary(
            location(&path, SourceAnchor::Bytes(start, start + 1)),
            RuleId::M004,
            UnitStage::Shape,
            UnitFault::WrongType {
                expected: ShapeKind::Table,
                actual: ShapeKind::Integer,
            },
        );
        assert_eq!(sources(text).unwrap_err(), expected);
        assert_eq!(sources(DISPLAY).unwrap().1.len(), 1);
    }
}
#[test]
fn companion_global_shapes_collect_one_per_source_and_numeric_faults_precede_all_shapes() {
    let errors = input::assembly_sources(
        &[source("units/skeletons/main.toml", "schema=1\n")],
        &[source("units/displays/a.toml", "schema=1\n")],
    )
    .unwrap_err();
    let expected = DocumentError::Boundary(
        UnitErrors::from_diagnostics(vec![
            UnitDiagnostic {
                rule: RuleId::M004,
                stage: UnitStage::Shape,
                unit: None,
                language: None,
                site: location(&[], SourceAnchor::WholeFile),
                fault: UnitFault::MissingField {
                    name: "display".into(),
                },
            },
            UnitDiagnostic {
                rule: RuleId::M004,
                stage: UnitStage::Shape,
                unit: None,
                language: None,
                site: UnitSite {
                    anchor: SourceAnchor::WholeFile,
                    ..input::site("units/skeletons/main.toml")
                },
                fault: UnitFault::MissingField {
                    name: "skeleton".into(),
                },
            },
        ])
        .unwrap(),
    );
    assert_eq!(errors, expected);
    assert_eq!(errors.to_string(), expected.to_string());
    let invalid = source("units/displays/a.toml", "schema=1\nextra=[1.0]\n");
    let mut site = location(&["extra"], SourceAnchor::Bytes(16, 19));
    site.path.push(PathPart::Index(0));
    assert_eq!(
        input::assembly_sources(
            &[source("units/skeletons/main.toml", "schema=1\n")],
            &[invalid]
        )
        .unwrap_err(),
        boundary(
            site,
            RuleId::M003,
            UnitStage::Numeric,
            UnitFault::NonIntegerNumber {
                token: "1.0".into()
            }
        )
    );
}
#[test]
fn companion_actual_file_mutations_restore_and_reread_the_same_inventory() {
    let directory = Directory::new();
    fs::create_dir_all(directory.0.join("units/displays")).unwrap();
    let path = directory.0.join("units/displays/a.toml");
    fs::write(&path, DISPLAY).unwrap();
    let read = || {
        let files = input::discover(&directory.0, "units/displays", true).unwrap();
        let sources = input::sources(&directory.0, &files).unwrap();
        input::assembly_sources(&[source("units/skeletons/main.toml", SKELETON)], &sources)
    };
    let original = fs::read(&path).unwrap();
    let baseline = read().unwrap();
    for token in ["+1", "0x1", "2"] {
        let changed = DISPLAY.replacen("schema=1", &format!("schema={token}"), 1);
        fs::write(&path, &changed).unwrap();
        let expected = boundary(
            location(&["schema"], SourceAnchor::Bytes(7, 7 + token.len())),
            RuleId::M003,
            UnitStage::Shape,
            UnitFault::SchemaValue {
                expected: "1".into(),
                actual: token.into(),
            },
        );
        assert_eq!(read().unwrap_err(), expected);
        assert_eq!(fs::read(&path).unwrap(), changed.as_bytes());
        fs::write(&path, &original).unwrap();
        assert_eq!(read().unwrap().1, baseline.1);
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}

#[test]
fn companion_source_inventory_and_byte_bounds_have_complete_controls() {
    let skeleton = source("units/skeletons/main.toml", SKELETON);
    for name in [
        "",
        "../escape.toml",
        "units/displays/a\\b.toml",
        "units/displays/a\0.toml",
    ] {
        assert_eq!(
            input::assembly_sources(std::slice::from_ref(&skeleton), &[source(name, DISPLAY)])
                .unwrap_err(),
            DocumentError::Input(DocumentInputFault::InvalidSourceName { value: name.into() })
        );
    }
    let display = source("units/displays/a.toml", DISPLAY);
    assert_eq!(
        input::assembly_sources(std::slice::from_ref(&skeleton), &[display.clone(), display])
            .unwrap_err(),
        DocumentError::Input(DocumentInputFault::DuplicateSource {
            file: "units/displays/a.toml".into(),
            positions: vec![1, 2]
        })
    );
    let mut bytes = DISPLAY.to_owned();
    bytes.push('#');
    bytes.push_str(&"x".repeat(1_048_576 - bytes.len()));
    assert_eq!(
        sources(&bytes).unwrap().1[0].unit.value,
        "sec:fixture:caption"
    );
    bytes.push('x');
    let site = AssemblySite::Source(input::site("units/displays/a.toml"));
    let fault = AssemblyFault::Limit {
        resource: ftoo_manifest::AssemblyResource::SourceBytes,
        limit: 1_048_576,
        actual: 1_048_577,
    };
    let actual = sources(&bytes).unwrap_err();
    assert_eq!(
        actual,
        failure(
            RuleId::M058,
            AssemblyStage::Source,
            site.clone(),
            fault.clone()
        )
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "M058 Source document=None unit=None language=None target=None at {site:?}: {fault:?}\n"
        )
    );
    let restored = sources(DISPLAY).unwrap();
    assert_eq!(restored.1[0].unit.value, "sec:fixture:caption");
}

#[cfg(unix)]
#[test]
fn companion_discovery_refuses_entries_and_restores_absent_and_empty_controls() {
    use std::os::unix::fs::symlink;
    let directory = Directory::new();
    let folder = directory.0.join("units/displays");
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(vec![])
    );
    fs::create_dir_all(&folder).unwrap();
    assert_eq!(
        input::discover(&directory.0, "units/displays", true),
        Ok(vec![])
    );
    let path = folder.join("a.toml");
    fs::write(&path, DISPLAY).unwrap();
    let read = || {
        let files = input::discover(&directory.0, "units/displays", true)?;
        input::sources(&directory.0, &files)
    };
    let baseline = vec![source("units/displays/a.toml", DISPLAY)];
    assert_eq!(read(), Ok(baseline.clone()));
    for kind in [
        DocumentEntryKind::Symlink,
        DocumentEntryKind::Directory,
        DocumentEntryKind::NonTomlFile,
    ] {
        fs::remove_file(&path).unwrap();
        let bad_path = if kind == DocumentEntryKind::NonTomlFile {
            folder.join("a.txt")
        } else {
            path.clone()
        };
        match kind {
            DocumentEntryKind::Symlink => symlink("missing", &bad_path).unwrap(),
            DocumentEntryKind::Directory => fs::create_dir(&bad_path).unwrap(),
            _ => fs::write(&bad_path, DISPLAY).unwrap(),
        }
        let fault = DocumentError::Input(DocumentInputFault::UnexpectedEntry {
            path: bad_path.clone(),
            kind,
        });
        let actual = read().unwrap_err();
        assert_eq!(actual, fault);
        assert_eq!(
            actual.to_string(),
            format!("Input(UnexpectedEntry {{ path: {bad_path:?}, kind: {kind:?} }})")
        );
        if kind == DocumentEntryKind::Directory {
            fs::remove_dir(&bad_path).unwrap();
        } else {
            fs::remove_file(&bad_path).unwrap();
        }
        fs::write(&path, DISPLAY).unwrap();
        assert_eq!(read(), Ok(baseline.clone()));
        assert_eq!(fs::read(&path).unwrap(), DISPLAY.as_bytes());
    }
    fs::remove_file(&path).unwrap();
    fs::remove_dir(&folder).unwrap();
    assert_eq!(read(), Ok(vec![]));
}

#[test]
fn display_owner_does_not_relax_closed_statement_schema_or_unused_math() {
    let declaration = source(
        "units/documents.toml",
        "schema=1\n[documents.main]\nrequired-languages=['en']\n",
    );
    let text = "schema=1\n[unit]\nid='sec:fixture:caption'\ndocument='main'\nmath={value='x'}\n[unit.texts]\nen='Caption.'\n";
    let value_at = text.find("value=").unwrap();
    let mut site = input::site("units/statements/caption.toml");
    site.path = vec![
        PathPart::Key("unit".into()),
        PathPart::Key("math".into()),
        PathPart::Key("value".into()),
    ];
    site.anchor = SourceAnchor::Bytes(value_at, value_at + 5);
    let expected = crate::UnitLoadError::Admission(
        UnitErrors::from_diagnostics(vec![UnitDiagnostic {
            rule: RuleId::M041,
            stage: UnitStage::Content,
            unit: Some("sec:fixture:caption".into()),
            language: None,
            site,
            fault: UnitFault::UnusedMath {
                name: "value".into(),
            },
        }])
        .unwrap(),
    );
    let companion = DISPLAY.replace("> A", "> @math{value}");
    assert_eq!(sources(&companion).unwrap().1.len(), 1);
    let actual = crate::load_unit_sources(
        declaration.clone(),
        vec![source("units/statements/caption.toml", text)],
    )
    .unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    let valid = text.replace("Caption.", "Caption @math{value}.");
    let baseline = crate::load_unit_sources(
        declaration.clone(),
        vec![source("units/statements/caption.toml", &valid)],
    )
    .unwrap();
    let changed = valid.replacen("math={value='x'}", "display={}\nmath={value='x'}", 1);
    let at = changed.find("display=").unwrap();
    let expected = crate::UnitLoadError::Admission(
        UnitErrors::from_diagnostics(vec![UnitDiagnostic {
            rule: RuleId::M004,
            stage: UnitStage::Shape,
            unit: Some("sec:fixture:caption".into()),
            language: None,
            site: UnitSite {
                file: "units/statements/caption.toml".into(),
                path: vec![
                    PathPart::Key("unit".into()),
                    PathPart::Key("display".into()),
                ],
                anchor: SourceAnchor::Bytes(at, at + 7),
                decoded: None,
                occurrence: None,
            },
            fault: UnitFault::UnknownField {
                name: "display".into(),
                expected: vec![
                    "id".into(),
                    "document".into(),
                    "math".into(),
                    "texts".into(),
                ],
            },
        }])
        .unwrap(),
    );
    assert_eq!(
        crate::load_unit_sources(
            declaration.clone(),
            vec![source("units/statements/caption.toml", &changed)]
        )
        .unwrap_err(),
        expected
    );
    assert_eq!(
        crate::load_unit_sources(
            declaration,
            vec![source("units/statements/caption.toml", &valid)]
        )
        .unwrap(),
        baseline
    );
}

fn fixture_plan(kind: DisplayKind, markdown: &str, latex: &str) -> plan::DocumentPlan {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let mut bodies = BTreeMap::new();
    for (target, text) in [
        (DocumentTarget::Markdown, markdown),
        (DocumentTarget::Latex, latex),
    ] {
        let source = ftoo_manifest::LocatedString {
            value: text.into(),
            site: location(
                &[
                    "display",
                    "bodies",
                    if target == DocumentTarget::Markdown {
                        "markdown"
                    } else {
                        "latex"
                    },
                    "en",
                ],
                SourceAnchor::Unavailable,
            ),
        };
        let parsed = ftoo_manifest::parse_display_body(kind, target, &source).unwrap();
        let mut regions = Vec::new();
        for region in parsed.regions() {
            let text = &region.source().value;
            let mut replacements = Vec::new();
            for tag in region.tags() {
                let site = region.site(tag.site());
                let token = format!(
                    "u1-d{}-en-o{}",
                    if target == DocumentTarget::Markdown {
                        "markdown"
                    } else {
                        "latex"
                    },
                    site.occurrence.unwrap()
                );
                replacements.push((
                    tag.site().decoded.clone().unwrap(),
                    plan::Inline::Tag {
                        handle: tag.input().into(),
                        label: book.resolve(tag.input()).unwrap().semantic_name().into(),
                        token,
                        site,
                    },
                ));
            }
            for math in region.math() {
                replacements.push((math.site().decoded.clone().unwrap(),plan::Inline::Math {key:math.name().into(),markdown:r"\underbrace{{c}_{1}}_{\text{pending: retained formulation def:langevin:coefficient-one}}".into(),latex:r"\FtooQCoefficientOne{}".into(),site:region.site(math.site())}));
            }
            for (value, site, citation) in region.spans() {
                replacements.push((
                    site.decoded.clone().unwrap(),
                    if citation {
                        plan::Inline::Citation {
                            text: value,
                            site: region.site(&site),
                        }
                    } else {
                        plan::Inline::Mention {
                            text: value,
                            site: region.site(&site),
                        }
                    },
                ));
            }
            replacements.sort_by_key(|(range, _)| range.start);
            let mut runs = Vec::new();
            let mut at = 0;
            for (range, run) in replacements {
                if at < range.start {
                    runs.push(plan::Inline::Text(text[at..range.start].into()));
                }
                runs.push(run);
                at = range.end;
            }
            if at < text.len() {
                runs.push(plan::Inline::Text(text[at..].into()));
            }
            if let Some(plan::Inline::Text(text)) = runs.first_mut() {
                *text = text.trim_start().into();
            }
            if let Some(plan::Inline::Text(text)) = runs.last_mut() {
                *text = text.trim_end().into();
            }
            regions.push(plan::Block {
                unit: "sec:fixture:caption".into(),
                depth: None,
                site: region.site(&region.source().site),
                runs,
                displays: BTreeMap::new(),
            });
        }
        bodies.insert(
            target,
            plan::DisplayBody {
                source: source.value,
                layout: parsed.layout().clone(),
                regions,
                site: source.site,
            },
        );
    }
    plan::DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: ["Coverage.".into(), "Slice.".into()],
        blocks: vec![plan::Block {
            unit: "sec:fixture:caption".into(),
            depth: None,
            site: input::site("units/statements/caption.toml"),
            runs: vec![plan::Inline::Text("Registered claims.".into())],
            displays: bodies,
        }],
    }
}
const TABLE_MD: &str = "| Coefficient | Claim |\n| :--- | ---: |\n| @tag{c:1} | @tag{v:1} |";
const TABLE_TEX: &str =
    r"\begin{tabular}{lr} Coefficient & Claim \\ \hline @tag{c:1} & @tag{v:1} \\ \end{tabular}";
const FIGURE_MD: &str = "> @tag{c:1} → @tag{c:3}";
const FIGURE_TEX: &str = r"\begin{picture}(120,20)\put(0,10){@tag{c:1}}\put(35,10){\vector(1,0){40}}\put(90,10){@tag{c:3}}\end{picture}";

fn expected_records(kind: DisplayKind, target: DocumentTarget) -> Vec<EmissionRecord> {
    let text = match (kind, target) {
        (DisplayKind::Table, DocumentTarget::Markdown) => TABLE_MD,
        (DisplayKind::Table, DocumentTarget::Latex) => TABLE_TEX,
        (DisplayKind::Figure, DocumentTarget::Markdown) => FIGURE_MD,
        (DisplayKind::Figure, DocumentTarget::Latex) => FIGURE_TEX,
    };
    let source_site = location(
        &[
            "display",
            "bodies",
            if target == DocumentTarget::Markdown {
                "markdown"
            } else {
                "latex"
            },
            "en",
        ],
        SourceAnchor::Unavailable,
    );
    let record = |kind, value: &str, site: UnitSite| EmissionRecord {
        kind,
        unit: "sec:fixture:caption".into(),
        language: "en".into(),
        target,
        occurrence: site.occurrence,
        value: value.into(),
        site,
    };
    let mut records = vec![
        record(
            EmissionKind::Caption,
            "Registered claims.",
            input::site("units/statements/caption.toml"),
        ),
        record(EmissionKind::Body, text, source_site.clone()),
    ];
    for (index, handle, label) in [
        (0, "c:1", "def:langevin:coefficient-one"),
        (
            1,
            if kind == DisplayKind::Table {
                "v:1"
            } else {
                "c:3"
            },
            if kind == DisplayKind::Table {
                "claim:langevin:coefficient-one-value"
            } else {
                "def:langevin:coefficient-three"
            },
        ),
    ] {
        let mut site = source_site.clone();
        site.path.push(PathPart::Key("regions".into()));
        if kind == DisplayKind::Table {
            site.path.extend([
                PathPart::Index(if target == DocumentTarget::Markdown {
                    2
                } else {
                    1
                }),
                PathPart::Index(index),
            ]);
        } else {
            site.path
                .push(PathPart::Index(if target == DocumentTarget::Markdown {
                    0
                } else {
                    index * 2
                }));
        }
        let start = text.find(&format!("@tag{{{handle}}}")).unwrap();
        site.decoded = Some(start..start + 9);
        site.occurrence = Some(index);
        records.push(record(EmissionKind::Tag, label, site));
    }
    records
}

#[test]
fn display_emission_omission_repetition_and_change_have_complete_inventories() {
    for (kind, md, tex) in [
        (DisplayKind::Table, TABLE_MD, TABLE_TEX),
        (DisplayKind::Figure, FIGURE_MD, FIGURE_TEX),
    ] {
        let plan = fixture_plan(kind, md, tex);
        for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
            let expected = expected_records(kind, target);
            assert_eq!(plan.records(target), expected);
            let origins: Vec<_> = expected
                .iter()
                .map(|record| SourceOrigin {
                    unit: Some(record.unit.clone()),
                    language: Some(record.language.clone()),
                    target: Some(target),
                    site: record.site.clone(),
                })
                .collect();
            assert_eq!(plan.origins(target), origins);
            let source = if target == DocumentTarget::Markdown {
                markdown::render(&plan)
            } else {
                latex::render(&plan)
            };
            let body = display::render(&plan.blocks[0].displays[&target], target);
            let beginning = source.find(&body).unwrap();
            let missing = if target == DocumentTarget::Markdown {
                source[..beginning - 1].to_owned()
            } else {
                source.replace(&format!("{body}\n"), "")
            };
            let repeated = source.replacen(&body, &format!("{body}\n{body}"), 1);
            let changed = source.replacen(
                "label def:langevin:coefficient-one",
                "label def:langevin:coefficient-three",
                1,
            );
            let mut changed_body = expected[1].clone();
            changed_body.value = body.trim_end_matches('\n').replacen(
                "label def:langevin:coefficient-one",
                "label def:langevin:coefficient-three",
                1,
            );
            let mut changed_tag = expected[2].clone();
            changed_tag.value = "def:langevin:coefficient-three".into();
            let cases = [
                (
                    missing,
                    AssemblyFault::EmissionMismatch {
                        missing: expected[1..].to_vec(),
                        unexpected: vec![],
                        changed: vec![],
                    },
                ),
                (
                    repeated,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: expected[1..].to_vec(),
                        changed: vec![],
                    },
                ),
                (
                    changed,
                    AssemblyFault::EmissionMismatch {
                        missing: vec![],
                        unexpected: vec![],
                        changed: vec![
                            EmissionChange {
                                expected: expected[1].clone(),
                                actual: changed_body,
                            },
                            EmissionChange {
                                expected: expected[2].clone(),
                                actual: changed_tag,
                            },
                        ],
                    },
                ),
            ];
            for (changed, fault) in cases {
                let marker = if target == DocumentTarget::Markdown {
                    "<!-- unit "
                } else {
                    "% unit "
                };
                let start = changed.find(marker).unwrap();
                let site = AssemblySite::Generated {
                    path: format!(
                        "generated/documents/main/en.{}",
                        if target == DocumentTarget::Markdown {
                            "md"
                        } else {
                            "tex"
                        }
                    ),
                    bytes: start..changed.len(),
                    origins: origins.clone(),
                };
                let expected = failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    site.clone(),
                    fault.clone(),
                );
                let actual = plan::check_emission(&plan, &changed, target).unwrap_err();
                assert_eq!(actual, expected);
                assert_eq!(
                    actual.to_string(),
                    format!(
                        "M055 Emission document=None unit=None language=None target=None at {site:?}: {fault:?}\n"
                    )
                );
                assert_eq!(plan::check_emission(&plan, &source, target), Ok(()));
            }
        }
    }
}
#[test]
fn display_writers_have_independent_full_document_goldens_and_neutral_harvests() {
    for (kind, md, tex, md_body, tex_body) in [
        (
            DisplayKind::Table,
            TABLE_MD,
            TABLE_TEX,
            "| Coefficient | Claim |\n| :--- | ---: |\n| c:1<!-- label def:langevin:coefficient-one --> | v:1<!-- label claim:langevin:coefficient-one-value --> |\n",
            "\\begin{center}\n\\begin{tabular}{lr}\nCoefficient & Claim \\\\\n\\hline\n\\FtdocTag{c:1}{u1-dlatex-en-o0}% label def:langevin:coefficient-one\n & \\FtdocTag{v:1}{u1-dlatex-en-o1}% label claim:langevin:coefficient-one-value\n \\\\\n\\end{tabular}\n\\end{center}\n",
        ),
        (
            DisplayKind::Figure,
            FIGURE_MD,
            FIGURE_TEX,
            "> c:1<!-- label def:langevin:coefficient-one --> → c:3<!-- label def:langevin:coefficient-three -->\n",
            "\\begingroup\\setlength{\\unitlength}{1pt}\n\\begin{picture}(120,20)\n\\put(0,10){\\FtdocTag{c:1}{u1-dlatex-en-o0}% label def:langevin:coefficient-one\n}\n\\put(35,10){\\vector(1,0){40}}\n\\put(90,10){\\FtdocTag{c:3}{u1-dlatex-en-o1}% label def:langevin:coefficient-three\n}\n\\end{picture}\n\\endgroup\n",
        ),
    ] {
        let plan = fixture_plan(kind, md, tex);
        let expected_md = format!(
            "<!-- generated document main language en -->\nCoverage.\nSlice.\n\n<!-- unit sec:fixture:caption document main language en -->\nRegistered claims.\n\n{md_body}"
        );
        let expected_tex = format!(
            "{}\\FtdocContext{{main}}{{en}}\n\\begin{{document}}\nCoverage.\n\nSlice.\n\n% unit sec:fixture:caption document main language en\nRegistered claims.\n\n{tex_body}\n\\end{{document}}\n",
            latex::PREAMBLE
        );
        assert_eq!(markdown::render(&plan), expected_md);
        assert_eq!(latex::render(&plan), expected_tex);
        assert_eq!(markdown::size(&plan), expected_md.len());
        assert_eq!(latex::size(&plan), expected_tex.len());
        for (target, source) in [
            (DocumentTarget::Markdown, expected_md.as_str()),
            (DocumentTarget::Latex, expected_tex.as_str()),
        ] {
            assert_eq!(plan::check_emission(&plan, source, target), Ok(()));
        }
        let harvest = ftoo_manifest::check_markdown_sources(vec![ftoo_manifest::MarkdownSource {
            file: "generated/documents/main/en.md".into(),
            bytes: expected_md.into_bytes(),
        }])
        .unwrap();
        assert_eq!(harvest.mints(), []);
        assert_eq!(harvest.citations(), []);
    }
}
#[test]
fn support_required_real_table_and_picture_consume_pending_math_and_literal_spans()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
    let lock = native_support::lock(&root).map_err(DocumentError::Native)?;
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let notation = fs::read_to_string(root.join("generated/notation.tex"))?;
    for (kind, md, tex) in [
        (
            DisplayKind::Table,
            "| Value |\n| --- |\n| @math{value} `dec:weld:equal-address-sets` |",
            r"\begin{tabular}{l} Value \\ @math{value} `dec:weld:equal-address-sets` \\ \end{tabular}",
        ),
        (
            DisplayKind::Figure,
            "> @math{value} @tag{c:1}",
            r"\begin{picture}(120,20)\put(0,10){@math{value} @tag{c:1}}\put(35,0){\line(1,0){40}}\end{picture}",
        ),
        (DisplayKind::Figure, FIGURE_MD, FIGURE_TEX),
    ] {
        let mut plan = fixture_plan(kind, md, tex);
        if kind == DisplayKind::Figure {
            relation_caption(&mut plan);
        }
        let address = runtime::render(&book, std::slice::from_ref(&plan))?;
        let source = latex::render(&plan);
        assert_eq!(
            plan::check_emission(&plan, &source, DocumentTarget::Latex),
            Ok(())
        );
        native::consume_plan(&support, &lock, &source, &notation, &address, &plan)?;
        assert_eq!(
            runtime::check(&book, std::slice::from_ref(&plan), &address),
            Ok(())
        );
    }
    Ok(())
}

fn relation_caption(plan: &mut plan::DocumentPlan) {
    let block = &mut plan.blocks[0];
    block.unit = "sec:coefficients:relation".into();
    block.runs=vec![
        plan::Inline::Text("Relation ".into()),
        plan::Inline::Math {key:"value".into(),markdown:r"({\underbrace{{c}_{1}}_{\text{pending: retained formulation def:langevin:coefficient-one}}}={\frac{1}{3}})".into(),latex:r"({\FtooQCoefficientOne{}}={\frac{1}{3}})".into(),site:input::site("units/statements/relation.toml")},
        plan::Inline::Text("; see ".into()),
        plan::Inline::Citation {text:"dec:weld:equal-address-sets".into(),site:input::site("units/statements/relation.toml")},
        plan::Inline::Text(".".into()),
    ];
    for body in block.displays.values_mut() {
        for region in &mut body.regions {
            region.unit.clone_from(&block.unit);
        }
    }
}

#[test]
fn complete_figure_caption_pending_math_and_citation_match_literal_blocks() {
    let mut plan = fixture_plan(DisplayKind::Figure, FIGURE_MD, FIGURE_TEX);
    relation_caption(&mut plan);
    let markdown = concat!(
        "<!-- generated document main language en -->\nCoverage.\nSlice.\n\n",
        "<!-- unit sec:coefficients:relation document main language en -->\n",
        r"Relation $({\underbrace{{c}_{1}}_{\text{pending: retained formulation def:langevin:coefficient-one}}}={\frac{1}{3}})$; see <code>dec:weld:equal-address-sets</code><!-- cite dec:weld:equal-address-sets -->.",
        "\n\n> c:1<!-- label def:langevin:coefficient-one --> → c:3<!-- label def:langevin:coefficient-three -->\n",
    );
    let latex = format!(
        "{}{}",
        latex::PREAMBLE,
        r"\FtdocContext{main}{en}
\begin{document}
Coverage.

Slice.

% unit sec:coefficients:relation document main language en
Relation \(({\FtooQCoefficientOne{}}={\frac{1}{3}})\); see \texttt{dec:weld:equal-address-sets}% cite dec:weld:equal-address-sets
.

\begingroup\setlength{\unitlength}{1pt}
\begin{picture}(120,20)
\put(0,10){\FtdocTag{c:1}{u1-dlatex-en-o0}% label def:langevin:coefficient-one
}
\put(35,10){\vector(1,0){40}}
\put(90,10){\FtdocTag{c:3}{u1-dlatex-en-o1}% label def:langevin:coefficient-three
}
\end{picture}
\endgroup

\end{document}
"
    );
    assert_eq!(markdown::render(&plan), markdown);
    assert_eq!(latex::render(&plan), latex);
    assert_eq!(markdown::size(&plan), markdown.len());
    assert_eq!(latex::size(&plan), latex.len());
    assert_eq!(
        plan::check_emission(&plan, markdown, DocumentTarget::Markdown),
        Ok(())
    );
    assert_eq!(
        plan::check_emission(&plan, &latex, DocumentTarget::Latex),
        Ok(())
    );
}
