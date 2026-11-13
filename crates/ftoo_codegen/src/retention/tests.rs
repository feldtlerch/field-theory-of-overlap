use super::*;

#[test]
fn projection_display_uses_the_wrapped_rule() {
    use crate::{ByteGateFault, ProjectionCheckError};
    let error = ProjectionCheckError::ByteGate {
        rule: ftoo_manifest::RuleId::M038,
        fault: ByteGateFault::NonCanonicalBytes {
            path: PROJECTION.into(),
            offset: 7,
            diff: "window".into(),
        },
    };
    assert_eq!(
        RetentionCheckError::Projection(error.clone()).to_string(),
        "retention provenance M038 at file \"generated/manifest.json\": M038: generated/manifest.json differs at byte 7\nwindow"
    );
    assert_eq!(
        RetentionCheckError::Projection(ProjectionCheckError::AfterReplacement(Box::new(error)))
            .to_string(),
        "retention provenance M038 at file \"generated/manifest.json\": artifact was replaced: M038: generated/manifest.json differs at byte 7\nwindow"
    );
    let error = ProjectionCheckError::Projection(ftoo_manifest::ProjectionError::Wire(
        ftoo_manifest::WireError {
            rule: ftoo_manifest::RuleId::M001,
            location: ftoo_manifest::WireLocation::ByteOffset(0),
            fault: ftoo_manifest::WireFault::InvalidUtf8,
        },
    ));
    assert_eq!(
        RetentionCheckError::Projection(error).to_string(),
        "retention provenance M001 at file \"generated/manifest.json\": Projection(Wire(WireError { rule: M001, location: ByteOffset(0), fault: InvalidUtf8 }))"
    );
}

struct Fixture {
    root: std::path::PathBuf,
    bytes: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = std::env::temp_dir().join(format!(
            "retention-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for (path, snapshot) in sources(&source).unwrap() {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, snapshot.bytes).unwrap();
        }
        for path in ["Cargo.lock", "crates/ftoo_audit/Cargo.toml", REPORT] {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source.join(path), target).unwrap();
        }
        let bytes = fs::read(root.join(REPORT)).unwrap();
        Self { root, bytes }
    }
    fn accepted(&self) {
        let report = check_retention(&self.root).unwrap();
        assert_eq!(report.bytes(), self.bytes);
        assert_eq!(fs::read(self.root.join(REPORT)).unwrap(), self.bytes);
    }
    fn restore(&self) {
        fs::write(self.root.join(REPORT), &self.bytes).unwrap();
        self.accepted();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn committed_report_is_fresh_byte_equal_without_support() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let started = std::time::Instant::now();
    let report = check_retention(&root).unwrap();
    assert_eq!(
        report
            .report()
            .outcomes()
            .iter()
            .map(|row| row.verdict().class())
            .collect::<Vec<_>>(),
        vec![
            ftoo_manifest::VerdictClass::Certified,
            ftoo_manifest::VerdictClass::Certified
        ]
    );
    println!(
        "retention check elapsed={:?} bytes={} measurements={:?}",
        started.elapsed(),
        report.bytes().len(),
        report.report().measurements()
    );
}

#[test]
fn regeneration_rereads_one_production_and_check_never_rewrites() {
    let fixture = Fixture::new();
    let started = std::time::Instant::now();
    let regenerated = regenerate_retention(&fixture.root).unwrap();
    let regeneration_time = started.elapsed();
    assert_eq!(regenerated.bytes(), fixture.bytes);
    let before = identity(&fs::metadata(fixture.root.join(REPORT)).unwrap());
    let started = std::time::Instant::now();
    let checked = check_retention(&fixture.root).unwrap();
    assert_eq!(checked.bytes(), fixture.bytes);
    assert_eq!(
        checked.report().measurements(),
        regenerated.report().measurements()
    );
    let check_time = started.elapsed();
    assert_eq!(
        identity(&fs::metadata(fixture.root.join(REPORT)).unwrap()),
        before
    );
    println!(
        "retention regeneration={regeneration_time:?} same-process-check={check_time:?} bytes={} shared-measurements={:?}",
        fixture.bytes.len(),
        regenerated.report().measurements()
    );
}

#[test]
fn faithfully_retained_incomplete_assessment_does_not_license_release() {
    let fixture = Fixture::new();
    fixture.accepted();
    let mut draft = ftoo_manifest::seed_draft();
    draft.coverage.release_gate = ftoo_manifest::ReleaseGate::Complete;
    let graph = ftoo_manifest::validate(draft).unwrap();
    let projection = ftoo_manifest::projection_bytes(&graph).unwrap();
    let production = retention::produce(projection.clone()).unwrap();
    fs::write(fixture.root.join(REPORT), production.bytes()).unwrap();
    let accepted =
        retention::accept(projection, fs::read(fixture.root.join(REPORT)).unwrap()).unwrap();
    let assessment = accepted.report().assessment();
    assert_eq!(
        assessment.disposition(),
        ftoo_manifest::Disposition::Incomplete
    );
    assert_eq!(assessment.exit_code(), 5);
    assert_eq!(
        assessment.diagnostics(),
        &[ftoo_manifest::Diagnostic {
            stage: ftoo_manifest::AdmissionStage::ReportConsumption,
            rule: ftoo_manifest::RuleId::M015,
            ids: vec![],
            edge: None,
            location: ftoo_manifest::DraftLocation::Coverage,
            explanation: "release predicate is not satisfied".into(),
        }]
    );
    let error = release_assessment(assessment).unwrap_err();
    assert_eq!(
        error,
        RetentionCheckError::ReleaseRejected(assessment.clone())
    );
    assert_eq!(
        error.to_string(),
        "retention assertions M015 at file \"generated/audit-report.json\": release-rejected disposition=incomplete exit-code=5\nretention provenance M015 at file \"generated/audit-report.json\": domain stage=report-consumption ids=[] edge=none location=coverage explanation=\"release predicate is not satisfied\""
    );
    fixture.restore();
    assert_eq!(
        release_assessment(
            check_retention(&fixture.root)
                .unwrap()
                .report()
                .assessment()
        ),
        Ok(())
    );
}

#[test]
fn actual_file_lexical_mutations_restore_and_reread() {
    use ftoo_manifest::{RuleId, WireError, WireFault, WireLocation};
    let fixture = Fixture::new();
    fixture.accepted();
    for (bytes, fault, display) in [
        (
            vec![255],
            WireFault::InvalidUtf8,
            "retention syntax M001 at byte-offset 0: M001 at ByteOffset(0): InvalidUtf8",
        ),
        (
            vec![239, 187, 191],
            WireFault::Bom,
            "retention syntax M001 at byte-offset 0: M001 at ByteOffset(0): Bom",
        ),
    ] {
        fs::write(fixture.root.join(REPORT), bytes).unwrap();
        let expected = RetentionCheckError::Audit(Box::new(RetentionError::Wire(
            retention::WireRefusal::Wire(WireError {
                rule: RuleId::M001,
                location: WireLocation::ByteOffset(0),
                fault,
            }),
        )));
        let actual = check_retention(&fixture.root).unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), display);
        fixture.restore();
    }
    fs::remove_file(fixture.root.join(REPORT)).unwrap();
    let actual = check_retention(&fixture.root).unwrap_err();
    assert_eq!(
        actual,
        RetentionCheckError::Io {
            primary: IoFailure {
                path: REPORT.into(),
                operation: Operation::Metadata,
                kind: io::ErrorKind::NotFound,
                raw_os_error: Some(libc::ENOENT)
            },
            cleanup: Vec::new(),
            replaced: Vec::new()
        }
    );
    assert_eq!(
        actual.to_string(),
        "retention read M062 at file \"generated/audit-report.json\": io path=\"generated/audit-report.json\" operation=metadata kind=\"entity not found\" raw-os-error=2 replaced=[]"
    );
    fixture.restore();
}

#[test]
fn every_write_boundary_keeps_primary_cleanup_and_replaced_paths() {
    let fixture = Fixture::new();
    fixture.accepted();
    for (operation, token, replaced) in [
        (Operation::Create, "create", false),
        (Operation::Write, "write", false),
        (Operation::Flush, "flush", false),
        (Operation::Sync, "sync", false),
        (Operation::Rename, "rename", false),
        (Operation::DirectorySync, "directory-sync", true),
        (Operation::Reread, "reread", true),
    ] {
        let before_identity = identity(&fs::metadata(fixture.root.join(REPORT)).unwrap());
        let mut failed_path = String::new();
        let actual = regenerate_with(&fixture.root, &mut |at, path| {
            if at == operation {
                failed_path = path.into();
                Err(io::ErrorKind::PermissionDenied.into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        let replaced_paths = if replaced {
            vec![REPORT.into()]
        } else {
            Vec::new()
        };
        assert_eq!(
            actual,
            RetentionCheckError::Io {
                primary: IoFailure {
                    path: failed_path.clone(),
                    operation,
                    kind: io::ErrorKind::PermissionDenied,
                    raw_os_error: None
                },
                cleanup: Vec::new(),
                replaced: replaced_paths
            }
        );
        let stage = if operation == Operation::Reread {
            "read"
        } else {
            "write"
        };
        let paths = if replaced {
            "[\"generated/audit-report.json\"]"
        } else {
            "[]"
        };
        assert_eq!(
            actual.to_string(),
            format!(
                "retention {stage} M062 at file \"{failed_path}\": io path=\"{failed_path}\" operation={token} kind=\"permission denied\" raw-os-error=none replaced={paths}"
            )
        );
        assert_eq!(fs::read(fixture.root.join(REPORT)).unwrap(), fixture.bytes);
        if !replaced {
            assert_eq!(
                identity(&fs::metadata(fixture.root.join(REPORT)).unwrap()),
                before_identity
            );
        }
        fixture.restore();
    }
}
#[test]
fn cleanup_failure_keeps_primary_error_and_temporary_path() {
    let fixture = Fixture::new();
    fixture.accepted();
    let mut temporary = String::new();
    let actual = regenerate_with(&fixture.root, &mut |at, path| match at {
        Operation::Write => {
            temporary = path.into();
            Err(io::ErrorKind::PermissionDenied.into())
        }
        Operation::Remove => Err(io::ErrorKind::Other.into()),
        _ => Ok(()),
    })
    .unwrap_err();
    assert_eq!(
        actual,
        RetentionCheckError::Io {
            primary: IoFailure {
                path: temporary.clone(),
                operation: Operation::Write,
                kind: io::ErrorKind::PermissionDenied,
                raw_os_error: None
            },
            cleanup: vec![IoFailure {
                path: temporary.clone(),
                operation: Operation::Remove,
                kind: io::ErrorKind::Other,
                raw_os_error: None
            }],
            replaced: Vec::new()
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention write M062 at file \"{temporary}\": io path=\"{temporary}\" operation=write kind=\"permission denied\" raw-os-error=none replaced=[]\ncleanup path=\"{temporary}\" operation=remove kind=\"other error\" raw-os-error=none"
        )
    );
    assert_eq!(fs::read(fixture.root.join(REPORT)).unwrap(), fixture.bytes);
    fs::remove_file(fixture.root.join(temporary)).unwrap();
    fixture.restore();
}

#[test]
fn stable_snapshot_refuses_changed_replaced_and_rewritten_sources() {
    let fixture = Fixture::new();
    fixture.accepted();
    for path in [PROJECTION, REPORT] {
        let original = fs::read(fixture.root.join(path)).unwrap();
        for replace in [false, true] {
            let expected = identity(&fs::metadata(fixture.root.join(path)).unwrap());
            let mut actual_identity = None;
            let error = check_with(&fixture.root, &mut || {
                if replace {
                    let temporary = fixture.root.join("replacement");
                    fs::write(&temporary, &original).unwrap();
                    fs::rename(&temporary, fixture.root.join(path)).unwrap();
                } else {
                    fs::write(fixture.root.join(path), &original).unwrap();
                }
                actual_identity = Some(identity(&fs::metadata(fixture.root.join(path)).unwrap()));
            })
            .unwrap_err();
            let actual_identity = actual_identity.unwrap();
            assert_eq!(
                error,
                RetentionCheckError::InputChanged {
                    path: path.into(),
                    expected: Box::new(expected.clone()),
                    actual: Some(Box::new(actual_identity.clone()))
                }
            );
            assert_eq!(
                error.to_string(),
                format!(
                    "retention stability M062 at file \"{path}\": input-changed expected={expected} actual={actual_identity}"
                )
            );
            fs::write(fixture.root.join(path), &original).unwrap();
            fixture.accepted();
        }
    }
}

#[test]
fn unrelated_sources_do_not_stale_retention_and_missing_inputs_refuse() {
    let fixture = Fixture::new();
    fixture.accepted();
    for path in [
        "units/new.toml",
        "registry/new.toml",
        "tools/new.toml",
        "README.md",
        "target/nested/README.md",
        ".git/nested/README.md",
    ] {
        let full = fixture.root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        let accepted = check_with(&fixture.root, &mut || {
            fs::write(&full, b"unrelated").unwrap();
        })
        .unwrap();
        assert_eq!(accepted.bytes(), fixture.bytes);
        let accepted = check_with(&fixture.root, &mut || fs::remove_file(&full).unwrap()).unwrap();
        assert_eq!(accepted.bytes(), fixture.bytes);
        fixture.restore();
    }
    missing_inputs(&fixture);
}

fn missing_inputs(fixture: &Fixture) {
    for path in [
        PROJECTION,
        REPORT,
        "Cargo.lock",
        "crates/ftoo_audit/Cargo.toml",
    ] {
        let full = fixture.root.join(path);
        let bytes = fs::read(&full).unwrap();
        let expected = identity(&fs::metadata(&full).unwrap());
        let actual = check_with(&fixture.root, &mut || {
            fs::remove_file(&full).unwrap();
            if path == "crates/ftoo_audit/Cargo.toml" {
                fs::remove_dir(full.parent().unwrap()).unwrap();
            }
        })
        .unwrap_err();
        if let Some(&(path, source)) = ENGINE_FILES.iter().find(|(name, _)| *name == path) {
            assert_eq!(
                actual,
                RetentionCheckError::Audit(Box::new(RetentionError::EngineRevisionUnavailable(
                    retention::EngineRevisionUnavailable {
                        source,
                        path,
                        package: "nrz-cas",
                        reason: retention::EngineRevisionReason::Missing,
                    }
                )))
            );
            let source = if path == "Cargo.lock" {
                "current-lock"
            } else {
                "cargo-manifest"
            };
            assert_eq!(
                actual.to_string(),
                format!(
                    "retention provenance M061 at file \"{path}\": engine-revision-unavailable source={source} package=\"nrz-cas\" reason=missing"
                )
            );
        } else {
            assert_eq!(
                actual,
                RetentionCheckError::InputChanged {
                    path: path.into(),
                    expected: Box::new(expected.clone()),
                    actual: None
                }
            );
            assert_eq!(
                actual.to_string(),
                format!(
                    "retention stability M062 at file \"{path}\": input-changed expected={expected} actual=missing"
                )
            );
        }
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
        fixture.accepted();
    }
}

#[test]
fn lock_metadata_and_unrelated_packages_are_informational() {
    let fixture = Fixture::new();
    fixture.accepted();
    for path in ["Cargo.lock", "crates/ftoo_audit/Cargo.toml"] {
        let full = fixture.root.join(path);
        let original = fs::read(&full).unwrap();
        let suffixes = if path == "Cargo.lock" {
            [
                "",
                "\n# informational formatting\n",
                "\n[[package]]\nname = \"unrelated\"\nversion = \"0.0.0\"\n",
            ]
        } else {
            [
                "",
                "\n# informational formatting\n",
                "\n[package.metadata.retention-test]\nvalue = true\n",
            ]
        };
        for suffix in suffixes {
            let mut changed = original.clone();
            changed.extend_from_slice(suffix.as_bytes());
            let accepted = check_with(&fixture.root, &mut || {
                fs::write(&full, &changed).unwrap();
            })
            .unwrap();
            assert_eq!(accepted.bytes(), fixture.bytes);
            assert_eq!(fs::read(fixture.root.join(REPORT)).unwrap(), fixture.bytes);
            fs::write(&full, &original).unwrap();
            fixture.accepted();
        }
    }
}

#[test]
fn engine_revision_changes_during_check_refuse_and_restore() {
    let fixture = Fixture::new();
    fixture.accepted();
    for (path, _) in ENGINE_FILES {
        let full = fixture.root.join(path);
        let original = fs::read_to_string(&full).unwrap();
        let changed = original.replace(ftoo_manifest::ENGINE_PIN.0, &"0".repeat(40));
        let actual = check_with(&fixture.root, &mut || {
            fs::write(&full, &changed).unwrap();
        })
        .unwrap_err();
        assert_eq!(
            actual,
            RetentionCheckError::Audit(Box::new(RetentionError::ProvenanceMismatch {
                component: retention::Component::EngineRevision,
                path: path.into(),
                difference: retention::ProvenanceDifference::Values {
                    expected: retention::ProvenanceValue::Text(ftoo_manifest::ENGINE_PIN.0.into()),
                    actual: retention::ProvenanceValue::Text("0".repeat(40)),
                },
            }))
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention provenance M061 at file \"{path}\": provenance-mismatch component=engine-revision expected=\"598ff683a57caca07cf9d44e3fd28c716afb81ef\" actual=\"0000000000000000000000000000000000000000\""
            )
        );
        fs::write(&full, &original).unwrap();
        fixture.accepted();
    }
}

#[test]
fn file_identity_display_has_every_component() {
    let identity = FileIdentity {
        length: 3,
        modified: Some(std::time::UNIX_EPOCH + std::time::Duration::new(5, 6)),
        device: 7,
        inode: 8,
        mode: 9,
        changed: (10, 11),
    };
    assert_eq!(
        identity.to_string(),
        "{length=3,modified=5.000000006,device=7,inode=8,mode=9,changed=10.000000011}"
    );
}

#[cfg(unix)]
#[test]
fn actual_file_directory_symlink_and_unreadable_refuse_and_restore() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new();
    fixture.accepted();
    let path = fixture.root.join(REPORT);
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let expected_identity = identity(&fs::symlink_metadata(&path).unwrap());
    let actual = check_retention(&fixture.root).unwrap_err();
    assert_eq!(
        actual,
        RetentionCheckError::NotRegular {
            path: REPORT.into(),
            identity: expected_identity.clone()
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention read M062 at file \"generated/audit-report.json\": not-regular identity={expected_identity}"
        )
    );
    fs::remove_dir(&path).unwrap();
    fixture.restore();
    fs::remove_file(&path).unwrap();
    symlink("manifest.json", &path).unwrap();
    let expected_identity = identity(&fs::symlink_metadata(&path).unwrap());
    let actual = check_retention(&fixture.root).unwrap_err();
    assert_eq!(
        actual,
        RetentionCheckError::NotRegular {
            path: REPORT.into(),
            identity: expected_identity.clone()
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention read M062 at file \"generated/audit-report.json\": not-regular identity={expected_identity}"
        )
    );
    fs::remove_file(&path).unwrap();
    fixture.restore();
    let permissions = fs::metadata(&path).unwrap().permissions();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let actual = check_retention(&fixture.root).unwrap_err();
    assert_eq!(
        actual,
        RetentionCheckError::Io {
            primary: IoFailure {
                path: REPORT.into(),
                operation: Operation::Read,
                kind: io::ErrorKind::PermissionDenied,
                raw_os_error: Some(libc::EACCES)
            },
            cleanup: Vec::new(),
            replaced: Vec::new()
        }
    );
    assert_eq!(
        actual.to_string(),
        "retention read M062 at file \"generated/audit-report.json\": io path=\"generated/audit-report.json\" operation=read kind=\"permission denied\" raw-os-error=13 replaced=[]"
    );
    fs::set_permissions(&path, permissions).unwrap();
    fixture.restore();
}

#[test]
fn source_instability_takes_precedence_over_mismatch_classification() {
    let fixture = Fixture::new();
    fixture.accepted();
    let mut changed = fixture.bytes.clone();
    let text = std::str::from_utf8(&changed).unwrap();
    let offset = text.find("\"source-hex\":\"").unwrap() + "\"source-hex\":\"".len();
    changed[offset] = if changed[offset] == b'0' { b'1' } else { b'0' };
    fs::write(fixture.root.join(REPORT), changed).unwrap();
    let projection = fs::read(fixture.root.join(PROJECTION)).unwrap();
    let expected = identity(&fs::metadata(fixture.root.join(PROJECTION)).unwrap());
    let mut observed = None;
    let actual = check_with(&fixture.root, &mut || {
        fs::write(fixture.root.join(PROJECTION), &projection).unwrap();
        observed = Some(identity(
            &fs::metadata(fixture.root.join(PROJECTION)).unwrap(),
        ));
    })
    .unwrap_err();
    let observed = observed.unwrap();
    assert_eq!(
        actual,
        RetentionCheckError::InputChanged {
            path: PROJECTION.into(),
            expected: Box::new(expected.clone()),
            actual: Some(Box::new(observed.clone()))
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention stability M062 at file \"generated/manifest.json\": input-changed expected={expected} actual={observed}"
        )
    );
    fs::write(fixture.root.join(PROJECTION), projection).unwrap();
    fixture.restore();
}

#[test]
fn generation_source_changes_refuse_before_and_after_replacement() {
    let fixture = Fixture::new();
    fixture.accepted();
    let path = PROJECTION;
    let original = fs::read(fixture.root.join(path)).unwrap();
    for operation in [Operation::Rename, Operation::Reread] {
        let expected = identity(&fs::metadata(fixture.root.join(path)).unwrap());
        let report_identity = identity(&fs::metadata(fixture.root.join(REPORT)).unwrap());
        let mut observed = None;
        let actual = regenerate_with(&fixture.root, &mut |at, _| {
            if at == operation {
                fs::write(fixture.root.join(path), &original).unwrap();
                observed = Some(identity(&fs::metadata(fixture.root.join(path)).unwrap()));
            }
            Ok(())
        })
        .unwrap_err();
        let observed = observed.unwrap();
        let fault = RetentionCheckError::InputChanged {
            path: path.into(),
            expected: Box::new(expected.clone()),
            actual: Some(Box::new(observed.clone())),
        };
        let display = format!(
            "retention stability M062 at file \"{path}\": input-changed expected={expected} actual={observed}"
        );
        if operation == Operation::Rename {
            assert_eq!(actual, fault);
            assert_eq!(actual.to_string(), display);
            assert_eq!(
                identity(&fs::metadata(fixture.root.join(REPORT)).unwrap()),
                report_identity
            );
        } else {
            assert_eq!(
                actual,
                RetentionCheckError::AfterReplacement {
                    error: Box::new(fault),
                    replaced: vec![REPORT.into()]
                }
            );
            assert_eq!(
                actual.to_string(),
                format!("{display}\nreplaced=[\"generated/audit-report.json\"]")
            );
        }
        fs::write(fixture.root.join(path), &original).unwrap();
        fixture.restore();
    }
}

#[cfg(unix)]
#[test]
fn engine_file_missing_unreadable_and_malformed_are_exact_refusals() {
    use retention::{
        EngineRevisionReason as Reason, EngineRevisionSource as Source, EngineRevisionUnavailable,
    };
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.accepted();
    let path = fixture.root.join("Cargo.lock");
    let original = fs::read(&path).unwrap();
    let permissions = fs::metadata(&path).unwrap().permissions();
    for (index, reason, detail) in [
        (0, Reason::Missing, "missing"),
        (
            1,
            Reason::Unreadable {
                kind: io::ErrorKind::PermissionDenied,
                raw_os_error: Some(libc::EACCES),
            },
            "unreadable kind=\"permission denied\" raw-os-error=13",
        ),
        (
            2,
            Reason::Malformed {
                message: "invalid utf-8 sequence of 1 bytes from index 0".into(),
                span: Some(0..1),
            },
            "malformed message=\"invalid utf-8 sequence of 1 bytes from index 0\" span=0..1",
        ),
    ] {
        match index {
            0 => fs::remove_file(&path).unwrap(),
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap(),
            2 => fs::write(&path, [255]).unwrap(),
            _ => unreachable!(),
        }
        let actual = check_retention(&fixture.root).unwrap_err();
        assert_eq!(
            actual,
            RetentionCheckError::Audit(Box::new(RetentionError::EngineRevisionUnavailable(
                EngineRevisionUnavailable {
                    source: Source::CurrentLock,
                    path: "Cargo.lock",
                    package: "nrz-cas",
                    reason
                }
            )))
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention provenance M061 at file \"Cargo.lock\": engine-revision-unavailable source=current-lock package=\"nrz-cas\" reason={detail}"
            )
        );
        if index == 1 {
            fs::set_permissions(&path, permissions.clone()).unwrap();
        }
        fs::write(&path, &original).unwrap();
        fixture.accepted();
    }
}
