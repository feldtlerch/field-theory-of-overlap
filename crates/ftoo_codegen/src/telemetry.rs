//! Optional measurement records; nested phase walls are inclusive.
//! Logical requests include memo reuse; submitted requests count completed writes.
//! Starts count successful launches. A phase's success is set only at an explicit
//! completion boundary; an unfinished full verification is not a complete pass.
use std::{
    cell::Cell,
    fs::OpenOptions,
    io::Write,
    sync::{Mutex, Once},
    time::Instant,
};

thread_local! {
    static HASHED: Cell<(u64, u64)> = const { Cell::new((0, 0)) };
}
static OUTPUT: Mutex<()> = Mutex::new(());
static INITIAL: Once = Once::new();

pub(crate) fn counter(name: &'static str, count: usize) {
    emit(name, 0, (0, 0), None, count);
}

fn emit(name: &str, ns: u128, hashed: (u64, u64), success: Option<bool>, count: usize) {
    let Some(directory) = std::env::var_os("FTOO_GATE_TELEMETRY") else {
        return;
    };
    let _guard = OUTPUT.lock().expect("telemetry lock");
    let record = serde_json::json!({
        "pid": std::process::id(), "phase": name, "count": count,
        "inclusive_ns": ns, "members": hashed.0, "bytes": hashed.1,
        "success": success,
    });
    let path = std::path::Path::new(&directory).join(format!("{}.jsonl", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("telemetry output");
    writeln!(file, "{record}").expect("telemetry record");
}

pub(crate) fn hashed(bytes: u64) {
    HASHED.with(|value| {
        let (members, total) = value.get();
        value.set((members + 1, total + bytes));
    });
}

pub(crate) struct Phase {
    name: &'static str,
    start: Instant,
    hashed: (u64, u64),
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
        let (members, bytes) = HASHED.with(Cell::get);
        emit(
            self.name,
            self.start.elapsed().as_nanos(),
            (members - self.hashed.0, bytes - self.hashed.1),
            self.outcome,
            1,
        );
    }
}
