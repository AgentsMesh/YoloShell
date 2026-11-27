use super::child_lifecycle::{
    EXIT_EXECUTABLE, INVALID_SIGNAL, SLEEP_ARGUMENT, SLEEP_EXECUTABLE, assert_reaped, bare_process,
};
use super::process_supervision_tests::Fixture;
use crate::constants::NULL_DEVICE;
use crate::posix_spawn::SpawnedProcess;
use std::fs::File;
use std::process::Command;
use std::time::{Duration, Instant};

const INJECTED_DROP_REAP_GRACE: Duration = Duration::from_millis(25);
const DROP_COMPLETION_LIMIT: Duration = Duration::from_secs(2);
const CUSTODY_DROP_DESCRIPTION: &str = "drop custody";
const GROUP_SIGNAL_ERROR: &str = "terminate child process group";
const DIRECT_SIGNAL_ERROR: &str = "terminate direct child process";

#[test]
fn drop_reap_fallback_is_bounded_when_signals_make_no_progress() {
    let mut child = Command::new(SLEEP_EXECUTABLE)
        .arg(SLEEP_ARGUMENT)
        .spawn()
        .unwrap();
    let pid = libc::pid_t::try_from(child.id()).unwrap();
    let output = File::open(NULL_DEVICE).unwrap();
    let mut spawned = SpawnedProcess::new(pid, output.try_clone().unwrap(), output, None);
    spawned.configure_drop_fault_for_test(INJECTED_DROP_REAP_GRACE);
    let started = Instant::now();
    drop(spawned);
    let elapsed = started.elapsed();
    let still_running = child.try_wait().unwrap().is_none();
    if still_running {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    assert!(elapsed < DROP_COMPLETION_LIMIT);
    assert!(still_running);
}

#[test]
fn failed_capture_rollback_kills_reaps_and_caches_the_exit_status() {
    let mut spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let pid = spawned.pid;
    spawned.rollback_failed_capture().unwrap();
    let status = spawned.wait().unwrap();
    assert!(!status.success());
    assert_eq!(spawned.wait().unwrap(), status);
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn default_drop_kills_and_reaps_a_live_unsettled_child() {
    let spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let pid = spawned.pid;
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn drop_after_a_direct_wait_settles_the_empty_process_group() {
    let mut spawned = bare_process(EXIT_EXECUTABLE, &[]);
    let pid = spawned.pid;
    assert!(spawned.wait().unwrap().success());
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn drop_settles_real_discovery_custody_and_reaps_the_child() {
    let fixture = Fixture::new();
    let command = fixture.command(
        SLEEP_EXECUTABLE,
        vec![SLEEP_ARGUMENT.to_owned()],
        CUSTODY_DROP_DESCRIPTION,
    );
    let spawned = crate::posix_spawn::spawn(&command).unwrap();
    let pid = spawned.pid;
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn custody_signal_preserves_both_group_and_direct_signal_errors() {
    let mut spawned = bare_process(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let error = format!("{:#}", spawned.signal_custody(INVALID_SIGNAL).unwrap_err());
    assert!(error.contains(GROUP_SIGNAL_ERROR), "{error}");
    assert!(error.contains(DIRECT_SIGNAL_ERROR), "{error}");
    assert!(!spawned.exited().unwrap());
    let pid = spawned.pid;
    drop(spawned);
    assert_reaped(pid);
}

#[test]
fn empty_group_after_wait_can_be_signalled_and_rolled_back_idempotently() {
    let mut spawned = bare_process(EXIT_EXECUTABLE, &[]);
    let pid = spawned.pid;
    assert!(spawned.wait().unwrap().success());
    spawned.signal_custody(libc::SIGTERM).unwrap();
    spawned.signal_custody(libc::SIGKILL).unwrap();
    spawned.rollback_failed_capture().unwrap();
    drop(spawned);
    assert_reaped(pid);
}
