use super::cleanup_tree::RuntimeProcessTrees;
use super::runtime_record_test_support::bind_record;
use super::*;
use yolo_runtime::model::{BootstrapDelivery, ExitReason, ExitRecord};

const TEST_PROCESS_EXECUTABLE: &str = "/bin/sleep";
const TEST_PROCESS_ARGUMENT: &str = "60";
const TEST_AUTH_TOKEN: &str = "cleanup-tree-auth";
const INITIAL_GENERATION: u64 = 1;
const RESUMED_GENERATION: u64 = INITIAL_GENERATION + 1;
const TEST_TIMESTAMP: i64 = i64::MIN;
const TEST_SEQUENCE: u64 = u64::MIN;

#[test]
fn settled_untracked_runtime_requires_its_exact_exit_epoch() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let process = SettledProcess::new();
    let record = fixture_record(&paths, Uuid::new_v4(), &process, INITIAL_GENERATION);
    let bound = bind_record(&paths, record.clone());
    process.settle();
    write_exit(&paths, record.terminal_id, Uuid::new_v4());
    let mut trees = RuntimeProcessTrees::default();

    assert!(
        trees
            .ensure_cleanup_records(&paths, std::slice::from_ref(&bound))
            .is_err()
    );

    write_exit(&paths, record.terminal_id, record.runtime_epoch);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(record.terminal_id), &record)
        .unwrap();
    let (mut cleanup, capture) = RuntimeCleanup::capture(&paths);
    capture.unwrap();
    cleanup.request();
    cleanup.force();
    cleanup.finish().unwrap();
}

#[test]
fn resumed_generation_accepts_only_the_settled_untracked_successor() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let predecessor = SettledProcess::new();
    let previous = fixture_record(&paths, terminal_id, &predecessor, INITIAL_GENERATION);
    let tracked = bind_record(&paths, previous.clone());
    let mut trees = RuntimeProcessTrees::default();
    trees
        .ensure_records(std::slice::from_ref(&tracked))
        .unwrap();
    predecessor.settle();
    let successor = SettledProcess::new();
    let resumed = fixture_record(&paths, terminal_id, &successor, RESUMED_GENERATION);
    let resumed_bound = bind_record(&paths, resumed.clone());
    successor.settle();
    write_exit(&paths, terminal_id, previous.runtime_epoch);

    assert!(
        trees
            .ensure_cleanup_records(&paths, &[tracked.clone(), resumed_bound.clone()])
            .is_err()
    );

    write_exit(&paths, terminal_id, resumed.runtime_epoch);
    yolo_runtime::paths::atomic_write_json(&paths.runtime_record(terminal_id), &resumed).unwrap();
    let (mut cleanup, capture) = RuntimeCleanup::with_trees(&paths, &[tracked], trees);
    capture.unwrap();
    cleanup.request();
    cleanup.force();
    cleanup.finish().unwrap();
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

fn fixture_record(
    paths: &Paths,
    terminal_id: Uuid,
    process: &SettledProcess,
    generation: u64,
) -> RuntimeRecord {
    paths.ensure_session(terminal_id).unwrap();
    RuntimeRecord {
        schema_version: yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process.identity.pid,
        worker_start_time: process.identity.start_time,
        child_pid: process.identity.pid,
        child_start_time: process.identity.start_time,
        boot_id: yolo_process_test_support::boot_id(),
        socket_path: paths.worker_socket(terminal_id).unwrap(),
        auth_token: TEST_AUTH_TOKEN.into(),
        controller_generation: generation,
        started_at_ms: TEST_TIMESTAMP,
        command: TEST_PROCESS_EXECUTABLE.into(),
        args: vec![TEST_PROCESS_ARGUMENT.into()],
        cwd: paths.home().to_path_buf(),
        cols: yolo_runtime::model::MIN_TERMINAL_COLUMNS,
        rows: yolo_runtime::model::MIN_TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
    }
}

fn write_exit(paths: &Paths, terminal_id: Uuid, runtime_epoch: Uuid) {
    let exit = ExitRecord {
        runtime_epoch,
        code: None,
        signal: Some(libc::SIGKILL),
        reason: ExitReason::UserTerminate,
        observed_at_ms: TEST_TIMESTAMP,
        last_output_sequence: TEST_SEQUENCE,
    };
    yolo_runtime::paths::atomic_write_json(&paths.exit_record(terminal_id), &exit).unwrap();
}
