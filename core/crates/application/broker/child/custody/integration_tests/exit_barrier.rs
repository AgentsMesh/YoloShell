use crate::pending::{PendingBroker, install_termination_failure};
use std::process::{Command, Stdio};
use std::time::Instant;
use yolo_process::is_process_alive;

const HELPER_MODE_ENVIRONMENT: &str = "YOLO_TERMINATION_EXIT_BARRIER_HELPER";
const HELPER_PID_FILE_ENVIRONMENT: &str = "YOLO_TERMINATION_EXIT_BARRIER_PID_FILE";
const HELPER_MODE_VALUE: &str = "enabled";
const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const INJECTED_TERMINATION_FAILURE: &str = "injected pre-exit termination failure";

#[test]
fn start_exit_waits_for_terminal_delivery_after_a_transient_kill_failure() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("child.pid");
    let status = Command::new(std::env::current_exe().unwrap())
        .arg(crate::fixture_constants::EXIT_BARRIER_HELPER_FILTER)
        .arg(crate::fixture_constants::TEST_HARNESS_EXACT_ARGUMENT)
        .arg(crate::fixture_constants::TEST_HARNESS_NOCAPTURE_ARGUMENT)
        .env(HELPER_MODE_ENVIRONMENT, HELPER_MODE_VALUE)
        .env(HELPER_PID_FILE_ENVIRONMENT, &pid_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .unwrap();
    let process_id = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<u32>()
        .unwrap();

    assert!(status.success());
    assert!(!is_process_alive(process_id));
}

#[test]
fn termination_exit_barrier_subprocess_helper() {
    if std::env::var(HELPER_MODE_ENVIRONMENT).as_deref() != Ok(HELPER_MODE_VALUE) {
        return;
    }
    let pid_file = std::env::var_os(HELPER_PID_FILE_ENVIRONMENT).unwrap();
    let child = Command::new(CHILD_EXECUTABLE)
        .arg(CHILD_DURATION)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let process_id = child.id();
    std::fs::write(pid_file, process_id.to_string()).unwrap();
    let _failure = install_termination_failure(INJECTED_TERMINATION_FAILURE);
    let error = PendingBroker::detached_for_test(child)
        .compensate()
        .unwrap_err();

    assert!(error.to_string().contains(INJECTED_TERMINATION_FAILURE));
    let settlement_deadline = Instant::now()
        .checked_add(yolo_remote_broker::BROKER_READY_TIMEOUT)
        .unwrap();
    let completion =
        crate::finish_after_termination_delivery(Err::<(), _>(error), settlement_deadline);
    assert!(
        completion
            .unwrap_err()
            .to_string()
            .contains(INJECTED_TERMINATION_FAILURE)
    );
    crate::pending::wait_for_custody_settlement(settlement_deadline).unwrap();
    assert!(!is_process_alive(process_id));
}
