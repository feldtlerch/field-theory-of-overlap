use super::*;

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
    run(&[
        "check".into(),
        "--support".into(),
        support,
        "--work".into(),
        scratch.0.join("build").into_os_string(),
    ])
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
fn actual_input_declarations_run_without_support() {
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
        "tools/paper/ledger.py",
        "tools/paper/pdf_text.py",
        "documents/paper-build/layout-pins.toml",
        "documents/paper-build/label-mints.tsv",
        "documents/paper-build/self-description.json",
        "tools/paper/fixtures/leak-control.tex",
        "generated/manifest.json",
        "registry/documentary-tags.toml",
    ] {
        assert!(plan["inputs"].get(path).is_none(), "{path}");
    }
    let status = Command::new("python3")
        .args(["-B", "tools/paper/test_identities.py"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
}
