use super::*;
use std::process::Command;

const CHILD_READY_CONTENT: &str = "ready";
const CHILD_SCRIPT: &str = "trap '' HUP; printf ready > \"$1\"; while :; do :; done";
const CHILD_SCRIPT_ARGUMENT: &str = "-c";
const CHILD_SCRIPT_ROLE: &str = "yolo-startup-untracked";
const CHILD_SHELL: &str = "/bin/sh";
const CHILD_EXPECTED_SIGNAL: i32 = libc::SIGKILL;
const CHILD_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const CHILD_WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
const CHILD_SETTLEMENT_TIMEOUT: std::time::Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;

#[test]
#[cfg(not(target_os = "linux"))]
fn untracked_hup_resistant_child_is_identified_forced_and_reaped() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(CHILD_READY_CONTENT);
    let child = crate::process::spawn_waitable_test_child(
        Command::new(CHILD_SHELL)
            .args([CHILD_SCRIPT_ARGUMENT, CHILD_SCRIPT, CHILD_SCRIPT_ROLE])
            .arg(&marker),
    );
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let mut guard = StartupChildGuard::new(Box::new(child));
    let completion = guard
        .fault_reaper
        .as_mut()
        .unwrap()
        .observe_startup_completion();
    let deadline = std::time::Instant::now() + CHILD_TEST_TIMEOUT;
    while !marker.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(CHILD_WAIT_INTERVAL);
    }

    drop(guard);

    let outcome = completion
        .receive_timeout(CHILD_SETTLEMENT_TIMEOUT)
        .unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert_eq!(
        outcome.status.unwrap().signal(),
        Some(CHILD_EXPECTED_SIGNAL)
    );
    assert!(!identity.matches());
}

#[cfg(target_os = "linux")]
#[path = "startup_untracked_linux_tests.rs"]
mod linux;
