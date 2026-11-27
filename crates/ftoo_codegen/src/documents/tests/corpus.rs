use super::*;
use sha2::Digest as _;

fn copy_tree(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn prefix_fixture() -> Result<NativeFixture, Box<dyn std::error::Error>> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = native_fixture();
    let root = &fixture.directory.0;
    for directory in [
        "crates",
        "docs",
        "documents",
        "generated",
        "licenses",
        "registry",
        "tools",
        "units",
    ] {
        copy_tree(&source.join(directory), &root.join(directory))?;
    }
    for file in ["Cargo.lock", "Cargo.toml", "README.md"] {
        fs::copy(source.join(file), root.join(file))?;
    }
    let path = root.join("documents/paper/explainer/explainer.tex");
    fs::copy(
        source.join("units/fixtures/explainer-prefix-source.tex"),
        &path,
    )?;
    let original = fs::read_to_string(&path)?;
    let lines: Vec<_> = original.split_inclusive('\n').collect();
    assert!(lines[43].starts_with(r"\section{The Whole Idea, in Plain Words}"));
    assert_eq!(lines[97], "\n");
    fs::write(
        &path,
        format!(
            "{}% generated: sec:explainer:plain\n{}",
            lines[..43].concat(),
            lines[98..].concat()
        ),
    )?;
    let path = root.join("units/scopes/explainer.toml");
    fs::copy(
        source.join("units/fixtures/explainer-prefix-scope.toml"),
        &path,
    )?;
    let original = fs::read_to_string(&path)?;
    let old = "unit = \"sec:reading:plain-intro\"\nrole = \"paragraph\"";
    assert_eq!(original.matches(old).count(), 1);
    fs::write(
        &path,
        original.replace(
            old,
            "unit = \"sec:explainer:plain\"\nrole = \"corpus-slot\"",
        ),
    )?;
    fs::remove_dir_all(root.join("units/corpus"))?;
    fs::create_dir_all(root.join("units/corpus"))?;
    fs::copy(
        source.join("units/fixtures/explainer-plain.toml"),
        root.join("units/corpus/explainer-plain.toml"),
    )?;
    Ok(fixture)
}

#[test]
fn support_required_corpus_slot_renders_and_recovers_the_real_prefix()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "FTOO_CORPUS_SLOT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // A child holds the exclusive fixture lock, so its holder shares no
        // process cache with the sibling tests that wait on that lock.
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "documents::tests::corpus::support_required_corpus_slot_renders_and_recovers_the_real_prefix",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("test result: ok. 1 passed;"),
            "child status: {:?}\n{}{}",
            output.status,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }
    let fixture = prefix_fixture()?;
    let root = &fixture.directory.0;
    let support = NativeSupport::from_environment(root).map_err(|error| format!("{error:?}"))?;
    let prepared = generate_documents(root, &support)?;
    let path = "generated/documents/explainer-pilot/sec-explainer-plain.tex";
    let artifact = prepared
        .artifacts()
        .iter()
        .find(|artifact| artifact.path() == path)
        .unwrap();
    let fragment = std::str::from_utf8(artifact.bytes())?;
    assert!(fragment.ends_with("\n\\par\n"));
    assert!(fragment.starts_with("\\section{The Whole Idea, in Plain Words}\n\\label{sec:explainer:plain}\n\n\\PaperNativeParagraph{sec:reading:plain-intro}"));
    assert_eq!(
        fragment
            .matches(r"\PaperNativeParagraph{sec:reading:plain-intro}")
            .count(),
        1
    );
    assert_eq!(fragment.matches(r"\enspace ").count(), 3);
    let markdown = prepared
        .corpus_slot_recovery_export(
            "markdown",
            "documents/paper/explainer/explainer.tex",
            44,
            98,
            "sec:explainer:plain",
        )
        .unwrap();
    let html = prepared
        .corpus_slot_recovery_export(
            "html",
            "documents/paper/explainer/explainer.tex",
            44,
            98,
            "sec:explainer:plain",
        )
        .unwrap();
    assert_eq!(markdown["schema"], 1);
    assert_eq!(markdown["coverage"], "excerpt");
    assert_eq!(markdown["selection"], html["selection"]);
    assert_eq!(markdown["text"], html["text"]);
    let text = markdown["text"].as_str().unwrap();
    assert!(text.starts_with("The Whole Idea, in Plain Words This first section says"));
    assert!(text.contains("Ties. Start with two lives."));
    assert!(text.contains("Work. Left alone, ties loosen."));
    assert!(text.contains("Help and cost. Any act can be sorted"));
    assert!(!text.contains("  "));
    for export in [&markdown, &html] {
        let path = export["rendering"]["path"].as_str().unwrap();
        let bytes = fs::read(root.join(path))?;
        let digest = crate::hex(&sha2::Sha256::digest(&bytes));
        assert_eq!(export["rendering"]["sha256"], digest);
    }
    assert!(
        prepared
            .corpus_slot_recovery_export(
                "html",
                "documents/paper/explainer/explainer.tex",
                0,
                98,
                "sec:explainer:plain"
            )
            .is_none()
    );
    let mut changed = artifact.bytes().to_vec();
    let position = changed
        .windows(5)
        .position(|bytes| bytes == b"Ties.")
        .unwrap();
    changed[position] = b'L';
    fs::write(root.join(path), changed)?;
    let error = artifacts::verify(root, &prepared).unwrap_err();
    assert!(format!("{error:?}").contains(path));
    Ok(())
}
