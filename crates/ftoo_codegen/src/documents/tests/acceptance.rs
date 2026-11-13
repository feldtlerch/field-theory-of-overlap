use super::*;
use ftoo_manifest::{PathPart, ShapeKind, UnitDiagnostic, UnitErrors, UnitFault, UnitStage};

#[test]
fn native_literal_runtime_prefix_is_not_a_protocol_record() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(256),
        stdout: b"stdout\n".to_vec(),
        stderr: b"stderr\n".to_vec(),
    };
    let log = b"l.12 ordinary FTDOC-prefix text\n! Undefined control sequence.\n".to_vec();
    let site = AssemblySite::Generated {
        path: "generated/documents/fixture/en.tex".into(),
        bytes: 0..42,
        origins: vec![],
    };
    let case = NativeCase::Document {
        document: "fixture".into(),
        language: "en".into(),
    };
    let fault = AssemblyFault::NativeRejected {
        case: case.clone(),
        status: Some(1),
        stdout: b"stdout\n".to_vec(),
        stderr: b"stderr\n".to_vec(),
        log: log.clone(),
    };
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M056,
            stage: AssemblyStage::Native,
            document: Some("fixture".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: site.clone(),
            fault: fault.clone(),
        }])
        .unwrap(),
    );
    let actual = native::classify(output, log, site.clone(), case, false).unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_string(),
        format!(
            "M056 Native document=Some(\"fixture\") unit=None language=Some(\"en\") target=Some(Latex) at {site:?}: {fault:?}\n"
        )
    );
}

fn native_negative_sources() -> Vec<(&'static str, String, String, NativeCase)> {
    let start = latex::start("fixture", "en");
    let complete = format!("{start}Test.\n\\end{{document}}\n");
    vec![
        (
            "stored",
            format!("{start}\\FtdocStored\n\\end{{document}}\n"),
            "\\def\\FtdocStored{\\UndefinedStoredCommand}\n".into(),
            NativeCase::Document {
                document: "fixture".into(),
                language: "en".into(),
            },
        ),
        (
            "collision",
            complete.replace(
                "\\input{notation.tex}",
                "\\newcommand{\\FtooQCoefficientOne}{occupied}\n\\input{notation.tex}",
            ),
            "\\newcommand{\\FtooQCoefficientOne}[0]{1}\n".into(),
            NativeCase::Collision {
                command: "FtooQCoefficientOne".into(),
            },
        ),
        (
            "include",
            complete.replace("\\input{addressing.tex}", "\\input{missing-addressing.tex}"),
            String::new(),
            NativeCase::Document {
                document: "fixture".into(),
                language: "en".into(),
            },
        ),
        (
            "closure",
            format!("{start}Test.\n"),
            String::new(),
            NativeCase::Document {
                document: "fixture".into(),
                language: "en".into(),
            },
        ),
    ]
}

pub(super) fn support_required_native_negative_contexts_pin_complete_evidence_and_restore(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    for (name, source, notation, case) in native_negative_sources() {
        if name == "stored" {
            let control = source.replace("\\FtdocStored\n", "Declaration control.\n");
            checks.push(CaseEvidence::new(
                format!("negative/{name}/declaration"),
                native::consume(
                    support,
                    lock,
                    &control,
                    &notation,
                    &addressing,
                    case.clone(),
                    vec![],
                ),
                Ok(()),
            ));
        }
        let origins = vec![SourceOrigin {
            unit: Some("sec:fixture:unit".into()),
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: UnitSite {
                file: "units/statements/fixture.toml".into(),
                path: vec![PathPart::Key("text".into()), PathPart::Key("en".into())],
                anchor: SourceAnchor::Bytes(30, 40),
                decoded: Some(0..8),
                occurrence: None,
            },
        }];
        let mut scratch = PathBuf::new();
        let result = native::consume_with(
            support,
            lock,
            &source,
            &notation,
            &addressing,
            case.clone(),
            origins.clone(),
            |path| scratch = path.into(),
            |_, _| {},
        );
        let (stdout, stderr, log) = if support.cold_format.get() {
            native_evidence::evidence(name)
        } else {
            native_evidence::seeded_evidence(name)
        };
        let document = (name != "collision").then(|| "fixture".to_owned());
        let language = (name != "collision").then(|| "en".to_owned());
        let target = (name != "collision").then_some(DocumentTarget::Latex);
        let site = AssemblySite::Generated {
            path: if name == "collision" {
                "document.tex"
            } else {
                "generated/documents/fixture/en.tex"
            }
            .into(),
            bytes: 0..source.len(),
            origins: origins.clone(),
        };
        let fault = AssemblyFault::NativeRejected {
            case: case.clone(),
            status: Some(1),
            stdout: stdout
                .replace("@SCRATCH@", scratch.to_str().unwrap())
                .into_bytes(),
            stderr: stderr.as_bytes().to_vec(),
            log: log.as_bytes().to_vec(),
        };
        let expected = DocumentError::Assembly(
            AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                rule: RuleId::M056,
                stage: AssemblyStage::Native,
                document: document.clone(),
                unit: None,
                language: language.clone(),
                target,
                site: site.clone(),
                fault: fault.clone(),
            }])
            .unwrap(),
        );
        let actual = result;
        checks.push(CaseEvidence::with_display(format!("negative/{name}/refusal"), actual, Err(expected), format!(
                "M056 Native document={document:?} unit=None language={language:?} target={target:?} at {site:?}: {fault:?}\n"
            )));
        let restored = if name == "stored" {
            source.clone()
        } else {
            format!(
                "{}Test.\n\\end{{document}}\n",
                latex::start("fixture", "en")
            )
        };
        let restored_notation = notation.replace("\\UndefinedStoredCommand", "Restored");
        checks.push(CaseEvidence::new(
            format!("negative/{name}/restored"),
            native::consume(
                support,
                lock,
                &restored,
                &restored_notation,
                &addressing,
                case,
                origins,
            ),
            Ok(()),
        ));
    }
    Ok(())
}

fn small_lock() -> String {
    let digest = "a".repeat(64);
    format!(
        "schema=1\nengine-version='0.15.0'\nformat-serial=33\n[executable]\nplatform='x86_64-unknown-linux-musl'\nurl='https://example.invalid/engine'\narchive-sha256='{digest}'\nmember='tectonic'\nsha256='{digest}'\n[bundle]\nurl='https://example.invalid/bundle'\narchive-sha256='{digest}'\nbundle-digest='{digest}'\nfiles=[{{member='SHA256SUM',sha256='{digest}'}}]\n"
    )
}

#[test]
fn native_lock_metadata_faults_keep_literal_sites_and_restore() {
    let directory = Directory::new();
    let path = directory.0.join("tools/assembly/native-lock.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = small_lock();
    let restore = || {
        fs::write(&path, &original).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let lock = native_support::lock(&directory.0).unwrap();
        assert_eq!(
            (lock.engine, lock.bundle_digest, lock.files),
            (
                "a".repeat(64),
                "a".repeat(64),
                BTreeMap::from([("SHA256SUM".into(), "a".repeat(64))])
            )
        );
    };
    restore();
    let check = |source: &str, site: UnitSite, reason: NativeMetadataReason| {
        fs::write(&path, source).unwrap();
        let expected = NativeAdapterFault::Metadata { site, reason };
        let actual = DocumentError::Native(native_support::lock(&directory.0).unwrap_err());
        assert_eq!(actual, DocumentError::Native(expected.clone()));
        assert_eq!(actual.to_string(), format!("Native({expected:?})"));
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        restore();
    };
    let literal = |keys: &[&str], bytes: std::ops::Range<usize>| UnitSite {
        file: "tools/assembly/native-lock.toml".into(),
        path: keys
            .iter()
            .map(|key| PathPart::Key((*key).into()))
            .collect(),
        anchor: SourceAnchor::Bytes(bytes.start, bytes.end),
        decoded: None,
        occurrence: None,
    };
    for duplicate in ["schema", "\"sche\\u006da\""] {
        let source = original.replacen("schema=1\n", &format!("schema=1\n{duplicate}=1\n"), 1);
        check(
            &source,
            literal(&[], 9..10),
            NativeMetadataReason::Boundary(UnitFault::TomlSyntax {
                message: "duplicate key `schema` in document root".into(),
            }),
        );
    }
    let source = format!("extra=[1.0]\n{original}");
    let mut site = literal(&["extra"], 7..10);
    site.path.push(PathPart::Index(0));
    check(
        &source,
        site,
        NativeMetadataReason::Boundary(UnitFault::NonIntegerNumber {
            token: "1.0".into(),
        }),
    );
    for (needle, replacement, keys, expected, actual) in [
        (
            "platform='x86_64-unknown-linux-musl'".to_owned(),
            "platform='aarch64'".to_owned(),
            vec!["executable", "platform"],
            "x86_64-unknown-linux-musl",
            "aarch64".to_owned(),
        ),
        (
            format!("archive-sha256='{}'", "a".repeat(64)),
            "archive-sha256='UPPERCASE'".to_owned(),
            vec!["executable", "archive-sha256"],
            "lowercase 64-hex SHA-256",
            "UPPERCASE".to_owned(),
        ),
        (
            "member='tectonic'".to_owned(),
            "member='../tectonic'".to_owned(),
            vec!["executable", "member"],
            "relative member with nonempty safe components",
            "../tectonic".to_owned(),
        ),
    ] {
        let source = original.replacen(&needle, &replacement, 1);
        let quoted = format!("'{actual}'");
        let start = source.find(&quoted).unwrap();
        check(
            &source,
            literal(&keys, start..start + quoted.len()),
            NativeMetadataReason::InvalidValue {
                expected: expected.into(),
                actual,
            },
        );
    }
    let row = format!("{{member='SHA256SUM',sha256='{}'}}", "a".repeat(64));
    let source = original.replace(&row, &format!("{row},{row}"));
    let sites: Vec<_> = source
        .match_indices("'SHA256SUM'")
        .enumerate()
        .map(|(index, (start, _))| UnitSite {
            file: "tools/assembly/native-lock.toml".into(),
            path: vec![
                PathPart::Key("bundle".into()),
                PathPart::Key("files".into()),
                PathPart::Index(index),
                PathPart::Key("member".into()),
            ],
            anchor: SourceAnchor::Bytes(start, start + 11),
            decoded: None,
            occurrence: None,
        })
        .collect();
    check(
        &source,
        sites[1].clone(),
        NativeMetadataReason::DuplicateMember {
            member: "SHA256SUM".into(),
            sites,
        },
    );
}

#[test]
fn native_lock_spelling_and_preallocation_bounds_restore() {
    let directory = Directory::new();
    let path = directory.0.join("tools/assembly/native-lock.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = small_lock();
    let control = |source: &str| {
        fs::write(&path, source).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        let lock = native_support::lock(&directory.0).unwrap();
        assert_eq!(
            (lock.engine, lock.bundle_digest, lock.files),
            (
                "a".repeat(64),
                "a".repeat(64),
                BTreeMap::from([("SHA256SUM".into(), "a".repeat(64))])
            )
        );
    };
    control(&original);
    for key in ["schema", "'schema'", "\"schema\"", "\"sch\\u0065ma\""] {
        for integer in ["33", "+33", "3_3", "0x21", "0o41", "0b100001"] {
            control(
                &original
                    .replacen("schema=1", &format!("{key}=1"), 1)
                    .replacen("format-serial=33", &format!("format-serial={integer}"), 1),
            );
            control(&original);
        }
    }
    for integer in ["+1", "0x1", "0o1", "0b1"] {
        let source = original.replacen("schema=1", &format!("schema={integer}"), 1);
        fs::write(&path, &source).unwrap();
        let expected = NativeAdapterFault::Metadata {
            site: UnitSite {
                file: "tools/assembly/native-lock.toml".into(),
                path: vec![PathPart::Key("schema".into())],
                anchor: SourceAnchor::Bytes(7, 7 + integer.len()),
                decoded: None,
                occurrence: None,
            },
            reason: NativeMetadataReason::InvalidValue {
                expected: "1".into(),
                actual: integer.into(),
            },
        };
        assert_eq!(native_support::lock(&directory.0).unwrap_err(), expected);
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        control(&original);
    }
    for (source, resource, maximum) in [
        (" ".repeat(16_777_217), "lock-bytes", 16_777_216),
        (
            format!("[bundle]\nfiles=[{}]\n", "{},".repeat(262_145)),
            "file-records",
            262_144,
        ),
    ] {
        fs::write(&path, &source).unwrap();
        assert_eq!(
            native_support::lock(&directory.0).unwrap_err(),
            NativeAdapterFault::Metadata {
                site: input::site("tools/assembly/native-lock.toml"),
                reason: NativeMetadataReason::Limit {
                    resource: resource.into(),
                    maximum,
                    actual: maximum + 1
                }
            }
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        control(&original);
    }
}

#[test]
fn preparation_lock_spelling_and_streaming_bounds_restore() {
    let directory = Directory::new();
    let fixture = directory.0.join("lock.toml");
    fs::write(&fixture, small_lock()).unwrap();
    let script = directory.0.join("check.py");
    fs::write(&script, r#"import importlib.util
from pathlib import Path
import sys
spec=importlib.util.spec_from_file_location('prepare',sys.argv[1])
prepare=importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)
path=Path(sys.argv[2])
original=path.read_text()
expected=prepare.read_lock(path)
def restore():
    path.write_text(original)
    assert path.read_text()==original
    assert prepare.read_lock(path)==expected
def reject(source, fault):
    path.write_text(source)
    try:
        prepare.read_lock(path)
    except ValueError as error:
        assert error.args==(fault,), (error.args,fault)
    else:
        raise AssertionError(('accepted',fault))
    assert path.read_text()==source
    restore()
for key in ['schema', "'schema'", '"schema"', '"sch\\u0065ma"']:
    for integer in ['33','+33','3_3','0x21','0o41','0b100001']:
        source=original.replace('schema=1',key+'=1',1).replace('format-serial=33','format-serial='+integer,1)
        path.write_text(source)
        assert prepare.read_lock(path)==expected
        assert path.read_text()==source
        restore()
for integer in ['+1','0x1','0o1','0b1']:
    reject(original.replace('schema=1','schema='+integer,1),'schema raw token')
reject(' '*16_777_217,('lock-bytes',16_777_217))
for source in ['[bundle]\nfiles=[{},{}]\n', 'bundle={files=[{},{}]}\n', '[[bundle.files]]\n[[bundle.files]]\n', '["bun\\u0064le"]\n"fi\\u006ces"=[{},{}]\n',
               '[bundle]\nfiles=[{member="""a""""},{member="""b"""""}]\n',
               "[bundle]\nfiles=[{member='''a''''},{member='''b'''''}]\n"]:
    assert prepare.lock_preflight(source,2) is None
    try:
        prepare.lock_preflight(source,1)
    except ValueError as error:
        assert error.args==(('file-records',2),)
    else:
        raise AssertionError('accepted excess')
    assert prepare.lock_preflight(source,2) is None
reject('[bundle]\nfiles=['+'{},'*262_145+']\n',('file-records',262_145))
print('lock fixtures: ok')
"#).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(script)
        .arg(root.join("tools/assembly/prepare-native.py"))
        .arg(fixture)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"lock fixtures: ok\n");
    assert_eq!(output.stderr, b"");
}

#[test]
fn shape_collects_each_source_before_any_ownership_bound() {
    let bad = "schema=1\nskeleton={document=17,sections=[]}\n";
    let sources: Vec<_> = ["b", "a"]
        .map(|name| crate::UnitSource {
            file: format!("units/skeletons/{name}.toml"),
            bytes: bad.as_bytes().to_vec(),
        })
        .into();
    let diagnostics: Vec<_> = ["a", "b"]
        .map(|name| UnitDiagnostic {
            rule: RuleId::M004,
            stage: UnitStage::Shape,
            unit: None,
            language: None,
            site: UnitSite {
                file: format!("units/skeletons/{name}.toml"),
                path: vec![
                    PathPart::Key("skeleton".into()),
                    PathPart::Key("document".into()),
                ],
                anchor: SourceAnchor::Bytes(28, 30),
                decoded: None,
                occurrence: None,
            },
            fault: UnitFault::WrongType {
                expected: ShapeKind::String,
                actual: ShapeKind::Integer,
            },
        })
        .into();
    let expected =
        DocumentError::Boundary(UnitErrors::from_diagnostics(diagnostics.clone()).unwrap());
    let actual = input::skeletons(&sources).unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_string(),
        concat!(
            "M004 Shape unit=None language=None at \"units/skeletons/a.toml\" $[\"skeleton\"][\"document\"] bytes 28..30: WrongType { expected: String, actual: Integer }\n",
            "M004 Shape unit=None language=None at \"units/skeletons/b.toml\" $[\"skeleton\"][\"document\"] bytes 28..30: WrongType { expected: String, actual: Integer }\n"
        )
    );
    let mut deep = "{kind='section',heading='sec:x:y',children=[]}".to_owned();
    for _ in 0..6 {
        deep = format!("{{kind='section',heading='sec:x:y',children=[{deep}]}}");
    }
    let deep = format!(
        "schema=1\nskeleton={{document='main',sections=[{{heading='sec:x:y',children=[{deep}]}}]}}\n"
    );
    let mut sources = vec![
        crate::UnitSource {
            file: "units/skeletons/deep.toml".into(),
            bytes: deep.into_bytes(),
        },
        sources[0].clone(),
    ];
    let expected = DocumentError::Boundary(
        UnitErrors::from_diagnostics(vec![diagnostics[1].clone()]).unwrap(),
    );
    assert_eq!(input::skeletons(&sources).unwrap_err(), expected);
    sources.reverse();
    assert_eq!(input::skeletons(&sources).unwrap_err(), expected);
    for source in &mut sources {
        source.bytes = b"schema=1\nskeleton={document='main',sections=[]}\n".to_vec();
    }
    let restored = input::skeletons(&sources).unwrap();
    assert_eq!(
        restored
            .iter()
            .map(|draft| (&*draft.document.value, draft.sections.as_slice()))
            .collect::<Vec<_>>(),
        [("main", [].as_slice()), ("main", [].as_slice())]
    );
}

#[test]
fn every_skeleton_table_refuses_missing_and_extra_fields_and_restores() {
    use PathPart::{Index, Key};
    let section_path = vec![Key("skeleton".into()), Key("sections".into()), Index(0)];
    let mut child_path = section_path.clone();
    child_path.extend([Key("children".into()), Index(0)]);
    let cases = [
        (
            "schema=1\nskeleton=",
            "\n",
            vec![("document", "document='main'"), ("sections", "sections=[]")],
            vec![Key("skeleton".into())],
        ),
        (
            "schema=1\nskeleton={document='main',sections=[",
            "]}\n",
            vec![
                ("heading", "heading='sec:x:y'"),
                ("children", "children=[]"),
            ],
            section_path,
        ),
        (
            "schema=1\nskeleton={document='main',sections=[{heading='sec:x:y',children=[",
            "]}]}\n",
            vec![("kind", "kind='unit'"), ("unit", "unit='sec:x:z'")],
            child_path.clone(),
        ),
        (
            "schema=1\nskeleton={document='main',sections=[{heading='sec:x:y',children=[",
            "]}]}\n",
            vec![
                ("kind", "kind='section'"),
                ("heading", "heading='sec:x:z'"),
                ("children", "children=[]"),
            ],
            child_path,
        ),
    ];
    for (prefix, suffix, fields, path) in cases {
        let load = |text: &str| {
            input::skeletons(&[crate::UnitSource {
                file: "units/skeletons/fixture.toml".into(),
                bytes: text.as_bytes().to_vec(),
            }])
        };
        let body = fields
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>()
            .join(",");
        let original = format!("{prefix}{{{body}}}{suffix}");
        let control = load(&original).unwrap();
        for (missing, _) in &fields {
            let body = fields
                .iter()
                .filter(|(key, _)| key != missing)
                .map(|(_, value)| *value)
                .collect::<Vec<_>>()
                .join(",");
            let source = format!("{prefix}{{{body}}}{suffix}");
            let site = UnitSite {
                file: "units/skeletons/fixture.toml".into(),
                path: path.clone(),
                anchor: SourceAnchor::Parent(prefix.len(), prefix.len() + body.len() + 2),
                decoded: None,
                occurrence: None,
            };
            let fault = UnitFault::MissingField {
                name: (*missing).into(),
            };
            let expected = DocumentError::Boundary(
                UnitErrors::from_diagnostics(vec![UnitDiagnostic {
                    rule: RuleId::M004,
                    stage: UnitStage::Shape,
                    unit: None,
                    language: None,
                    site: site.clone(),
                    fault: fault.clone(),
                }])
                .unwrap(),
            );
            let actual = load(&source).unwrap_err();
            assert_eq!(actual, expected, "{source}");
            assert_eq!(
                actual.to_string(),
                format!("M004 Shape unit=None language=None at {site}: {fault:?}\n")
            );
            assert_eq!(load(&original).unwrap(), control);
        }
        let source = format!("{prefix}{{{body},extra=0}}{suffix}");
        let start = prefix.len() + body.len() + 2;
        let mut path = path;
        path.push(Key("extra".into()));
        let site = UnitSite {
            file: "units/skeletons/fixture.toml".into(),
            path,
            anchor: SourceAnchor::Bytes(start, start + 5),
            decoded: None,
            occurrence: None,
        };
        let fault = UnitFault::UnknownField {
            name: "extra".into(),
            expected: fields.iter().map(|(key, _)| (*key).into()).collect(),
        };
        let expected = DocumentError::Boundary(
            UnitErrors::from_diagnostics(vec![UnitDiagnostic {
                rule: RuleId::M004,
                stage: UnitStage::Shape,
                unit: None,
                language: None,
                site: site.clone(),
                fault: fault.clone(),
            }])
            .unwrap(),
        );
        let actual = load(&source).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.to_string(),
            format!("M004 Shape unit=None language=None at {site}: {fault:?}\n")
        );
        assert_eq!(load(&original).unwrap(), control);
    }
}

#[test]
fn skeleton_syntax_precedes_unknown_fields_and_variants_globally() {
    for (source, message, bytes) in [
        (
            "schema=1\nschema=1\n",
            "duplicate key `schema` in document root",
            9..10,
        ),
        (
            "schema=1\n\"schema\"=1\n",
            "duplicate key `schema` in document root",
            9..10,
        ),
        ("x={a=1,a=2}\n", "duplicate key `a`", 3..4),
        ("x=[{a=1,a=2}]\n", "duplicate key `a`", 4..5),
    ] {
        let good = b"schema=1\nskeleton={document='main',sections=[]}\n".to_vec();
        let mut sources = vec![
            crate::UnitSource {
                file: "units/skeletons/a.toml".into(),
                bytes: b"extra=0\n".to_vec(),
            },
            crate::UnitSource {
                file: "units/skeletons/b.toml".into(),
                bytes: source.as_bytes().to_vec(),
            },
        ];
        let site = UnitSite {
            file: "units/skeletons/b.toml".into(),
            path: vec![],
            anchor: SourceAnchor::Bytes(bytes.start, bytes.end),
            decoded: None,
            occurrence: None,
        };
        let fault = UnitFault::TomlSyntax {
            message: message.into(),
        };
        let expected = DocumentError::Boundary(
            UnitErrors::from_diagnostics(vec![UnitDiagnostic {
                rule: RuleId::M002,
                stage: UnitStage::Syntax,
                unit: None,
                language: None,
                site: site.clone(),
                fault: fault.clone(),
            }])
            .unwrap(),
        );
        let actual = input::skeletons(&sources).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.to_string(),
            format!("M002 Syntax unit=None language=None at {site}: {fault:?}\n")
        );
        for item in &mut sources {
            item.bytes.clone_from(&good);
        }
        assert_eq!(
            input::skeletons(&sources)
                .unwrap()
                .iter()
                .map(|draft| draft.document.value.as_str())
                .collect::<Vec<_>>(),
            ["main", "main"]
        );
    }
}

#[test]
fn lock_record_preflight_counts_semantic_paths_without_allocating_rows() {
    for source in [
        "[bundle]\nfiles=[{},{}]\n",
        "bundle.files = [ {}, # a\n {} ]\n",
        "bundle = { files = [{},{}] }\n",
        "[[bundle.files]]\nmember='one'\n[[bundle.files]]\nmember='two'\n",
        "[\"bundle\"]\n'files'=[{},{}]\n",
        "[\"bun\\u0064le\"]\n\"fi\\u006ces\"=[{},{}]\n",
        "[bundle]\nfiles=[{member=\"\"\"a\"\"\"\"},{member=\"\"\"b\"\"\"\"}]\n",
        "[bundle]\nfiles=[{member='''a''''},{member='''b'''''}]\n",
    ] {
        assert_eq!(lock_scan::records(source, 2), Ok(()), "{source}");
        assert_eq!(lock_scan::records(source, 1), Err(()), "{source}");
        assert_eq!(lock_scan::records(source, 2), Ok(()), "{source}");
    }
    for source in [
        "other=[{},{},{}]\n",
        "description='[[bundle.files]]'\n",
        "# [[bundle.files]]\n",
        "bundle={files=[]}\n",
    ] {
        assert_eq!(lock_scan::records(source, 0), Ok(()));
    }
    let source = format!("[bundle]\nfiles=[{}]\n", "{},".repeat(262_144));
    assert_eq!(lock_scan::records(&source, 262_144), Ok(()));
    let excess = format!("{}{{}}]\n", source.strip_suffix("]\n").unwrap());
    assert_eq!(lock_scan::records(&excess, 262_144), Err(()));
    assert_eq!(lock_scan::records(&source, 262_144), Ok(()));
}

fn live_record() -> EmissionRecord {
    EmissionRecord {
        kind: EmissionKind::Tag,
        unit: "c:1".into(),
        language: "live".into(),
        target: DocumentTarget::Latex,
        occurrence: None,
        value: "c:1\ndef:langevin:coefficient-one".into(),
        site: UnitSite {
            file: "generated/addressing.tex".into(),
            path: vec![PathPart::Key("live".into()), PathPart::Key("c:1".into())],
            anchor: SourceAnchor::WholeFile,
            decoded: None,
            occurrence: None,
        },
    }
}

#[test]
fn support_required_committed_documents_pass_native_mutation_byte_and_independent_audit_consumers()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
    let (retained, prepared) = crate::retention::check_release_with_documents(&root, &support)?;
    assert_eq!(
        prepared
            .artifacts()
            .iter()
            .map(DocumentArtifact::path)
            .collect::<Vec<_>>(),
        [
            "generated/documents/main/en.md",
            "generated/documents/main/en.tex",
            "generated/notation.tex",
            "generated/addressing.tex"
        ]
    );
    let projection = fs::read(root.join("generated/manifest.json"))?;
    assert_eq!(prepared.projection(), projection);
    let loaded = ftoo_manifest::load_projection(&projection)?;
    let report = retained.report();
    assert_eq!(
        report
            .outcomes()
            .iter()
            .map(ftoo_audit::ClaimOutcome::claim)
            .collect::<Vec<_>>(),
        [
            ftoo_manifest::NodeId::LangevinCoefficientOneValue,
            ftoo_manifest::NodeId::LangevinCoefficientThreeValue
        ]
    );
    for outcome in report.outcomes() {
        assert_eq!(
            report.status(outcome.claim()).unwrap().evidence(),
            ftoo_audit::AuditedEvidence::Certified
        );
        assert_eq!(
            loaded
                .manifest()
                .status(outcome.claim())
                .unwrap()
                .evidence(),
            ftoo_manifest::Evidence::Deferred
        );
    }
    for artifact in &prepared.artifacts()[..2] {
        retained_seed_header(
            std::str::from_utf8(artifact.bytes())?,
            artifact.path().as_bytes().ends_with(b".tex"),
        );
    }
    let coverage = "Coverage declaration: explicit-partial. Closed world: referents=false; relations=false; audit=true. Evidence context: retained audit accepted.";
    for artifact in &prepared.artifacts()[..2] {
        let text = std::str::from_utf8(artifact.bytes())?;
        assert_eq!(
            text.lines()
                .find(|line| line.starts_with("Coverage declaration:")),
            Some(coverage)
        );
    }
    assert_eq!(prepared.projection(), projection);
    assert_eq!(fs::read(root.join("generated/manifest.json"))?, projection);
    let directory = Directory::new();
    let restore = || {
        assert_eq!(
            artifacts::publish(&directory.0, &prepared, |_, _| Ok(())),
            Ok(())
        );
        assert_eq!(artifacts::verify(&directory.0, &prepared), Ok(()));
        for artifact in prepared.artifacts() {
            assert_eq!(
                fs::read(directory.0.join(artifact.path())).unwrap(),
                artifact.bytes()
            );
        }
    };
    restore();
    let token = |bytes: &[u8], offset: usize| {
        use std::fmt::Write as _;
        let start = offset.saturating_sub(64).min(bytes.len());
        let end = (offset + 257).min(bytes.len());
        let mut hex = String::new();
        for byte in &bytes[start..end] {
            write!(hex, "{byte:02x}").unwrap();
        }
        format!(
            "length={} window@{start}={hex}{}",
            bytes.len(),
            if end == bytes.len() { " EOF" } else { "" }
        )
    };
    for artifact in prepared.artifacts() {
        let original = artifact.bytes();
        let path = directory.0.join(artifact.path());
        let mut changed = original.to_vec();
        changed[0] ^= 1;
        let newline = original.iter().position(|byte| *byte == b'\n').unwrap();
        let crlf = String::from_utf8(original.to_vec())?
            .replace('\n', "\r\n")
            .into_bytes();
        let mut mutations = vec![
            (changed, 0),
            (original[..original.len() - 1].to_vec(), original.len() - 1),
            (crlf, newline),
        ];
        for needle in [
            "retained audit accepted",
            "Audit outcomes:",
            "certified",
            "Audit scope:",
            "Certification concerns",
            "g-audit-0",
            "pending",
            "label def:",
            "\\providecommand",
            "\\FtdocTag",
            "| Coefficient | Claim |",
            "\\begin{tabular}",
        ] {
            if let Some(offset) = std::str::from_utf8(original)?.find(needle) {
                let mut changed = original.to_vec();
                changed[offset] = b'X';
                mutations.push((changed, offset));
            }
        }
        for (changed, offset) in mutations {
            fs::write(&path, &changed)?;
            let site = AssemblySite::Generated {
                path: artifact.path().into(),
                bytes: offset..offset + usize::from(offset < changed.len()),
                origins: artifact.origins().to_vec(),
            };
            let fault = AssemblyFault::NonCanonicalDocument {
                path: artifact.path().into(),
                offset,
                diff: format!(
                    "byte {offset}: expected {}; actual {}\n",
                    token(original, offset),
                    token(&changed, offset)
                ),
            };
            let actual = artifacts::verify(&directory.0, &prepared).unwrap_err();
            assert_eq!(
                actual,
                failure(
                    RuleId::M057,
                    AssemblyStage::Bytes,
                    site.clone(),
                    fault.clone()
                )
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "M057 Bytes document=None unit=None language=None target=None at {site:?}: {fault:?}\n"
                )
            );
            assert_eq!(fs::read(&path)?, changed);
            restore();
        }
        fs::remove_file(&path)?;
        assert_eq!(
            artifacts::verify(&directory.0, &prepared),
            Err(failure(
                RuleId::M057,
                AssemblyStage::Bytes,
                AssemblySite::Inventory {
                    path: "generated/documents".into()
                },
                AssemblyFault::ArtifactInventory {
                    missing: vec![artifact.path().into()],
                    unexpected: vec![]
                }
            ))
        );
        assert_eq!(
            fs::read(&path).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        restore();
        fs::remove_file(&path)?;
        std::os::unix::fs::symlink(root.join(artifact.path()), &path)?;
        assert_eq!(
            artifacts::verify(&directory.0, &prepared),
            Err(DocumentError::Input(DocumentInputFault::UnexpectedEntry {
                path: path.clone(),
                kind: DocumentEntryKind::Symlink
            }))
        );
        assert!(fs::symlink_metadata(&path)?.file_type().is_symlink());
        fs::remove_file(&path)?;
        restore();
    }
    for extra in [
        "generated/documents/main/de.md",
        "generated/documents/obsolete/en.tex",
        "generated/documents/unexpected",
    ] {
        let path = directory.0.join(extra);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(&path, b"obsolete\n")?;
        assert_eq!(
            artifacts::verify(&directory.0, &prepared),
            Err(failure(
                RuleId::M057,
                AssemblyStage::Bytes,
                AssemblySite::Inventory {
                    path: "generated/documents".into()
                },
                AssemblyFault::ArtifactInventory {
                    missing: vec![],
                    unexpected: vec![extra.into()]
                }
            ))
        );
        assert_eq!(fs::read(&path)?, b"obsolete\n");
        fs::remove_file(&path)?;
        restore();
    }
    input::reread(&root, prepared.inputs())?;
    assert_eq!(artifacts::verify(&root, &prepared), Ok(()));
    Ok(())
}

#[test]
fn independent_runtime_inventory_refuses_missing_repeated_and_changed_targets()
-> Result<(), Box<dyn std::error::Error>> {
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let original = runtime::render(&book, &[])?;
    let line = original
        .lines()
        .find(|line| line.starts_with("\\prop_gput:Nxx \\g_ftdoc_live_prop "))
        .unwrap();
    let mut altered = live_record();
    altered.value = "c:1\nxef:langevin:coefficient-one".into();
    for (replacement, missing, unexpected, changed) in [
        (String::new(), vec![live_record()], vec![], vec![]),
        (
            format!("{line}\n{line}\n"),
            vec![],
            vec![live_record()],
            vec![],
        ),
        (
            format!(
                "{}\n",
                line.replacen(
                    "\\char_generate:nn {100}{12}",
                    "\\char_generate:nn {120}{12}",
                    1
                )
            ),
            vec![],
            vec![],
            vec![EmissionChange {
                expected: live_record(),
                actual: altered,
            }],
        ),
    ] {
        let source = original.replacen(&format!("{line}\n"), &replacement, 1);
        let start = source.find("\\prop_gput:Nxx \\g_ftdoc_live_prop ").unwrap()
            + if unexpected.is_empty() {
                0
            } else {
                line.len() + 1
            };
        let end = start + source[start..].find('\n').unwrap() + 1;
        let site = AssemblySite::Generated {
            path: "generated/addressing.tex".into(),
            bytes: start..end,
            origins: vec![],
        };
        let fault = AssemblyFault::EmissionMismatch {
            missing,
            unexpected,
            changed,
        };
        let actual = runtime::check(&book, &[], &source).unwrap_err();
        assert_eq!(
            actual,
            failure(
                RuleId::M055,
                AssemblyStage::Emission,
                site.clone(),
                fault.clone()
            )
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "M055 Emission document=None unit=None language=None target=None at {site:?}: {fault:?}\n"
            )
        );
        assert_eq!(runtime::check(&book, &[], &original), Ok(()));
        assert_eq!(runtime::render(&book, &[])?, original);
    }
    Ok(())
}

#[test]
fn runtime_refusal_localizes_one_authored_occurrence() {
    let origin = UnitSite {
        file: "units/statements/fixture.toml".into(),
        path: vec![PathPart::Key("text".into()), PathPart::Key("en".into())],
        anchor: SourceAnchor::Bytes(51, 80),
        decoded: Some(7..19),
        occurrence: Some(2),
    };
    let plan = plan::DocumentPlan {
        document: "main".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("a", "b"),
        blocks: vec![plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:heading".into(),
            depth: Some(1),
            site: input::site("units/statements/fixture.toml"),
            runs: vec![
                plan::Inline::Text("Before ".into()),
                plan::Inline::Tag {
                    handle: "c:01".into(),
                    label: "def:fixture:missing".into(),
                    token: "u0-t2".into(),
                    site: origin.clone(),
                },
            ],
        }],
    };
    let source = "prefix \\FtdocTag{c:01}{u0-t2} suffix";
    assert_eq!(native::tag_range(source, "u0-t2"), Some(7..29));
    assert_eq!(native::tag_range(source, "u0-t0"), None);
    let site = AssemblySite::Generated {
        path: "generated/documents/main/en.tex".into(),
        bytes: 7..29,
        origins: plan.origins(DocumentTarget::Latex),
    };
    let fault = AssemblyFault::RuntimeTagRejected {
        input: "c:01".into(),
        site_token: "u0-t2".into(),
        message: "tag \"c:01\": unknown".into(),
    };
    let original = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: Some("main".into()),
            unit: None,
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site,
            fault: fault.clone(),
        }])
        .unwrap(),
    );
    let site = AssemblySite::Generated {
        path: "generated/documents/main/en.tex".into(),
        bytes: 7..29,
        origins: vec![SourceOrigin {
            unit: Some("sec:fixture:heading".into()),
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: origin,
        }],
    };
    let expected = DocumentError::Assembly(
        AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
            rule: RuleId::M055,
            stage: AssemblyStage::Native,
            document: Some("main".into()),
            unit: Some("sec:fixture:heading".into()),
            language: Some("en".into()),
            target: Some(DocumentTarget::Latex),
            site: site.clone(),
            fault: fault.clone(),
        }])
        .unwrap(),
    );
    let actual = native::runtime_origin(original, &plan);
    assert_eq!(actual, expected);
    assert_eq!(
        actual.to_string(),
        format!(
            "M055 Native document=Some(\"main\") unit=Some(\"sec:fixture:heading\") language=Some(\"en\") target=Some(Latex) at {site:?}: {fault:?}\n"
        )
    );
}

#[test]
fn independent_emission_compares_complete_ordered_blocks() {
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: ["a", "b", "c"]
            .map(|name| plan::Block {
                pending_names: BTreeMap::new(),
                displays: BTreeMap::new(),
                unit: format!("sec:fixture:{name}"),
                depth: (name == "a").then_some(1),
                site: input::site(&format!("units/statements/{name}.toml")),
                runs: vec![plan::Inline::Text(name.to_uppercase())],
            })
            .into(),
    };
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let (header, a, b, c, tail) = if target == DocumentTarget::Markdown {
            (
                "<!-- generated document fixture language en -->\nCoverage.\nSlice.\n\n".to_owned(),
                "<!-- unit sec:fixture:a document fixture language en -->\n# A\n\n",
                "<!-- unit sec:fixture:b document fixture language en -->\nB\n\n",
                "<!-- unit sec:fixture:c document fixture language en -->\nC\n",
                "",
            )
        } else {
            (
                format!(
                    "{}\\FtdocContext{{fixture}}{{en}}\n\\begin{{document}}\nCoverage.\n\nSlice.\n\n",
                    latex::PREAMBLE
                ),
                "% unit sec:fixture:a document fixture language en\n\\section*{A}\n\n",
                "% unit sec:fixture:b document fixture language en\nB\n\n",
                "% unit sec:fixture:c document fixture language en\nC\n\n",
                "\\end{document}\n",
            )
        };
        let original = format!("{header}{a}{b}{c}{tail}");
        assert_eq!(
            if target == DocumentTarget::Markdown {
                markdown::render(&plan)
            } else {
                latex::render(&plan)
            },
            original
        );
        let record = EmissionRecord {
            kind: EmissionKind::Heading,
            unit: "sec:fixture:a".into(),
            language: "en".into(),
            target,
            occurrence: None,
            value: "A".into(),
            site: input::site("units/statements/a.toml"),
        };
        for (source, bytes, missing, unexpected) in [
            (
                format!("{header}{b}{c}{tail}"),
                header.len()..header.len() + b.len(),
                vec![record.clone()],
                vec![],
            ),
            (
                format!("{header}{a}{a}{b}{c}{tail}"),
                header.len() + a.len()..header.len() + 2 * a.len(),
                vec![],
                vec![record.clone()],
            ),
            (
                format!("{header}{b}{a}{c}{tail}"),
                header.len()..header.len() + b.len(),
                vec![record.clone()],
                vec![record.clone()],
            ),
        ] {
            let site = AssemblySite::Generated {
                path: format!(
                    "generated/documents/fixture/en.{}",
                    if target == DocumentTarget::Markdown {
                        "md"
                    } else {
                        "tex"
                    }
                ),
                bytes,
                origins: ["a", "b", "c"]
                    .map(|name| SourceOrigin {
                        unit: Some(format!("sec:fixture:{name}")),
                        language: Some("en".into()),
                        target: Some(target),
                        site: input::site(&format!("units/statements/{name}.toml")),
                    })
                    .into(),
            };
            let fault = AssemblyFault::EmissionMismatch {
                missing,
                unexpected,
                changed: vec![],
            };
            let actual = plan::check_emission(&plan, &source, target).unwrap_err();
            assert_eq!(
                actual,
                failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    site.clone(),
                    fault.clone()
                )
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "M055 Emission document=None unit=None language=None target=None at {site:?}: {fault:?}\n"
                )
            );
            assert_eq!(plan::check_emission(&plan, &original, target), Ok(()));
        }
    }
}

#[test]
fn emission_retains_distinct_math_keys_with_identical_renderings() {
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:unit".into(),
            depth: None,
            site: input::site("units/statements/fixture.toml"),
            runs: ["first", "second", "first"]
                .into_iter()
                .enumerate()
                .map(|(index, key)| plan::Inline::Math {
                    definition_site: super::super::input::site("fixture-math.toml"),
                    key: key.into(),
                    markdown: "1".into(),
                    latex: "1".into(),
                    pending: vec![],
                    site: UnitSite {
                        occurrence: Some(index),
                        ..input::site("units/statements/fixture.toml")
                    },
                })
                .collect(),
        }],
    };
    assert_eq!(
        plan::check_emission(&plan, &markdown::render(&plan), DocumentTarget::Markdown),
        Ok(())
    );
    assert_eq!(
        plan::check_emission(&plan, &latex::render(&plan), DocumentTarget::Latex),
        Ok(())
    );
}

#[test]
fn independent_emission_has_exact_omitted_repeated_swapped_and_changed_use_records() {
    let site = input::site("units/statements/fixture.toml");
    let occurrence = |index| UnitSite {
        occurrence: Some(index),
        ..site.clone()
    };
    let plan = plan::DocumentPlan {
        document: "fixture".into(),
        language: "en".into(),
        coverage: plan::Coverage::fixture("Coverage.", "Slice."),
        blocks: vec![plan::Block {
            pending_names: BTreeMap::new(),
            displays: BTreeMap::new(),
            unit: "sec:fixture:paragraph".into(),
            depth: None,
            site: site.clone(),
            runs: vec![
                plan::Inline::Tag {
                    handle: "c:1".into(),
                    label: "def:fixture:one".into(),
                    token: "u0-t0".into(),
                    site: occurrence(0),
                },
                plan::Inline::Text(" ".into()),
                plan::Inline::Math {
                    definition_site: super::super::input::site("fixture-math.toml"),
                    key: "value".into(),
                    markdown: "1".into(),
                    latex: "1".into(),
                    pending: vec![],
                    site: occurrence(1),
                },
                plan::Inline::Text(" ".into()),
                plan::Inline::Citation {
                    identity: synthetic_citation_identity(
                        "dec:fixture:source",
                        "sec:fixture:paragraph",
                        "record",
                        "u0-c0",
                    ),
                    text: "dec:fixture:source".into(),
                    site: occurrence(2),
                },
            ],
        }],
    };
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        let rendered = if target == DocumentTarget::Markdown {
            markdown::render(&plan)
        } else {
            latex::render(&plan)
        };
        let record = |kind, occurrence_index, value: &str| EmissionRecord {
            kind,
            unit: "sec:fixture:paragraph".into(),
            language: "en".into(),
            target,
            occurrence: occurrence_index,
            value: value.into(),
            site: occurrence_index.map_or_else(|| site.clone(), occurrence),
        };
        let body = record(
            EmissionKind::Paragraph,
            None,
            "@tag{c:1} @math{value} `dec:fixture:source`",
        );
        let tag = record(EmissionKind::Tag, Some(0), "def:fixture:one");
        let math = record(EmissionKind::Math, Some(1), "value");
        let cite = record(
            EmissionKind::Citation(Box::new(synthetic_citation_identity(
                "dec:fixture:source",
                "sec:fixture:paragraph",
                "record",
                "u0-c0",
            ))),
            Some(2),
            "dec:fixture:source",
        );
        let (tag_source, math_source, cite_source, marker) = if target == DocumentTarget::Markdown {
            (
                "c:1<!-- label def:fixture:one -->",
                "$1$",
                "record<!-- cite dec:fixture:source -->",
                "<!-- unit ",
            )
        } else {
            (
                "\\FtdocTag{c:1}{u0-t0}% label def:fixture:one\n",
                "\\(1\\)",
                "\\FtdocTag{record}{u0-c0}% cite dec:fixture:source\n",
                "% unit ",
            )
        };
        let mut cases = Vec::new();
        for (needle, canonical, use_record) in [
            (tag_source, "@tag{c:1}", &tag),
            (math_source, "@math{value}", &math),
            (cite_source, "`dec:fixture:source`", &cite),
        ] {
            for repeat in [false, true] {
                let replacement = if repeat {
                    needle.repeat(2)
                } else {
                    String::new()
                };
                let source = rendered.replacen(needle, &replacement, 1);
                let mut actual_body = body.clone();
                actual_body.value = body.value.replacen(
                    canonical,
                    &if repeat {
                        canonical.repeat(2)
                    } else {
                        String::new()
                    },
                    1,
                );
                cases.push((
                    source,
                    if repeat {
                        vec![]
                    } else {
                        vec![use_record.clone()]
                    },
                    if repeat {
                        vec![use_record.clone()]
                    } else {
                        vec![]
                    },
                    vec![EmissionChange {
                        expected: body.clone(),
                        actual: actual_body,
                    }],
                ));
            }
        }
        let prose_space = if target == DocumentTarget::Latex {
            r"\space{}"
        } else {
            " "
        };
        let swapped = rendered.replacen(
            &format!("{tag_source}{prose_space}{math_source} {cite_source}"),
            &format!("{math_source} {tag_source}{prose_space}{cite_source}"),
            1,
        );
        let mut actual_body = body.clone();
        actual_body.value = "@math{value} @tag{c:1} `dec:fixture:source`".into();
        cases.push((
            swapped,
            vec![tag.clone()],
            vec![tag.clone()],
            vec![EmissionChange {
                expected: body.clone(),
                actual: actual_body,
            }],
        ));
        let changed = rendered.replacen("label def:fixture:one", "label def:fixture:two", 1);
        let mut actual_tag = tag.clone();
        actual_tag.value = "def:fixture:two".into();
        cases.push((
            changed,
            vec![],
            vec![],
            vec![EmissionChange {
                expected: tag.clone(),
                actual: actual_tag,
            }],
        ));
        let malformed_math = if target == DocumentTarget::Markdown {
            "$broken"
        } else {
            "\\(broken"
        };
        let malformed = rendered.replacen(math_source, malformed_math, 1);
        let mut actual_body = body.clone();
        actual_body.value = format!("{tag_source}{prose_space}{malformed_math} {cite_source}");
        cases.push((
            malformed,
            vec![tag.clone(), math.clone(), cite.clone()],
            vec![],
            vec![EmissionChange {
                expected: body.clone(),
                actual: actual_body,
            }],
        ));
        let malformed = rendered.replacen("unit sec:fixture:paragraph", "unit bad", 1);
        let start = malformed.find(marker).unwrap();
        let path = format!(
            "generated/documents/fixture/en.{}",
            if target == DocumentTarget::Markdown {
                "md"
            } else {
                "tex"
            }
        );
        let unexpected = EmissionRecord {
            kind: EmissionKind::Paragraph,
            unit: "bad".into(),
            language: "en".into(),
            target,
            occurrence: None,
            value: malformed[start..].into(),
            site: UnitSite {
                file: path,
                path: vec![],
                anchor: SourceAnchor::Bytes(start, malformed.len()),
                decoded: None,
                occurrence: None,
            },
        };
        cases.push((
            malformed,
            vec![body, tag, math, cite],
            vec![unexpected],
            vec![],
        ));
        for (source, missing, unexpected, changed) in cases {
            let start = source.find(marker).unwrap();
            let generated = AssemblySite::Generated {
                path: format!(
                    "generated/documents/fixture/en.{}",
                    if target == DocumentTarget::Markdown {
                        "md"
                    } else {
                        "tex"
                    }
                ),
                bytes: start..source.len(),
                origins: vec![
                    SourceOrigin {
                        unit: Some("sec:fixture:paragraph".into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: site.clone(),
                    },
                    SourceOrigin {
                        unit: Some("sec:fixture:paragraph".into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: occurrence(0),
                    },
                    SourceOrigin {
                        unit: Some("sec:fixture:paragraph".into()),
                        language: Some("en".into()),
                        target: Some(target),
                        site: occurrence(1),
                    },
                    SourceOrigin {
                        unit: Some("sec:fixture:paragraph".into()),
                        language: Some("en".into()),
                        target: None,
                        site: occurrence(2),
                    },
                    SourceOrigin {
                        unit: None,
                        language: None,
                        target: None,
                        site: UnitSite {
                            file: "fixture.md".into(),
                            path: vec![],
                            anchor: SourceAnchor::Bytes(0, 18),
                            decoded: None,
                            occurrence: None,
                        },
                    },
                ],
            };
            let fault = AssemblyFault::EmissionMismatch {
                missing,
                unexpected,
                changed,
            };
            let actual = plan::check_emission(&plan, &source, target).unwrap_err();
            assert_eq!(
                actual,
                failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    generated.clone(),
                    fault.clone()
                ),
                "{source}"
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "M055 Emission document=None unit=None language=None target=None at {generated:?}: {fault:?}\n"
                )
            );
            assert_eq!(plan::check_emission(&plan, &rendered, target), Ok(()));
        }
    }
}

pub(super) fn support_required_native_lifecycle_tables_reject_at_prose_and_display_sites_and_restore(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let encode = |value: &str| {
        let mut result = String::new();
        for scalar in value.chars() {
            write!(result, "\\char_generate:nn {{{}}}{{12}}", u32::from(scalar)).unwrap();
        }
        result
    };
    for (fixture, handle) in [
        ("tag-retirement", "c:3"),
        ("identity-retirement", "c:3"),
        ("target-absent", "c:3"),
        ("selected-one", "v:3"),
        ("selected-one", "c:3"),
        ("empty", "c:1"),
    ] {
        let rows = include_str!("../../../tests/fixtures/assembly/lifecycle.txt")
            .lines()
            .filter_map(|line| {
                let fields = line.splitn(4, '\t').collect::<Vec<_>>();
                (fields[0] == fixture).then(|| (fields[1], fields[2], fields[3]))
            })
            .collect::<Vec<_>>();
        let message = rows.iter().find(|(_, key, _)| *key == handle).unwrap().2;
        for position in 0..5 {
            let depth = (position == 0).then_some(1);
            let mut origin = UnitSite {
                file: "units/statements/fixture.toml".into(),
                path: vec![PathPart::Key("text".into()), PathPart::Key("en".into())],
                anchor: SourceAnchor::Bytes(100, 120),
                decoded: Some(0..10),
                occurrence: Some(0),
            };
            if position >= 2 {
                origin.file = "units/displays/fixture.toml".into();
                origin.path = vec![
                    PathPart::Key("display".into()),
                    PathPart::Key("bodies".into()),
                    PathPart::Key("latex".into()),
                    PathPart::Key("en".into()),
                    PathPart::Key("regions".into()),
                    PathPart::Index(usize::from(position == 3)),
                ];
                if position < 4 {
                    origin.path.push(PathPart::Index(0));
                }
            }
            let mut plan = plan::DocumentPlan {
                document: "fixture".into(),
                language: "en".into(),
                coverage: plan::Coverage::fixture("Coverage.", "Slice."),
                blocks: vec![plan::Block {
                    pending_names: BTreeMap::new(),
                    displays: BTreeMap::new(),
                    unit: "sec:fixture:unit".into(),
                    depth,
                    site: input::site("units/statements/fixture.toml"),
                    runs: vec![plan::Inline::Tag {
                        handle: handle.into(),
                        label: book.resolve(handle)?.label().into(),
                        token: "u0-t0".into(),
                        site: origin.clone(),
                    }],
                }],
            };
            if position >= 2 {
                let tag_region = plan::Block {
                    pending_names: BTreeMap::new(),
                    unit: "sec:fixture:unit".into(),
                    depth: None,
                    site: origin.clone(),
                    runs: std::mem::take(&mut plan.blocks[0].runs),
                    displays: BTreeMap::new(),
                };
                plan.blocks[0].runs = vec![plan::Inline::Text("Caption.".into())];
                let other_cell = plan::Block {
                    pending_names: BTreeMap::new(),
                    unit: "sec:fixture:unit".into(),
                    depth: None,
                    site: input::site("units/displays/fixture.toml"),
                    runs: vec![plan::Inline::Text("Cell.".into())],
                    displays: BTreeMap::new(),
                };
                let (layout, regions) = if position == 4 {
                    (
                        ftoo_manifest::DisplayLayout::Picture {
                            width: 100,
                            height: 20,
                            commands: vec![ftoo_manifest::PictureCommand::Label {
                                x: 0,
                                y: 10,
                                region: 0,
                            }],
                        },
                        vec![tag_region],
                    )
                } else {
                    (
                        ftoo_manifest::DisplayLayout::Table {
                            columns: vec!["l".into()],
                            rows: vec![vec![0], vec![1]],
                            rules: vec![],
                        },
                        if position == 2 {
                            vec![tag_region, other_cell]
                        } else {
                            vec![other_cell, tag_region]
                        },
                    )
                };
                plan.blocks[0].displays.insert(
                    DocumentTarget::Latex,
                    plan::DisplayBody {
                        source: "private lifecycle body".into(),
                        layout,
                        regions,
                        site: input::site("units/displays/fixture.toml"),
                    },
                );
            }
            let original = runtime::render(&book, std::slice::from_ref(&plan))?;
            let start = original
                .find("\\prop_gput:Nxx \\g_ftdoc_live_prop ")
                .unwrap();
            let sites = original
                .find("\\prop_gput:Nxx \\g_ftdoc_sites_prop ")
                .unwrap();
            let mut source = original[..start].to_owned();
            let mut expected_records = Vec::new();
            for &(store, key, value) in &rows {
                let value_bytes = if store == "live" {
                    format!("{{{}}}{{{}}}", encode(key), encode(value))
                } else {
                    encode(value)
                };
                writeln!(
                    source,
                    "\\prop_gput:Nxx \\g_ftdoc_{store}_prop {{{}}} {{{value_bytes}}}",
                    encode(key)
                )?;
                let mut site = input::site("generated/addressing.tex");
                site.path = vec![PathPart::Key(store.into()), PathPart::Key(key.into())];
                expected_records.push(EmissionRecord {
                    kind: EmissionKind::Tag,
                    unit: key.into(),
                    language: store.into(),
                    target: DocumentTarget::Latex,
                    occurrence: None,
                    value: if store == "live" {
                        format!("{key}\n{value}")
                    } else {
                        value.into()
                    },
                    site,
                });
            }
            source.push_str(&original[sites..]);
            let mut site = input::site("generated/addressing.tex");
            site.path = vec![
                PathPart::Key("sites".into()),
                PathPart::Key("fixture/en/u0-t0".into()),
            ];
            expected_records.push(EmissionRecord {
                kind: EmissionKind::Tag,
                unit: "fixture/en/u0-t0".into(),
                language: "sites".into(),
                target: DocumentTarget::Latex,
                occurrence: None,
                value: "sec:fixture:unit".into(),
                site,
            });
            assert_eq!(
                runtime::compare(&expected_records, std::slice::from_ref(&plan), &source),
                Ok(())
            );
            let tampered = source.replacen(&encode(message), &encode("wrong cause"), 1);
            let expected_record = expected_records
                .iter()
                .find(|record| record.unit == handle)
                .unwrap()
                .clone();
            let mut actual_record = expected_record.clone();
            actual_record.value = "wrong cause".into();
            let prefix = format!(
                "\\prop_gput:Nxx \\g_ftdoc_{}_prop {{{}}} ",
                expected_record.language,
                encode(handle)
            );
            let start = tampered.find(&prefix).unwrap();
            let end = start + tampered[start..].find('\n').unwrap() + 1;
            let generated = AssemblySite::Generated {
                path: "generated/addressing.tex".into(),
                bytes: start..end,
                origins: plan.origins(DocumentTarget::Latex),
            };
            let fault = AssemblyFault::EmissionMismatch {
                missing: vec![],
                unexpected: vec![],
                changed: vec![EmissionChange {
                    expected: expected_record,
                    actual: actual_record,
                }],
            };
            let actual =
                runtime::compare(&expected_records, std::slice::from_ref(&plan), &tampered)
                    .unwrap_err();
            assert_eq!(
                actual,
                failure(
                    RuleId::M055,
                    AssemblyStage::Emission,
                    generated.clone(),
                    fault.clone()
                )
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "M055 Emission document=None unit=None language=None target=None at {generated:?}: {fault:?}\n"
                )
            );
            assert_eq!(
                runtime::compare(&expected_records, std::slice::from_ref(&plan), &source),
                Ok(())
            );
            let tex = latex::render(&plan);
            let command = format!("\\FtdocTag{{{handle}}}{{u0-t0}}");
            let start = tex.find(&command).unwrap();
            let site = AssemblySite::Generated {
                path: "generated/documents/fixture/en.tex".into(),
                bytes: start..start + command.len(),
                origins: vec![SourceOrigin {
                    unit: Some("sec:fixture:unit".into()),
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: origin,
                }],
            };
            let fault = AssemblyFault::RuntimeTagRejected {
                input: handle.into(),
                site_token: "u0-t0".into(),
                message: message.into(),
            };
            let expected = DocumentError::Assembly(
                AssemblyErrors::from_diagnostics(vec![AssemblyDiagnostic {
                    rule: RuleId::M055,
                    stage: AssemblyStage::Native,
                    document: Some("fixture".into()),
                    unit: Some("sec:fixture:unit".into()),
                    language: Some("en".into()),
                    target: Some(DocumentTarget::Latex),
                    site: site.clone(),
                    fault: fault.clone(),
                }])
                .unwrap(),
            );
            let actual = native::consume_plan(support, lock, &tex, "", &source, &plan);
            checks.push(CaseEvidence::with_display(format!("lifecycle/{fixture}/{handle}/{position}/refusal"), actual, Err(expected), format!(
                    "M055 Native document=Some(\"fixture\") unit=Some(\"sec:fixture:unit\") language=Some(\"en\") target=Some(Latex) at {site:?}: {fault:?}\n"
                )));
            assert_eq!(
                runtime::render(&book, std::slice::from_ref(&plan))?,
                original
            );
            assert_eq!(
                runtime::check(&book, std::slice::from_ref(&plan), &original),
                Ok(())
            );
            checks.push(CaseEvidence::new(
                format!("lifecycle/{fixture}/{handle}/{position}/restored"),
                native::consume_plan(support, lock, &tex, "", &original, &plan),
                Ok(()),
            ));
        }
    }
    Ok(())
}

pub(super) fn support_required_native_unknown_unicode_quotes_backslashes_and_controls_keep_complete_records(
    support: &NativeSupport,
    lock: &native_support::Lock,
    checks: &mut Vec<CaseEvidence>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write;
    let graph = ftoo_manifest::seed()?;
    let book = ftoo_manifest::addressing(&graph)?;
    let addressing = runtime::render(&book, &[])?;
    let input = "é\"\\\t\n\r\u{1}\u{7f}\u{80}";
    let mut definition = String::new();
    for scalar in input.chars() {
        write!(
            definition,
            "\\char_generate:nn {{{}}}{{12}}",
            u32::from(scalar)
        )?;
    }
    for (index, position) in [
        r"\section*{\FtdocTag{\fixtureInput}{u0-t0}}",
        r"\begin{tabular}{l}\FtdocTag{\fixtureInput}{u0-t0} \\ Cell \\ \end{tabular}",
        r"\begin{tabular}{l}Cell \\ \FtdocTag{\fixtureInput}{u0-t0} \\ \end{tabular}",
        r"\begin{picture}(100,20)\put(0,10){\FtdocTag{\fixtureInput}{u0-t0}}\end{picture}",
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!(
            "{}\\ExplSyntaxOn\n\\cs_new:Npn \\fixtureInput {{{definition}}}\n\\ExplSyntaxOff\n{position}\n\\end{{document}}\n",
            latex::start("fixture", "en")
        );
        let case = NativeCase::Document {
            document: "fixture".into(),
            language: "en".into(),
        };
        let expected = expected_runtime_refusal(
            &source,
            input,
            "tag \"é\\\"\\\\\\t\\n\\r\\u{1}\\u{7f}\\u{80}\": unknown",
        );
        let actual = native::consume(
            support,
            lock,
            &source,
            "",
            &addressing,
            case.clone(),
            vec![],
        );
        checks.push(CaseEvidence::new(
            format!("unicode/{index}/refusal"),
            actual,
            Err(expected),
        ));
        let restored = source.replace(&definition, "c:1");
        checks.push(CaseEvidence::new(
            format!("unicode/{index}/restored"),
            native::consume(support, lock, &restored, "", &addressing, case, vec![]),
            Ok(()),
        ));
        assert_eq!(runtime::render(&book, &[])?, addressing);
    }
    Ok(())
}

fn retained_seed_header(source: &str, latex: bool) {
    let declaration = "Coverage declaration: explicit-partial. Closed world: referents=false; relations=false; audit=true. Evidence context: retained audit accepted.";
    let scope = "Audit scope: finite source jet of order 5, residual order 4 and derivative order 1; analytic identification is not certified.";
    let axis = "Certification concerns the finite coefficient claim; pending notation concerns a proposed formulation.";
    let expected = if latex {
        format!(
            r"{declaration}

Declared slice: \FtdocTag{{v:1}}{{g-slice-0}}% label claim:langevin:coefficient-one-value
, \FtdocTag{{v:3}}{{g-slice-1}}% label claim:langevin:coefficient-three-value
.

Audit outcomes: \FtdocTag{{v:1}}{{g-audit-0}}% label claim:langevin:coefficient-one-value
: certified; \FtdocTag{{v:3}}{{g-audit-1}}% label claim:langevin:coefficient-three-value
: certified.

{scope}

{axis}

"
        )
    } else {
        format!(
            "{declaration}\nDeclared slice: v:1<!-- label claim:langevin:coefficient-one-value -->, v:3<!-- label claim:langevin:coefficient-three-value -->.\nAudit outcomes: v:1<!-- label claim:langevin:coefficient-one-value -->: certified; v:3<!-- label claim:langevin:coefficient-three-value -->: certified.\n{scope}\n{axis}\n\n"
        )
    };
    let start = source.find("Coverage declaration:").unwrap();
    let end = source[start..]
        .find(if latex { "% unit " } else { "<!-- unit " })
        .unwrap()
        + start;
    assert_eq!(&source[start..end], expected);
}
