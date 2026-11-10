use ftoo_codegen::{
    LabelFileError, check_label_files, check_projection, load_markdown_files, load_unit_files,
};
use ftoo_manifest::{
    LabelDiagnostic, LabelFault, LabelSite, LabelStage, MintKind, MintOrigin, NodeId,
    ReferenceDomain, RuleId, integrate_labels, resolve_unit_tags,
};
use std::fmt::Write;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture(full: bool) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "ftoo-labels-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(directory.join("units/statements")).unwrap();
    fs::create_dir_all(directory.join("docs")).unwrap();
    for file in [
        "units/documents.toml",
        "units/statements/first-value.toml",
        "units/statements/value-context.toml",
    ] {
        fs::copy(root().join(file), directory.join(file)).unwrap();
    }
    if full {
        for source in load_markdown_files(&root()).unwrap().sources() {
            let path = directory.join(&source.file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &source.bytes).unwrap();
        }
    } else {
        fs::write(directory.join("docs/records.md"), "# Kernel · `proposal:manifest:kernel-bite`\n# Addressing · `proposal:manifest:addressing-lifecycle-bite`\n").unwrap();
    }
    directory
}
fn collision() -> LabelDiagnostic {
    LabelDiagnostic {
        rule: RuleId::M043,
        stage: LabelStage::Namespace,
        unit: None,
        language: None,
        site: LabelSite::RustIdentity {
            file: "crates/ftoo_manifest/src/identity.rs".into(),
            identity: NodeId::LangevinCoefficientOne,
            field: "semantic_name".into(),
        },
        fault: LabelFault::DuplicateMint {
            label: "def:langevin:coefficient-one".into(),
            expected: 1,
            actual: 2,
            origins: vec![
                MintOrigin {
                    kind: MintKind::RustIdentity,
                    site: LabelSite::RustIdentity {
                        file: "crates/ftoo_manifest/src/identity.rs".into(),
                        identity: NodeId::LangevinCoefficientOne,
                        field: "semantic_name".into(),
                    },
                },
                MintOrigin {
                    kind: MintKind::Markdown,
                    site: LabelSite::Markdown {
                        file: "docs/weld-collision.md".into(),
                        bytes: 12..45,
                    },
                },
            ],
        },
    }
}
#[test]
fn w04_actual_production_identity_collision_and_restoration() {
    let directory = fixture(true);
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_label_files(&directory, &book).unwrap();
    let path = directory.join("docs/weld-collision.md");
    let bytes = "# Collision · `def:langevin:coefficient-one`\n";
    assert_eq!(bytes.len(), 46);
    fs::write(&path, bytes).unwrap();
    let local = load_markdown_files(&directory).unwrap();
    assert_eq!(
        local.mints().last().unwrap().label,
        NodeId::LangevinCoefficientOne.semantic_name()
    );
    let output = Command::new("python3")
        .arg("-I")
        .arg(root().join("tools/check_labels.py"))
        .current_dir(&directory)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "labels: {} mints, {} citations\n",
            baseline.markdown_mints + 1,
            baseline.markdown_citations
        )
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
    let actual = check_label_files(&directory, &book).unwrap_err();
    let LabelFileError::Labels(errors) = &actual else {
        panic!("{actual}")
    };
    assert_eq!(errors.diagnostics(), [collision()]);
    assert_eq!(
        actual.to_string(),
        concat!(
            "M043 Namespace unit=None language=None at rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\": DuplicateMint { label: \"def:langevin:coefficient-one\", expected: 1, actual: 2, origins: [",
            "MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }, ",
            "MintOrigin { kind: Markdown, site: Markdown { file: \"docs/weld-collision.md\", bytes: 12..45 } }] }\n"
        )
    );
    assert_eq!(fs::read(&path).unwrap(), bytes.as_bytes());
    fs::remove_file(&path).unwrap();
    assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    fs::write(&path, "# Distinct · `sec:fixture:distinct`\n").unwrap();
    let distinct = check_label_files(&directory, &book).unwrap();
    assert_eq!(distinct.markdown_mints, baseline.markdown_mints + 1);
    fs::remove_file(&path).unwrap();
    assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn w04_actual_empty_slice_and_absent_identity_keep_permanent_ownership() {
    let directory = fixture(true);
    let seed = ftoo_manifest::seed().unwrap();
    let seed_book = ftoo_manifest::addressing(&seed).unwrap();
    let original = check_label_files(&directory, &seed_book).unwrap();
    let mut saved = Vec::new();
    for file in [
        "units/statements/first-value.toml",
        "units/statements/value-context.toml",
    ] {
        let path = directory.join(file);
        let bytes = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            bytes
                .replace("@tag{c:1}", "coefficient")
                .replace("@tag{v:1}", "value"),
        )
        .unwrap();
        saved.push((path, bytes));
    }
    let path = directory.join("docs/weld-collision.md");
    for absent in [false, true] {
        let mut draft = ftoo_manifest::seed_draft();
        draft.coverage.slice.clear();
        if absent {
            draft
                .nodes
                .retain(|n| n.id != NodeId::LangevinCoefficientOne);
            draft.edges.retain(|e| {
                e.source != NodeId::LangevinCoefficientOne
                    && e.target != NodeId::LangevinCoefficientOne
            });
        }
        let graph = ftoo_manifest::validate(draft).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let corpus = load_unit_files(&directory).unwrap();
        assert_eq!(
            corpus
                .units()
                .iter()
                .map(|u| (u.label().as_str(), u.document()))
                .collect::<Vec<_>>(),
            [
                ("sec:coefficients:first-value", "main"),
                ("sec:coefficients:value-context", "main")
            ]
        );
        assert!(resolve_unit_tags(&corpus, &book).unwrap().tags().is_empty());
        let baseline = check_label_files(&directory, &book).unwrap();
        fs::write(&path, "# Collision · `def:langevin:coefficient-one`\n").unwrap();
        assert_eq!(
            load_markdown_files(&directory).unwrap().mints().len(),
            baseline.markdown_mints + 1
        );
        let error = check_label_files(&directory, &book).unwrap_err();
        let LabelFileError::Labels(errors) = &error else {
            panic!("{error}")
        };
        assert_eq!(errors.diagnostics(), [collision()]);
        assert_eq!(
            error.to_string(),
            "M043 Namespace unit=None language=None at rust-identity \"crates/ftoo_manifest/src/identity.rs\" def:langevin:coefficient-one field \"semantic_name\": DuplicateMint { label: \"def:langevin:coefficient-one\", expected: 1, actual: 2, origins: [MintOrigin { kind: RustIdentity, site: RustIdentity { file: \"crates/ftoo_manifest/src/identity.rs\", identity: LangevinCoefficientOne, field: \"semantic_name\" } }, MintOrigin { kind: Markdown, site: Markdown { file: \"docs/weld-collision.md\", bytes: 12..45 } }] }\n"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Collision · `def:langevin:coefficient-one`\n"
        );
        fs::remove_file(&path).unwrap();
        assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
        fs::write(&path, "# Distinct · `sec:fixture:distinct`\n").unwrap();
        assert_eq!(
            check_label_files(&directory, &book).unwrap().markdown_mints,
            baseline.markdown_mints + 1
        );
        fs::remove_file(&path).unwrap();
        assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    }
    for (path, bytes) in saved {
        fs::write(path, bytes).unwrap();
    }
    assert_eq!(check_label_files(&directory, &seed_book).unwrap(), original);
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn w05_missing_documentary_records_pin_all_real_events_and_restore() {
    let directory = fixture(false);
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_label_files(&directory, &book).unwrap();
    let path = directory.join("docs/records.md");
    let saved = fs::read_to_string(&path).unwrap();
    for (label, ordinals) in [
        ("proposal:manifest:kernel-bite", 1..=4),
        ("proposal:manifest:addressing-lifecycle-bite", 5..=8),
    ] {
        let changed = saved.replace(&format!("· `{label}`"), label);
        fs::write(&path, &changed).unwrap();
        assert_eq!(load_markdown_files(&directory).unwrap().mints().len(), 1);
        let actual = check_label_files(&directory, &book).unwrap_err();
        let LabelFileError::Labels(errors) = &actual else {
            panic!("{actual}")
        };
        let expected: Vec<_> = ordinals
            .clone()
            .map(|ordinal| LabelDiagnostic {
                rule: RuleId::M044,
                stage: LabelStage::RustCitation,
                unit: None,
                language: None,
                site: LabelSite::RustHistory {
                    file: "crates/ftoo_manifest/src/tag_history.rs".into(),
                    ordinal,
                    field: "authority_record".into(),
                },
                fault: LabelFault::UnresolvedRustCitation {
                    label: label.into(),
                    expected: 1,
                    actual: 0,
                    domain: ReferenceDomain::MarkdownOnly,
                },
            })
            .collect();
        assert_eq!(errors.diagnostics(), expected);
        let mut display = String::new();
        for ordinal in ordinals {
            writeln!(display, "M044 RustCitation unit=None language=None at rust-history \"crates/ftoo_manifest/src/tag_history.rs\" event {ordinal} field \"authority_record\": UnresolvedRustCitation {{ label: {label:?}, expected: 1, actual: 0, domain: MarkdownOnly }}").unwrap();
        }
        assert_eq!(actual.to_string(), display);
        assert_eq!(fs::read_to_string(&path).unwrap(), changed);
        fs::write(&path, &saved).unwrap();
        assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    }
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn w06_committed_projection_and_rust_books_have_identical_label_authority() {
    let root = root();
    check_projection(&root).unwrap();
    let bytes = fs::read(root.join("generated/manifest.json")).unwrap();
    let loaded = ftoo_manifest::load_projection(&bytes).unwrap();
    let loaded_book = loaded.addressing().unwrap();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let snapshot = ftoo_manifest::Snapshot::capture(&graph);
    let corpus = load_unit_files(&root).unwrap();
    let markdown = load_markdown_files(&root).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    assert!(std::ptr::eq(integrated.resolved(), &raw const resolved));
    assert!(std::ptr::eq(integrated.markdown(), &raw const markdown));
    assert_eq!(
        integrated
            .rust_citations()
            .iter()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        vec!["proposal:manifest:kernel-bite"; 4]
            .into_iter()
            .chain(vec!["proposal:manifest:addressing-lifecycle-bite"; 4])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        integrated
            .mints()
            .values()
            .filter(|m| m.kind == MintKind::RustIdentity)
            .count(),
        4
    );
    assert_eq!(
        NodeId::ALL.map(NodeId::semantic_name),
        [
            "def:langevin:coefficient-one",
            "def:langevin:coefficient-three",
            "claim:langevin:coefficient-one-value",
            "claim:langevin:coefficient-three-value"
        ]
    );
    assert_eq!(
        check_label_files(&root, &book).unwrap(),
        check_label_files(&root, &loaded_book).unwrap()
    );
    assert_eq!(
        book.history().collect::<Vec<_>>(),
        loaded_book.history().collect::<Vec<_>>()
    );
    assert_eq!(ftoo_manifest::Snapshot::capture(&graph), snapshot);
    assert_eq!(
        fs::read(root.join("generated/manifest.json")).unwrap(),
        bytes
    );
}

#[test]
fn w05_full_record_removal_fails_markdown_before_rust_citations() {
    let directory = fixture(true);
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_label_files(&directory, &book).unwrap();
    let path = directory.join("docs/design/manifest-kernel.md");
    let saved = fs::read_to_string(&path).unwrap();
    let changed = saved.replacen(
        "· `proposal:manifest:kernel-bite`",
        "proposal:manifest:kernel-bite",
        1,
    );
    fs::write(&path, &changed).unwrap();
    let error = check_label_files(&directory, &book).unwrap_err();
    let LabelFileError::Source(ftoo_manifest::LabelSourceError::Admission(errors)) = &error else {
        panic!("{error}")
    };
    let locations = [
        ("docs/design/manifest-addressing.md", 1257..1288),
        ("docs/design/manifest-audit-binding.md", 914..945),
    ];
    let expected: Vec<_> = locations
        .iter()
        .map(|(file, bytes)| LabelDiagnostic {
            rule: RuleId::M043,
            stage: LabelStage::Markdown,
            unit: None,
            language: None,
            site: LabelSite::Markdown {
                file: (*file).into(),
                bytes: bytes.clone(),
            },
            fault: LabelFault::UnresolvedMarkdownCitation {
                label: "proposal:manifest:kernel-bite".into(),
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::MarkdownOnly,
            },
        })
        .collect();
    assert_eq!(errors.diagnostics(), expected);
    assert_eq!(
        error.to_string(),
        concat!(
            "M043 Markdown unit=None language=None at markdown \"docs/design/manifest-addressing.md\" bytes 1257..1288: UnresolvedMarkdownCitation { label: \"proposal:manifest:kernel-bite\", expected: 1, actual: 0, domain: MarkdownOnly }\n",
            "M043 Markdown unit=None language=None at markdown \"docs/design/manifest-audit-binding.md\" bytes 914..945: UnresolvedMarkdownCitation { label: \"proposal:manifest:kernel-bite\", expected: 1, actual: 0, domain: MarkdownOnly }\n"
        )
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), changed);
    fs::write(&path, &saved).unwrap();
    assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn w06_different_language_tag_sets_pass_label_integration_without_weld() {
    let directory = fixture(false);
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let baseline = check_label_files(&directory, &book).unwrap();
    let declarations = directory.join("units/documents.toml");
    let statement = directory.join("units/statements/first-value.toml");
    let saved_declarations = fs::read(&declarations).unwrap();
    let saved_statement = fs::read(&statement).unwrap();
    fs::write(&declarations,"schema=1\n[documents.main]\nrequired-languages=['en']\n[documents.explainer]\nrequired-languages=['en','fr']\n").unwrap();
    fs::write(&statement,"schema=1\n[unit]\nid='sec:coefficients:first-value'\ndocument='explainer'\nmath={}\n[unit.texts]\nen='The coefficient @tag{c:1}.'\nfr='La valeur @tag{v:1}.'\n").unwrap();
    let corpus = load_unit_files(&directory).unwrap();
    let resolved = resolve_unit_tags(&corpus, &book).unwrap();
    let markdown = load_markdown_files(&directory).unwrap();
    let integrated = integrate_labels(&resolved, &markdown).unwrap();
    assert_eq!(
        integrated
            .resolved()
            .tags()
            .iter()
            .filter(|tag| tag.unit().as_str() == "sec:coefficients:first-value")
            .map(|tag| (tag.language(), tag.occurrence().input(), tag.target()))
            .collect::<Vec<_>>(),
        vec![
            ("en", "c:1", NodeId::LangevinCoefficientOne),
            ("fr", "v:1", NodeId::LangevinCoefficientOneValue)
        ]
    );
    let summary = check_label_files(&directory, &book).unwrap();
    assert_eq!(summary.texts, 3);
    assert_eq!(summary.tag_sites, 4);
    fs::write(&declarations, &saved_declarations).unwrap();
    fs::write(&statement, &saved_statement).unwrap();
    assert_eq!(check_label_files(&directory, &book).unwrap(), baseline);
    fs::remove_dir_all(directory).unwrap();
}
