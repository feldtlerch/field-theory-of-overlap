use super::{AcceptedRetention, Produced, RetentionError, acceptance};
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Private file boundary shared by assertion and stale-input controls.
pub(super) struct FileFixture {
    path: PathBuf,
    production: Arc<Produced>,
}
impl FileFixture {
    pub(super) fn new(production: &Arc<Produced>) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let fixture = Self {
            path: std::env::temp_dir().join(format!(
                "retention-assertion-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )),
            production: Arc::clone(production),
        };
        fixture.restore();
        fixture
    }
    pub(super) fn write_read(&self, bytes: &[u8]) -> Vec<u8> {
        fs::write(&self.path, bytes).unwrap();
        fs::read(&self.path).unwrap()
    }
    pub(super) fn restore(&self) {
        let accepted = acceptance::accept_with(
            self.production.key.projection.clone(),
            self.write_read(&self.production.bytes),
            &mut VecDeque::new(),
            &mut |_, _| Ok(Arc::clone(&self.production)),
        )
        .unwrap();
        assert_eq!(accepted.bytes(), self.production.bytes);
    }
    pub(super) fn refuse_before_production(
        &self,
        current: Vec<u8>,
        retained: &[u8],
    ) -> Result<AcceptedRetention, RetentionError> {
        let result = acceptance::accept_with(
            current,
            self.write_read(retained),
            &mut VecDeque::new(),
            &mut |_, _| panic!("prerequisite refusal precedes production"),
        );
        self.restore();
        result
    }
}
impl Drop for FileFixture {
    fn drop(&mut self) {
        fs::remove_file(&self.path).unwrap();
    }
}
