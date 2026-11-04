use super::*;
use crate::support::runtime_record_test_support::{bind_record, record_for_identity, session_view};

pub(super) const SUCCESSOR_GENERATION: u64 = 2;
pub(super) const FOREIGN_GENERATION: u64 = SUCCESSOR_GENERATION + 1;
pub(super) const QUERY_FAILURE: &str = "injected adoption view failure";
pub(super) const REPLACEMENT_SUFFIX: &str = "-replacement";

pub(super) struct SnapshotFixture {
    pub(super) directory: TempDir,
    pub(super) paths: Paths,
    pub(super) expected: RuntimeAdoptionExpectation,
    pub(super) record: RuntimeRecord,
}

impl SnapshotFixture {
    pub(super) fn new() -> Self {
        Self::with_bootstrap(yolo_runtime::model::BootstrapDelivery::NotRequired)
    }

    pub(super) fn with_bootstrap(bootstrap: yolo_runtime::model::BootstrapDelivery) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::new(directory.path());
        let identity = ProcessIdentity::capture(std::process::id()).unwrap();
        let mut record = record_for_identity(&paths, Uuid::new_v4(), identity);
        record.bootstrap_delivery = bootstrap;
        let expected = RuntimeAdoptionExpectation::new(bind_record(&paths, record.clone()));
        record.controller_generation = SUCCESSOR_GENERATION;
        let fixture = Self {
            directory,
            paths,
            expected,
            record,
        };
        fixture.publish(&fixture.record);
        fixture
    }

    pub(super) fn publish(&self, record: &RuntimeRecord) {
        yolo_runtime::paths::atomic_write_json(
            &self.paths.runtime_record(self.record.terminal_id),
            record,
        )
        .unwrap();
    }

    pub(super) fn view(&self) -> TerminalView {
        session_view(&self.record)
    }

    pub(super) fn changed_dimensions(&self) -> RuntimeRecord {
        let mut changed = self.record.clone();
        changed.rows += 1;
        changed
    }

    pub(super) fn query_error() -> ClientError {
        ClientError::Remote {
            code: yolo_runtime::protocol::error_code::WORKER_UNREACHABLE.into(),
            message: QUERY_FAILURE.into(),
            retryable: true,
            details: None,
        }
    }
}
