use super::*;
use std::io;
use std::process::Command;
use std::time::Duration;

const TEST_EXIT_EXECUTABLE: &str = crate::test_executables::TRUE;
const TEST_KILL_EXECUTABLE: &str = "/bin/kill";
const TEST_SLEEP_ARGUMENT: &str = "60";
const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_REAP_ATTEMPTS: usize = 100;
const TEST_REAP_INTERVAL: Duration = Duration::from_millis(10);
const WAIT_FOR_EXIT: libc::c_int = 0;
const WAIT_WITHOUT_REAP: libc::c_int = libc::WEXITED | libc::WNOWAIT | libc::WNOHANG;
const WAIT_SUCCESS: libc::c_int = 0;
const NO_EXIT_PID: libc::pid_t = 0;

#[test]
fn drop_poll_failure_retries_termination_under_reserved_custody() {
    let child = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let process_id = child.id();
    let mut spawned = SpawnedHost::with_poll_errors(child);
    let error = spawned.try_wait_status().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);

    drop(spawned);

    wait_until_reaped(process_id);
}

#[test]
fn externally_reaped_child_reports_echild_before_drop() {
    let mut child = Command::new(TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let process_id = child.id();
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(process_id as libc::pid_t, &mut status, WAIT_FOR_EXIT) };
    assert_eq!(waited, process_id as libc::pid_t);
    let error = Child::try_wait(&mut child).unwrap_err();
    assert!(yolo_runtime::process::is_child_already_reaped(&error));

    drop(SpawnedHost::new(child));
}

#[test]
fn termination_failure_retries_under_reserved_custody() {
    let child = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let process_id = child.id();

    drop(SpawnedHost::with_termination_error(child));

    wait_until_reaped(process_id);
}

#[test]
fn child_status_and_preservation_retire_exact_wait_authority() {
    let child = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let process_id = child.id();
    let mut spawned = SpawnedHost::new(child);
    assert_eq!(spawned.process_id(), Some(process_id));
    assert_eq!(spawned.try_wait_status().unwrap(), None);

    spawned.preserve();
    assert_eq!(spawned.process_id(), None);
    assert_eq!(spawned.try_wait_status().unwrap(), None);
    drop(spawned);
    terminate_fixture(process_id);
    reap_direct_fixture(process_id);
}

#[test]
fn child_status_reaps_exit_and_remains_retired() {
    let child = Command::new(TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let process_id = child.id();
    let mut spawned = SpawnedHost::new(child);
    let mut status = None;
    for _ in 0..TEST_REAP_ATTEMPTS {
        status = spawned.try_wait_status().unwrap();
        if status.is_some() {
            break;
        }
        std::thread::sleep(TEST_REAP_INTERVAL);
    }
    assert!(status.is_some());
    assert_eq!(spawned.process_id(), None);
    assert_eq!(spawned.try_wait_status().unwrap(), None);
    drop(spawned);
    assert!(!yolo_runtime::process::is_process_alive(process_id));
}

#[test]
fn drop_terminates_running_child_and_reaps_observed_exit() {
    let running = Command::new(TEST_SLEEP_EXECUTABLE)
        .arg(TEST_SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let running_id = running.id();
    drop(SpawnedHost::new(running));
    assert!(!yolo_runtime::process::is_process_alive(running_id));

    let exited = Command::new(TEST_EXIT_EXECUTABLE).spawn().unwrap();
    let exited_id = exited.id();
    wait_until_exited_without_reaping(exited_id);
    drop(SpawnedHost::new(exited));
    assert!(!yolo_runtime::process::is_process_alive(exited_id));
}

fn terminate_fixture(process_id: u32) {
    let status = Command::new(TEST_KILL_EXECUTABLE)
        .arg(process_id.to_string())
        .status()
        .unwrap();
    assert!(status.success());
}

fn wait_until_reaped(process_id: u32) {
    for _ in 0..TEST_REAP_ATTEMPTS {
        if !yolo_runtime::process::is_process_alive(process_id) {
            return;
        }
        std::thread::sleep(TEST_REAP_INTERVAL);
    }
    panic!("reserved child settlement did not complete");
}

fn reap_direct_fixture(process_id: u32) {
    let mut status = libc::c_int::default();
    let waited = unsafe { libc::waitpid(process_id as libc::pid_t, &mut status, WAIT_FOR_EXIT) };
    assert_eq!(waited, process_id as libc::pid_t);
}

fn wait_until_exited_without_reaping(process_id: u32) {
    for _ in 0..TEST_REAP_ATTEMPTS {
        let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                process_id as libc::id_t,
                information.as_mut_ptr(),
                WAIT_WITHOUT_REAP,
            )
        };
        assert_eq!(result, WAIT_SUCCESS);
        if unsafe { information.assume_init().si_pid() } != NO_EXIT_PID {
            return;
        }
        std::thread::sleep(TEST_REAP_INTERVAL);
    }
    panic!("child did not exit before the test deadline");
}
