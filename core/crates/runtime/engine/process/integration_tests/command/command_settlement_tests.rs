use super::command_settlement::{
    PRIVATE_INPUT_KILL_CONTEXT, PRIVATE_INPUT_POLL_CONTEXT, terminate_test_child,
};
use super::command_settlement_test_hook::{PollOverride, SettlementScript, install};
use super::{DetachedStandardError, is_process_alive, spawn_detached};
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

const QUICK_EXECUTABLE: &str = "/bin/sh";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const QUICK_EXIT_SCRIPT: &str = "exit 0";
const SLEEP_SECONDS: &str = "30";
const EXIT_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(2);
const EXIT_OBSERVATION_POLL: Duration = Duration::from_millis(5);
const POLL_FAILURE_DETAIL: &str = "injected private input child poll failure";
const KILL_FAILURE_DETAIL: &str = "injected private input child termination failure";
const WAITPID_BLOCKING_OPTIONS: libc::c_int = 0;
const WAITPID_FAILURE: libc::pid_t = -1;

#[test]
fn settlement_accepts_children_observed_exited_or_already_reaped() {
    let mut exited = spawn_quick_exit();
    wait_until_exited(&mut exited);
    terminate_test_child(exited).unwrap();

    let reaped = spawn_quick_exit();
    reap_pid(reaped.id());
    terminate_test_child(reaped).unwrap();
}

#[test]
fn settlement_preserves_poll_failure_while_background_custody_reaps() {
    let child = spawn_sleep();
    let process_id = child.id();
    let _script = install(SettlementScript::new([PollOverride::Failure(
        POLL_FAILURE_DETAIL,
    )]));

    let error = terminate_test_child(child).unwrap_err();

    assert_error(&error, &[PRIVATE_INPUT_POLL_CONTEXT, POLL_FAILURE_DETAIL]);
    drop(_script);
    wait_until_not_alive(process_id);
}

#[test]
fn settlement_preserves_kill_failure_and_worker_retries_termination() {
    let child = spawn_sleep();
    let process_id = child.id();
    let _script = install(
        SettlementScript::new([PollOverride::Running]).fail_termination(KILL_FAILURE_DETAIL),
    );

    let error = terminate_test_child(child).unwrap_err();

    assert_error(&error, &[PRIVATE_INPUT_KILL_CONTEXT, KILL_FAILURE_DETAIL]);
    drop(_script);
    wait_until_not_alive(process_id);
}

#[test]
fn interrupted_initial_poll_is_reported_without_foreground_wait() {
    let child = spawn_sleep();
    let process_id = child.id();
    let _script = install(SettlementScript::new([PollOverride::Interrupted]));

    let error = terminate_test_child(child).unwrap_err();

    assert_error(&error, &[PRIVATE_INPUT_POLL_CONTEXT]);
    drop(_script);
    wait_until_not_alive(process_id);
}

fn spawn_quick_exit() -> Child {
    spawn_detached(
        Path::new(QUICK_EXECUTABLE),
        &[SHELL_COMMAND_ARGUMENT.into(), QUICK_EXIT_SCRIPT.into()],
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn spawn_sleep() -> Child {
    spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_SECONDS.into()],
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn wait_until_exited(child: &mut Child) {
    let deadline = Instant::now() + EXIT_OBSERVATION_TIMEOUT;
    while child.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(EXIT_OBSERVATION_POLL);
    }
}

fn wait_until_not_alive(process_id: u32) {
    let deadline = Instant::now() + EXIT_OBSERVATION_TIMEOUT;
    while is_process_alive(process_id) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(EXIT_OBSERVATION_POLL);
    }
}

fn reap_pid(pid: u32) {
    let waited = unsafe {
        libc::waitpid(
            pid as libc::pid_t,
            std::ptr::null_mut(),
            WAITPID_BLOCKING_OPTIONS,
        )
    };
    assert_ne!(waited, WAITPID_FAILURE);
}

fn assert_error(error: &anyhow::Error, expected: &[&str]) {
    let detail = format!("{error:#}");
    for expected in expected {
        assert!(
            detail.contains(expected),
            "missing {expected:?} in {detail:?}"
        );
    }
}
