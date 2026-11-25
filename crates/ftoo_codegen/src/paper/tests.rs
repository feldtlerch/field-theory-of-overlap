use super::*;

#[test]
fn output_diagnostics_refuse_duplicate_labels_and_destinations() {
    let scratch = Scratch::new();
    let paper = scratch.0.join("paper");
    fs::create_dir_all(&paper).unwrap();
    fs::write(paper.join("main.blg"), b"clean").unwrap();
    fs::write(scratch.0.join("main-2.txt"), b"clean").unwrap();
    fs::write(
        paper.join("main.log"),
        "LaTeX Warning: Label `sec:say:direction' multiply defined.",
    )
    .unwrap();
    assert_eq!(
        validate_output(&scratch.0, "main", 2),
        Err(PaperFault::Input(
            "Duplicate label or named output object: main".into()
        ))
    );
    fs::write(
        paper.join("main.log"),
        b"Output written without diagnostics",
    )
    .unwrap();
    fs::write(
        scratch.0.join("main-2.txt"),
        b"warning: Object @same-destination already defined.\n",
    )
    .unwrap();
    assert_eq!(
        validate_output(&scratch.0, "main", 2),
        Err(PaperFault::Input(
            "Duplicate label or named output object: main".into()
        ))
    );
    fs::write(scratch.0.join("main-2.txt"), b"clean").unwrap();
    validate_output(&scratch.0, "main", 2).unwrap();
}

#[test]
fn documents_commands_refuse_work_and_missing_support() {
    let scratch = Scratch::new();
    let work = scratch.0.join("absent");
    assert_eq!(
        run(&["documents-check".into(), "--release".into()])
            .unwrap_err()
            .to_string(),
        "Release flag applies only to the paper pair"
    );
    assert_eq!(
        run(&[
            "documents-check".into(),
            "--work".into(),
            work.into_os_string(),
        ])
        .unwrap_err()
        .to_string(),
        "Documents commands take no work area"
    );
    assert_eq!(
        require_notation_support(None).unwrap_err().to_string(),
        "Notation support required"
    );
}

#[test]
fn registered_gap_check_is_separate_from_release_command() {
    let req = request(&[
        "check-registered-html-gaps".into(),
        "--built".into(),
        "/unused-built-pair".into(),
    ])
    .unwrap();
    assert!(req.release);
    assert!(req.registered_html_gaps);
    assert_eq!(
        request(&[
            "check-registered-html-gaps".into(),
            "--release".into(),
            "--built".into(),
            "/unused-built-pair".into(),
        ])
        .err()
        .unwrap()
        .to_string(),
        "Registered development gaps require the built product check without a release flag"
    );
    assert!(
        !request(&["check".into(), "--release".into()])
            .unwrap()
            .registered_html_gaps
    );
}

#[test]
fn support_required_release_refuses_each_registered_gap_part() -> Result<()> {
    let req = request(&[
        "check".into(),
        "--release".into(),
        "--built".into(),
        "/unused-built-pair".into(),
    ])?;
    assert!(!req.registered_html_gaps);
    let PaperFault::Release { part, reason } = check_release_documents(&req).unwrap_err() else {
        panic!("ordinary release must refuse the checked screen destination gap")
    };
    assert_eq!(part, "screen destinations");
    assert!(reason.contains("entries: 1"));
    let assembly = assembly_support(&req)?;
    let mut prepared = crate::check_documents(&req.root, &assembly)
        .map_err(|error| PaperFault::Input(error.to_string()))?;
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.0.join("registry")).unwrap();
    fs::copy(
        req.root.join("registry/html-block-gaps.json"),
        scratch.0.join("registry/html-block-gaps.json"),
    )
    .unwrap();
    fs::copy(
        req.root.join("registry/screen-destination-gaps.json"),
        scratch.0.join("registry/screen-destination-gaps.json"),
    )
    .unwrap();
    prepared.clear_screen_gaps_for_test();
    let PaperFault::Release { part, reason } =
        release::screen_parts(&scratch.0, &prepared, false).unwrap_err()
    else {
        panic!("stale screen register must refuse ordinary release")
    };
    assert_eq!(part, "screen destinations");
    assert!(reason.contains("registry/screen-destination-gaps.json"));
    fs::write(
        scratch.0.join("registry/screen-destination-gaps.json"),
        release::canonical_destinations(&prepared.screen_gap_inventory())?,
    )
    .unwrap();
    let PaperFault::Release { part, reason } =
        release::screen_parts(&scratch.0, &prepared, false).unwrap_err()
    else {
        panic!("ordinary release must refuse the checked HTML block gap")
    };
    assert_eq!(part, "HTML block gaps");
    assert!(reason.contains("units/statements/coda-audit-bottoms.toml"));
    Ok(())
}

#[test]
fn reading_acceptance_locates_bytes_and_reports_complete_faults() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    check_reading_acceptance(&root).unwrap();
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.0.join("documents/paper-build")).unwrap();
    let mut record = json!({"passages": (1..=19).map(|id| {
        let (file, data) = if id == 7 { ("text.tex", "abcd") } else { ("other.tex", "safe") };
        json!({"id":id,"file":file,"start":0,"end":4,
            "anchor": &data[..2],"sha256":hex(&Sha256::digest(data.as_bytes())),"reading_acceptance":"accepted"})
    }).collect::<Vec<_>>()});
    fs::write(scratch.0.join("other.tex"), b"safe").unwrap();
    let path = scratch
        .0
        .join("documents/paper-build/self-description.json");
    save(&path, &record).unwrap();
    fs::write(scratch.0.join("text.tex"), b"unrelated prefix abcd").unwrap();
    check_reading_acceptance(&scratch.0).unwrap();
    for data in [b"abXd".as_slice(), b"abcd abcd"] {
        fs::write(scratch.0.join("text.tex"), data).unwrap();
        let fault = check_reading_acceptance(&scratch.0).unwrap_err();
        assert_eq!(fault, PaperFault::ReadingAcceptance { passage: 7 });
        assert_eq!(
            fault.to_string(),
            "Paper passage 7: accepted bytes must occur exactly once with the recorded hash and a filled reading acceptance"
        );
    }
    fs::write(scratch.0.join("text.tex"), b"abcd").unwrap();
    record["passages"][6]["reading_acceptance"] = json!("");
    save(&path, &record).unwrap();
    assert_eq!(
        check_reading_acceptance(&scratch.0),
        Err(PaperFault::ReadingAcceptance { passage: 7 })
    );
    record["passages"].as_array_mut().unwrap().remove(6);
    save(&path, &record).unwrap();
    let fault = check_reading_acceptance(&scratch.0).unwrap_err();
    assert_eq!(
        fault,
        PaperFault::PassageSet {
            ids: (1..=19).filter(|id| *id != 7).collect()
        }
    );
    assert_eq!(
        fault.to_string(),
        "Paper acceptance requires exactly passage ids 1 through 19; found [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]"
    );
}

#[test]
fn release_conjunction_refuses_each_failed_condition() {
    let scratch = Scratch::new();
    let root = scratch.0.join("repo");
    let work = scratch.0.join("work");
    fs::create_dir_all(root.join("documents/paper-build")).unwrap();
    fs::create_dir_all(root.join("generated/paper")).unwrap();
    fs::create_dir_all(&work).unwrap();
    let record = json!({"passages": (1..=19).map(|id| json!({"id":id,"file":"text.tex",
        "start":0,"end":4,"anchor":"ab","sha256":hex(&Sha256::digest(b"abcd")),
        "reading_acceptance":"accepted"})).collect::<Vec<_>>()});
    save(
        &root.join("documents/paper-build/self-description.json"),
        &record,
    )
    .unwrap();
    fs::write(root.join("text.tex"), b"abcd").unwrap();
    let ledger = json!([{"key":"e:test","spelling":"E:TEST","status":"paper"}]);
    save(&work.join("identity-ledger.json"), &ledger).unwrap();
    let receipt = json!({"inputs": {}});
    save(&root.join(RECEIPT), &receipt).unwrap();
    for (_, name) in OUTPUTS {
        fs::write(root.join("generated/paper").join(name), b"abcd").unwrap();
        fs::write(work.join(name), b"abcd").unwrap();
    }
    release::check(&root, &work, &receipt).unwrap();
    for (part, reason) in [
        (
            "self-description acceptance",
            "Paper passage 1: accepted bytes must occur exactly once with the recorded hash and a filled reading acceptance",
        ),
        ("printed identities", "Provisional printed identity: e:test"),
        (
            "paper byte gate",
            "Paper byte gate failed; generated/paper/paper.pdf: offset 2, lengths 4 / 4, window 0: expected 61626364 (\"abcd\"), actual 61625864 (\"abXd\")",
        ),
    ] {
        match part {
            "self-description acceptance" => fs::write(root.join("text.tex"), b"abXd").unwrap(),
            "printed identities" => {
                let mut changed = ledger.clone();
                changed[0]["status"] = json!("provisional");
                save(&work.join("identity-ledger.json"), &changed).unwrap();
            }
            _ => fs::write(work.join("paper.pdf"), b"abXd").unwrap(),
        }
        let fault = release::check(&root, &work, &receipt).unwrap_err();
        assert_eq!(
            fault,
            PaperFault::Release {
                part,
                reason: reason.into()
            }
        );
        assert_eq!(
            fault.to_string(),
            format!("Paper release M065 refused at {part}: {reason}")
        );
        fs::write(root.join("text.tex"), b"abcd").unwrap();
        save(&work.join("identity-ledger.json"), &ledger).unwrap();
        fs::write(work.join("paper.pdf"), b"abcd").unwrap();
    }
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("paper-control-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn support_required_paper_gate() {
    let support =
        std::env::var_os("FTOO_ASSEMBLY_SUPPORT").expect("FTOO_ASSEMBLY_SUPPORT is required");
    let scratch = Scratch::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/test_identity_tex.py"])
            .arg(&root)
            .arg(&support)
            .arg(scratch.0.join("identity-controls")),
    )
    .unwrap();
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/checking_capture.py", "check"])
            .arg(&root)
            .arg(&support)
            .arg(scratch.0.join("checking-capture")),
    )
    .unwrap();
    run(&[
        "check".into(),
        "--support".into(),
        support,
        "--work".into(),
        scratch.0.join("build").into_os_string(),
    ])
    .unwrap();
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/check_citation_pdf.py"])
            .arg(&root)
            .arg(scratch.0.join("build/paper.pdf")),
    )
    .unwrap();
}

#[test]
fn checking_capture_fixture_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/checking_capture.py", "selftest"])
            .arg(root.join("tools/paper/fixtures/checking-capture.json")),
    )
    .unwrap();
}

#[test]
fn checking_capture_provenance_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/test_checking_capture.py"]),
    )
    .unwrap();
}

#[test]
fn table_evidence_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    command_output(
        Command::new("python3")
            .current_dir(&root)
            .args(["-B", "tools/paper/test_table_evidence.py"]),
    )
    .unwrap();
}

#[test]
fn bibliography_pdf_inventory_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    command_output(Command::new("python3").current_dir(&root).args([
        "-B",
        "tools/paper/check_citation_pdf.py",
        "selftest",
    ]))
    .unwrap();
}

#[test]
fn real_pdf_and_receipt_comparisons_report_complete_faults() {
    let scratch = Scratch::new();
    let root = scratch.0.join("repo");
    let work = scratch.0.join("work");
    fs::create_dir_all(root.join("generated/paper")).unwrap();
    fs::create_dir_all(&work).unwrap();
    let receipt = json!({"inputs":{"a.tex":"before"}});
    save(&root.join(RECEIPT), &receipt).unwrap();
    for (_, name) in OUTPUTS {
        fs::write(root.join("generated/paper").join(name), b"abcd").unwrap();
        fs::write(work.join(name), b"abcd").unwrap();
    }
    fs::write(work.join("paper.pdf"), b"abXde").unwrap();
    let fault = compare(&root, &work, &receipt).unwrap_err();
    assert_eq!(
        fault,
        PaperFault::PdfMismatch(vec![ByteDifference {
            path: "generated/paper/paper.pdf".into(),
            expected_length: 4,
            actual_length: 5,
            first_difference: 2,
            window_start: 0,
            expected_hex: "61626364".into(),
            actual_hex: "6162586465".into(),
            expected_text: "abcd".into(),
            actual_text: "abXde".into(),
        }])
    );
    assert_eq!(
        fault.to_string(),
        "Paper byte gate failed; generated/paper/paper.pdf: offset 2, lengths 4 / 5, window 0: expected 61626364 (\"abcd\"), actual 6162586465 (\"abXde\")"
    );
    fs::write(work.join("paper.pdf"), b"abcd").unwrap();
    for (fresh, expected, message) in [
        (
            json!({"inputs":{"a.tex":"after"}}),
            PaperFault::StaleReceipt {
                entry: "a.tex".into(),
                expected: Some("before".into()),
                actual: Some("after".into()),
            },
            "Paper receipt stale at a.tex: committed Some(\"before\"), fresh Some(\"after\"); regenerate",
        ),
        (
            json!({"inputs":{}}),
            PaperFault::StaleReceipt {
                entry: "a.tex".into(),
                expected: Some("before".into()),
                actual: None,
            },
            "Paper receipt stale at a.tex: committed Some(\"before\"), fresh None; regenerate",
        ),
    ] {
        let fault = compare(&root, &work, &fresh).unwrap_err();
        assert_eq!(fault, expected);
        assert_eq!(fault.to_string(), message);
    }
    compare(&root, &work, &receipt).unwrap();
}

#[test]
fn committed_pdf_mutation_reports_the_complete_byte_window() {
    let scratch = Scratch::new();
    let root = scratch.0.join("repo");
    let work = scratch.0.join("work");
    fs::create_dir_all(root.join("generated/paper")).unwrap();
    fs::create_dir_all(&work).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../generated/paper");
    let receipt = json!({"inputs":{}});
    save(&root.join(RECEIPT), &receipt).unwrap();
    for (_, name) in OUTPUTS {
        fs::copy(source.join(name), root.join("generated/paper").join(name)).unwrap();
        fs::copy(source.join(name), work.join(name)).unwrap();
    }
    let mut actual = fs::read(work.join("paper.pdf")).unwrap();
    let length = actual.len();
    actual[0] = b'!';
    fs::write(work.join("paper.pdf"), actual).unwrap();
    let fault = compare(&root, &work, &receipt).unwrap_err();
    assert_eq!(
        fault,
        PaperFault::PdfMismatch(vec![ByteDifference {
            path: "generated/paper/paper.pdf".into(),
            expected_length: length,
            actual_length: length,
            first_difference: 0,
            window_start: 0,
            expected_hex: "255044462d312e350a25e4f0edf80a352030206f626a0a3c3c2f547970652f58".into(),
            actual_hex: "215044462d312e350a25e4f0edf80a352030206f626a0a3c3c2f547970652f58".into(),
            expected_text: "%PDF-1.5\n%����\n5 0 obj\n<</Type/X".into(),
            actual_text: "!PDF-1.5\n%����\n5 0 obj\n<</Type/X".into(),
        }])
    );
    assert_eq!(
        fault.to_string(),
        format!(
            "Paper byte gate failed; generated/paper/paper.pdf: offset 0, lengths {length} / {length}, window 0: expected 255044462d312e350a25e4f0edf80a352030206f626a0a3c3c2f547970652f58 (\"%PDF-1.5\\n%����\\n5 0 obj\\n<</Type/X\"), actual 215044462d312e350a25e4f0edf80a352030206f626a0a3c3c2f547970652f58 (\"!PDF-1.5\\n%����\\n5 0 obj\\n<</Type/X\")"
        )
    );
}

#[test]
fn cycle_cap_and_pass_drift_report_complete_faults() {
    let a = BTreeMap::from([("main.aux".into(), b"a".to_vec())]);
    let b = BTreeMap::from([("main.aux".into(), b"b".to_vec())]);
    assert_eq!(fixed_point(None, &a, 1, 2), Ok(false));
    assert_eq!(fixed_point(Some(&a), &a, 2, 2), Ok(true));
    let fault = fixed_point(Some(&a), &b, 2, 2).unwrap_err();
    assert_eq!(fault, PaperFault::CycleCap { cycles: 2 });
    assert_eq!(
        fault.to_string(),
        "No joint fixed point after 2 pair cycles; retained auxiliary bytes and windows in cycles.json"
    );
    for (log, tex, bibtex, message) in [
        (
            "Running TeX\nRunning BibTeX\nRerunning TeX\n",
            2,
            1,
            "Explicit pass recipe failed for main: TeX 2 (expected 3), bibliography 1 (expected 1), exit Some(0)",
        ),
        (
            "Running TeX\nRerunning TeX\nRerunning TeX\n",
            3,
            0,
            "Explicit pass recipe failed for main: TeX 3 (expected 3), bibliography 0 (expected 1), exit Some(0)",
        ),
    ] {
        let fault = pass_counts("main", log, Some(0)).unwrap_err();
        assert_eq!(
            fault,
            PaperFault::PassCounts {
                job: "main".into(),
                tex,
                bibtex,
                exit: Some(0)
            }
        );
        assert_eq!(fault.to_string(), message);
    }
    assert_eq!(
        pass_counts(
            "main",
            "Running TeX\nRunning BibTeX\nRerunning TeX\nRerunning TeX\n",
            Some(0)
        ),
        Ok((3, 1))
    );
}

#[test]
fn byte_windows_locate_changes_lengths_and_empty_auxiliary_presence() {
    let row = difference("paper.pdf", b"abcd", b"abXd");
    assert_eq!(row["first_difference"], 2);
    assert_eq!(row["expected_length"], 4);
    assert_eq!(row["actual_hex"], "61625864");
    assert_eq!(
        difference("paper.pdf", b"ab", b"abc")["first_difference"],
        2
    );
    let rows = auxiliary_delta(
        &BTreeMap::from([("nested/a.aux".into(), vec![])]),
        &BTreeMap::from([("main.aux".into(), vec![])]),
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["expected_present"], false);
    assert_eq!(rows[1]["actual_present"], false);
}

#[test]
fn paper_input_declarations_are_fixed_without_support() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plan = inputs(&root, None).unwrap();
    let sources = fs::read_to_string(root.join("documents/paper-build/sources.tsv")).unwrap();
    assert_eq!(sources.lines().count(), 26);
    for path in sources.lines() {
        assert!(plan["inputs"].get(path).is_some(), "{path}");
    }
    for path in [
        "tools/paper/common.py",
        "tools/paper/bibroute.py",
        "tools/assembly/native-lock.toml",
    ] {
        assert!(plan["inputs"].get(path).is_some(), "{path}");
    }
    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates/ftoo_codegen/src/paper/mod.rs",
        "crates/ftoo_codegen/src/paper/tests.rs",
        "tools/paper/test_identities.py",
        "tools/paper/test_composition.py",
        "tools/paper/test_reference.py",
        "tools/paper/ledger.py",
        "tools/paper/pdf_text.py",
        "documents/paper-build/label-mints.tsv",
        "documents/paper-build/self-description.json",
        "tools/paper/fixtures/leak-control.tex",
        "generated/manifest.json",
        "registry/documentary-tags.toml",
        "units/documents.toml",
        "units/skeletons/main.toml",
        "units/skeletons/paper-pilot.toml",
        "units/scopes/front.toml",
    ] {
        assert!(plan["inputs"].get(path).is_none(), "{path}");
    }
}

#[test]
fn paper_python_identity_composition_reference_suites() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Command::new("python3")
        .args(["-B", "tools/paper/test_identities.py"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        Command::new("python3")
            .args(["-B", "tools/paper/test_composition.py"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("python3")
            .args(["-B", "tools/paper/test_reference.py"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn import_request_stays_outside_paper_production() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let args = vec![
        OsString::from("import-dry-run"),
        OsString::from("--repository"),
        root.clone().into_os_string(),
        OsString::from("--output-dir"),
        root.join("generated/import-test-output").into_os_string(),
    ];
    let req = request(&args).unwrap();
    assert_eq!(req.operation, "import-dry-run");
    assert!(
        run(&args)
            .unwrap_err()
            .to_string()
            .contains("output overlaps the source tree")
    );
}
