#![allow(clippy::too_many_lines)]
use super::*;
use ftoo_manifest::{Callee, ExprDraft, PrimitiveId};
use ftoo_manifest::{DefinitionId, NotationResource};
use std::{
    fmt::Write as _,
    fs,
    os::unix::{fs::symlink, process::ExitStatusExt},
    path::{Path, PathBuf},
    process::Output,
    sync::atomic::{AtomicUsize, Ordering},
};

fn exact_adapter(actual: NotationAdapterFault, expected: NotationAdapterFault) {
    let actual = NotationRenderError::Adapter(actual);
    let expected = NotationRenderError::Adapter(expected);
    assert_eq!(actual, expected);
    assert_eq!(actual.to_string(), format!("{expected:?}"));
}
fn exact_integrity(actual: SupportIntegrityFault, expected: SupportIntegrityFault) {
    exact_adapter(
        NotationAdapterFault::Integrity(actual),
        NotationAdapterFault::Integrity(expected),
    );
}
fn output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
    Output {
        status: std::process::ExitStatus::from_raw(code),
        stdout: stdout.into(),
        stderr: stderr.into(),
    }
}
fn prepared() -> support::Support {
    let location = std::env::var_os("FTOO_NOTATION_SUPPORT").map(PathBuf::from);
    support::resolve(location.as_deref()).unwrap()
}
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        fn copy_tree(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let destination = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy_tree(&entry.path(), &destination);
                } else if entry.file_type().unwrap().is_file() {
                    fs::copy(entry.path(), destination).unwrap();
                }
            }
        }
        let original = prepared();
        original.check().unwrap();
        let root = original.root.parent().unwrap().join(format!(
            "notation-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        copy_tree(&original.root.join("packages"), &root.join("packages"));
        fs::create_dir_all(root.join("runtime/bin")).unwrap();
        fs::copy(&original.runtime, root.join("runtime/bin/node")).unwrap();
        Self { root }
    }
    fn support(&self) -> support::Support {
        support::resolve(Some(&self.root)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn support_configuration_and_protocol_are_exact() {
    for (value, reason) in [
        (None, SupportReason::Missing),
        (Some(PathBuf::from("relative")), SupportReason::Relative),
        (
            Some(support::repository()),
            SupportReason::RepositoryOverlap,
        ),
        (
            Some(support::repository().parent().unwrap().to_path_buf()),
            SupportReason::RepositoryOverlap,
        ),
    ] {
        exact_adapter(
            support::resolve(value.as_deref()).unwrap_err(),
            NotationAdapterFault::SupportConfiguration { value, reason },
        );
    }
    for (status, stdout, stderr, version, reason) in [
        (0, vec![255], vec![], false, ProtocolReason::InvalidUtf8),
        (
            0,
            "\u{feff}<math>".as_bytes().to_vec(),
            vec![],
            false,
            ProtocolReason::InvalidUtf8,
        ),
        (
            3 << 8,
            vec![],
            b"setup".to_vec(),
            false,
            ProtocolReason::SetupFailure,
        ),
        (9, vec![], vec![], false, ProtocolReason::UnexpectedStatus),
        (
            7 << 8,
            vec![],
            vec![],
            false,
            ProtocolReason::UnexpectedStatus,
        ),
        (
            0,
            b"<math>".to_vec(),
            b"warning".to_vec(),
            false,
            ProtocolReason::UnexpectedStderr,
        ),
        (0, vec![], vec![], false, ProtocolReason::EmptyOutput),
        (
            0,
            b"<html>".to_vec(),
            vec![],
            false,
            ProtocolReason::MissingMathml,
        ),
        (
            0,
            b"22.16.0\n0.16.22".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
        (
            0,
            b"22.16.0\n0.16.22\nextra\n".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
        (
            0,
            b"v22.16.0\n0.16.22\n".to_vec(),
            vec![],
            true,
            ProtocolReason::InvalidVersionResponse,
        ),
    ] {
        exact_adapter(
            katex::classify(output(status, &stdout, &stderr), version).unwrap_err(),
            NotationAdapterFault::Protocol {
                status: if status == 9 { None } else { Some(status >> 8) },
                stdout,
                stderr,
                reason,
            },
        );
    }
    for (bytes, component, expected, actual) in [
        (
            "22.15.0\n0.16.22\n",
            AdapterComponent::Runtime,
            "22.16.0",
            "22.15.0",
        ),
        (
            "22.16.0\n0.16.21\n",
            AdapterComponent::KaTeX,
            "0.16.22",
            "0.16.21",
        ),
    ] {
        exact_adapter(
            katex::classify(output(0, bytes.as_bytes(), &[]), true).unwrap_err(),
            NotationAdapterFault::VersionMismatch {
                component,
                expected: expected.into(),
                actual: actual.into(),
            },
        );
    }
    assert_eq!(
        katex::classify(output(0, b"22.16.0\n0.16.22\n", &[]), true).unwrap(),
        Ok("22.16.0\n0.16.22\n".into())
    );
    assert_eq!(
        katex::classify(output(512, b"complete parse error", &[]), false).unwrap(),
        Err("complete parse error".into())
    );
}

#[test]
fn strict_metadata_bytes_and_limits() {
    use SupportMetadataReason as R;
    let file = Path::new("metadata.json");
    for (bytes, path, reason) in [
        (
            vec![b' ', 255],
            "$",
            R::InvalidUtf8 {
                offset: 1,
                error_len: Some(1),
            },
        ),
        ("\u{feff}{}".as_bytes().to_vec(), "$", R::Bom),
        (
            br#"{"a":1,"\u0061":2}"#.to_vec(),
            "$",
            R::DuplicateKey { key: "a".into() },
        ),
        (
            br#"{"a":{"b":0,"b":1}}"#.to_vec(),
            "$.a",
            R::DuplicateKey { key: "b".into() },
        ),
        (
            br#"{"a":[1.0]}"#.to_vec(),
            "$.a[0]",
            R::NonInteger {
                token: "1.0".into(),
            },
        ),
        (
            b"1e2".to_vec(),
            "$",
            R::NonInteger {
                token: "1e2".into(),
            },
        ),
        (
            b"{".to_vec(),
            "$",
            R::Syntax {
                message: "EOF while parsing an object at line 1 column 1".into(),
                line: 1,
                column: 1,
            },
        ),
        (
            vec![b' '; 1_048_577],
            "$",
            R::SizeLimit {
                limit: 1_048_576,
                actual: 1_048_577,
            },
        ),
        (
            format!("{}0{}", "[".repeat(129), "]".repeat(129)).into_bytes(),
            "$",
            R::DepthLimit {
                limit: 128,
                actual: 129,
            },
        ),
    ] {
        exact_integrity(
            support::parse(file, &bytes).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: file.into(),
                path: path.into(),
                reason,
            },
        );
    }
    let mut boundary = vec![b' '; 1_048_575];
    boundary.push(b'0');
    assert_eq!(
        support::parse(file, &boundary).unwrap(),
        support::Value::Integer("0".into())
    );
    support::parse(
        file,
        format!("{}0{}", "[".repeat(128), "]".repeat(128)).as_bytes(),
    )
    .unwrap();
    assert_eq!(
        support::parse(file, br#"{"z":true,"a":[null,"x",-1]}"#).unwrap(),
        support::parse(file, br#"{ "a": [null, "x", -1], "z": true }"#).unwrap()
    );
}

#[test]
fn digest_schema_and_provenance_are_exact() {
    use SupportMetadataReason as R;
    let original: serde_json::Value = serde_json::from_slice(support::DIGESTS).unwrap();
    let file = PathBuf::from("tools/notation/support-digests.json");
    let mut fixtures = vec![];
    let mut v = original.clone();
    v["schema"] = 2.into();
    fixtures.push((
        v,
        "$.schema",
        R::InvalidValue {
            expected: "1".into(),
            actual: "2".into(),
        },
    ));
    let mut v = original.clone();
    v["schema"] = "1".into();
    fixtures.push((
        v,
        "$.schema",
        R::WrongType {
            expected: "integer".into(),
            actual: "string".into(),
        },
    ));
    let mut v = original.clone();
    v.as_object_mut().unwrap().remove("schema");
    fixtures.push((
        v,
        "$",
        R::MissingField {
            name: "schema".into(),
        },
    ));
    let mut v = original.clone();
    v["extra"] = true.into();
    fixtures.push((
        v,
        "$",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][0]["sha256"] = "ABC".into();
    fixtures.push((
        v,
        "$.packages[0].sha256",
        R::InvalidValue {
            expected: "64 lowercase hexadecimal characters".into(),
            actual: "ABC".into(),
        },
    ));
    for name in [
        "/absolute",
        "node_modules/katex/../escape",
        "node_modules/katex//empty",
        "node_modules/katex/./dot",
        "node_modules/katex/back\\slash",
        "node_modules/other/file",
    ] {
        let mut v = original.clone();
        v["packages"][0]["path"] = name.into();
        fixtures.push((
            v,
            "$.packages[0].path",
            R::InvalidValue {
                expected: "relative locked-package file path".into(),
                actual: name.into(),
            },
        ));
    }
    let mut v = original.clone();
    v["packages"][0]["extra"] = true.into();
    fixtures.push((
        v,
        "$.packages[0]",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][0].as_object_mut().unwrap().remove("sha256");
    fixtures.push((
        v,
        "$.packages[0]",
        R::MissingField {
            name: "sha256".into(),
        },
    ));
    let mut v = original.clone();
    v["packages"][1] = v["packages"][0].clone();
    fixtures.push((
        v,
        "$.packages[1]",
        R::DuplicateEntry {
            key: "node_modules/commander/LICENSE".into(),
        },
    ));
    let mut v = original.clone();
    let runtime = v["runtimes"][0].clone();
    v["runtimes"].as_array_mut().unwrap().push(runtime);
    fixtures.push((
        v,
        "$.runtimes[1]",
        R::DuplicateEntry {
            key: "linux-x64".into(),
        },
    ));
    let mut v = original.clone();
    v["runtimes"][0]["extra"] = true.into();
    fixtures.push((
        v,
        "$.runtimes[0]",
        R::UnknownField {
            name: "extra".into(),
        },
    ));
    let mut v = original.clone();
    v["runtimes"][0]
        .as_object_mut()
        .unwrap()
        .remove("executable-sha256");
    fixtures.push((
        v,
        "$.runtimes[0]",
        R::MissingField {
            name: "executable-sha256".into(),
        },
    ));
    for (value, path, reason) in fixtures {
        exact_integrity(
            support::inventory(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: file.clone(),
                path: path.into(),
                reason,
            },
        );
    }
    let inventory = support::inventory(support::DIGESTS).unwrap();
    assert_eq!(inventory.packages.len(), 219);
    assert_eq!(
        inventory.runtimes["linux-x64"],
        "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d"
    );
    assert_eq!(
        original["runtimes"][0]["release-sha256"],
        "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e"
    );
    assert_eq!(
        original["runtimes"][0]["release-artifact"],
        "node-v22.16.0-linux-x64.tar.xz"
    );
}

#[test]
fn digest_path_byte_order_refuses_first_pair_and_restores() {
    let original: serde_json::Value = serde_json::from_slice(support::DIGESTS).unwrap();
    let admitted = support::inventory(support::DIGESTS).unwrap();
    for (from, to, previous, current) in [
        (
            133,
            135,
            "node_modules/katex/src/environments/cd.js",
            "node_modules/katex/src/environments.js",
        ),
        (
            146,
            192,
            "node_modules/katex/src/functions/verb.js",
            "node_modules/katex/src/functions.js",
        ),
    ] {
        let mut changed = original.clone();
        let records = changed["packages"].as_array_mut().unwrap();
        assert_eq!(records[from]["path"], current);
        let record = records.remove(from);
        records.insert(to, record);
        assert_eq!(records[to - 1]["path"], previous);
        exact_integrity(
            support::inventory(&serde_json::to_vec(&changed).unwrap()).unwrap_err(),
            SupportIntegrityFault::Metadata {
                file: PathBuf::from("tools/notation/support-digests.json"),
                path: format!("$.packages[{to}]"),
                reason: SupportMetadataReason::OutOfOrder {
                    previous: previous.into(),
                    current: current.into(),
                },
            },
        );
        let records = changed["packages"].as_array_mut().unwrap();
        let record = records.remove(to);
        records.insert(from, record);
        assert_eq!(changed, original);
        assert_eq!(
            support::inventory(&serde_json::to_vec(&changed).unwrap())
                .unwrap()
                .packages,
            admitted.packages
        );
    }
}

#[test]
fn every_invocation_rechecks_copies_hashes_and_postprocess_bytes() {
    let fixture = Fixture::new();
    let support = fixture.support();
    assert_eq!(
        katex::invoke(&support, None).unwrap(),
        Ok("22.16.0\n0.16.22\n".into())
    );
    assert!(
        katex::invoke(&support, Some("α"))
            .unwrap()
            .unwrap()
            .contains("<math")
    );
    for (name, copy, expected_bytes, length) in [
        (
            "package.json",
            SupportCopy::Manifest,
            support::MANIFEST,
            166,
        ),
        (
            "package-lock.json",
            SupportCopy::Lockfile,
            support::LOCK,
            1094,
        ),
    ] {
        let file = fixture.root.join("packages").join(name);
        let original = fs::read(&file).unwrap();
        assert_eq!(original, expected_bytes);
        assert_eq!(original.len(), length);
        let mut changed = original.clone();
        changed[0] ^= 1;
        fs::write(&file, &changed).unwrap();
        let actual = katex::invoke_with(&support, true, || {
            panic!("copy refusal must precede subprocess")
        });
        fs::write(&file, &original).unwrap();
        exact_adapter(
            actual.unwrap_err(),
            NotationAdapterFault::Integrity(SupportIntegrityFault::CopyMismatch {
                copy,
                external: file.clone(),
                committed: format!("tools/notation/{name}"),
                first_difference: 0,
                expected_length: length,
                actual_length: length,
            }),
        );
        support.check().unwrap();
        let mut changed = original.clone();
        changed.push(b' ');
        fs::write(&file, &changed).unwrap();
        let actual = support.check();
        fs::write(&file, &original).unwrap();
        exact_adapter(
            actual.unwrap_err(),
            NotationAdapterFault::Integrity(SupportIntegrityFault::CopyMismatch {
                copy,
                external: file,
                committed: format!("tools/notation/{name}"),
                first_difference: length,
                expected_length: length,
                actual_length: length + 1,
            }),
        );
        support.check().unwrap();
    }
    // These SHA-256 answers were measured independently with Python hashlib.
    for (name, expected, actual) in [
        (
            "packages/node_modules/katex/dist/katex.js",
            "8f1f4fb1080439c012bb8fddf424fa253e9144ab4cdd7dc0eae0e92cd04a5e33",
            "5b31ed9174171dd8d94617604c7ac0d620b8a8623bf0a1db2b80e4a05dc5b903",
        ),
        (
            "packages/node_modules/katex/package.json",
            "bd6430ca96592a055393a1ce980925162da3d40cd0231813d41ef0c56daacd91",
            "e567ce8fd5bd5b5cf7d8832b052a732a98432d6b870b734517d3e03e0273b117",
        ),
        (
            "packages/node_modules/commander/index.js",
            "d4f8af488a7a08cd71df8503fcc3e5f2fd61e9b26eb368ecd5cc8159862cde3f",
            "d7cbe63dbdc73c6d3c39617002ea6ae83d4660e5d54ecf3c75ba68e6dd516311",
        ),
        (
            "runtime/bin/node",
            "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d",
            "4cec40aecc844435f116dfd5f39bec2008d39ec854e19b010a728f2f33b9daa1",
        ),
    ] {
        let file = fixture.root.join(name);
        let original = fs::read(&file).unwrap();
        let mut changed = original.clone();
        changed[0] ^= 1;
        fs::write(&file, &changed).unwrap();
        let result = katex::invoke_with(&support, true, || {
            panic!("digest refusal must precede subprocess")
        });
        fs::write(&file, &original).unwrap();
        let fault = NotationAdapterFault::Integrity(SupportIntegrityFault::DigestMismatch {
            file: file.clone(),
            expected: expected.into(),
            actual: actual.into(),
        });
        exact_adapter(result.unwrap_err(), fault.clone());
        support.check().unwrap();
        // An earlier successful call is never a trust cache. A mutation after
        // precheck also suppresses an otherwise successful subprocess result.
        assert!(
            katex::invoke(&support, Some("β"))
                .unwrap()
                .unwrap()
                .contains("<math")
        );
        let result = katex::invoke_with(&support, false, || {
            fs::write(&file, &changed).unwrap();
            Ok(output(0, b"<math>untrusted result</math>", &[]))
        });
        fs::write(&file, &original).unwrap();
        exact_adapter(result.unwrap_err(), fault);
        support.check().unwrap();
    }
    for operation in [AdapterOperation::WriteInput, AdapterOperation::WaitOutput] {
        let fault = NotationAdapterFault::Io {
            operation,
            kind: std::io::ErrorKind::BrokenPipe,
        };
        exact_adapter(
            katex::invoke_with(&support, false, || Err(fault.clone())).unwrap_err(),
            fault,
        );
        support.check().unwrap();
    }
    let fault = NotationAdapterFault::Unavailable {
        component: AdapterComponent::Runtime,
        kind: std::io::ErrorKind::NotFound,
    };
    exact_adapter(
        katex::invoke_with(&support, true, || Err(fault.clone())).unwrap_err(),
        fault,
    );
    assert!(
        katex::invoke(&support, Some("\\alpha{}"))
            .unwrap()
            .unwrap()
            .contains("<math")
    );
    exact_adapter(
        katex::invoke_with(&support, true, || Ok(output(0, b"22.15.0\n0.16.22\n", &[])))
            .unwrap_err(),
        NotationAdapterFault::VersionMismatch {
            component: AdapterComponent::Runtime,
            expected: "22.16.0".into(),
            actual: "22.15.0".into(),
        },
    );
    for (bytes, message) in [
        (
            vec![255],
            "The encoded data was not valid for encoding utf-8",
        ),
        (vec![239, 187, 191, b'x'], "input BOM"),
    ] {
        exact_adapter(
            katex::invoke_bytes(&support, &bytes).unwrap_err(),
            NotationAdapterFault::Protocol {
                status: Some(3),
                stdout: vec![],
                stderr: message.as_bytes().to_vec(),
                reason: ProtocolReason::SetupFailure,
            },
        );
        assert!(
            katex::invoke(&support, Some("x"))
                .unwrap()
                .unwrap()
                .contains("<math")
        );
    }
}

#[test]
fn installed_record_and_file_inventory_are_complete() {
    let fixture = Fixture::new();
    let support = fixture.support();
    let packages = fixture.root.join("packages");
    let hidden = packages.join("node_modules/.package-lock.json");
    let original = fs::read(&hidden).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let commander = r#"{"engines":{"node":">= 12"},"integrity":"sha512-OkTL9umf+He2DZkUq8f8J9of7yL6RJKI24dVITBmNfZBmri9zYZQrKkuXiKhyfPSu8tUhnVBB1iKXevvnlR4Ww==","license":"MIT","resolved":"https://registry.npmjs.org/commander/-/commander-8.3.0.tgz","version":"8.3.0"}"#;
    let mut changed = value.clone();
    changed["packages"]["node_modules/commander"]["version"] = "8.3.1".into();
    fs::write(&hidden, serde_json::to_vec(&changed).unwrap()).unwrap();
    let result = katex::invoke_with(&support, true, || {
        panic!("installed record must precede subprocess")
    });
    fs::write(&hidden, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec![],
            unexpected: vec![],
            changed: vec![InstalledPackageChange {
                package: "node_modules/commander".into(),
                expected: commander.into(),
                actual: commander.replace("\"version\":\"8.3.0\"", "\"version\":\"8.3.1\""),
            }],
        }),
    );
    support.check().unwrap();
    // Whitespace and object order are insignificant; only the root project is excluded.
    support::installed(&hidden, &serde_json::to_vec(&value).unwrap()).unwrap();
    let mut changed = value.clone();
    changed["packages"]
        .as_object_mut()
        .unwrap()
        .remove("node_modules/commander");
    changed["packages"]["extra"] = serde_json::json!({});
    changed["packages"][""] = serde_json::json!({});
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec!["node_modules/commander".into()],
            unexpected: vec![String::new(), "extra".into()],
            changed: vec![],
        },
    );
    let mut changed = value.clone();
    changed["packages"]["node_modules/commander"]["version"] = 8.into();
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::InstalledTreeMismatch {
            file: hidden.clone(),
            missing: vec![],
            unexpected: vec![],
            changed: vec![InstalledPackageChange {
                package: "node_modules/commander".into(),
                expected: commander.into(),
                actual: commander.replace("\"version\":\"8.3.0\"", "\"version\":8"),
            }],
        },
    );
    let mut changed = value.clone();
    changed["unknown"] = true.into();
    exact_integrity(
        support::installed(&hidden, &serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        SupportIntegrityFault::Metadata {
            file: hidden.clone(),
            path: "$".into(),
            reason: SupportMetadataReason::UnknownField {
                name: "unknown".into(),
            },
        },
    );
    fs::remove_file(&hidden).unwrap();
    let result = support.check();
    fs::write(&hidden, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::SupportIo {
            file: hidden,
            operation: SupportIoOperation::Inspect,
            kind: std::io::ErrorKind::NotFound,
        }),
    );
    support.check().unwrap();
    let missing_name = "node_modules/commander/index.js";
    let missing = packages.join(missing_name);
    let original = fs::read(&missing).unwrap();
    let extra_name = "node_modules/commander/unexpected.js";
    let extra = packages.join(extra_name);
    fs::remove_file(&missing).unwrap();
    fs::write(&extra, b"extra").unwrap();
    let result = support.check();
    fs::write(&missing, &original).unwrap();
    fs::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![missing_name.into()],
            unexpected: vec![extra_name.into()],
        }),
    );
    support.check().unwrap();
    fs::remove_file(&missing).unwrap();
    let result = support.check();
    fs::write(&missing, &original).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![missing_name.into()],
            unexpected: vec![],
        }),
    );
    support.check().unwrap();
    fs::write(&extra, b"extra").unwrap();
    let result = support.check();
    fs::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec![],
            unexpected: vec![extra_name.into()],
        }),
    );
    support.check().unwrap();
    symlink(&missing, &extra).unwrap();
    let result = support.check();
    fs::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::UnexpectedSupportEntry {
            file: extra.clone(),
            kind: SupportEntryKind::Symlink,
        }),
    );
    support.check().unwrap();
    let socket = std::os::unix::net::UnixListener::bind(&extra).unwrap();
    let result = support.check();
    drop(socket);
    fs::remove_file(&extra).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::UnexpectedSupportEntry {
            file: extra,
            kind: SupportEntryKind::Other,
        }),
    );
    support.check().unwrap();
    for (os, architecture) in [("linux", "aarch64"), ("macos", "x86_64")] {
        exact_integrity(
            support
                .check_inventory(support::DIGESTS, os, architecture)
                .unwrap_err(),
            SupportIntegrityFault::UnsupportedPlatform {
                os: os.into(),
                architecture: architecture.into(),
            },
        );
        support.check().unwrap();
    }
    let entry = packages.join("node_modules/katex/dist/katex.js");
    let bytes = fs::read(&entry).unwrap();
    fs::remove_file(&entry).unwrap();
    let result = katex::invoke_with(&support, true, || {
        panic!("a missing entry cannot fall back to another package")
    });
    fs::write(&entry, bytes).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::FileInventoryMismatch {
            directory: packages.clone(),
            missing: vec!["node_modules/katex/dist/katex.js".into()],
            unexpected: vec![],
        }),
    );
    support.check().unwrap();
    let mut archive_as_binary: serde_json::Value =
        serde_json::from_slice(support::DIGESTS).unwrap();
    archive_as_binary["runtimes"][0]["executable-sha256"] =
        "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e".into();
    exact_integrity(
        support
            .check_inventory(
                &serde_json::to_vec(&archive_as_binary).unwrap(),
                "linux",
                "x86_64",
            )
            .unwrap_err(),
        SupportIntegrityFault::DigestMismatch {
            file: support.runtime.clone(),
            expected: "f4cb75bb036f0d0eddf6b79d9596df1aaab9ddccd6a20bf489be5abe9467e84e".into(),
            actual: "8142d37c6f2f372ef040419e7a111a6baf17df89f8078d02447d6c639ae20c1d".into(),
        },
    );
    support.check().unwrap();
    let known = fixture.root.join("known-answer");
    fs::write(&known, b"abc").unwrap();
    assert_eq!(
        support::hash(&known).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    fs::write(&known, b"").unwrap();
    assert_eq!(
        support::hash(&known).unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let absent = fixture.root.join("absent");
    exact_adapter(
        support::resolve(Some(&absent)).unwrap_err(),
        NotationAdapterFault::Integrity(SupportIntegrityFault::SupportIo {
            file: absent,
            operation: SupportIoOperation::Canonicalize,
            kind: std::io::ErrorKind::NotFound,
        }),
    );
    let runtime = fixture.root.join("runtime/bin/node");
    let saved = fixture.root.join("saved-node");
    fs::rename(&runtime, &saved).unwrap();
    let escaped = prepared().runtime.canonicalize().unwrap();
    symlink(&escaped, &runtime).unwrap();
    let result = support::resolve(Some(&fixture.root));
    fs::remove_file(&runtime).unwrap();
    fs::rename(&saved, &runtime).unwrap();
    exact_adapter(
        result.unwrap_err(),
        NotationAdapterFault::SupportConfiguration {
            value: Some(fixture.root.clone()),
            reason: SupportReason::EscapingEntry {
                component: AdapterComponent::Runtime,
                resolved: escaped,
            },
        },
    );
    support.check().unwrap();
}

const ONE: &str =
    r"\underbrace{{c}_{1}}_{\text{pending: retained formulation def:langevin:coefficient-one}}";
const THREE: &str =
    r"\underbrace{{c}_{3}}_{\text{pending: retained formulation def:langevin:coefficient-three}}";
const ONE_REASON: &str = "retained formulation claim:langevin:coefficient-one-value; retained formulation def:langevin:coefficient-one; claim evidence claim:langevin:coefficient-one-value awaits audit-run-required";
const THREE_REASON: &str = "retained formulation claim:langevin:coefficient-three-value; retained formulation def:langevin:coefficient-three; claim evidence claim:langevin:coefficient-three-value awaits audit-run-required";
fn literal(s: &str) -> ExprDraft {
    ExprDraft::Literal { value: s.into() }
}
fn primitive(id: PrimitiveId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Primitive(id),
        arguments,
    }
}
fn definition(id: DefinitionId, arguments: Vec<ExprDraft>) -> ExprDraft {
    ExprDraft::Call {
        callee: Callee::Definition(id),
        arguments,
    }
}
fn pending(body: &str, reason: &str) -> String {
    format!("\\underbrace{{{body}}}_{{\\text{{pending: {reason}}}}}")
}
fn declaration_goldens() -> Vec<(DefinitionId, String)> {
    use DefinitionId::{
        CoefficientOne, CoefficientOneValue, CoefficientThree, CoefficientThreeValue, Dimension,
        FirstCoefficient, Product,
    };
    vec![
        (
            CoefficientOne,
            format!("\\newcommand{{\\FtooQCoefficientOne}}[0]{{{ONE}}}\n"),
        ),
        (
            CoefficientOneValue,
            format!(
                "\\newcommand{{\\FtooOCoefficientOneValue}}[0]{{{}}}\n",
                pending(r"({\FtooQCoefficientOne{}}={\frac{1}{3}})", ONE_REASON)
            ),
        ),
        (
            CoefficientThree,
            format!("\\newcommand{{\\FtooQCoefficientThree}}[0]{{{THREE}}}\n"),
        ),
        (
            CoefficientThreeValue,
            format!(
                "\\newcommand{{\\FtooOCoefficientThreeValue}}[0]{{{}}}\n",
                pending(
                    r"({\FtooQCoefficientThree{}}={(-{\frac{1}{45}})})",
                    THREE_REASON
                )
            ),
        ),
        (
            Dimension,
            "\\newcommand{\\FtooNDimension}[1]{[{#1}]}\n".into(),
        ),
        (
            FirstCoefficient,
            format!(
                "\\newcommand{{\\FtooAFirstCoefficient}}[0]{{{}}}\n",
                pending(
                    r"\FtooQCoefficientOne{}",
                    "retained formulation def:langevin:coefficient-one"
                )
            ),
        ),
        (
            Product,
            "\\newcommand{\\FtooNProduct}[2]{({#1}\\cdot{}{#2})}\n".into(),
        ),
    ]
}

#[test]
fn declarations_claims_aliases_and_nested_grouping_goldens() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    assert_eq!(
        records
            .iter()
            .map(|r| (r.id, r.source.clone()))
            .collect::<Vec<_>>(),
        declaration_goldens()
    );
    for (expr, macro_bytes, expanded) in [
        (
            definition(DefinitionId::CoefficientOneValue, vec![]),
            r"\FtooOCoefficientOneValue{}",
            pending(&format!("({{{ONE}}}={{\\frac{{1}}{{3}}}})"), ONE_REASON),
        ),
        (
            definition(DefinitionId::CoefficientThreeValue, vec![]),
            r"\FtooOCoefficientThreeValue{}",
            pending(
                &format!("({{{THREE}}}={{(−{{\\frac{{1}}{{45}}}})}})"),
                THREE_REASON,
            ),
        ),
        (
            definition(DefinitionId::FirstCoefficient, vec![]),
            r"\FtooAFirstCoefficient{}",
            pending(ONE, "retained formulation def:langevin:coefficient-one"),
        ),
        (
            definition(
                DefinitionId::Product,
                vec![
                    definition(DefinitionId::Dimension, vec![literal("x")]),
                    definition(DefinitionId::Product, vec![literal("y"), literal("z")]),
                ],
            ),
            r"\FtooNProduct{\FtooNDimension{x}}{\FtooNProduct{y}{z}}",
            "({[{x}]}⋅{({y}⋅{z})})".into(),
        ),
        (
            primitive(
                PrimitiveId::Superscript,
                vec![
                    primitive(PrimitiveId::Subscript, vec![literal("x"), literal("1")]),
                    primitive(PrimitiveId::Superscript, vec![literal("y"), literal("2")]),
                ],
            ),
            r"{{x}_{1}}^{{y}^{2}}",
            "{{x}_{1}}^{{y}^{2}}".into(),
        ),
        (
            definition(
                DefinitionId::Dimension,
                vec![primitive(
                    PrimitiveId::Add,
                    vec![literal("x"), literal("1")],
                )],
            ),
            r"\FtooNDimension{({x}+{1})}",
            "[{({x}+{1})}]".into(),
        ),
    ] {
        let checked = ftoo_manifest::check_expression(&n, expr).unwrap();
        for (target, expected) in [
            (RenderTarget::Macro, macro_bytes),
            (RenderTarget::Unicode, expanded.as_str()),
        ] {
            assert_eq!(
                render::expression(
                    &n,
                    checked.draft(),
                    target,
                    RenderOwner::Expression,
                    "golden"
                )
                .unwrap()
                .source,
                expected
            );
        }
    }
}

fn exact_admission(
    actual: NotationRenderError,
    rule: RuleId,
    stage: NotationStage,
    site: NotationSite,
    fault: NotationFault,
) {
    let text = format!("{rule:?} {stage:?} at {site:?}: {fault:?}\n");
    let expected = NotationRenderError::Admission(
        NotationErrors::from_diagnostics(vec![NotationDiagnostic {
            rule,
            stage,
            site,
            fault,
        }])
        .unwrap(),
    );
    assert_eq!(actual.to_string(), text);
    let NotationRenderError::Admission(errors) = actual else {
        panic!("expected numbered admission refusal");
    };
    assert_eq!(NotationRenderError::Admission(errors), expected);
}
#[test]
fn sweep_inventory_mutations_are_exact_and_restore() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let original = render::declarations(&n).unwrap();
    let catalogue = NotationSite::Catalogue {
        file: "crates/ftoo_manifest/src/notation/catalogue.rs".into(),
    };
    let mut records = original.clone();
    records.retain(|r| r.id != DefinitionId::CoefficientThree);
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        catalogue.clone(),
        NotationFault::SweepInventory {
            missing: vec![DefinitionId::CoefficientThree],
            repeated: vec![],
        },
    );
    sweep::inventory(&n, &original).unwrap();
    for count in [2, 3] {
        let mut records = original.clone();
        for _ in 1..count {
            records.push(original[0].clone());
        }
        exact_admission(
            sweep::inventory(&n, &records).unwrap_err(),
            RuleId::M052,
            NotationStage::Names,
            catalogue.clone(),
            NotationFault::SweepInventory {
                missing: vec![],
                repeated: vec![(DefinitionId::CoefficientOne, count)],
            },
        );
        sweep::inventory(&n, &original).unwrap();
    }
    let mut records = original.clone();
    records.retain(|r| r.id != DefinitionId::CoefficientThree);
    records.push(original[0].clone());
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        catalogue,
        NotationFault::SweepInventory {
            missing: vec![DefinitionId::CoefficientThree],
            repeated: vec![(DefinitionId::CoefficientOne, 2)],
        },
    );
    sweep::inventory(&n, &original).unwrap();
    let site = NotationSite::Rendered {
        owner: RenderOwner::Definition(DefinitionId::CoefficientOne),
        case: "declaration".into(),
        bytes: 0..declaration_goldens()[0].1.len(),
        origins: vec![NotationOrigin {
            definition: Some(DefinitionId::CoefficientOne),
            path: vec![],
        }],
    };
    let mut records = original.clone();
    records[0].command = "\\Wrong".into();
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        site.clone(),
        NotationFault::SweepCommand {
            definition: DefinitionId::CoefficientOne,
            expected: "\\FtooQCoefficientOne".into(),
            actual: "\\Wrong".into(),
        },
    );
    sweep::inventory(&n, &original).unwrap();
    let mut records = original.clone();
    records[0].arity = 1;
    exact_admission(
        sweep::inventory(&n, &records).unwrap_err(),
        RuleId::M052,
        NotationStage::Names,
        site,
        NotationFault::SweepSignature {
            definition: DefinitionId::CoefficientOne,
            expected: 0,
            actual: 1,
        },
    );
    sweep::inventory(&n, &original).unwrap();
}

#[test]
fn incomplete_and_output_boundaries_have_exact_sites() {
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    for (source, remaining) in [
        (r"\FtooQCoefficientOne{}", r"\FtooQCoefficientOne"),
        ("#1", "#1"),
    ] {
        let candidate = render::Candidate {
            source: source.into(),
            origins: vec![origin.clone()],
        };
        exact_admission(
            render::complete(&candidate, RenderOwner::Expression, "mutation").unwrap_err(),
            RuleId::M051,
            NotationStage::Names,
            NotationSite::Rendered {
                owner: RenderOwner::Expression,
                case: "mutation".into(),
                bytes: 0..source.len(),
                origins: vec![origin.clone()],
            },
            NotationFault::IncompleteExpansion {
                remaining: remaining.into(),
                origin: origin.clone(),
            },
        );
    }
    let candidate = render::Candidate {
        source: "α".into(),
        origins: vec![origin.clone()],
    };
    render::complete(&candidate, RenderOwner::Expression, "mutation").unwrap();
    let prefix = "0".repeat(262_143);
    let site = NotationSite::Rendered {
        owner: RenderOwner::Expression,
        case: "output".into(),
        bytes: 0..2,
        origins: vec![origin],
    };
    assert_eq!(
        render::combine(&prefix, "1", site.clone()).unwrap().len(),
        262_144
    );
    exact_admission(
        render::combine(&prefix, "12", site.clone()).unwrap_err(),
        RuleId::M051,
        NotationStage::Names,
        site.clone(),
        NotationFault::NotationLimit {
            resource: NotationResource::OutputBytes,
            limit: 262_144,
            actual: 262_145,
        },
    );
    assert_eq!(render::combine(&prefix, "1", site).unwrap().len(), 262_144);
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    let mut candidate = render::Candidate {
        source: prefix.clone(),
        origins: vec![],
    };
    exact_admission(
        render::append(
            &mut candidate,
            "α",
            vec![origin.clone()],
            RenderOwner::Expression,
            "writer-limit",
        )
        .unwrap_err(),
        RuleId::M051,
        NotationStage::Names,
        NotationSite::Rendered {
            owner: RenderOwner::Expression,
            case: "writer-limit".into(),
            bytes: 262_143..262_145,
            origins: vec![origin.clone()],
        },
        NotationFault::NotationLimit {
            resource: NotationResource::OutputBytes,
            limit: 262_144,
            actual: 262_145,
        },
    );
    assert_eq!(candidate.source, prefix);
    candidate.source.pop();
    render::append(
        &mut candidate,
        "α",
        vec![origin],
        RenderOwner::Expression,
        "writer-limit",
    )
    .unwrap();
    assert_eq!(candidate.source.len(), 262_144);
}

// This oracle has no descriptor, declaration or writer reads. Its closed input
// inventory and exact strings deliberately duplicate the reviewed specification.
const PRIMITIVES: [(PrimitiveId, &str, usize); 12] = [
    (PrimitiveId::Alpha, "alpha", 0),
    (PrimitiveId::Beta, "beta", 0),
    (PrimitiveId::Fraction, "fraction", 2),
    (PrimitiveId::Subscript, "subscript", 2),
    (PrimitiveId::Superscript, "superscript", 2),
    (PrimitiveId::Add, "add", 2),
    (PrimitiveId::Equal, "equal", 2),
    (PrimitiveId::Negate, "negate", 1),
    (PrimitiveId::Multiply, "multiply", 2),
    (PrimitiveId::Juxtapose, "juxtapose", 2),
    (PrimitiveId::Parentheses, "parentheses", 1),
    (PrimitiveId::Dimension, "dimension", 1),
];
const DEFINITIONS: [(DefinitionId, &str, usize); 7] = [
    (DefinitionId::CoefficientOne, "coefficient-one", 0),
    (DefinitionId::CoefficientThree, "coefficient-three", 0),
    (
        DefinitionId::CoefficientOneValue,
        "coefficient-one-value",
        0,
    ),
    (
        DefinitionId::CoefficientThreeValue,
        "coefficient-three-value",
        0,
    ),
    (DefinitionId::FirstCoefficient, "first-coefficient", 0),
    (DefinitionId::Dimension, "dimension", 1),
    (DefinitionId::Product, "product", 2),
];
fn oracle(expr: &ExprDraft, unicode: bool) -> String {
    let ExprDraft::Call { callee, arguments } = expr else {
        return match expr {
            ExprDraft::Literal { value } => value.clone(),
            _ => panic!("oracle consumes closed ground calls"),
        };
    };
    let a: Vec<_> = arguments.iter().map(|e| oracle(e, unicode)).collect();
    match callee {
        Callee::Primitive(id) => match id {
            PrimitiveId::Alpha => if unicode { "α" } else { r"\alpha{}" }.into(),
            PrimitiveId::Beta => if unicode { "β" } else { r"\beta{}" }.into(),
            PrimitiveId::Fraction => format!("\\frac{{{}}}{{{}}}", a[0], a[1]),
            PrimitiveId::Subscript => format!("{{{}}}_{{{}}}", a[0], a[1]),
            PrimitiveId::Superscript => format!("{{{}}}^{{{}}}", a[0], a[1]),
            PrimitiveId::Add => format!("({{{}}}+{{{}}})", a[0], a[1]),
            PrimitiveId::Equal => format!("({{{}}}={{{}}})", a[0], a[1]),
            PrimitiveId::Negate => format!("({}{{{}}})", if unicode { "−" } else { "-" }, a[0]),
            PrimitiveId::Multiply => format!(
                "({{{}}}{}{{{}}})",
                a[0],
                if unicode { "⋅" } else { r"\cdot{}" },
                a[1]
            ),
            PrimitiveId::Juxtapose => format!("{{{}}}\\,{{{}}}", a[0], a[1]),
            PrimitiveId::Parentheses => format!("({{{}}})", a[0]),
            PrimitiveId::Dimension => format!("[{{{}}}]", a[0]),
        },
        Callee::Definition(id) if !unicode => {
            let command = match id {
                DefinitionId::CoefficientOne => r"\FtooQCoefficientOne",
                DefinitionId::CoefficientThree => r"\FtooQCoefficientThree",
                DefinitionId::CoefficientOneValue => r"\FtooOCoefficientOneValue",
                DefinitionId::CoefficientThreeValue => r"\FtooOCoefficientThreeValue",
                DefinitionId::FirstCoefficient => r"\FtooAFirstCoefficient",
                DefinitionId::Dimension => r"\FtooNDimension",
                DefinitionId::Product => r"\FtooNProduct",
            };
            format!(
                "{command}{}",
                if a.is_empty() {
                    "{}".into()
                } else {
                    a.iter().fold(String::new(), |mut out, s| {
                        write!(out, "{{{s}}}").unwrap();
                        out
                    })
                }
            )
        }
        Callee::Definition(id) => match id {
            DefinitionId::CoefficientOne => ONE.into(),
            DefinitionId::CoefficientThree => THREE.into(),
            DefinitionId::CoefficientOneValue => {
                pending(&format!("({{{ONE}}}={{\\frac{{1}}{{3}}}})"), ONE_REASON)
            }
            DefinitionId::CoefficientThreeValue => pending(
                &format!("({{{THREE}}}={{(−{{\\frac{{1}}{{45}}}})}})"),
                THREE_REASON,
            ),
            DefinitionId::FirstCoefficient => {
                pending(ONE, "retained formulation def:langevin:coefficient-one")
            }
            DefinitionId::Dimension => format!("[{{{}}}]", a[0]),
            DefinitionId::Product => format!("({{{}}}⋅{{{}}})", a[0], a[1]),
        },
    }
}

#[test]
fn complete_row_key_invocation_and_both_form_inventory() {
    use std::collections::BTreeMap;
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let ground = |arity: usize| {
        ["x", "y"][..arity]
            .iter()
            .map(|s| literal(s))
            .collect::<Vec<_>>()
    };
    let mut expected = BTreeMap::new();
    for (id, token, arity) in DEFINITIONS {
        expected.insert((token, "base".to_string()), definition(id, ground(arity)));
        if arity > 0 {
            expected.insert(
                (token, "compound".into()),
                definition(
                    id,
                    ground(arity)
                        .into_iter()
                        .map(|e| primitive(PrimitiveId::Add, vec![e, literal("1")]))
                        .collect(),
                ),
            );
            for index in 0..arity {
                for (name, arg) in [
                    ("alpha", primitive(PrimitiveId::Alpha, vec![])),
                    ("pending", definition(DefinitionId::CoefficientOne, vec![])),
                    (
                        "nested-dimension",
                        definition(DefinitionId::Dimension, vec![literal("x")]),
                    ),
                    (
                        "nested-product",
                        definition(DefinitionId::Product, vec![literal("x"), literal("y")]),
                    ),
                ] {
                    let mut args = ground(arity);
                    args[index] = arg;
                    expected.insert(
                        (token, format!("slot-{index}-{name}")),
                        definition(id, args),
                    );
                }
            }
        }
    }
    // Owners distinguish the primitive dimension from its project constructor.
    let mut expected: BTreeMap<_, _> = expected
        .into_iter()
        .map(|((token, case), expr)| ((format!("definition:{token}"), case), expr))
        .collect();
    for (id, token, arity) in PRIMITIVES {
        expected.insert(
            (format!("primitive:{token}"), "base".into()),
            primitive(id, ground(arity)),
        );
        for index in 0..arity {
            for (inner, name, inner_arity) in PRIMITIVES {
                let mut args = ground(arity);
                args[index] = primitive(inner, ground(inner_arity));
                expected.insert(
                    (
                        format!("primitive:{token}"),
                        format!("argument-{index}-primitive-{name}"),
                    ),
                    primitive(id, args),
                );
            }
        }
    }
    let actual: BTreeMap<_, _> = sweep::cases(&n)
        .into_iter()
        .map(|(owner, case, expr)| {
            let name = match owner {
                RenderOwner::Definition(id) => format!("definition:{}", id.token()),
                RenderOwner::Primitive(id) => format!("primitive:{}", id.token()),
                RenderOwner::Expression => unreachable!(),
            };
            ((name, case), expr)
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 237);
    for (owner, case, expr) in sweep::cases(&n) {
        let checked = ftoo_manifest::check_expression(&n, expr).unwrap();
        for target in [RenderTarget::Macro, RenderTarget::Unicode] {
            assert_eq!(
                render::expression(&n, checked.draft(), target, owner, &case)
                    .unwrap()
                    .source,
                oracle(checked.draft(), target == RenderTarget::Unicode),
                "{owner:?}/{case}/{target:?}"
            );
        }
    }
}

#[test]
fn nullary_boundaries_operators_and_nested_contexts() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let mut operands = vec![
        primitive(PrimitiveId::Alpha, vec![]),
        primitive(PrimitiveId::Beta, vec![]),
    ];
    operands.extend(
        DEFINITIONS
            .into_iter()
            .filter(|(_, _, arity)| *arity == 0)
            .map(|(id, _, _)| definition(id, vec![])),
    );
    for operand in operands {
        for suffix in [
            literal("x"),
            literal("1"),
            primitive(PrimitiveId::Alpha, vec![]),
            primitive(PrimitiveId::Superscript, vec![literal("x"), literal("2")]),
            primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
        ] {
            let expr = primitive(PrimitiveId::Juxtapose, vec![operand.clone(), suffix]);
            for target in [RenderTarget::Macro, RenderTarget::Unicode] {
                assert_eq!(
                    render::expression(&n, &expr, target, RenderOwner::Expression, "boundary")
                        .unwrap()
                        .source,
                    oracle(&expr, target == RenderTarget::Unicode)
                );
            }
        }
    }
    for id in [PrimitiveId::Add, PrimitiveId::Multiply, PrimitiveId::Equal] {
        for args in [
            vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            vec![literal("x"), primitive(PrimitiveId::Beta, vec![])],
        ] {
            let expr = primitive(id, args);
            for target in [RenderTarget::Macro, RenderTarget::Unicode] {
                assert_eq!(
                    render::expression(&n, &expr, target, RenderOwner::Expression, "operator")
                        .unwrap()
                        .source,
                    oracle(&expr, target == RenderTarget::Unicode)
                );
            }
        }
    }
    let chars: Vec<_> = r"\alpha{}#1".chars().collect();
    assert_eq!(chars, ['\\', 'a', 'l', 'p', 'h', 'a', '{', '}', '#', '1']);
}

#[test]
fn substitution_repeats_formals_without_capturing_nested_arguments() {
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let body = primitive(
        PrimitiveId::Add,
        vec![ExprDraft::Slot { index: 0 }, ExprDraft::Slot { index: 0 }],
    );
    let args = vec![definition(
        DefinitionId::Product,
        vec![literal("x"), literal("y")],
    )];
    assert_eq!(
        render::fixture_body(&n, &body, &args).unwrap().source,
        "({({x}⋅{y})}+{({x}⋅{y})})"
    );
    let body = definition(
        DefinitionId::Product,
        vec![ExprDraft::Slot { index: 1 }, ExprDraft::Slot { index: 0 }],
    );
    let args = vec![
        definition(DefinitionId::Dimension, vec![literal("x")]),
        definition(DefinitionId::Product, vec![literal("y"), literal("z")]),
    ];
    assert_eq!(
        render::fixture_body(&n, &body, &args).unwrap().source,
        "({({y}⋅{z})}⋅{[{x}]})"
    );
}

#[test]
fn retained_quantity_examples_and_adversarial_atom_concatenation() {
    let mut draft = ftoo_manifest::seed_draft();
    for node in &mut draft.nodes {
        node.maturity = ftoo_manifest::Maturity::Retained;
    }
    let graph = ftoo_manifest::validate(draft).unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    for (id, source) in [
        (
            DefinitionId::CoefficientOne,
            "\\newcommand{\\FtooQCoefficientOne}[0]{{c}_{1}}\n",
        ),
        (
            DefinitionId::FirstCoefficient,
            "\\newcommand{\\FtooAFirstCoefficient}[0]{\\FtooQCoefficientOne{}}\n",
        ),
        (
            DefinitionId::Dimension,
            "\\newcommand{\\FtooNDimension}[1]{[{#1}]}\n",
        ),
        (
            DefinitionId::Product,
            "\\newcommand{\\FtooNProduct}[2]{({#1}\\cdot{}{#2})}\n",
        ),
    ] {
        assert_eq!(records.iter().find(|r| r.id == id).unwrap().source, source);
    }
    for (expr, macro_bytes, unicode) in [
        (
            definition(DefinitionId::CoefficientOne, vec![]),
            r"\FtooQCoefficientOne{}",
            "{c}_{1}",
        ),
        (
            definition(DefinitionId::FirstCoefficient, vec![]),
            r"\FtooAFirstCoefficient{}",
            "{c}_{1}",
        ),
        (
            definition(
                DefinitionId::Dimension,
                vec![definition(DefinitionId::CoefficientOne, vec![])],
            ),
            r"\FtooNDimension{\FtooQCoefficientOne{}}",
            "[{{c}_{1}}]",
        ),
        (
            definition(
                DefinitionId::Product,
                vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            ),
            r"\FtooNProduct{\alpha{}}{x}",
            "({α}⋅{x})",
        ),
        (
            primitive(
                PrimitiveId::Juxtapose,
                vec![primitive(PrimitiveId::Alpha, vec![]), literal("x")],
            ),
            r"{\alpha{}}\,{x}",
            r"{α}\,{x}",
        ),
        (
            primitive(
                PrimitiveId::Equal,
                vec![
                    definition(DefinitionId::CoefficientOne, vec![]),
                    primitive(PrimitiveId::Fraction, vec![literal("1"), literal("3")]),
                ],
            ),
            r"({\FtooQCoefficientOne{}}={\frac{1}{3}})",
            r"({{c}_{1}}={\frac{1}{3}})",
        ),
        (
            primitive(
                PrimitiveId::Subscript,
                vec![
                    primitive(PrimitiveId::Add, vec![literal("x"), literal("y")]),
                    literal("1"),
                ],
            ),
            "{({x}+{y})}_{1}",
            "{({x}+{y})}_{1}",
        ),
        (
            primitive(
                PrimitiveId::Negate,
                vec![primitive(
                    PrimitiveId::Fraction,
                    vec![literal("1"), literal("45")],
                )],
            ),
            r"(-{\frac{1}{45}})",
            r"(−{\frac{1}{45}})",
        ),
    ] {
        for (target, expected) in [
            (RenderTarget::Macro, macro_bytes),
            (RenderTarget::Unicode, unicode),
        ] {
            assert_eq!(
                render::expression(&n, &expr, target, RenderOwner::Expression, "example")
                    .unwrap()
                    .source,
                expected
            );
        }
    }
    let support = prepared();
    let seed = ftoo_manifest::seed().unwrap();
    let seed_book = ftoo_manifest::addressing(&seed).unwrap();
    let n = ftoo_manifest::notation(&seed_book).unwrap();
    let declarations = render::declarations(&n)
        .unwrap()
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let mut atoms = vec![
        primitive(PrimitiveId::Alpha, vec![]),
        primitive(PrimitiveId::Beta, vec![]),
    ];
    atoms.extend(
        DEFINITIONS
            .into_iter()
            .filter(|(_, _, a)| *a == 0)
            .map(|(id, _, _)| definition(id, vec![])),
    );
    for target in [RenderTarget::Macro, RenderTarget::Unicode] {
        let mut concatenations = vec![];
        for atom in &atoms {
            for suffix in ["x", "1", r"\beta{}", "^2", "{x}"] {
                let actual = render::expression(&n, atom, target, RenderOwner::Expression, "atom")
                    .unwrap()
                    .source;
                let joined = format!("{actual}{suffix}");
                assert_eq!(
                    joined,
                    format!("{}{suffix}", oracle(atom, target == RenderTarget::Unicode))
                );
                concatenations.push(format!("{{{joined}}}"));
            }
        }
        let source = concatenations.join(r"\,");
        let source = if target == RenderTarget::Macro {
            format!("{declarations}{source}")
        } else {
            source
        };
        typeset(
            &support,
            target,
            &source,
            rendered_site(
                RenderOwner::Expression,
                "concatenated-atoms",
                &source,
                vec![],
            ),
        )
        .unwrap();
    }
}

#[test]
fn pending_annotations_use_the_exported_reason_text_in_both_targets() {
    use ftoo_manifest::{Awaiting, CapabilityId, ClaimAudit, Maturity, NodeId, NodePayload};
    for (awaits, text) in [
        (Awaiting::AuditBinding, "audit-binding"),
        (
            Awaiting::Capability(CapabilityId::ExactRationalLinearEquivalence),
            "capability exact-rational-linear-equivalence",
        ),
        (
            Awaiting::Capability(CapabilityId::SymbolicTrigonometry),
            "capability symbolic-trigonometry",
        ),
        (
            Awaiting::Capability(CapabilityId::TrigonometricIntegration),
            "capability trigonometric-integration",
        ),
    ] {
        let mut draft = ftoo_manifest::seed_draft();
        draft.coverage.slice.clear();
        draft.coverage.closed_world.audit = false;
        draft
            .budgets
            .retain(|b| b.producer.descriptor().claim != NodeId::LangevinCoefficientOneValue);
        for node in &mut draft.nodes {
            node.maturity = Maturity::Retained;
            if node.id == NodeId::LangevinCoefficientOneValue
                && let NodePayload::Claim { audit, .. } = &mut node.payload
            {
                *audit = ClaimAudit::Awaiting(awaits);
            }
        }
        let graph = ftoo_manifest::validate(draft).unwrap();
        let book = ftoo_manifest::addressing(&graph).unwrap();
        let n = ftoo_manifest::notation(&book).unwrap();
        let reason = format!("claim evidence claim:langevin:coefficient-one-value awaits {text}");
        let source = render::expression(
            &n,
            &definition(DefinitionId::CoefficientOneValue, vec![]),
            RenderTarget::Unicode,
            RenderOwner::Expression,
            "reason",
        )
        .unwrap()
        .source;
        assert_eq!(source, pending(r"({{c}_{1}}={\frac{1}{3}})", &reason));
        let declarations = render::declarations(&n).unwrap();
        assert_eq!(
            declarations
                .iter()
                .find(|d| d.id == DefinitionId::CoefficientOneValue)
                .unwrap()
                .source,
            format!(
                "\\newcommand{{\\FtooOCoefficientOneValue}}[0]{{{}}}\n",
                pending(r"({\FtooQCoefficientOne{}}={\frac{1}{3}})", &reason)
            )
        );
        assert_eq!(
            n.definition(DefinitionId::CoefficientOneValue)
                .status()
                .unwrap()
                .reasons()[0]
                .to_string(),
            reason
        );
    }
}

#[test]
fn external_markdown_and_links_do_not_change_either_source_inventory() {
    use sha2::{Digest, Sha256};
    use std::process::Command;
    let fixture = Fixture::new();
    let root = support::repository();
    let python = || {
        let result = Command::new("python3").arg("-I").arg("-c").arg("import pathlib,json; print(json.dumps([(p.as_posix(),p.read_text(encoding='utf-8')) for p in sorted(pathlib.Path('.').rglob('*.md'))]))").current_dir(&root).output().unwrap();
        assert_eq!(result.status.code(), Some(0));
        assert_eq!(result.stderr, b"");
        result.stdout
    };
    let before = crate::load_markdown_files(&root).unwrap();
    let py_before = python();
    let external = fixture.root.join("external.md");
    fs::write(&external, b"unrelated external prose\n").unwrap();
    symlink(&external, fixture.root.join("external-link.md")).unwrap();
    let after = crate::load_markdown_files(&root).unwrap();
    assert_eq!(after.sources(), before.sources());
    assert_eq!(python(), py_before);
    let expected: Vec<(String, String)> = before
        .sources()
        .iter()
        .map(|s| (s.file.clone(), String::from_utf8(s.bytes.clone()).unwrap()))
        .collect();
    assert_eq!(
        serde_json::from_slice::<Vec<(String, String)>>(&py_before).unwrap(),
        expected
    );
    let mut digest = Sha256::new();
    for source in before.sources() {
        digest.update(source.file.as_bytes());
        digest.update([0]);
        digest.update(Sha256::digest(&source.bytes));
    }
    eprintln!(
        "RUST_SOURCE_INVENTORY {} {:x}",
        before.sources().len(),
        digest.finalize()
    );
}

#[test]
fn committed_adapter_and_ci_pins_are_fixed() {
    assert_eq!(support::NODE_PIN.as_bytes(), b"22.16.0\n");
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    assert!(ci.contains("actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020"));
    assert!(ci.contains("node-version-file: tools/notation/.node-version"));
    assert!(ci.contains("npm ci --ignore-scripts"));
    let script = include_str!("../../../../tools/notation/check.cjs");
    assert!(script.contains("require(entry)"));
    assert!(script.contains("throwOnError: true, strict: 'error', trust: false"));
    assert!(
        script.contains("output: 'htmlAndMathml', displayMode: false, maxExpand: 1000, macros: {}")
    );
}

#[test]
fn actual_grammar_refusals_and_host_collision_restore() {
    let support = prepared();
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let records = render::declarations(&n).unwrap();
    let origin = NotationOrigin {
        definition: None,
        path: vec![],
    };
    for (source, message) in [
        (
            r"\NotDefined",
            "KaTeX parse error: Undefined control sequence: \\NotDefined at position 1: \\\u{332}N\u{332}o\u{332}t\u{332}D\u{332}e\u{332}f\u{332}i\u{332}n\u{332}e\u{332}d\u{332}",
        ),
        (
            "{x",
            "KaTeX parse error: Expected '}', got 'EOF' at end of input: {x",
        ),
        (
            "x^1^2",
            "KaTeX parse error: Double superscript at position 4: x^1^\u{332}2",
        ),
        (
            "☃",
            "KaTeX parse error: LaTeX-incompatible input and strict mode is set to 'error': Unrecognized Unicode character \"☃\" (9731) [unknownSymbol] at position 1: ☃\u{332}",
        ),
        (
            r"\alphax",
            "KaTeX parse error: Undefined control sequence: \\alphax at position 1: \\\u{332}a\u{332}l\u{332}p\u{332}h\u{332}a\u{332}x\u{332}",
        ),
    ] {
        let target = if source == r"\alphax" {
            RenderTarget::Macro
        } else {
            RenderTarget::Unicode
        };
        let alpha = primitive(PrimitiveId::Alpha, vec![]);
        let mut emitted =
            render::expression(&n, &alpha, target, RenderOwner::Expression, "grammar").unwrap();
        let restored = if target == RenderTarget::Macro {
            assert_eq!(emitted.source, r"\alpha{}");
            let adjacent = format!("{}x", emitted.source);
            assert_eq!(adjacent, r"\alpha{}x");
            assert_eq!(adjacent.replace("{}", ""), source);
            adjacent
        } else {
            assert_eq!(emitted.source, "α");
            "α".into()
        };
        emitted.source = source.into();
        let site = NotationSite::Rendered {
            owner: RenderOwner::Expression,
            case: "grammar".into(),
            bytes: 0..source.len(),
            origins: vec![origin.clone()],
        };
        assert_eq!(emitted.site(RenderOwner::Expression, "grammar"), site);
        exact_admission(
            typeset(&support, target, &emitted.source, site.clone()).unwrap_err(),
            RuleId::M051,
            NotationStage::Typesetting,
            site,
            NotationFault::KaTeXRejected {
                target,
                source: source.into(),
                message: message.into(),
            },
        );
        emitted.source = restored;
        typeset(
            &support,
            target,
            &emitted.source,
            emitted.site(RenderOwner::Expression, "restored"),
        )
        .unwrap();
    }
    let context = r"\newcommand{\FtooQCoefficientOne}[0]{x}";
    exact_admission(sweep::namespace(&support, &records, context).unwrap_err(),
        RuleId::M050, NotationStage::Typesetting,
        NotationSite::Rendered { owner: RenderOwner::Definition(DefinitionId::CoefficientOne), case: "declaration".into(), bytes: 39..166, origins: vec![NotationOrigin { definition: Some(DefinitionId::CoefficientOne), path: vec![] }] },
        NotationFault::HostCollision { command: r"\FtooQCoefficientOne".into(), message: r"KaTeX parse error: \newcommand{\FtooQCoefficientOne} attempting to redefine \FtooQCoefficientOne; use \renewcommand".into() });
    sweep::namespace(&support, &records, "").unwrap();
    let declaration = "\\newcommand{\\FtooQCoefficientOne}[0]{\\NotDefined}\n";
    assert!(
        katex::invoke(&support, Some(declaration))
            .unwrap()
            .unwrap()
            .contains("<math")
    );
    let mut mutated = records.clone();
    mutated[0].source = declaration.into();
    sweep::inventory(&n, &mutated).unwrap();
    let bundle = mutated
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let mut expected_declarations = declaration_goldens();
    expected_declarations[0].1 = declaration.into();
    let expected_bundle = expected_declarations
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<String>();
    assert_eq!(bundle, expected_bundle);
    let full = format!("{expected_bundle}\\FtooQCoefficientOne{{}}");
    let site = NotationSite::Rendered {
        owner: RenderOwner::Definition(DefinitionId::CoefficientOne),
        case: "base".into(),
        bytes: 0..full.len(),
        origins: vec![
            NotationOrigin {
                definition: None,
                path: vec![],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![ftoo_manifest::NotationPathPart::Field("body".into())],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![
                    ftoo_manifest::NotationPathPart::Field("body".into()),
                    ftoo_manifest::NotationPathPart::Field("arguments".into()),
                    ftoo_manifest::NotationPathPart::Argument(0),
                ],
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![
                    ftoo_manifest::NotationPathPart::Field("body".into()),
                    ftoo_manifest::NotationPathPart::Field("arguments".into()),
                    ftoo_manifest::NotationPathPart::Argument(1),
                ],
            },
        ],
    };
    exact_admission(sweep::row(&n, &support, &bundle, RenderOwner::Definition(DefinitionId::CoefficientOne), "base", definition(DefinitionId::CoefficientOne, vec![])).unwrap_err(),
        RuleId::M051, NotationStage::Typesetting, site,
        NotationFault::KaTeXRejected { target: RenderTarget::Macro, source: full,
            message: "KaTeX parse error: Undefined control sequence: \\NotDefined at position 38: …ficientOne}[0]{\\\u{332}N\u{332}o\u{332}t\u{332}D\u{332}e\u{332}f\u{332}i\u{332}n\u{332}e\u{332}d\u{332}}\n\\newcommand{\\…".into() });
    mutated[0] = records[0].clone();
    sweep::inventory(&n, &mutated).unwrap();
    let bundle = mutated
        .iter()
        .map(|r| r.source.as_str())
        .collect::<String>();
    let restored_row = sweep::row(
        &n,
        &support,
        &bundle,
        RenderOwner::Definition(DefinitionId::CoefficientOne),
        "base",
        definition(DefinitionId::CoefficientOne, vec![]),
    )
    .unwrap();
    assert_eq!(restored_row.macro_source(), r"\FtooQCoefficientOne{}");
    assert_eq!(restored_row.unicode_source(), ONE);
    let restored = format!(
        "{}\\FtooQCoefficientOne{{}}\\alpha{{}}",
        declaration_goldens()[0].1
    );
    typeset(
        &support,
        RenderTarget::Macro,
        &restored,
        rendered_site(
            RenderOwner::Definition(DefinitionId::CoefficientOne),
            "restored",
            &restored,
            vec![],
        ),
    )
    .unwrap();
}

#[test]
fn expanded_origins_retain_constructor_argument_and_definition_paths() {
    use ftoo_manifest::NotationPathPart::{Argument, Field};
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    let expr = definition(
        DefinitionId::Dimension,
        vec![definition(DefinitionId::CoefficientOne, vec![])],
    );
    let expanded = render::expression(
        &n,
        &expr,
        RenderTarget::Unicode,
        RenderOwner::Expression,
        "origins",
    )
    .unwrap();
    assert_eq!(
        expanded.origins,
        [
            NotationOrigin {
                definition: None,
                path: vec![]
            },
            NotationOrigin {
                definition: Some(DefinitionId::Dimension),
                path: vec![Field("body".into())]
            },
            NotationOrigin {
                definition: Some(DefinitionId::Dimension),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: None,
                path: vec![Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into())]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(0)]
            },
            NotationOrigin {
                definition: Some(DefinitionId::CoefficientOne),
                path: vec![Field("body".into()), Field("arguments".into()), Argument(1)]
            },
        ]
    );
}

#[test]
fn primitive_goldens_are_independent() {
    use PrimitiveId::{
        Add, Alpha, Beta, Dimension, Equal, Fraction, Juxtapose, Multiply, Negate, Parentheses,
        Subscript, Superscript,
    };
    let graph = ftoo_manifest::seed().unwrap();
    let book = ftoo_manifest::addressing(&graph).unwrap();
    let n = ftoo_manifest::notation(&book).unwrap();
    for (id, args, macro_source, unicode) in [
        (Alpha, vec![], "\\alpha{}", "α"),
        (Beta, vec![], "\\beta{}", "β"),
        (Fraction, vec!["x", "y"], "\\frac{x}{y}", "\\frac{x}{y}"),
        (Subscript, vec!["x", "y"], "{x}_{y}", "{x}_{y}"),
        (Superscript, vec!["x", "y"], "{x}^{y}", "{x}^{y}"),
        (Add, vec!["x", "y"], "({x}+{y})", "({x}+{y})"),
        (Equal, vec!["x", "y"], "({x}={y})", "({x}={y})"),
        (Negate, vec!["x"], "(-{x})", "(−{x})"),
        (Multiply, vec!["x", "y"], "({x}\\cdot{}{y})", "({x}⋅{y})"),
        (Juxtapose, vec!["1", "2"], "{1}\\,{2}", "{1}\\,{2}"),
        (Parentheses, vec!["x"], "({x})", "({x})"),
        (Dimension, vec!["x"], "[{x}]", "[{x}]"),
    ] {
        let expr = ExprDraft::Call {
            callee: Callee::Primitive(id),
            arguments: args
                .iter()
                .map(|v| ExprDraft::Literal { value: (*v).into() })
                .collect(),
        };
        for (target, expected) in [
            (RenderTarget::Macro, macro_source),
            (RenderTarget::Unicode, unicode),
        ] {
            assert_eq!(
                render::expression(&n, &expr, target, RenderOwner::Expression, "golden")
                    .unwrap()
                    .source,
                expected
            );
        }
    }
}
