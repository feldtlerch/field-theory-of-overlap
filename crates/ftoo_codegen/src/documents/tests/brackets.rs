use super::*;

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
    let results = support.scope(&lock, || -> Result<_, DocumentError> {
        // Helpers retain pure source assertions; consumer evidence stays private.
        let mut checks = Vec::new();
        let run = |checks: &mut Vec<CaseEvidence>| -> Result<(), Box<dyn std::error::Error>> {
            support_required_native_unexpanded_live_lookup_is_refused_and_restores(&support, &lock, checks)?;
            support_required_actual_native_faces_framing_and_expanded_live_keys(&support, &lock, checks)?;
            support_required_actual_native_unknown_tag_has_complete_decoded_refusal(&support, &lock, checks)?;
            support_required_native_lost_character_is_a_complete_hard_error(&support, &lock, checks)?;
            acceptance::support_required_native_negative_contexts_pin_complete_evidence_and_restore(&support, &lock, checks)?;
            acceptance::support_required_native_lifecycle_tables_reject_at_prose_and_display_sites_and_restore(&support, &lock, checks)?;
            acceptance::support_required_native_unknown_unicode_quotes_backslashes_and_controls_keep_complete_records(&support, &lock, checks)?;
            sweep::support_required_native_sweep_row_has_complete_evidence_and_restores(&support, &lock, checks)?;
            displays::support_required_real_table_and_picture_consume_pending_math_and_literal_spans(&support, &lock, checks)?;
            Ok(())
        };
        // A pure fixture failure unwinds the owning scope and discards evidence.
        run(&mut checks).unwrap();
        Ok(checks)
    })?;
    assert_eq!(support.closing_hash(), Some(Ok(())));
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
    assert_eq!(
        results.iter().map(|r| &r.name).collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>()
    );
    for result in results {
        result.check();
    }
    Ok(())
}

pub(super) static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    fs::write(directory.0.join("runtime/bin/tectonic"), b"a")?;
    let digest = "d".repeat(64);
    fs::write(directory.0.join("bundle/SHA256SUM"), &digest)?;
    let path = directory.0.join("bundle/member");
    fs::write(&path, b"a")?;
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
            fs::write(&probe, b"probe")?;
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
                    fs::write(&path, b"a").unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "mtime" => {
                    fs::write(&path, b"b").unwrap();
                    fs::File::options()
                        .write(true)
                        .open(&path)
                        .unwrap()
                        .set_times(fs::FileTimes::new().set_modified(modified))
                        .unwrap();
                }
                "rename" => {
                    let replacement = directory.0.join("tmp/replacement");
                    fs::write(&replacement, b"a").unwrap();
                    fs::rename(replacement, &path).unwrap();
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
            fs::write(&path, b"a").unwrap();
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
            .set_times(fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
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
    fs::File::open(ancestor)?.set_times(fs::FileTimes::new().set_modified(modified))?;
    assert_eq!(fs::metadata(ancestor)?.modified()?, modified);
    assert_eq!(fs::read(&path)?, b"a");
    assert_eq!(support.scope(&lock, || support.observe(&lock)), Ok(()));
    let result = support.scope(&lock, || -> Result<_, NativeAdapterFault> {
        support.observe(&lock)?;
        fs::write(&path, b"b").unwrap();
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
    fs::write(&path, b"a")?;
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
        |_| fs::write(&path, b"b").unwrap(),
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
    fs::write(&path, b"a")?;
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
