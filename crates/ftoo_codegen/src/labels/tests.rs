use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, process::Command};
static NEXT: AtomicUsize = AtomicUsize::new(0);
#[test]
fn w02_discovery_order_matches_pathlib_components() {
    let root = directory();
    let baseline = files::collect(&root).unwrap();
    fs::create_dir(root.join("sort")).unwrap();
    fs::write(root.join("sort/nested.md"), "").unwrap();
    fs::write(root.join("sort.md"), "").unwrap();
    let sources = files::collect(&root).unwrap();
    let names: Vec<_> = sources.iter().map(|s| s.file.as_str()).collect();
    assert_eq!(
        names,
        [
            ".hidden/nested/.note.md",
            "root.md",
            "sort/nested.md",
            "sort.md"
        ]
    );
    let output = Command::new("python3").args(["-I", "-B", "-c", "import pathlib,json; print(json.dumps([p.as_posix() for p in sorted(pathlib.Path('.').rglob('*.md'))]))"]).current_dir(&root).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, Vec::<u8>::new());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::json!(names)
    );
    fs::remove_file(root.join("sort.md")).unwrap();
    fs::remove_file(root.join("sort/nested.md")).unwrap();
    fs::remove_dir(root.join("sort")).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::remove_dir_all(root).unwrap();
}
fn directory() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ftoo-markdown-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join(".hidden/nested")).unwrap();
    fs::write(root.join("root.md"), "· `a:b:c`\r\n(`a:b:c`)\r").unwrap();
    fs::write(root.join(".hidden/nested/.note.md"), "· `d:e:f`\n").unwrap();
    fs::write(root.join("ignored.MD"), "(`unknown:label:example`)").unwrap();
    root
}
#[test]
fn w02_recursive_hidden_inventory_and_io_fault_restoration() {
    let root = directory();
    let baseline = files::collect(&root).unwrap();
    assert_eq!(
        baseline,
        vec![
            ftoo_manifest::MarkdownSource {
                file: ".hidden/nested/.note.md".into(),
                bytes: b"\xc2\xb7 `d:e:f`\n".to_vec()
            },
            ftoo_manifest::MarkdownSource {
                file: "root.md".into(),
                bytes: b"\xc2\xb7 `a:b:c`\r\n(`a:b:c`)\r".to_vec()
            }
        ]
    );
    for (operation, path) in [
        (LabelIoOperation::ListDirectory, root.join(".hidden")),
        (LabelIoOperation::InspectEntry, root.join("root.md")),
        (LabelIoOperation::ReadMarkdown, root.join("root.md")),
    ] {
        let error = files::collect_with(&root, |op, p| {
            if op == operation && p == path {
                Err(io::ErrorKind::PermissionDenied.into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(
            error,
            LabelFileError::Io {
                operation,
                file: path.clone(),
                kind: io::ErrorKind::PermissionDenied
            }
        );
        assert_eq!(
            error.to_string(),
            format!("Io {{ operation: {operation:?}, file: {path:?}, kind: PermissionDenied }}")
        );
        assert_eq!(files::collect(&root).unwrap(), baseline);
    }
    let path = root.join("directory.md");
    fs::create_dir(&path).unwrap();
    let error = load_markdown_files(&root).unwrap_err();
    assert_eq!(
        error,
        LabelFileError::Input(LabelEntryFault::UnexpectedEntry {
            file: path.clone(),
            kind: MarkdownEntryKind::Directory
        })
    );
    assert_eq!(
        error.to_string(),
        format!("Input(UnexpectedEntry {{ file: {path:?}, kind: Directory }})")
    );
    fs::remove_dir(path).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn w02_symlinks_nonutf8_and_nonregular_entries_are_refused() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let root = directory();
    let baseline = files::collect(&root).unwrap();
    let link = root.join("link.md");
    symlink(root.join("root.md"), &link).unwrap();
    let error = load_markdown_files(&root).unwrap_err();
    assert_eq!(
        error,
        LabelFileError::Input(LabelEntryFault::UnexpectedEntry {
            file: link.clone(),
            kind: MarkdownEntryKind::Symlink
        })
    );
    assert_eq!(
        error.to_string(),
        format!("Input(UnexpectedEntry {{ file: {link:?}, kind: Symlink }})")
    );
    fs::remove_file(&link).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    let link = root.join("linked-directory");
    symlink(root.join(".hidden"), &link).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::remove_file(link).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    let bad = root.join(std::ffi::OsString::from_vec(vec![255, b'.', b'm', b'd']));
    let ignored = root.join(std::ffi::OsString::from_vec(vec![
        255, b'.', b't', b'x', b't',
    ]));
    fs::write(&ignored, b"not Markdown").unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::remove_file(&ignored).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::write(&bad, b"").unwrap();
    let error = load_markdown_files(&root).unwrap_err();
    assert_eq!(
        error,
        LabelFileError::Input(LabelEntryFault::UnexpectedEntry {
            file: bad.clone(),
            kind: MarkdownEntryKind::NonUtf8Name
        })
    );
    assert_eq!(
        error.to_string(),
        format!("Input(UnexpectedEntry {{ file: {bad:?}, kind: NonUtf8Name }})")
    );
    fs::remove_file(&bad).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    let socket = root.join("socket.md");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let error = load_markdown_files(&root).unwrap_err();
    assert_eq!(
        error,
        LabelFileError::Input(LabelEntryFault::UnexpectedEntry {
            file: socket.clone(),
            kind: MarkdownEntryKind::Other
        })
    );
    assert_eq!(
        error.to_string(),
        format!("Input(UnexpectedEntry {{ file: {socket:?}, kind: Other }})")
    );
    drop(listener);
    fs::remove_file(socket).unwrap();
    assert_eq!(files::collect(&root).unwrap(), baseline);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn w02_actual_encoding_and_w03_local_citation_faults_restore_all_sources() {
    use ftoo_manifest::{
        LabelDiagnostic, LabelFault, LabelSite, LabelStage, ReferenceDomain, RuleId,
    };
    let root = directory();
    let baseline = load_markdown_files(&root).unwrap();
    let path = root.join("root.md");
    let saved = fs::read(&path).unwrap();
    for (bytes, rule, stage, range, fault, display) in [
        (
            vec![255],
            RuleId::M001,
            LabelStage::Encoding,
            0..1,
            LabelFault::InvalidUtf8 {
                offset: 0,
                error_len: Some(1),
            },
            "InvalidUtf8 { offset: 0, error_len: Some(1) }",
        ),
        (
            vec![0xef, 0xbb, 0xbf],
            RuleId::M001,
            LabelStage::Encoding,
            0..3,
            LabelFault::Bom,
            "Bom",
        ),
        (
            b"(`x:y:z`)".to_vec(),
            RuleId::M043,
            LabelStage::Markdown,
            1..8,
            LabelFault::UnresolvedMarkdownCitation {
                label: "x:y:z".into(),
                expected: 1,
                actual: 0,
                domain: ReferenceDomain::MarkdownOnly,
            },
            "UnresolvedMarkdownCitation { label: \"x:y:z\", expected: 1, actual: 0, domain: MarkdownOnly }",
        ),
    ] {
        fs::write(&path, &bytes).unwrap();
        let snapshot = files::collect(&root).unwrap();
        let error = load_markdown_files(&root).unwrap_err();
        let LabelFileError::Source(LabelSourceError::Admission(errors)) = &error else {
            panic!("{error}")
        };
        assert_eq!(
            errors.diagnostics(),
            [LabelDiagnostic {
                rule,
                stage,
                unit: None,
                language: None,
                site: LabelSite::Markdown {
                    file: "root.md".into(),
                    bytes: range.clone()
                },
                fault
            }]
        );
        assert_eq!(
            error.to_string(),
            format!(
                "{rule:?} {stage:?} unit=None language=None at markdown \"root.md\" bytes {}..{}: {display}\n",
                range.start, range.end
            )
        );
        assert_eq!(files::collect(&root).unwrap(), snapshot);
        fs::write(&path, &saved).unwrap();
        assert_eq!(load_markdown_files(&root).unwrap(), baseline);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn w06_python_differential_compares_discovery_snapshots_and_all_occurrences() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let python_script = root.join("tools/check_labels.py").canonicalize().unwrap();
    differential(&root, &python_script);
    let fixture = directory();
    fs::write(
        fixture.join("spans.md"),
        "See `a:b:c`; ``d:e:f``; (`a:b:c`, ``d:e:f``).\n",
    )
    .unwrap();
    differential(&fixture, &python_script);
    fs::remove_file(fixture.join("spans.md")).unwrap();
    differential(&fixture, &python_script);
    fs::remove_dir_all(fixture).unwrap();
}
fn differential(root: &Path, python_script: &Path) {
    let sources = files::collect(root).unwrap();
    let checked = check_markdown_sources(sources.clone()).unwrap();
    let script = r"
import importlib.util, json, pathlib, sys
spec = importlib.util.spec_from_file_location('labels', sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
result = []
for path in sorted(pathlib.Path('.').rglob('*.md')):
    raw = path.read_bytes()
    text = path.read_text(encoding='utf-8')
    mints, cites = module.harvest(text)
    result.append([path.as_posix(), list(raw), mints, cites])
print(json.dumps(result))
";
    let output = Command::new("python3")
        .args(["-I", "-B", "-c", script])
        .arg(python_script)
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
    let python: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: Vec<_> = sources.iter().map(|source| {
        let in_file = |o: &&ftoo_manifest::LabelOccurrence| matches!(&o.site, ftoo_manifest::LabelSite::Markdown { file, .. } if file == &source.file);
        serde_json::json!([source.file, source.bytes, checked.mints().iter().filter(in_file).map(|o| &o.label).collect::<Vec<_>>(), checked.citations().iter().filter(in_file).map(|o| &o.label).collect::<Vec<_>>()])
    }).collect();
    assert_eq!(python, serde_json::json!(expected));
    assert_eq!(files::collect(root).unwrap(), sources);
    for source in &sources {
        assert_eq!(fs::read(root.join(&source.file)).unwrap(), source.bytes);
    }
}
