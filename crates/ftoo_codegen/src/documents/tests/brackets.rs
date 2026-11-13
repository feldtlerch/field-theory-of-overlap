use super::*;
use crate::support_bracket::mutation::FileMutation as _;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;

// Both tiny fixtures anchor ancestor identities in the crate directory. Keep
// their creation/removal outside each other's observation intervals, including
// the separate process used by the consumer-epoch fixture.
static CONFINEMENT_FIXTURES: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn confinement_native_fault_poison_discards_provisional_results_and_rereads() {
    let _serial = CONFINEMENT_FIXTURES.lock().unwrap();
    let directory = Directory(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".confinement-fixture-{}", std::process::id())),
    );
    for name in ["runtime/bin", "bundle", "tmp", "downloads"] {
        fs::create_dir_all(directory.0.join(name)).unwrap();
    }
    let engine = b"#!/bin/sh\nif [ \"$2\" = attempt ]; then\n  if (printf changed > \"$1\") 2>/dev/null; then exit 7; fi\n  printf 'refused\\n'\nelse\n  printf consumer > \"$1\"\nfi\n";
    let executable = directory.0.join("runtime/bin/tectonic");
    fs::write(&executable, engine).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = "d".repeat(64);
    fs::write(directory.0.join("bundle/SHA256SUM"), &digest).unwrap();
    let lock = native_support::Lock {
        source: None,
        engine: format!("{:x}", Sha256::digest(engine)),
        files: BTreeMap::from([(
            "SHA256SUM".into(),
            format!("{:x}", Sha256::digest(digest.as_bytes())),
        )]),
        bundle_digest: digest,
    };
    let support = NativeSupport::at(directory.0.clone());
    let scratch =
        crate::support_bracket::confinement::Scratch::new(&directory.0.join("tmp")).unwrap();
    let marker = scratch.path().join("marker");
    let expected = NativeAdapterFault::Confinement {
        operation: crate::ConfinementOperation::MakePrivate,
        path: "/".into(),
        kind: std::io::ErrorKind::PermissionDenied,
    };
    let result: Result<&str, DocumentError> = support.scope(&lock, || {
        support.before_consumer(&lock)?;
        let refused = crate::support_bracket::confinement::with_fault(
            crate::ConfinementOperation::MakePrivate,
            || {
                native::measured_output(
                    std::process::Command::new(&executable)
                        .arg(&marker)
                        .current_dir(scratch.path()),
                    &[scratch.path().to_owned()],
                )
                .map_err(|error| support.spawn_error(&executable, &error))
            },
        );
        assert_eq!(refused.map(|_| ()), Err(expected.clone()));
        assert_eq!(support.before_consumer(&lock), Err(expected.clone()));
        Ok("provisional success cannot escape")
    });
    assert_eq!(result, Err(DocumentError::Native(expected)));
    assert_eq!(
        result.unwrap_err().to_string(),
        "Native(Confinement { operation: MakePrivate, path: \"/\", kind: PermissionDenied })"
    );
    assert_eq!(support.closing_hash(), Some(Ok(())));
    assert_eq!(
        fs::read(&marker).map_err(|error| error.raw_os_error()),
        Err(Some(libc::ENOENT))
    );
    assert_eq!(fs::read(&executable).unwrap(), engine);
    let restored: Result<(), DocumentError> = support.scope(&lock, || {
        support.before_consumer(&lock)?;
        let output = native::measured_output(
            std::process::Command::new(&executable)
                .arg(&marker)
                .current_dir(scratch.path()),
            &[scratch.path().to_owned()],
        )
        .map_err(|error| support.spawn_error(&executable, &error))?;
        support.after_consumer(&lock)?;
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"");
        assert_eq!(output.stderr, b"");
        support.before_consumer(&lock)?;
        let protected = directory.0.join("bundle/SHA256SUM");
        let output = native::measured_output(
            std::process::Command::new(&executable)
                .arg(&protected)
                .arg("attempt")
                .current_dir(directory.0.join("bundle")),
            &[],
        )
        .map_err(|error| support.spawn_error(&executable, &error))?;
        support.after_consumer(&lock)?;
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"refused\n");
        assert_eq!(output.stderr, b"");
        assert_eq!(fs::read_to_string(protected).unwrap(), lock.bundle_digest);
        Ok(())
    });
    assert_eq!(restored, Ok(()));
    assert_eq!(fs::read(marker).unwrap(), b"consumer");
}

#[test]
fn intervening_and_inflight_consumers_invalidate_shared_observation()
-> Result<(), Box<dyn std::error::Error>> {
    // Isolate the process-wide clock from other concurrently running tests.
    const CHILD: &str = "FTOO_OBSERVATION_CONTROL_CHILD";
    let _serial = CONFINEMENT_FIXTURES.lock().unwrap();
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "documents::tests::brackets::intervening_and_inflight_consumers_invalidate_shared_observation", "--nocapture"])
            .env(CHILD, "1").output()?;
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }
    // The general temporary root changes during parallel tests. Keep the tiny
    // fixture beneath the stable checkout so its ancestor identities are real.
    let directory = Directory(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".counter-fixture-{}", std::process::id())),
    );
    for name in ["runtime/bin", "bundle", "tmp", "downloads"] {
        fs::create_dir_all(directory.0.join(name))?;
    }
    let engine = b"#!/bin/sh\nif [ \"$1\" = wait ]; then read ignored; fi\nexit 0\n";
    let executable = directory.0.join("runtime/bin/tectonic");
    fs::write(&executable, engine)?;
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
    let digest = "d".repeat(64);
    fs::write(directory.0.join("bundle/SHA256SUM"), &digest)?;
    let lock = native_support::Lock {
        source: None,
        engine: format!("{:x}", Sha256::digest(engine)),
        files: BTreeMap::from([(
            "SHA256SUM".into(),
            format!("{:x}", Sha256::digest(digest.as_bytes())),
        )]),
        bundle_digest: digest,
    };
    let support = NativeSupport::at(directory.0.clone());
    let probe = directory.0.join("tmp/mutation");
    let other = directory.0.join("other-private-root");
    fs::create_dir_all(other.join("tmp"))?;
    crate::support_bracket::mutation::register(&other);
    support.scope(&lock, || -> Result<(), DocumentError> {
        support.after_consumer(&lock)?;
        let count = support.identity_observations.get();
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count);

        support.after_consumer(&lock)?;
        let count = support.identity_observations.get();
        let epoch = crate::support_bracket::consumer_epoch();
        let output = native::measured_output(
            std::process::Command::new(support.directory().join("runtime/bin/tectonic"))
                .arg("--version"),
            &[],
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(crate::support_bracket::consumer_epoch(), epoch + 1);
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count + 1);

        support.after_consumer_observed(&lock, || {
            let output = native::measured_output(
                std::process::Command::new(&executable).arg("--version"),
                &[],
            )
            .unwrap();
            assert!(output.status.success());
        })?;
        let count = support.identity_observations.get();
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count + 1);

        let flight = crate::support_bracket::consumer_started();
        let mut command = std::process::Command::new(&executable);
        command.arg("wait").stdin(std::process::Stdio::piped());
        let mut child = crate::support_bracket::confinement::spawn(&mut command, &[]).unwrap();
        assert_eq!(child.try_wait().unwrap(), None);
        support.after_consumer(&lock)?;
        let count = support.identity_observations.get();
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count + 1);
        drop(child.stdin.take());
        assert!(child.wait().unwrap().success());
        drop(flight);
        // An observation made while a consumer ran stays unavailable after wait.
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count + 2);

        support.after_consumer(&lock)?;
        let count = support.identity_observations.get();
        crate::support_bracket::mutation::write(other.join("tmp/control"), b"unrelated").unwrap();
        assert_eq!(fs::read(other.join("tmp/control")).unwrap(), b"unrelated");
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count);

        support.after_consumer(&lock)?;
        let count = support.identity_observations.get();
        crate::support_bracket::mutation::write(&probe, b"control").unwrap();
        assert_eq!(fs::read(&probe).unwrap(), b"control");
        support.before_consumer(&lock)?;
        assert_eq!(support.identity_observations.get(), count + 1);
        Ok(())
    })?;
    assert_eq!(support.closing_hash(), Some(Ok(())));
    Ok(())
}

pub(super) struct CaseEvidence {
    name: String,
    actual: Result<(), DocumentError>,
    expected: Result<(), DocumentError>,
    display: Option<String>,
}
impl CaseEvidence {
    pub(super) fn new(
        name: impl Into<String>,
        actual: Result<(), DocumentError>,
        expected: Result<(), DocumentError>,
    ) -> Self {
        let display = expected.as_ref().err().map(|error| {
            use std::fmt::Write;
            let DocumentError::Assembly(errors) = error else {
                return format!("{error:?}");
            };
            let mut text = String::new();
            for row in errors.diagnostics() {
                writeln!(
                    text,
                    "{:?} {:?} document={:?} unit={:?} language={:?} target={:?} at {:?}: {:?}",
                    row.rule,
                    row.stage,
                    row.document,
                    row.unit,
                    row.language,
                    row.target,
                    row.site,
                    row.fault
                )
                .unwrap();
            }
            text
        });
        Self {
            name: name.into(),
            actual,
            expected,
            display,
        }
    }
    pub(super) fn with_display(
        name: impl Into<String>,
        actual: Result<(), DocumentError>,
        expected: Result<(), DocumentError>,
        display: String,
    ) -> Self {
        Self {
            name: name.into(),
            actual,
            expected,
            display: Some(display),
        }
    }
    fn check(self) {
        assert_eq!(self.actual, self.expected, "{}", self.name);
        assert_eq!(
            self.actual.as_ref().err().map(ToString::to_string),
            self.display,
            "{}",
            self.name
        );
    }
}

#[test]
fn support_required_native_steady_cases_close_before_exact_assertions()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
    let lock = native_support::lock(&root).map_err(|e| format!("{e:?}"))?;
    *support.evidence.borrow_mut() = Some(Vec::new());
    let results = support.scope(&lock, || -> Result<_, DocumentError> {
        // Helpers retain pure source assertions; consumer evidence stays private.
        let mut checks = Vec::new();
        let run = |checks: &mut Vec<CaseEvidence>| -> Result<(), Box<dyn std::error::Error>> {
            support.cold_format.set(true);
            support_required_actual_native_faces_framing_and_expanded_live_keys(&support, &lock, checks)?;
            support_required_actual_native_unknown_tag_has_complete_decoded_refusal(&support, &lock, checks)?;
            support_required_native_lost_character_is_a_complete_hard_error(&support, &lock, checks)?;
            acceptance::support_required_native_negative_contexts_pin_complete_evidence_and_restore(&support, &lock, checks)?;
            displays::support_required_real_table_and_picture_consume_pending_math_and_literal_spans(&support, &lock, checks)?;
            for check in checks.iter_mut() { check.name = format!("cold/{}", check.name); }
            support.cold_format.set(false);
            support_required_native_unexpanded_live_lookup_is_refused_and_restores(&support, &lock, checks)?;
            support_required_actual_native_faces_framing_and_expanded_live_keys(&support, &lock, checks)?;
            support_required_actual_native_unknown_tag_has_complete_decoded_refusal(&support, &lock, checks)?;
            support_required_native_lost_character_is_a_complete_hard_error(&support, &lock, checks)?;
            acceptance::support_required_native_negative_contexts_pin_complete_evidence_and_restore(&support, &lock, checks)?;
            acceptance::support_required_native_lifecycle_tables_reject_at_prose_and_display_sites_and_restore(&support, &lock, checks)?;
            acceptance::support_required_native_unknown_unicode_quotes_backslashes_and_controls_keep_complete_records(&support, &lock, checks)?;
            sweep::support_required_native_sweep_row_has_complete_evidence_and_restores(&support, &lock, checks)?;
            displays::support_required_real_table_and_picture_consume_pending_math_and_literal_spans(&support, &lock, checks)?;
            overfull::support_required_overfull_boxes_refuse_complete_frames_and_restore_at_boundary(&support, &lock, checks)?;
            support_required_native_citation_keys_and_retirement(&support,&lock,checks)?;
            Ok(())
        };
        // A pure fixture failure unwinds the owning scope and discards evidence.
        run(&mut checks).unwrap();
        Ok(checks)
    })?;
    assert_eq!(support.closing_hash(), Some(Ok(())));
    let raw = support.evidence.borrow_mut().take().unwrap();
    assert_eq!(raw.len(), results.len());
    for (result, evidence) in results.iter().zip(&raw) {
        let name = result.name.strip_prefix("cold/").unwrap_or(&result.name);
        if matches!(
            name,
            "faces/accepted"
                | "unknown/refusal"
                | "lost-character/refusal"
                | "negative/stored/refusal"
                | "negative/collision/refusal"
                | "negative/include/refusal"
                | "negative/closure/refusal"
                | "display/0"
                | "display/1"
                | "display/2"
                | "citation/literal"
                | "citation/composed"
                | "citation/retired"
                | "citation/restored"
                | "citation/encoded"
        ) {
            let (status, stdout, stderr, log) = native_evidence::raw_evidence(&result.name);
            let stdout = stdout.replace("@SCRATCH@", evidence.scratch.to_str().unwrap());
            // Successful PDF sizes are independently observed filesystem facts,
            // like scratch paths; the engine's PDF bytes vary between runs.
            let stdout = if status == 0 {
                let size =
                    u32::try_from(evidence.pdf_bytes.expect("successful private PDF")).unwrap();
                stdout.replace("@PDF-KIB@", &format!("{:.2}", f64::from(size) / 1024.0))
            } else {
                stdout
            };
            // The engine enumerates its two output files in either order.
            // Admit both complete frames, without normalizing actual evidence.
            let mut alternate: Vec<_> = stdout.split_inclusive('\n').collect();
            let writes: Vec<_> = alternate
                .iter()
                .enumerate()
                .filter_map(|(index, line)| line.starts_with("note: Writing `").then_some(index))
                .collect();
            if writes.len() == 2 {
                assert_eq!(writes[1], writes[0] + 1);
                alternate.swap(writes[0], writes[1]);
            }
            assert_eq!(evidence.status, Some(status), "{}", result.name);
            assert!(
                evidence.stdout == stdout.as_bytes()
                    || evidence.stdout == alternate.concat().as_bytes(),
                "{} stdout: {:?}",
                result.name,
                evidence.stdout
            );
            assert_eq!(evidence.stderr, stderr.as_bytes(), "{}", result.name);
            assert_eq!(evidence.log, log.as_bytes(), "{}", result.name);
        }
    }
    let mut expected: Vec<String> = [
        "lookup/refusal",
        "lookup/restored",
        "faces/accepted",
        "unknown/refusal",
        "lost-character/refusal",
    ]
    .map(str::to_owned)
    .into();
    for name in ["stored", "collision", "include", "closure"] {
        if name == "stored" {
            expected.push(format!("negative/{name}/declaration"));
        }
        for outcome in ["refusal", "restored"] {
            expected.push(format!("negative/{name}/{outcome}"));
        }
    }
    for (fixture, handle) in [
        ("tag-retirement", "c:3"),
        ("identity-retirement", "c:3"),
        ("target-absent", "c:3"),
        ("selected-one", "v:3"),
        ("selected-one", "c:3"),
        ("empty", "c:1"),
    ] {
        for position in 0..5 {
            for outcome in ["refusal", "restored"] {
                expected.push(format!("lifecycle/{fixture}/{handle}/{position}/{outcome}"));
            }
        }
    }
    for position in 0..4 {
        for outcome in ["refusal", "restored"] {
            expected.push(format!("unicode/{position}/{outcome}"));
        }
    }
    expected.extend(
        [
            "sweep/refusal",
            "sweep/restored",
            "display/0",
            "display/1",
            "display/2",
        ]
        .map(str::to_owned),
    );
    assert_eq!(expected.len(), 87);
    for index in 0..2 {
        expected.push(format!("overfull/{index}/refusal"));
        expected.push(format!("overfull/{index}/restored"));
    }
    expected.extend(
        [
            "citation/literal",
            "citation/composed",
            "citation/retired",
            "citation/restored",
            "citation/encoded",
        ]
        .map(str::to_owned),
    );
    let mut cold = vec![
        "cold/faces/accepted".into(),
        "cold/unknown/refusal".into(),
        "cold/lost-character/refusal".into(),
    ];
    for name in ["stored", "collision", "include", "closure"] {
        if name == "stored" {
            cold.push(format!("cold/negative/{name}/declaration"));
        }
        for outcome in ["refusal", "restored"] {
            cold.push(format!("cold/negative/{name}/{outcome}"));
        }
    }
    for index in 0..3 {
        cold.push(format!("cold/display/{index}"));
    }
    assert_eq!(cold.len(), 15);
    cold.extend(expected);
    let expected = cold;
    assert_eq!(
        results.iter().map(|r| &r.name).collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>()
    );
    // Each mode has its own complete byte oracle above. Compare the semantic
    // causes and sites as well, excluding only engine progress/log payloads.
    for cold in results
        .iter()
        .filter(|result| result.name.starts_with("cold/"))
    {
        let name = cold.name.strip_prefix("cold/").unwrap();
        let seeded = results.iter().find(|result| result.name == name).unwrap();
        assert_eq!(semantic(&cold.actual), semantic(&seeded.actual), "{name}");
    }
    for result in results {
        result.check();
    }
    Ok(())
}

pub(super) static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn support_required_held_directory_replacement_is_observed_and_restores()
-> Result<(), Box<dyn std::error::Error>> {
    let _serial = MUTATIONS.lock().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let support = NativeSupport::from_environment(&repository).map_err(|e| format!("{e:?}"))?;
    let directory = Directory(
        support
            .directory()
            .join("downloads")
            .join(format!("held-{}", std::process::id())),
    );
    let selected = directory.0.join("selected");
    let replacement = directory.0.join("replacement");
    let saved = directory.0.join("saved");
    fs::create_dir_all(&selected)?;
    fs::create_dir(&replacement)?;
    crate::support_bracket::mutation::write(selected.join("member"), b"same bytes")?;
    crate::support_bracket::mutation::write(replacement.join("member"), b"same bytes")?;
    let roots = vec![selected.clone()];
    let before = identity(&selected.join("member"));
    let snapshot =
        crate::support_bracket::Snapshot::capture(&roots).map_err(|e| format!("{e:?}"))?;
    assert_eq!(snapshot.check().unwrap(), snapshot.member_count());
    // The retained descriptor still reaches the original directory. Its parent
    // entry must nevertheless detect that the selected path now reaches another.
    crate::support_bracket::mutation::rename(&selected, &saved)?;
    crate::support_bracket::mutation::rename(&replacement, &selected)?;
    let fault = crate::support_bracket::Fault::Changed {
        path: selected.join("member"),
        expected: before,
        actual: Some(identity(&selected.join("member"))),
    };
    assert_eq!(
        format!("{:?}", snapshot.check().unwrap_err()),
        format!("{fault:?}")
    );
    drop(snapshot);
    crate::support_bracket::mutation::rename(&selected, &replacement)?;
    crate::support_bracket::mutation::rename(&saved, &selected)?;
    assert_eq!(fs::read(selected.join("member"))?, b"same bytes");
    let restored =
        crate::support_bracket::Snapshot::capture(&roots).map_err(|e| format!("{e:?}"))?;
    assert_eq!(restored.check().unwrap(), restored.member_count());
    Ok(())
}

#[test]
fn native_scope_keeps_pure_failure_before_lazy_support_opening() {
    let directory = Directory::new();
    let support = NativeSupport::at(directory.0.clone());
    let lock = native_support::Lock {
        source: None,
        engine: "unused".into(),
        files: BTreeMap::new(),
        bundle_digest: "unused".into(),
    };
    let expected = DocumentError::Input(DocumentInputFault::EmptySkeletons {
        directory: "units/skeletons".into(),
    });
    let result = support.scope(&lock, || input::skeletons(&[]).map(|_| ()));
    assert_eq!(result, Err(expected));
    assert_eq!(support.closing_hash(), None);
    assert!(fs::read_dir(&directory.0).unwrap().next().is_none());
}

pub(super) fn identity(path: &Path) -> crate::SupportIdentity {
    use std::os::unix::fs::MetadataExt;
    let value = fs::metadata(path).unwrap();
    crate::SupportIdentity {
        device: value.dev(),
        inode: value.ino(),
        size: value.size(),
        mode: value.mode(),
        mtime_seconds: value.mtime(),
        mtime_nanoseconds: value.mtime_nsec(),
        ctime_seconds: value.ctime(),
        ctime_nanoseconds: value.ctime_nsec(),
    }
}

#[test]
fn support_required_native_identity_mutations_poison_and_close_discards_results()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let _serial = MUTATIONS.lock().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = NativeSupport::from_environment(&repository).map_err(|e| format!("{e:?}"))?;
    let directory = Directory(
        original
            .directory()
            .join("downloads")
            .join(format!("identity-{}", std::process::id())),
    );
    for name in ["runtime/bin", "bundle", "tmp", "downloads"] {
        fs::create_dir_all(directory.0.join(name))?;
    }
    crate::support_bracket::mutation::write(directory.0.join("runtime/bin/tectonic"), b"a")?;
    let digest = "d".repeat(64);
    crate::support_bracket::mutation::write(directory.0.join("bundle/SHA256SUM"), &digest)?;
    let path = directory.0.join("bundle/member");
    crate::support_bracket::mutation::write(&path, b"a")?;
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let lock = native_support::Lock {
        source: None,
        engine: hash(b"a"),
        files: BTreeMap::from([
            ("SHA256SUM".into(), hash(digest.as_bytes())),
            ("member".into(), hash(b"a")),
        ]),
        bundle_digest: digest,
    };
    let support = NativeSupport::at(directory.0.clone());
    for mutation in ["identical", "mtime", "rename"] {
        let before = identity(&path);
        let modified = fs::metadata(&path)?.modified()?;
        let probe = directory.0.join("tmp/quantum");
        let start = std::time::Instant::now();
        loop {
            crate::support_bracket::mutation::write(&probe, b"probe")?;
            let now = identity(&probe);
            if (now.ctime_seconds, now.ctime_nanoseconds)
                != (before.ctime_seconds, before.ctime_nanoseconds)
            {
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
        }
        let mut expected = None;
        let result = support.scope(&lock, || -> Result<Vec<&str>, NativeAdapterFault> {
            support.observe(&lock)?;
            match mutation {
                "identical" => {
                    crate::support_bracket::mutation::write(&path, b"a").unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_support_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "mtime" => {
                    crate::support_bracket::mutation::write(&path, b"b").unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_support_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "rename" => {
                    let replacement = directory.0.join("tmp/replacement");
                    crate::support_bracket::mutation::write(&replacement, b"a").unwrap();
                    crate::support_bracket::mutation::rename(replacement, &path).unwrap();
                }
                _ => unreachable!(),
            }
            let after = identity(&path);
            assert_ne!(after, before);
            if mutation != "rename" {
                assert_eq!(before.size, after.size);
                assert_ne!(
                    (before.ctime_seconds, before.ctime_nanoseconds),
                    (after.ctime_seconds, after.ctime_nanoseconds)
                );
                assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
            }
            if mutation == "rename" {
                assert_ne!(after.inode, before.inode);
            }
            let fault = NativeAdapterFault::SupportChanged {
                member: path.to_string_lossy().into_owned(),
                expected: before,
                actual: Some(after),
            };
            expected = Some(fault.clone());
            assert_eq!(support.observe(&lock), Err(fault.clone()));
            crate::support_bracket::mutation::write(&path, b"a").unwrap();
            assert_eq!(fs::read(&path).unwrap(), b"a");
            assert_eq!(support.observe(&lock), Err(fault.clone()));
            let started = std::cell::Cell::new(false);
            let refused = native::consume_with(
                &support,
                &lock,
                "",
                "",
                "",
                NativeCase::Document {
                    document: "fixture".into(),
                    language: "en".into(),
                },
                vec![],
                |_| started.set(true),
                |_, _| {},
            );
            assert_eq!(refused, Err(DocumentError::Native(fault)));
            assert!(!started.get());
            Ok(vec!["provisional"])
        });
        let fault = expected.unwrap();
        assert_eq!(result, Err(fault.clone()));
        let error = DocumentError::Native(result.unwrap_err());
        assert_eq!(error.to_string(), format!("Native({fault:?})"));
        assert_eq!(support.closing_hash(), Some(Ok(())));
        assert_eq!(fs::read(&path)?, b"a");
        assert_eq!(
            support.scope(&lock, || -> Result<_, NativeAdapterFault> {
                support.observe(&lock)?;
                Ok(vec!["restored"])
            }),
            Ok(vec!["restored"])
        );
    }
    let ancestor = path.parent().unwrap();
    let before = identity(ancestor);
    let modified = fs::metadata(ancestor)?.modified()?;
    let result = support.scope(&lock, || -> Result<(), NativeAdapterFault> {
        support.observe(&lock)?;
        fs::File::open(ancestor)
            .unwrap()
            .set_support_times(fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
            .unwrap();
        support.observe(&lock)
    });
    let changed = NativeAdapterFault::SupportChanged {
        member: ancestor.to_string_lossy().into_owned(),
        expected: before,
        actual: Some(identity(ancestor)),
    };
    assert_eq!(result, Err(changed.clone()));
    assert_eq!(
        DocumentError::Native(result.unwrap_err()).to_string(),
        format!("Native({changed:?})")
    );
    assert_eq!(support.closing_hash(), Some(Ok(())));
    fs::File::open(ancestor)?.set_support_times(fs::FileTimes::new().set_modified(modified))?;
    assert_eq!(fs::metadata(ancestor)?.modified()?, modified);
    assert_eq!(fs::read(&path)?, b"a");
    assert_eq!(support.scope(&lock, || support.observe(&lock)), Ok(()));
    let result = support.scope(&lock, || -> Result<_, NativeAdapterFault> {
        support.observe(&lock)?;
        crate::support_bracket::mutation::write(&path, b"b").unwrap();
        Ok(vec!["first provisional", "second provisional"])
    });
    let fault = NativeAdapterFault::Integrity {
        component: NativeComponent::Bundle,
        member: "member".into(),
        expected: hash(b"a"),
        actual: hash(b"b"),
    };
    assert_eq!(result, Err(fault.clone()));
    assert_eq!(support.closing_hash(), Some(Err(fault.clone())));
    assert_eq!(
        DocumentError::Native(result.unwrap_err()).to_string(),
        format!("Native({fault:?})")
    );
    assert_eq!(fs::read(&path)?, b"b");
    crate::support_bracket::mutation::write(&path, b"a")?;
    assert_eq!(fs::read(&path)?, b"a");
    assert_eq!(
        support.scope(&lock, || -> Result<_, NativeAdapterFault> {
            support.observe(&lock)?;
            Ok(vec!["restored"])
        }),
        Ok(vec!["restored"])
    );
    // The invalid executable would produce a spawn fault if the pre-invocation
    // identity comparison were omitted. The mutation must take precedence.
    let before = identity(&path);
    let result = native::consume_with(
        &support,
        &lock,
        "",
        "",
        "",
        NativeCase::Document {
            document: "fixture".into(),
            language: "en".into(),
        },
        vec![],
        |_| crate::support_bracket::mutation::write(&path, b"b").unwrap(),
        |_, _| {},
    );
    let changed = NativeAdapterFault::SupportChanged {
        member: path.to_string_lossy().into_owned(),
        expected: before,
        actual: Some(identity(&path)),
    };
    assert_eq!(result, Err(DocumentError::Native(changed.clone())));
    assert_eq!(
        result.unwrap_err().to_string(),
        format!("Native({changed:?})")
    );
    assert_eq!(support.closing_hash(), Some(Err(fault)));
    assert_eq!(fs::read(&path)?, b"b");
    crate::support_bracket::mutation::write(&path, b"a")?;
    assert_eq!(fs::read(&path)?, b"a");
    assert_eq!(
        support.scope(&lock, || -> Result<_, NativeAdapterFault> {
            support.observe(&lock)?;
            Ok(())
        }),
        Ok(())
    );
    Ok(())
}

fn semantic(result: &Result<(), DocumentError>) -> Result<(), DocumentError> {
    let mut result = result.clone();
    if let Err(DocumentError::Assembly(errors)) = &mut result {
        let rows = errors
            .diagnostics()
            .iter()
            .cloned()
            .map(|mut row| {
                if let AssemblyFault::NativeRejected {
                    stdout,
                    stderr,
                    log,
                    ..
                } = &mut row.fault
                {
                    stdout.clear();
                    stderr.clear();
                    log.clear();
                }
                row
            })
            .collect();
        *errors = AssemblyErrors::from_diagnostics(rows).unwrap();
    }
    result
}

#[test]
fn support_required_format_state_fault_poison_discards_provisional_results_and_never_resumes() {
    use super::super::{native_format::Seed, native_support::Lock};
    use sha2::{Digest, Sha256};
    let _serial = MUTATIONS.lock().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = NativeSupport::from_environment(&repository).unwrap();
    let directory = Directory(
        original
            .directory()
            .join("downloads")
            .join(format!("format-state-{}", std::process::id())),
    );
    fs::create_dir(&directory.0).unwrap();
    for name in ["runtime/bin", "bundle", "downloads", "tmp/case/cache"] {
        fs::create_dir_all(directory.0.join(name)).unwrap();
    }
    crate::support_bracket::mutation::write(directory.0.join("runtime/bin/tectonic"), b"engine")
        .unwrap();
    let digest = "d".repeat(64);
    crate::support_bracket::mutation::write(directory.0.join("bundle/SHA256SUM"), &digest).unwrap();
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let lock = Lock {
        source: None,
        engine: hash(b"engine"),
        files: BTreeMap::from([("SHA256SUM".into(), hash(digest.as_bytes()))]),
        bundle_digest: digest,
    };
    let support = NativeSupport::at(directory.0.clone());
    let cache = directory.0.join("tmp/case/cache");
    let seed = Seed {
        files: BTreeMap::from([("a.fmt".into(), b"seed".to_vec())]),
    };
    for phase in [FormatPhase::BeforeCase, FormatPhase::AfterCase] {
        let fault = NativeAdapterFault::FormatState {
            phase: phase.clone(),
            missing: vec![],
            unexpected: vec![],
            changed: vec![FormatChange {
                member: "a.fmt".into(),
                expected: b"seed".to_vec(),
                actual: b"mutation".to_vec(),
            }],
        };
        let result = support.scope(&lock, || -> Result<Vec<&str>, NativeAdapterFault> {
            support.observe(&lock)?;
            seed.copy(&cache)?;
            crate::support_bracket::mutation::write(
                cache.join("Tectonic/formats/a.fmt"),
                b"mutation",
            )
            .unwrap();
            assert_eq!(
                seed.check(&cache, phase)
                    .map_err(|fault| support.poison(fault)),
                Err(fault.clone())
            );
            seed.copy(&cache)?;
            assert_eq!(support.observe(&lock), Err(fault.clone()));
            let started = std::cell::Cell::new(false);
            let refused = super::super::native::consume_with(
                &support,
                &lock,
                "",
                "",
                "",
                super::super::NativeCase::Document {
                    document: "fixture".into(),
                    language: "en".into(),
                },
                vec![],
                |_| started.set(true),
                |_, _| {},
            );
            assert_eq!(refused, Err(DocumentError::Native(fault.clone())));
            assert!(!started.get());
            Ok(vec!["provisional"])
        });
        assert_eq!(result, Err(fault.clone()));
        assert_eq!(support.closing_hash(), Some(Ok(())));
        assert_eq!(
            fs::read(cache.join("Tectonic/formats/a.fmt")).unwrap(),
            b"seed"
        );
        assert_eq!(
            support.scope(&lock, || {
                support.observe(&lock)?;
                seed.check(&cache, FormatPhase::BeforeCase)
            }),
            Ok(())
        );
    }
}
