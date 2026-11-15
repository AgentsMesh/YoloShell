use super::*;
use std::process::Command;

const EXIT_COMMAND: &str = "exit 23";
const TEST_EXIT_CODE: i32 = 23;
const SHELL_ARGUMENT: &str = "-c";
const SHELL_PATH: &str = "/bin/sh";

#[test]
fn direct_waitpid_reaps_once_without_portable_child_callbacks() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(SHELL_PATH).args([SHELL_ARGUMENT, EXIT_COMMAND]),
    );
    let process_id = child.id();

    let outcome = wait_direct_child(process_id);

    assert!(outcome.fault.is_none());
    assert_eq!(outcome.status.unwrap().code(), Some(TEST_EXIT_CODE));
    drop(child);
    let repeated = wait_direct_child(process_id);
    assert!(repeated.status.is_none());
    assert_eq!(
        repeated
            .fault
            .unwrap()
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn invalid_direct_child_pid_fails_without_waiting() {
    let outcome = wait_direct_child(0);

    assert!(outcome.status.is_none());
    assert!(outcome.fault.is_some());
}
