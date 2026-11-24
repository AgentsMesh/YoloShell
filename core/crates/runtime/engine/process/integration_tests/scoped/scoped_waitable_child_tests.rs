use super::{ScopedWaitableChild, UNSUCCESSFUL_EXIT, WAIT_DEADLINE_EXPIRED};
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const CLEAN_EXIT_SCRIPT: &str = "exit 0";
const FAILED_EXIT_SCRIPT: &str = "exit 23";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_ARGUMENT: &str = "30";
const APPLICATION_FAILURE: &str = "injected scoped child operation failure";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(3);
const SHORT_WAIT_BUDGET: Duration = Duration::from_millis(1);
const REAP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WAITID_FAILURE: libc::c_int = -1;
const WAITID_SUCCESS: libc::c_int = 0;
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;

#[path = "scoped_background_handoff_tests.rs"]
mod background;
#[path = "scoped_termination_status_tests.rs"]
mod termination_status;

#[tokio::test]
async fn clean_exit_is_accepted_and_reaped() {
    let child = spawn_shell(CLEAN_EXIT_SCRIPT);
    let process_id = child.process_id();

    child
        .wait_for_clean_exit(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_reaped(process_id);
}

#[tokio::test]
async fn unsuccessful_exit_is_rejected_after_reaping() {
    let child = spawn_shell(FAILED_EXIT_SCRIPT);
    let process_id = child.process_id();

    let error = child
        .wait_for_clean_exit(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap_err();

    assert!(format!("{error:#}").contains(UNSUCCESSFUL_EXIT));
    assert_reaped(process_id);
}

#[tokio::test]
async fn direct_termination_is_reaped_before_returning() {
    let child = spawn_sleep();
    let process_id = child.process_id();

    child
        .terminate_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_reaped(process_id);
}

#[tokio::test]
async fn deadline_terminates_and_reaps_the_child() {
    let child = spawn_sleep();
    let process_id = child.process_id();

    let error = child
        .wait_for_clean_exit(Instant::now() + SHORT_WAIT_BUDGET)
        .await
        .unwrap_err();

    assert!(format!("{error:#}").contains(WAIT_DEADLINE_EXPIRED));
    wait_until_reaped(process_id).await;
}

#[tokio::test]
async fn already_expired_wait_terminates_and_reaps_the_child() {
    let child = spawn_sleep();
    let process_id = child.process_id();

    let error = child.wait_for_clean_exit(Instant::now()).await.unwrap_err();

    assert!(format!("{error:#}").contains(WAIT_DEADLINE_EXPIRED));
    wait_until_reaped(process_id).await;
}

#[tokio::test]
async fn early_application_error_terminates_and_reaps_the_child() {
    let child = spawn_sleep();
    let process_id = child.process_id();

    let error = fail_with_owned_child(child).unwrap_err();

    assert!(format!("{error:#}").contains(APPLICATION_FAILURE));
    wait_until_reaped(process_id).await;
}

#[tokio::test]
async fn aborted_wait_terminates_and_reaps_the_child() {
    let child = spawn_sleep();
    let process_id = child.process_id();
    let (started_send, started_receive) = tokio::sync::oneshot::channel();
    let wait = tokio::spawn(async move {
        started_send.send(()).unwrap();
        child
            .wait_for_clean_exit(Instant::now() + OPERATION_TIMEOUT)
            .await
    });
    started_receive.await.unwrap();
    tokio::task::yield_now().await;

    wait.abort();
    assert!(wait.await.unwrap_err().is_cancelled());
    wait_until_reaped(process_id).await;
}

fn spawn_shell(script: &str) -> ScopedWaitableChild {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.args([SHELL_COMMAND_ARGUMENT, script]);
    spawn(command)
}

fn spawn_sleep() -> ScopedWaitableChild {
    let mut command = Command::new(SLEEP_EXECUTABLE);
    command.arg(SLEEP_DURATION_ARGUMENT);
    spawn(command)
}

fn spawn(mut command: Command) -> ScopedWaitableChild {
    ScopedWaitableChild::spawn_before_deadline(&mut command, Instant::now() + OPERATION_TIMEOUT)
        .unwrap()
}

fn fail_with_owned_child(_child: ScopedWaitableChild) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(APPLICATION_FAILURE))
}

async fn wait_until_reaped(process_id: u32) {
    let deadline = Instant::now() + OPERATION_TIMEOUT;
    loop {
        if wait_authority_is_gone(process_id) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "child {process_id} was not reaped"
        );
        tokio::time::sleep(REAP_POLL_INTERVAL).await;
    }
}

fn assert_reaped(process_id: u32) {
    assert!(
        wait_authority_is_gone(process_id),
        "child {process_id} retained wait authority"
    );
}

fn wait_authority_is_gone(process_id: u32) -> bool {
    let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            information.as_mut_ptr(),
            WAITID_OPTIONS,
        )
    };
    if result == WAITID_SUCCESS {
        return false;
    }
    assert_eq!(result, WAITID_FAILURE);
    let error = io::Error::last_os_error();
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    true
}
