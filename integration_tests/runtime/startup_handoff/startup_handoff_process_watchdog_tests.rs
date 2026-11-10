use super::{MARKER_TIMEOUT_DETAIL, MARKER_WORKER_EXIT_DETAIL, WorkerWatchdog};
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;
use yolo_runtime::model::BootstrapDelivery;
use yolo_runtime::paths::atomic_write_json;
use yolo_runtime::worker::startup_handoff_test_hook::StartupHandoffMarker;

const SHELL_PATH: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const LIVE_COMMAND: &str = "sleep 1";
const EARLY_EXIT_DELAY_SECONDS: &str = "0.05";
const EARLY_EXIT_CODE: u8 = 7;
const EARLY_EXIT_DIAGNOSTIC: &str = "bounded startup diagnostic";
const MARKER_FILE: &str = "expected.json";
const FAILURE_MARKER_FILE: &str = "failure.json";
const EARLY_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const LIVE_TIMEOUT: Duration = Duration::from_millis(30);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn marker_arrives_while_worker_lives() {
    let fixture = TestWatchdog::spawn(LIVE_COMMAND);
    let marker = test_marker();
    atomic_write_json(&fixture.marker, &marker).unwrap();

    let mut worker = fixture.worker;
    assert_eq!(
        worker.wait_marker(&fixture.marker, LIVE_TIMEOUT).unwrap(),
        marker
    );
    terminate(&mut worker);
}

#[test]
fn worker_exit_before_marker_reports_status_stderr_and_path() {
    let command = format!(
        "sleep {EARLY_EXIT_DELAY_SECONDS}; printf '%s' '{EARLY_EXIT_DIAGNOSTIC}' >&2; exit {EARLY_EXIT_CODE}"
    );
    let fixture = TestWatchdog::spawn(&command);
    let mut worker = fixture.worker;

    let detail = worker
        .wait_marker(&fixture.marker, EARLY_EXIT_TIMEOUT)
        .unwrap_err()
        .to_string();

    assert!(detail.contains(MARKER_WORKER_EXIT_DETAIL));
    assert!(detail.contains(&EARLY_EXIT_CODE.to_string()));
    assert!(detail.contains(EARLY_EXIT_DIAGNOSTIC));
    assert!(detail.contains(&fixture.marker.display().to_string()));
}

#[test]
fn live_worker_marker_timeout_reports_expected_path() {
    let fixture = TestWatchdog::spawn(LIVE_COMMAND);
    let mut worker = fixture.worker;

    let detail = worker
        .wait_marker(&fixture.marker, LIVE_TIMEOUT)
        .unwrap_err()
        .to_string();

    assert!(detail.contains(MARKER_TIMEOUT_DETAIL));
    assert!(detail.contains(&fixture.marker.display().to_string()));
    terminate(&mut worker);
}

struct TestWatchdog {
    _directory: TempDir,
    marker: std::path::PathBuf,
    worker: WorkerWatchdog,
}

impl TestWatchdog {
    fn spawn(command: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join(MARKER_FILE);
        let failure_marker = directory.path().join(FAILURE_MARKER_FILE);
        let child = Command::new(SHELL_PATH)
            .arg(SHELL_COMMAND_ARGUMENT)
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let worker = WorkerWatchdog::new(child, failure_marker).unwrap();
        Self {
            _directory: directory,
            marker,
            worker,
        }
    }
}

fn terminate(worker: &mut WorkerWatchdog) {
    yolo_process_test_support::send_signal(worker.identity.pid, libc::SIGKILL).unwrap();
    worker.wait_for_exit(CLEANUP_TIMEOUT).unwrap();
}

fn test_marker() -> StartupHandoffMarker {
    StartupHandoffMarker {
        run_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_pid: std::process::id(),
        worker_start_time: 1,
        child_pid: std::process::id(),
        child_start_time: 1,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}
