#![allow(clippy::too_many_lines)] // Keep complete boundary matrices and their restorations together.
use super::*;
use ftoo_manifest::{PathPart, SourceAnchor, UnitSite};

fn assert_refusal(result: Result<impl std::fmt::Debug, DocumentError>, expected: DocumentError) {
    let actual = result.unwrap_err();
    assert_eq!(actual.to_string(), expected.to_string());
    assert_eq!(Some(actual), Some(expected));
}

#[test]
fn documentary_unknown_and_duplicate_fields_restore_and_reread() {
    let directory = super::super::tests::Directory::new();
    std::fs::create_dir(directory.0.join("registry")).unwrap();
    let path = directory.0.join(FILE);
    let baseline = load(&source(LEDGER)).unwrap();
    let admit = [
        "ordinal",
        "change",
        "handle",
        "authority-record",
        "reason",
        "label",
    ];
    let retire = ["ordinal", "change", "handle", "authority-record", "reason"];
    for (changed, key, event, allowed) in [
        (
            format!("{LEDGER}extra = 1\n"),
            "extra",
            true,
            admit.to_vec(),
        ),
        (
            LEDGER.replace("\"admit\"", "\"retire\""),
            "label",
            true,
            retire.to_vec(),
        ),
        (
            format!("extra = 1\n{LEDGER}"),
            "extra",
            false,
            vec!["schema", "events"],
        ),
    ] {
        let begin = changed.find(&format!("{key} =")).unwrap();
        let mut site = field_site(&changed, key, event);
        site.anchor = SourceAnchor::Bytes(begin, begin + key.len());
        let expected = fault(
            RuleId::M004,
            UnitStage::Shape,
            site,
            UnitFault::UnknownField {
                name: key.into(),
                expected: allowed.into_iter().map(String::from).collect(),
            },
        );
        std::fs::write(&path, &changed).unwrap();
        assert_refusal(load(&read(&directory.0).unwrap()), expected);
        assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
        std::fs::write(&path, LEDGER).unwrap();
        assert_eq!(load(&read(&directory.0).unwrap()), Ok(baseline.clone()));
    }
    let changed = format!("{LEDGER}handle = \"other\"\n");
    let mut site = input::site(FILE);
    site.anchor = SourceAnchor::Bytes(LEDGER.len(), LEDGER.len() + 1);
    let expected = fault(
        RuleId::M002,
        UnitStage::Syntax,
        site,
        UnitFault::TomlSyntax {
            message: "duplicate key `handle` in table `events`".into(),
        },
    );
    std::fs::write(&path, &changed).unwrap();
    assert_refusal(load(&read(&directory.0).unwrap()), expected);
    assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
    std::fs::write(&path, LEDGER).unwrap();
    assert_eq!(load(&read(&directory.0).unwrap()), Ok(baseline));
}

#[test]
fn documentary_snapshot_change_refuses_and_restores() {
    let directory = super::super::tests::Directory::new();
    std::fs::create_dir(directory.0.join("registry")).unwrap();
    let path = directory.0.join(FILE);
    std::fs::write(&path, LEDGER).unwrap();
    let loaded = read(&directory.0).unwrap();
    let baseline = load(&loaded).unwrap();
    let inventory = std::collections::BTreeMap::from([(loaded.file, loaded.bytes)]);
    let changed = LEDGER.replace("Addresses the weld decision.", "Changed authority reason.");
    std::fs::write(&path, &changed).unwrap();
    assert_refusal(
        input::reread(&directory.0, &inventory),
        DocumentError::Input(super::super::DocumentInputFault::SourceChanged {
            file: FILE.into(),
            expected: LEDGER.as_bytes().to_vec(),
            actual: Some(changed.as_bytes().to_vec()),
        }),
    );
    assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
    std::fs::write(&path, LEDGER).unwrap();
    assert_eq!(input::reread(&directory.0, &inventory), Ok(()));
    assert_eq!(load(&read(&directory.0).unwrap()), Ok(baseline));
}

#[test]
fn preparation_joint_mints_then_registry_then_authored_tags_restore() {
    use ftoo_manifest::*;
    use std::{collections::BTreeMap, fs, path::Path};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = super::super::tests::Directory::new();
    let mut saved = BTreeMap::new();
    for source in crate::load_markdown_files(&root).unwrap().sources() {
        saved.insert(source.file.clone(), source.bytes.clone());
    }
    for name in [
        "generated/manifest.json",
        "tools/assembly/native-lock.toml",
        "tools/assembly/prepare-native.py",
        "tools/notation/package-lock.json",
        "tools/notation/package.json",
        "tools/notation/support-digests.json",
        "tools/notation/.node-version",
        FILE,
    ] {
        saved.insert(name.into(), fs::read(root.join(name)).unwrap());
    }
    let unit = "schema=1\n[unit]\nid='sec:fixture:reader'\ndocument='main'\nmath={}\n[unit.texts]\nen='Use @tag{c:1}'\n";
    saved.insert(
        "units/documents.toml".into(),
        b"schema=1\n[documents.main]\nrequired-languages=['en']\n".to_vec(),
    );
    saved.insert(
        "units/statements/fixture.toml".into(),
        unit.as_bytes().to_vec(),
    );
    for (name, bytes) in &saved {
        let path = directory.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let corpus = crate::load_unit_files(&directory.0).unwrap();
    let markdown = crate::load_markdown_files(&directory.0).unwrap();
    let joint = collect_joint_mints(&corpus, &markdown).unwrap();
    let graph = seed().unwrap();
    let events = load(&read(&directory.0).unwrap()).unwrap();
    let book = addressing(&graph)
        .unwrap()
        .with_documentary(&joint, events.clone())
        .unwrap();
    let baseline = crate::check_weld_files(&directory.0, &book).unwrap();
    let altered_unit = unit.replace("@tag{c:1}", "@tag{missing}");
    fs::write(
        directory.0.join("units/statements/fixture.toml"),
        &altered_unit,
    )
    .unwrap();
    let altered_ledger = LEDGER.replace("handle = \"dec:weld\"", "handle = \"bad:label:token\"");
    fs::write(directory.0.join(FILE), &altered_ledger).unwrap();
    let collision = "· `def:langevin:coefficient-one`\n";
    fs::write(directory.0.join("docs/collision-fixture.md"), collision).unwrap();
    let support_directory = super::super::tests::Directory::new();
    let support = super::super::NativeSupport::at(support_directory.0.clone());
    let refuse = || {
        let before = source_snapshot(&directory.0);
        let result = super::super::prepare_documents(&directory.0, &support).unwrap_err();
        assert_eq!(source_snapshot(&directory.0), before);
        result
    };
    let origin = MintOrigin {
        kind: MintKind::RustIdentity,
        site: LabelSite::RustIdentity {
            file: "crates/ftoo_manifest/src/identity.rs".into(),
            identity: NodeId::LangevinCoefficientOne,
            field: "semantic_name".into(),
        },
    };
    let expected = vec![LabelDiagnostic {
        rule: RuleId::M043,
        stage: LabelStage::Namespace,
        unit: None,
        language: None,
        site: origin.site.clone(),
        fault: LabelFault::DuplicateMint {
            label: "def:langevin:coefficient-one".into(),
            expected: 1,
            actual: 2,
            origins: vec![
                origin,
                MintOrigin {
                    kind: MintKind::Markdown,
                    site: LabelSite::Markdown {
                        file: "docs/collision-fixture.md".into(),
                        bytes: 0..collision.len() - 1,
                    },
                },
            ],
        },
    }];
    let actual = refuse();
    let DocumentError::Labels(crate::LabelFileError::Labels(ref errors)) = actual else {
        panic!("wrong wrapper: {actual:?}")
    };
    assert_eq!(errors.diagnostics(), expected);
    assert_eq!(actual.to_string(), format!("{}\n", expected[0]));
    fs::remove_file(directory.0.join("docs/collision-fixture.md")).unwrap();
    let expected = DocumentError::DocumentaryRegistry(
        RegistryErrors::from_diagnostics(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Catalogue,
            site: field_site(&altered_ledger, "handle", true),
            fault: RegistryFault::LabelShapedHandle {
                handle: "bad:label:token".into(),
            },
        }])
        .unwrap(),
    );
    let actual = refuse();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    fs::write(directory.0.join(FILE), LEDGER).unwrap();
    let begin = altered_unit.find("'Use").unwrap();
    let expected = DocumentError::Units(crate::UnitLoadError::Admission(
        UnitErrors::from_diagnostics(vec![UnitDiagnostic {
            rule: RuleId::M042,
            stage: UnitStage::Resolution,
            unit: Some("sec:fixture:reader".into()),
            language: Some("en".into()),
            site: UnitSite {
                file: "units/statements/fixture.toml".into(),
                path: ["unit", "texts", "en"]
                    .map(|p| PathPart::Key(p.into()))
                    .to_vec(),
                anchor: SourceAnchor::Bytes(begin, begin + 19),
                decoded: Some(4..17),
                occurrence: Some(0),
            },
            fault: UnitFault::ResolutionFault {
                cause: TagResolutionFault::UnknownTag {
                    input: "missing".into(),
                },
            },
        }])
        .unwrap(),
    ));
    let actual = refuse();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    fs::write(directory.0.join("units/statements/fixture.toml"), unit).unwrap();
    assert_eq!(load(&read(&directory.0).unwrap()), Ok(events));
    assert_eq!(
        crate::check_weld_files(&directory.0, &book).unwrap(),
        baseline
    );
    for (name, bytes) in saved {
        assert_eq!(fs::read(directory.0.join(name)).unwrap(), bytes);
    }
    assert_eq!(support.closing_hash(), None);
}
// Each boundary mutation is reread from a private file and restored completely.
#[test]
fn documentary_exclusive_fields_and_tokens_restore() {
    let directory = super::super::tests::Directory::new();
    std::fs::create_dir(directory.0.join("registry")).unwrap();
    let path = directory.0.join(FILE);
    let baseline = load(&source(LEDGER)).unwrap();
    for field in [
        "ordinal",
        "change",
        "handle",
        "authority-record",
        "reason",
        "label",
    ] {
        let changed = LEDGER
            .split_inclusive('\n')
            .filter(|line| !line.starts_with(&format!("{field} = ")))
            .collect::<String>();
        let expected = fault(
            RuleId::M004,
            UnitStage::Shape,
            UnitSite {
                file: FILE.into(),
                path: vec![PathPart::Key("events".into()), PathPart::Index(0)],
                anchor: SourceAnchor::Parent(11, changed.trim_end().len()),
                decoded: None,
                occurrence: None,
            },
            UnitFault::MissingField { name: field.into() },
        );
        std::fs::write(&path, &changed).unwrap();
        let actual = load(&read(&directory.0).unwrap()).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), expected.to_string());
        assert_eq!(std::fs::read(&path).unwrap(), changed.as_bytes());
        std::fs::write(&path, LEDGER).unwrap();
        assert_eq!(load(&read(&directory.0).unwrap()), Ok(baseline.clone()));
    }
    let changed = LEDGER.replace("change = \"admit\"", "change = \"redirect\"");
    let expected = fault(
        RuleId::M004,
        UnitStage::Shape,
        field_site(&changed, "change", true),
        UnitFault::UnknownToken {
            value: "redirect".into(),
            expected: vec!["admit".into(), "retire".into()],
        },
    );
    assert_refusal(load(&source(&changed)), expected);
    assert_eq!(load(&source(LEDGER)), Ok(baseline));
}
const LEDGER: &str = "schema = 1\n[[events]]\nordinal = 1\nchange = \"admit\"\nhandle = \"dec:weld\"\nlabel = \"dec:weld:equal-address-sets\"\nauthority-record = \"dec:tags:bound-to-labels\"\nreason = \"Addresses the weld decision.\"\n";
fn source(text: &str) -> UnitSource {
    UnitSource {
        file: FILE.into(),
        bytes: text.as_bytes().to_vec(),
    }
}
fn field_site(text: &str, field: &str, event: bool) -> UnitSite {
    let prefix = format!("{field} = ");
    let start = text.find(&prefix).unwrap() + prefix.len();
    let end = start + text[start..].find('\n').unwrap_or(text.len() - start);
    let mut path = if event {
        vec![PathPart::Key("events".into()), PathPart::Index(0)]
    } else {
        vec![]
    };
    path.push(PathPart::Key(field.into()));
    UnitSite {
        file: FILE.into(),
        path,
        anchor: SourceAnchor::Bytes(start, end),
        decoded: None,
        occurrence: None,
    }
}
fn fault(rule: RuleId, stage: UnitStage, site: UnitSite, fault: UnitFault) -> DocumentError {
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
#[test]
fn documentary_production_ledger_has_exact_events_and_permitted_handle() {
    let committed = include_str!("../../../../../registry/documentary-tags.toml");
    assert_eq!(committed, LEDGER);
    let events = load(&source(committed)).unwrap();
    let expected = [
        ("ordinal", "1"),
        ("handle", "dec:weld"),
        ("label", "dec:weld:equal-address-sets"),
        ("authority-record", "dec:tags:bound-to-labels"),
        ("reason", "Addresses the weld decision."),
    ];
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].site,
        UnitSite {
            file: FILE.into(),
            path: vec![PathPart::Key("events".into()), PathPart::Index(0)],
            anchor: SourceAnchor::Bytes(11, LEDGER.len() - 1),
            decoded: None,
            occurrence: None,
        }
    );
    assert_eq!(
        [
            &events[0].ordinal,
            &events[0].handle,
            events[0].label.as_ref().unwrap(),
            &events[0].authority_record,
            &events[0].reason
        ]
        .map(|s| (s.value.clone(), s.site.clone())),
        expected.map(|(field, value)| (value.into(), field_site(LEDGER, field, true)))
    );
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = crate::load_unit_files(&root).unwrap();
    let markdown = crate::load_markdown_files(&root).unwrap();
    let joint = ftoo_manifest::collect_joint_mints(&corpus, &markdown).unwrap();
    let admitted = book.with_documentary(&joint, events).unwrap();
    assert_eq!(
        admitted.resolve("dec:weld"),
        Ok(ftoo_manifest::AddressTarget::Documentary(
            "dec:weld:equal-address-sets".into()
        ))
    );
    assert_eq!(load(&source("schema = 1\nevents = []\n")), Ok(vec![]));
}
#[test]
fn documentary_strict_fields_restore_exact_bytes_and_drafts() {
    let directory = super::super::tests::Directory::new();
    std::fs::create_dir(directory.0.join("registry")).unwrap();
    let path = directory.0.join(FILE);
    let original = load(&source(LEDGER)).unwrap();
    for (field, event, old, replacement, expected, actual) in [
        (
            "schema",
            false,
            "1",
            "\"1\"",
            ShapeKind::Integer,
            ShapeKind::String,
        ),
        (
            "ordinal",
            true,
            "1",
            "\"1\"",
            ShapeKind::Integer,
            ShapeKind::String,
        ),
        (
            "change",
            true,
            "\"admit\"",
            "0",
            ShapeKind::String,
            ShapeKind::Integer,
        ),
        (
            "handle",
            true,
            "\"dec:weld\"",
            "0",
            ShapeKind::String,
            ShapeKind::Integer,
        ),
        (
            "label",
            true,
            "\"dec:weld:equal-address-sets\"",
            "0",
            ShapeKind::String,
            ShapeKind::Integer,
        ),
        (
            "authority-record",
            true,
            "\"dec:tags:bound-to-labels\"",
            "0",
            ShapeKind::String,
            ShapeKind::Integer,
        ),
        (
            "reason",
            true,
            "\"Addresses the weld decision.\"",
            "0",
            ShapeKind::String,
            ShapeKind::Integer,
        ),
    ] {
        let text = LEDGER.replacen(
            &format!("{field} = {old}"),
            &format!("{field} = {replacement}"),
            1,
        );
        std::fs::write(&path, &text).unwrap();
        let reread = read(&directory.0).unwrap();
        assert_eq!(reread.bytes, text.as_bytes());
        let wanted = fault(
            RuleId::M004,
            UnitStage::Shape,
            field_site(&text, field, event),
            UnitFault::WrongType { expected, actual },
        );
        let actual = load(&reread).unwrap_err();
        assert_eq!(actual, wanted);
        assert_eq!(actual.to_string(), wanted.to_string());
        assert_eq!(std::fs::read(&path).unwrap(), text.as_bytes());
        std::fs::write(&path, LEDGER).unwrap();
        let reread = read(&directory.0).unwrap();
        assert_eq!(reread.bytes, LEDGER.as_bytes());
        assert_eq!(load(&reread), Ok(original.clone()));
    }
    let text = LEDGER.replacen("schema = 1", "schema = 2", 1);
    assert_refusal(
        load(&source(&text)),
        fault(
            RuleId::M003,
            UnitStage::Shape,
            field_site(&text, "schema", false),
            UnitFault::SchemaValue {
                expected: "1".into(),
                actual: "2".into(),
            },
        ),
    );
    let text = LEDGER.replacen("ordinal = 1", "ordinal = 1.5", 1);
    assert_refusal(
        load(&source(&text)),
        fault(
            RuleId::M003,
            UnitStage::Numeric,
            field_site(&text, "ordinal", true),
            UnitFault::NonIntegerNumber {
                token: "1.5".into(),
            },
        ),
    );
}
#[test]
fn documentary_encoding_duplicates_bounds_and_missing_file_refuse_exactly() {
    let mut where_ = input::site(FILE);
    where_.anchor = SourceAnchor::Bytes(0, 1);
    assert_refusal(
        load(&UnitSource {
            file: FILE.into(),
            bytes: vec![255],
        }),
        fault(
            RuleId::M001,
            UnitStage::Encoding,
            where_.clone(),
            UnitFault::InvalidUtf8,
        ),
    );
    where_.anchor = SourceAnchor::Bytes(0, 3);
    assert_refusal(
        load(&source("\u{feff}schema = 1\nevents = []\n")),
        fault(
            RuleId::M001,
            UnitStage::Encoding,
            where_.clone(),
            UnitFault::Bom,
        ),
    );
    where_.anchor = SourceAnchor::Bytes(9, 10);
    assert_refusal(
        load(&source("schema=1\nschema=1\n")),
        fault(
            RuleId::M002,
            UnitStage::Syntax,
            where_,
            UnitFault::TomlSyntax {
                message: "duplicate key `schema` in document root".into(),
            },
        ),
    );
    let expected = DocumentError::DocumentaryRegistry(
        RegistryErrors::from_diagnostics(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Source,
            site: input::site(FILE),
            fault: RegistryFault::Limit {
                resource: RegistryResource::Bytes,
                limit: 1_048_576,
                actual: 1_048_577,
            },
        }])
        .unwrap(),
    );
    assert_refusal(
        load(&UnitSource {
            file: FILE.into(),
            bytes: vec![b' '; 1_048_577],
        }),
        expected.clone(),
    );
    let directory = super::super::tests::Directory::new();
    std::fs::create_dir(directory.0.join("registry")).unwrap();
    let path = directory.0.join(FILE);
    assert_refusal(
        read(&directory.0),
        DocumentError::Io {
            operation: super::super::DocumentIoOperation::Inspect,
            path: path.clone(),
            kind: std::io::ErrorKind::NotFound,
            replaced: vec![],
            removed: vec![],
            cleanup: vec![],
        },
    );
    std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    assert_refusal(read(&directory.0), expected);
    std::fs::write(&path, LEDGER).unwrap();
    let restored = read(&directory.0).unwrap();
    assert_eq!(restored, source(LEDGER));
    assert_eq!(load(&restored), load(&source(LEDGER)));
}

#[test]
fn documentary_event_bound_precedes_owned_event_allocation_and_shape() {
    let text = format!("schema = 1\nevents = [{}]\n", vec!["0"; 4097].join(","));
    let expected = DocumentError::DocumentaryRegistry(
        RegistryErrors::from_diagnostics(vec![RegistryDiagnostic {
            rule: RuleId::M060,
            stage: RegistryStage::Source,
            site: UnitSite {
                file: FILE.into(),
                path: vec![PathPart::Key("events".into())],
                anchor: SourceAnchor::Bytes(20, text.len() - 1),
                decoded: None,
                occurrence: None,
            },
            fault: RegistryFault::Limit {
                resource: RegistryResource::Events,
                limit: 4096,
                actual: 4097,
            },
        }])
        .unwrap(),
    );
    let actual = load(&source(&text)).unwrap_err();
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), expected.to_string());
    assert_eq!(load(&source("schema = 1\nevents = []\n")), Ok(vec![]));
}

fn source_snapshot(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(source_snapshot(&path));
        } else {
            result.insert(path.clone(), std::fs::read(path).unwrap());
        }
    }
    result
}
