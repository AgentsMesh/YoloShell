use super::*;
use crate::fixture_constants as fixture;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SELF_KILL_SCRIPT: &str = "kill -KILL $$";

#[test]
fn compensation_terminates_a_running_child_and_accepts_an_exited_child() {
    let running = Command::new(fixture::TEST_SLEEP_EXECUTABLE)
        .arg(fixture::TEST_SLEEP_DURATION)
        .spawn()
        .unwrap();
    PendingBroker::detached_for_test(running)
        .compensate()
        .unwrap();

    let mut exited = Command::new(fixture::TEST_EXIT_EXECUTABLE).spawn().unwrap();
    exited.wait().unwrap();
    PendingBroker::detached_for_test(exited)
        .compensate()
        .unwrap();
}

#[test]
fn successful_handoff_transfers_a_quick_exit_to_the_reaper() {
    let child = Command::new(fixture::TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let process_id = child.id();

    PendingBroker::detached_for_test(child).adopt().unwrap();

    for _ in 0..fixture::BROKER_REAP_WAIT_ATTEMPTS {
        if !yolo_process::is_process_alive(process_id) {
            break;
        }
        std::thread::sleep(fixture::BROKER_REAP_WAIT_INTERVAL);
    }
    assert!(!yolo_process::is_process_alive(process_id));
}

#[test]
fn failed_handoff_compensates_with_kill_and_wait() {
    let child = Command::new(fixture::TEST_SLEEP_EXECUTABLE)
        .arg(fixture::TEST_SLEEP_DURATION)
        .spawn()
        .unwrap();
    let process_id = child.id();
    let source = io::Error::other(fixture::TEST_REAPER_HANDOFF_FAILURE);
    let termination = reserve_termination_custody().unwrap();

    let error = PendingBroker::unmanaged()
        .compensate_handoff_failure(source.into(), child, termination)
        .unwrap_err();

    assert!(error.to_string().contains(HANDOFF_CHILD_CONTEXT));
    for _ in 0..fixture::BROKER_REAP_WAIT_ATTEMPTS {
        if !yolo_process::is_process_alive(process_id) {
            break;
        }
        std::thread::sleep(fixture::BROKER_REAP_WAIT_INTERVAL);
    }
    assert!(!yolo_process::is_process_alive(process_id));
}

#[test]
fn bounded_completion_preserves_an_exact_signal_exit() {
    let child = Command::new(SHELL_EXECUTABLE)
        .arg(SHELL_COMMAND_ARGUMENT)
        .arg(SELF_KILL_SCRIPT)
        .spawn()
        .unwrap();

    let status = PendingBroker::detached_for_test(child)
        .wait_for_exit(fixture::START_TIMEOUT)
        .unwrap();

    assert_eq!(status.signal(), Some(libc::SIGKILL));
}

#[test]
fn bounded_completion_terminates_and_reaps_a_hung_child() {
    let child = Command::new(fixture::TEST_SLEEP_EXECUTABLE)
        .arg(fixture::TEST_SLEEP_DURATION)
        .spawn()
        .unwrap();
    let process_id = child.id();

    let error = PendingBroker::detached_for_test(child)
        .wait_for_exit(yolo_remote_broker::BROKER_READY_POLL_INTERVAL)
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(super::wait::WAIT_CHILD_TIMEOUT_DETAIL)
    );
    assert!(!yolo_process::is_process_alive(process_id));
}
