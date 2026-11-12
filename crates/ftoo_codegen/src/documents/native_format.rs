//! Owned bootstrap bytes; only a successfully closed bracket publishes a seed.
use super::{
    FormatChange, FormatPhase, NativeAdapterFault, NativeOperation,
    native_support::{self, Lock, NativeSupport},
};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::Path,
    sync::{Arc, Condvar, Mutex, OnceLock},
};

pub(super) const BOOTSTRAP: &str =
    "\\documentclass{article}\n\\begin{document}\nFormat bootstrap.\n\\end{document}\n";
// These are the fixed format-affecting command/environment choices. Scratch
// paths select private storage, not a different initial engine configuration.
const OPTIONS: &str =
    "latex;33;only-cached;untrusted;keep-logs;color=never;LANG=C.UTF-8;empty-config;local-bundle";

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Key {
    engine: Vec<u8>,
    lock: Vec<u8>,
    inventory: BTreeMap<String, String>,
    digest: String,
    bootstrap: String,
    options: String,
}
impl Key {
    pub(super) fn read(
        support: &NativeSupport,
        lock: &Lock,
        bootstrap: &str,
    ) -> Result<Self, NativeAdapterFault> {
        let read = |path: &Path| {
            let mut bytes = Vec::new();
            crate::support_bracket::open_file(path)
                .and_then(|mut file| file.read_to_end(&mut bytes))
                .map_err(|e| native_support::io_error(NativeOperation::Read, path, &e))?;
            Ok::<_, NativeAdapterFault>(bytes)
        };
        Ok(Self {
            engine: read(&support.root.join("runtime/bin/tectonic"))?,
            lock: match &lock.source {
                Some((path, _)) => read(path)?,
                None => vec![],
            },
            inventory: lock.files.clone(),
            digest: lock.bundle_digest.clone(),
            bootstrap: bootstrap.into(),
            options: OPTIONS.into(),
        })
    }
}

#[derive(Debug)]
pub(super) struct Seed {
    pub(super) files: BTreeMap<String, Vec<u8>>,
}
#[derive(Debug)]
struct Entry {
    key: Arc<Key>,
    seed: Option<Arc<Seed>>,
}
#[derive(Default)]
struct Cache {
    entries: Mutex<Vec<Entry>>,
    changed: Condvar,
}
fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Cache::default)
}

#[derive(Debug)]
pub(super) struct Lease {
    key: Arc<Key>,
    pub(super) seed: Option<Arc<Seed>>,
    reserved: bool,
}
impl Lease {
    pub(super) fn matches(&self, bootstrap: &str) -> bool {
        self.key.bootstrap == bootstrap
    }
    pub(super) fn acquire(key: Key) -> Self {
        let cache = cache();
        let mut entries = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(entry) = entries.iter().find(|entry| *entry.key == key) {
                if let Some(seed) = &entry.seed {
                    return Self {
                        key: entry.key.clone(),
                        seed: Some(seed.clone()),
                        reserved: false,
                    };
                }
                entries = cache
                    .changed
                    .wait(entries)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            } else {
                let key = Arc::new(key);
                entries.push(Entry {
                    key: key.clone(),
                    seed: None,
                });
                return Self {
                    key,
                    seed: None,
                    reserved: true,
                };
            }
        }
    }
    pub(super) fn publish(&mut self) {
        if !self.reserved {
            return;
        }
        if let Some(seed) = &self.seed {
            let cache = cache();
            let mut entries = cache
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = entries
                .iter_mut()
                .find(|entry| Arc::ptr_eq(&entry.key, &self.key))
            {
                entry.seed = Some(seed.clone());
                self.reserved = false;
                cache.changed.notify_all();
            }
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.reserved {
            let cache = cache();
            cache
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retain(|entry| !Arc::ptr_eq(&entry.key, &self.key));
            cache.changed.notify_all();
        }
    }
}

fn inventory(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, NativeAdapterFault> {
    fn visit(
        root: &Path,
        path: &Path,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), NativeAdapterFault> {
        let directory = match crate::support_bracket::open_directory(path) {
            Ok(directory) => directory,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && path == root => {
                return Ok(());
            }
            Err(error) => {
                return Err(native_support::io_error(
                    NativeOperation::Inspect,
                    path,
                    &error,
                ));
            }
        };
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && path == root => {
                return Ok(());
            }
            Err(error) => {
                return Err(native_support::io_error(
                    NativeOperation::ReadOutput,
                    path,
                    &error,
                ));
            }
        };
        for entry in entries {
            let entry = entry
                .map_err(|e| native_support::io_error(NativeOperation::ReadOutput, path, &e))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|e| native_support::io_error(NativeOperation::Inspect, &path, &e))?;
            let member = path
                .strip_prefix(root)
                .expect("descendant")
                .to_str()
                .ok_or_else(|| {
                    native_support::io_error(
                        NativeOperation::ReadOutput,
                        &path,
                        &std::io::ErrorKind::InvalidData.into(),
                    )
                })?;
            if kind.is_dir() {
                files.insert(format!("{member}/"), vec![]);
                visit(root, &path, files)?;
            } else if kind.is_file() {
                let mut bytes = Vec::new();
                crate::support_bracket::file_beneath(&directory, Path::new(&entry.file_name()))
                    .and_then(|mut file| file.read_to_end(&mut bytes))
                    .map_err(|e| {
                        native_support::io_error(NativeOperation::ReadOutput, &path, &e)
                    })?;
                files.insert(member.into(), bytes);
            } else {
                return Err(native_support::io_error(
                    NativeOperation::Inspect,
                    &path,
                    &std::io::ErrorKind::InvalidInput.into(),
                ));
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}
impl Seed {
    pub(super) fn capture(cache: &Path, digest: &str) -> Result<Self, NativeAdapterFault> {
        let files = inventory(&cache.join("Tectonic/formats"))?;
        let expected = format!("{digest}-latex-33.fmt");
        let unexpected: Vec<_> = files
            .keys()
            .filter(|name| **name != expected)
            .cloned()
            .collect();
        let missing = if files.contains_key(&expected) {
            vec![]
        } else {
            vec![expected]
        };
        if !missing.is_empty() || !unexpected.is_empty() {
            return Err(NativeAdapterFault::FormatState {
                phase: FormatPhase::Seed,
                missing,
                unexpected,
                changed: vec![],
            });
        }
        Ok(Self { files })
    }
    pub(super) fn copy(&self, cache: &Path) -> Result<(), NativeAdapterFault> {
        let phase = crate::telemetry::Phase::new("native.format_copy");
        let result = (|| {
            for (name, bytes) in &self.files {
                let path = cache.join("Tectonic/formats").join(name);
                fs::create_dir_all(path.parent().expect("format parent"))
                    .and_then(|()| fs::write(&path, bytes))
                    .map_err(|e| {
                        native_support::io_error(NativeOperation::WriteInput, &path, &e)
                    })?;
            }
            self.check(cache, FormatPhase::BeforeCase)
        })();
        phase.finish(result.is_ok());
        result
    }
    pub(super) fn check(&self, cache: &Path, phase: FormatPhase) -> Result<(), NativeAdapterFault> {
        let actual = inventory(&cache.join("Tectonic/formats"))?;
        let missing: Vec<_> = self
            .files
            .keys()
            .filter(|name| !actual.contains_key(*name))
            .cloned()
            .collect();
        let unexpected: Vec<_> = actual
            .keys()
            .filter(|name| !self.files.contains_key(*name))
            .cloned()
            .collect();
        let changed: Vec<_> = self
            .files
            .iter()
            .filter_map(|(name, bytes)| {
                actual
                    .get(name)
                    .filter(|value| *value != bytes)
                    .map(|value| FormatChange {
                        member: name.clone(),
                        expected: bytes.clone(),
                        actual: value.clone(),
                    })
            })
            .collect();
        if missing.is_empty() && unexpected.is_empty() && changed.is_empty() {
            Ok(())
        } else {
            Err(NativeAdapterFault::FormatState {
                phase,
                missing,
                unexpected,
                changed,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::{DocumentError, tests::Directory};

    #[test]
    fn format_copies_keep_exact_faults_and_restore_without_changing_seed() {
        let directory = Directory::new();
        let seed = Seed {
            files: BTreeMap::from([
                ("a.fmt".into(), b"first".to_vec()),
                ("b.fmt".into(), b"second".to_vec()),
            ]),
        };
        for phase in [
            FormatPhase::Seed,
            FormatPhase::BeforeCase,
            FormatPhase::AfterCase,
        ] {
            seed.copy(&directory.0).unwrap();
            let formats = directory.0.join("Tectonic/formats");
            fs::remove_file(formats.join("a.fmt")).unwrap();
            fs::write(formats.join("b.fmt"), b"changed").unwrap();
            fs::write(formats.join("c.fmt"), b"unexpected").unwrap();
            let fault = NativeAdapterFault::FormatState {
                phase: phase.clone(),
                missing: vec!["a.fmt".into()],
                unexpected: vec!["c.fmt".into()],
                changed: vec![FormatChange {
                    member: "b.fmt".into(),
                    expected: b"second".to_vec(),
                    actual: b"changed".to_vec(),
                }],
            };
            assert_eq!(seed.check(&directory.0, phase.clone()), Err(fault.clone()));
            assert_eq!(
                DocumentError::Native(fault.clone()).to_string(),
                format!("Native({fault:?})")
            );
            fs::remove_file(formats.join("c.fmt")).unwrap();
            seed.copy(&directory.0).unwrap();
            assert_eq!(fs::read(formats.join("a.fmt")).unwrap(), b"first");
            assert_eq!(fs::read(formats.join("b.fmt")).unwrap(), b"second");
            assert_eq!(seed.check(&directory.0, phase), Ok(()));
            assert_eq!(
                seed.files,
                BTreeMap::from([
                    ("a.fmt".into(), b"first".to_vec()),
                    ("b.fmt".into(), b"second".to_vec())
                ])
            );
        }
        let next = Directory::new();
        seed.copy(&next.0).unwrap();
        assert_eq!(seed.check(&next.0, FormatPhase::BeforeCase), Ok(()));
    }

    fn key(bootstrap: &str) -> Key {
        Key {
            engine: b"engine".to_vec(),
            lock: b"lock".to_vec(),
            inventory: BTreeMap::new(),
            digest: "bundle".into(),
            bootstrap: bootstrap.into(),
            options: OPTIONS.into(),
        }
    }

    #[test]
    fn format_reservations_publish_only_on_close_and_failed_creators_wake_waiters() {
        use std::{sync::mpsc, time::Duration};
        for success in [false, true] {
            let source = format!("private reservation {success}");
            let mut creator = Lease::acquire(key(&source));
            assert!(creator.seed.is_none());
            let (send, receive) = mpsc::channel();
            let waiting_source = source.clone();
            let waiter = std::thread::spawn(move || {
                let lease = Lease::acquire(key(&waiting_source));
                send.send(lease.seed.is_some()).unwrap();
            });
            creator.seed = Some(Arc::new(Seed {
                files: BTreeMap::from([("a.fmt".into(), b"seed".to_vec())]),
            }));
            assert_eq!(
                receive.recv_timeout(Duration::from_millis(30)),
                Err(mpsc::RecvTimeoutError::Timeout)
            );
            if success {
                creator.publish();
            }
            drop(creator);
            assert_eq!(receive.recv_timeout(Duration::from_secs(5)), Ok(success));
            waiter.join().unwrap();
        }
        let mut first = Lease::acquire(key("configuration original"));
        first.seed = Some(Arc::new(Seed {
            files: BTreeMap::new(),
        }));
        first.publish();
        for field in 0..6 {
            let mut changed = key("configuration original");
            match field {
                0 => changed.engine.push(0),
                1 => changed.lock.push(0),
                2 => {
                    changed.inventory.insert("member".into(), "digest".into());
                }
                3 => changed.digest.push('x'),
                4 => changed.bootstrap.push('x'),
                _ => changed.options.push('x'),
            }
            assert!(Lease::acquire(changed).seed.is_none());
            assert!(Lease::acquire(key("configuration original")).seed.is_some());
        }
    }

    #[test]
    fn support_required_format_copy_after_case_invalidates_bracket_and_restores()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::documents::{NativeCase, native};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
        let lock = native_support::lock(&root).map_err(|e| format!("{e:?}"))?;
        let configuration = "\\documentclass{article}\n\\begin{document}\nCopy isolation bootstrap.\n\\end{document}\n";
        let member = format!("{}-latex-33.fmt", lock.bundle_digest);
        let fault = NativeAdapterFault::FormatState {
            phase: FormatPhase::AfterCase,
            missing: vec![member.clone()],
            unexpected: vec!["extra.fmt".into()],
            changed: vec![],
        };
        let case = NativeCase::Document {
            document: "fixture".into(),
            language: "en".into(),
        };
        let mut evidence = None;
        let result = support.scope(&lock, || -> Result<_, DocumentError> {
            support.format_seed(&lock, configuration)?;
            native::consume(&support, &lock, BOOTSTRAP, "", "", case.clone(), vec![])?;
            let provisional = native::consume_with(
                &support,
                &lock,
                BOOTSTRAP,
                "",
                "",
                case.clone(),
                vec![],
                |_| {},
                |scratch, result| {
                    evidence = Some(result.clone());
                    let directory = scratch.join("cache/Tectonic/formats");
                    fs::rename(directory.join(&member), directory.join("extra.fmt")).unwrap();
                },
            );
            // Even a caller retaining a successful earlier value cannot publish
            // it after the copied-format check poisons this owning bracket.
            Ok(vec![Ok(()), provisional])
        });
        assert_eq!(evidence, Some(Ok(())));
        assert_eq!(result, Err(DocumentError::Native(fault.clone())));
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Native({fault:?})")
        );
        assert_eq!(support.closing_hash(), Some(Ok(())));
        assert!(
            !cache()
                .entries
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.key.bootstrap == configuration)
        );
        let restored = support.scope(&lock, || -> Result<_, DocumentError> {
            let seed = support.format_seed(&lock, configuration)?;
            native::consume(&support, &lock, BOOTSTRAP, "", "", case, vec![])?;
            Ok(seed.files.keys().cloned().collect::<Vec<_>>())
        })?;
        assert_eq!(restored, vec![member]);
        assert_eq!(support.closing_hash(), Some(Ok(())));
        assert!(
            cache()
                .entries
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.key.bootstrap == configuration && entry.seed.is_some())
        );
        Ok(())
    }

    #[test]
    fn support_required_format_configurations_build_privately_and_failed_bootstrap_publishes_nothing()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::documents::NativeProtocolReason;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let support = NativeSupport::from_environment(&root).map_err(|e| format!("{e:?}"))?;
        let lock = native_support::lock(&root).map_err(|e| format!("{e:?}"))?;
        let alternative = "\\documentclass{article}\n\\begin{document}\nAlternate bootstrap control.\n\\end{document}\n";
        let invalid = "\\documentclass{article}\n\\begin{document}\n\\UndefinedBootstrapControl\n\\end{document}\n";
        let evidence = support.scope(&lock, || -> Result<_, DocumentError> {
            support.ensure_open(&lock)?;
            let first = support.format_seed(&lock, BOOTSTRAP)?;
            let repeated = support.format_seed(&lock, BOOTSTRAP)?;
            let other = support.format_seed(&lock, alternative)?;
            let failed = support.format_seed(&lock, invalid).unwrap_err();
            let unpublished = !cache()
                .entries
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.key.bootstrap == invalid);
            let private = cache()
                .entries
                .lock()
                .unwrap()
                .iter()
                .any(|entry| entry.key.bootstrap == alternative && entry.seed.is_none());
            Ok((
                Arc::ptr_eq(&first, &repeated),
                Arc::ptr_eq(&first, &other),
                failed,
                unpublished,
                private,
                first.files == other.files,
            ))
        })?;
        assert_eq!(support.closing_hash(), Some(Ok(())));
        let raw = support.bootstrap_evidence.borrow_mut().take().unwrap();
        assert!(evidence.0);
        assert!(!evidence.1);
        assert!(evidence.3);
        assert!(evidence.4);
        assert!(evidence.5);
        let (status, stdout, stderr, log) =
            crate::documents::tests::native_evidence::raw_evidence("bootstrap");
        let stdout = stdout
            .replace("@SCRATCH@", raw.scratch.to_str().unwrap())
            .into_bytes();
        assert_eq!(raw.status, Some(status));
        assert_eq!(raw.stdout, stdout);
        assert_eq!(raw.stderr, stderr.as_bytes());
        assert_eq!(raw.log, log.as_bytes());
        let fault = NativeAdapterFault::Protocol {
            reason: NativeProtocolReason::UnexpectedStatus,
            status: Some(status),
            stdout,
            stderr: stderr.as_bytes().into(),
            log: log.as_bytes().into(),
        };
        let expected = DocumentError::Native(fault.clone());
        assert_eq!(evidence.2, expected);
        assert_eq!(evidence.2.to_string(), format!("Native({fault:?})"));
        Ok(())
    }
}
