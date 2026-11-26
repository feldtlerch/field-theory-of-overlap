use super::*;
use sha2::Digest as _;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

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

fn prefix_fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = Fixture(
        std::env::temp_dir().join(format!("ftoo-corpus-slot-{}-{nonce}", std::process::id())),
    );
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
        copy_tree(&source.join(directory), &root.0.join(directory))?;
    }
    for file in ["Cargo.lock", "Cargo.toml", "README.md"] {
        fs::copy(source.join(file), root.0.join(file))?;
    }
    let path = root.0.join("documents/paper/explainer/explainer.tex");
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
    let path = root.0.join("units/scopes/explainer.toml");
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
    fs::create_dir_all(root.0.join("units/corpus"))?;
    fs::copy(
        source.join("units/fixtures/explainer-plain.toml"),
        root.0.join("units/corpus/explainer-plain.toml"),
    )?;
    Ok(root)
}

#[test]
fn support_required_corpus_slot_renders_and_recovers_the_real_prefix()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = prefix_fixture()?;
    let support =
        NativeSupport::from_environment(&fixture.0).map_err(|error| format!("{error:?}"))?;
    let prepared = generate_documents(&fixture.0, &support)?;
    let path = "generated/documents/explainer-pilot/sec-explainer-plain.tex";
    let artifact = prepared
        .artifacts()
        .iter()
        .find(|artifact| artifact.path() == path)
        .unwrap();
    let fragment = std::str::from_utf8(artifact.bytes())?;
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
        let bytes = fs::read(fixture.0.join(path))?;
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
    fs::write(fixture.0.join(path), changed)?;
    let error = artifacts::verify(&fixture.0, &prepared).unwrap_err();
    assert!(format!("{error:?}").contains(path));
    Ok(())
}
