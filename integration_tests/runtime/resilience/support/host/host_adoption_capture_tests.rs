use super::*;

const CORRUPT_EXIT_EVIDENCE: &[u8] = b"not-exit-evidence-json";
const CORRUPT_TERMINATION_EVIDENCE: &[u8] = b"not-termination-evidence-json";

struct CaptureFixture {
    _directory: TempDir,
    paths: Paths,
    record: RuntimeRecord,
    bound: runtime_record_authority::BoundRuntimeRecord,
}

impl CaptureFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::new(directory.path());
        let identity = ProcessIdentity::capture(std::process::id()).unwrap();
        let record = super::super::super::runtime_record_test_support::record_for_identity(
            &paths,
            Uuid::new_v4(),
            identity,
        );
        yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
            .unwrap();
        let bound =
            super::super::super::runtime_record_test_support::bind_record(&paths, record.clone());
        Self {
            _directory: directory,
            paths,
            record,
            bound,
        }
    }

    fn capture(&self) -> Result<Option<RuntimeAdoptionExpectation>, String> {
        capture_expectation(&self.paths, &self.bound)
    }

    fn write_exit(&self, runtime_epoch: Uuid) {
        let exit = ExitRecord {
            runtime_epoch,
            code: None,
            signal: None,
            reason: yolo_runtime::model::ExitReason::Natural,
            observed_at_ms: i64::default(),
            last_output_sequence: u64::default(),
        };
        yolo_runtime::paths::atomic_write_json(
            &self.paths.exit_record(self.record.terminal_id),
            &exit,
        )
        .unwrap();
    }

    fn write_tombstone(&self, runtime_epoch: Uuid) {
        let tombstone = yolo_runtime::TerminationTombstone::user_request(
            self.record.terminal_id,
            runtime_epoch,
            false,
        );
        yolo_runtime::paths::atomic_write_json(
            &self.paths.terminate_tombstone(self.record.terminal_id),
            &tombstone,
        )
        .unwrap();
    }
}

#[test]
fn matching_exit_evidence_excludes_runtime_from_adoption() {
    let fixture = CaptureFixture::new();
    fixture.write_exit(fixture.record.runtime_epoch);
    assert!(fixture.capture().unwrap().is_none());
}

#[test]
fn matching_tombstone_excludes_runtime_from_adoption() {
    let fixture = CaptureFixture::new();
    fixture.write_tombstone(fixture.record.runtime_epoch);
    assert!(fixture.capture().unwrap().is_none());
}

#[test]
fn corrupt_exit_evidence_fails_closed() {
    let fixture = CaptureFixture::new();
    std::fs::write(
        fixture.paths.exit_record(fixture.record.terminal_id),
        CORRUPT_EXIT_EVIDENCE,
    )
    .unwrap();
    assert!(fixture.capture().is_err());
}

#[test]
fn mismatched_exit_evidence_fails_closed() {
    let fixture = CaptureFixture::new();
    fixture.write_exit(Uuid::new_v4());
    assert!(fixture.capture().is_err());
}

#[test]
fn corrupt_tombstone_fails_closed() {
    let fixture = CaptureFixture::new();
    std::fs::write(
        fixture.paths.terminate_tombstone(fixture.record.terminal_id),
        CORRUPT_TERMINATION_EVIDENCE,
    )
    .unwrap();
    assert!(fixture.capture().is_err());
}

#[test]
fn mismatched_tombstone_fails_closed() {
    let fixture = CaptureFixture::new();
    fixture.write_tombstone(Uuid::new_v4());
    assert!(fixture.capture().is_err());
}
