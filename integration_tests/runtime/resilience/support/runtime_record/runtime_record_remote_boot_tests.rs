use super::*;
use yolo_runtime::model::{BootstrapDelivery, ExitReason, ExitRecord};

const REMOTE_BOOT_ID: &str = "remote-boot-authority";
const OTHER_REMOTE_BOOT_ID: &str = "other-remote-boot-authority";
const UNKNOWN_REMOTE_BOOT_ID: &str = "unknown:remote-boot-authority";
const TEST_COMMAND: &str = "/bin/cat";
const TEST_PROCESS_ARGUMENT: &str = "60";
const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";
const TEST_AUTH_TOKEN: &str = "remote-cleanup-auth";
const TEST_GENERATION: u64 = 1;
const TEST_TIMESTAMP: i64 = i64::MIN;
const TEST_SEQUENCE: u64 = u64::MIN;

#[test]
fn remote_rescan_uses_authenticated_boot_without_observer_alias() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let process = SettledProcess::new();
    let record = fixture_record(&paths, REMOTE_BOOT_ID, process.identity);
    let (mut cleanup, capture) = RuntimeCleanup::capture_for_boot(&paths, REMOTE_BOOT_ID);
    capture.unwrap();
    process.settle();
    write_record_and_exit(&paths, &record);

    cleanup.rescan_for_boot(&paths, REMOTE_BOOT_ID).unwrap();
    cleanup.finish().unwrap();

    let (mut local_cleanup, capture) = RuntimeCleanup::capture(&paths);
    assert!(capture.is_err());
    assert!(local_cleanup.finish().is_err());
}

#[test]
fn remote_boot_authority_rejects_mismatch_and_unknown_alias() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let process = SettledProcess::new();
    let record = fixture_record(&paths, REMOTE_BOOT_ID, process.identity);
    process.settle();
    write_record_and_exit(&paths, &record);

    let (mut mismatched, capture) = RuntimeCleanup::capture_for_boot(&paths, OTHER_REMOTE_BOOT_ID);
    assert!(capture.is_err());
    assert!(mismatched.finish().is_err());

    let unknown = fixture_record(&paths, UNKNOWN_REMOTE_BOOT_ID, process.identity);
    write_record_and_exit(&paths, &unknown);
    let (mut aliased, capture) = RuntimeCleanup::capture_for_boot(&paths, UNKNOWN_REMOTE_BOOT_ID);
    assert!(capture.is_err());
    assert!(aliased.finish().is_err());
}

fn fixture_record(paths: &Paths, boot_id: &str, process: ProcessIdentity) -> RuntimeRecord {
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process.pid,
        worker_start_time: process.start_time,
        child_pid: process.pid,
        child_start_time: process.start_time,
        boot_id: boot_id.into(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: TEST_GENERATION,
        started_at_ms: TEST_TIMESTAMP,
        command: TEST_COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_path_buf(),
        cols: yolo_runtime::model::MIN_TERMINAL_COLUMNS,
        rows: yolo_runtime::model::MIN_TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    }
}

struct SettledProcess {
    child: std::cell::RefCell<Option<Child>>,
    identity: ProcessIdentity,
}

impl SettledProcess {
    fn new() -> Self {
        let child = Command::new(TEST_PROCESS_EXECUTABLE)
            .arg(TEST_PROCESS_ARGUMENT)
            .spawn()
            .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child: std::cell::RefCell::new(Some(child)),
            identity,
        }
    }

    fn settle(&self) {
        assert!(self.identity.signal(libc::SIGKILL).unwrap());
        self.child.borrow_mut().take().unwrap().wait().unwrap();
    }
}

impl Drop for SettledProcess {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        if let Some(mut child) = self.child.get_mut().take() {
            child.wait().unwrap();
        }
    }
}

fn write_record_and_exit(paths: &Paths, record: &RuntimeRecord) {
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), record)
        .unwrap();
    let exit = ExitRecord {
        runtime_epoch: record.runtime_epoch,
        code: Some(i32::default()),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: TEST_TIMESTAMP,
        last_output_sequence: TEST_SEQUENCE,
    };
    yolo_runtime::paths::atomic_write_json(&paths.exit_record(record.terminal_id), &exit).unwrap();
}
