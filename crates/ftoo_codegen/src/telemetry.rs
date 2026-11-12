//! Optional measurement records; nested phase walls are inclusive.
//! Logical requests include memo reuse; submitted requests count completed writes.
//! Starts count successful launches. A phase's success is set only at an explicit
//! completion boundary; an unfinished full verification is not a complete pass.
use std::{
    cell::Cell,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Mutex, Once, OnceLock},
    time::Instant,
};

thread_local! {
    static HASHED: Cell<(u64, u64, u64)> = const { Cell::new((0, 0, 0)) };
}
static OUTPUT: OnceLock<Option<Recorder<File>>> = OnceLock::new();
static INITIAL: Once = Once::new();

// Keep the validated destination open. Measurement is optional: a missing or
// unusable destination, a poisoned lock or a later write failure drops records.
// In particular, emitting a Phase from Drop must never panic during unwinding.
struct Recorder<W> {
    output: Mutex<Option<W>>,
}
impl<W: Write> Recorder<W> {
    fn record(&self, record: &serde_json::Value) {
        let Ok(mut output) = self.output.lock() else {
            return;
        };
        if let Some(writer) = output.as_mut()
            && writeln!(writer, "{record}").is_err()
        {
            *output = None;
        }
    }
}
impl Recorder<File> {
    fn open(directory: &Path) -> Option<Self> {
        if !directory.metadata().ok()?.is_dir() {
            return None;
        }
        let path = directory.join(format!("{}.jsonl", std::process::id()));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        Some(Self {
            output: Mutex::new(Some(file)),
        })
    }
}

pub(crate) fn counter(name: &'static str, count: usize) {
    emit(name, 0, (0, 0, 0), None, count);
}

fn emit(name: &str, ns: u128, hashed: (u64, u64, u64), success: Option<bool>, count: usize) {
    let Some(output) = OUTPUT.get_or_init(|| {
        let directory = std::env::var_os("FTOO_GATE_TELEMETRY")?;
        Recorder::open(Path::new(&directory))
    }) else {
        return;
    };
    let record = serde_json::json!({
        "pid": std::process::id(), "phase": name, "count": count,
        "inclusive_ns": ns, "members": hashed.0, "bytes": hashed.1,
        "hash_identity_observations": hashed.2,
        "success": success,
    });
    output.record(&record);
}

pub(crate) fn hashed(bytes: u64) {
    HASHED.with(|value| {
        let (members, total, observations) = value.get();
        value.set((members + 1, total + bytes, observations));
    });
}

pub(crate) fn hash_identity() {
    HASHED.with(|value| {
        let (members, bytes, observations) = value.get();
        value.set((members, bytes, observations + 1));
    });
}

pub(crate) struct Phase {
    name: &'static str,
    start: Instant,
    hashed: (u64, u64, u64),
    outcome: Option<bool>,
}
impl Phase {
    pub(crate) fn new(name: &'static str) -> Self {
        INITIAL.call_once(|| {
            counter("identity_checks", 0);
            counter("native.format_copies", 0);
        });
        Self {
            name,
            start: Instant::now(),
            hashed: HASHED.with(Cell::get),
            outcome: None,
        }
    }
    pub(crate) fn finish(mut self, success: bool) {
        self.outcome = Some(success);
    }
}
impl Drop for Phase {
    fn drop(&mut self) {
        let (members, bytes, observations) = HASHED.with(Cell::get);
        emit(
            self.name,
            self.start.elapsed().as_nanos(),
            (
                members - self.hashed.0,
                bytes - self.hashed.1,
                observations - self.hashed.2,
            ),
            self.outcome,
            1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    #[test]
    fn telemetry_unavailable_destination_drops_records() {
        assert!(Recorder::open(Path::new("/dev/null")).is_none());
        assert!(Recorder::open(Path::new("/dev/null/missing")).is_none());
        // The directory exists but the per-process destination cannot be made.
        assert!(Recorder::open(Path::new("/proc/self")).is_none());
    }

    #[test]
    fn telemetry_write_failure_disables_output() {
        struct Failed(Arc<AtomicUsize>);
        impl Write for Failed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                self.0.fetch_add(1, Ordering::Relaxed);
                Err(io::ErrorKind::StorageFull.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let writes = Arc::new(AtomicUsize::new(0));
        let recorder = Recorder {
            output: Mutex::new(Some(Failed(Arc::clone(&writes)))),
        };
        for _ in 0..2 {
            recorder.record(&serde_json::json!({"phase": "fixture"}));
        }
        assert_eq!(writes.load(Ordering::Relaxed), 1);
        assert!(recorder.output.lock().unwrap().is_none());
    }

    #[test]
    fn telemetry_keeps_complete_records_and_ignores_a_poisoned_lock() {
        let recorder = Recorder {
            output: Mutex::new(Some(Vec::new())),
        };
        let record = serde_json::json!({"phase": "fixture", "inclusive_ns": 17});
        recorder.record(&record);
        recorder.record(&record);
        assert_eq!(
            recorder.output.lock().unwrap().as_ref().unwrap(),
            format!("{record}\n{record}\n").as_bytes(),
        );
        let panic = std::panic::catch_unwind(|| {
            let _guard = recorder.output.lock().unwrap();
            panic!("injected lock poisoning");
        });
        assert!(panic.is_err());
        recorder.record(&record);
    }
}
