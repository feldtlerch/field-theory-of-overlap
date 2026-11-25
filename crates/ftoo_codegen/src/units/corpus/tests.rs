use super::*;

fn output(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("ftoo-import-{name}-{}-{nonce}", std::process::id()))
}

fn composition_copy() -> std::path::PathBuf {
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let copied = output("composition");
    let list = "documents/paper-build/sources.tsv";
    let mut files = vec!["units/documents.toml".to_owned(), list.to_owned()];
    let source_list = fs::read_to_string(original.join(list)).unwrap();
    files.extend(
        source_list
            .lines()
            .filter(|path| body_path(path))
            .map(str::to_owned),
    );
    for entry in fs::read_dir(original.join("units/scopes")).unwrap() {
        let entry = entry.unwrap();
        files.push(format!(
            "units/scopes/{}",
            entry.file_name().to_string_lossy()
        ));
    }
    for file in files {
        let target = copied.join(&file);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(original.join(&file), target).unwrap();
    }
    copied
}

#[test]
fn declared_corpus_is_accounted_and_repeatable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let first = output("first");
    let second = output("second");
    let a = dry_run(&root, &first).unwrap();
    let b = dry_run(&root, &second).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["source_count"], 12);
    assert_eq!(a["accounted_bytes"], a["source_bytes"]);
    assert_eq!(a["transfer_ready"], false);
    let declared_native = fs::read_to_string(root.join("generated/documents/inclusions.tsv"))
        .unwrap()
        .lines()
        .map(|row| row.split_once('\t').unwrap().0)
        .filter(|source| body_path(source))
        .map(|_| 1_u64)
        .sum::<u64>();
    assert_eq!(
        a["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["existing_native_inclusions"].as_u64().unwrap())
            .sum::<u64>(),
        declared_native
    );
    assert_eq!(
        fs::read(first.join("draft.json")).unwrap(),
        fs::read(second.join("draft.json")).unwrap()
    );
    fs::remove_dir_all(first).unwrap();
    fs::remove_dir_all(second).unwrap();
}

#[test]
fn all_fault_rows_count_occurrences_blocks_and_solo_clears() {
    let path = "documents/paper/parts/example.tex";
    let source = "A \\unknown{x}, $x$, and \\unknown{y}.\n\nOnly \\quad.\n";
    let draft = import_source(path, source.as_bytes(), "digest".into(), &[]).unwrap();
    let bytes = BTreeMap::from([(path.to_owned(), source.as_bytes().to_vec())]);
    let rows = all_fault_rows(&[draft], &bytes);
    assert_eq!(rows["unsupported command \\unknown"]["occurrences"], 2);
    assert_eq!(rows["unsupported command \\unknown"]["blocks"], 1);
    assert_eq!(rows["unsupported command \\unknown"]["sole_kind_blocks"], 0);
    assert_eq!(rows["unresolved mathematics"]["occurrences"], 1);
    assert_eq!(rows["unsupported command \\quad"]["sole_kind_blocks"], 1);
    let display_source = "\\begin{equation}x\\end{equation}\n";
    let display = import_source(path, display_source.as_bytes(), "digest".into(), &[]).unwrap();
    assert_eq!(display.math.len(), 1);
    let display_bytes = BTreeMap::from([(path.to_owned(), display_source.as_bytes().to_vec())]);
    let display_rows = all_fault_rows(&[display], &display_bytes);
    assert_eq!(display_rows.as_object().unwrap().len(), 1);
    assert_eq!(
        display_rows["unsupported environment equation"]["sole_kind_blocks"],
        1
    );
}

#[test]
fn output_inside_a_source_directory_refuses_before_writing() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join("generated/import-dry-run-test");
    assert!(
        dry_run(&root, &output)
            .unwrap_err()
            .contains("output overlaps the source tree")
    );
    assert!(!output.exists());
}

#[test]
fn duplicate_and_missing_composition_declarations_are_not_native_inclusions() {
    let copied = composition_copy();
    fs::copy(
        copied.join("units/scopes/front.toml"),
        copied.join("units/scopes/front-copy.toml"),
    )
    .unwrap();
    let fault = input::import_inclusions(&copied).unwrap_err().to_string();
    assert!(fault.contains("duplicate source"), "{fault}");
    fs::remove_file(copied.join("units/scopes/front-copy.toml")).unwrap();
    fs::remove_file(copied.join("units/scopes/front.toml")).unwrap();
    let out = output("missing-composition");
    let report = dry_run(&copied, &out).unwrap();
    assert_eq!(report["refusal_kinds"]["unowned generated marker"][0], 1);
    fs::remove_dir_all(out).unwrap();
    fs::remove_dir_all(copied).unwrap();
}
