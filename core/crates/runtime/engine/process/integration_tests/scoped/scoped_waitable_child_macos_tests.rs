use super::ScopedWaitableChild;
use std::cell::RefCell;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const STDIN_GATE_SCRIPT: &str = "read gate";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_ARGUMENT: &str = "30";
const SIGNAL_FAILURE_DETAIL: &str = "injected exact termination delivery failure";
const STOP_TIMEOUT_DETAIL: &str = "child did not enter stopped state";
const GATE_RELEASE: &[u8] = b"\n";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(3);
const RESUME_DEADLINE_BUDGET: Duration = Duration::from_millis(20);
const RESUME_DEADLINE_OVERRUN: Duration = Duration::from_millis(50);
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOWAIT;

#[tokio::test]
async fn deadline_expiring_inside_exact_resume_never_continues_and_reaps() {
    let child = spawn_sleep();
    let process_id = child.process_id();
    let live = child.exact_live_identity().unwrap();
    assert!(live.signal(libc::SIGSTOP).unwrap());
    wait_until_stopped(live);
    let deadline = Instant::now() + RESUME_DEADLINE_BUDGET;
    let _hook = crate::process::signal_hook::install_pre_signal_hook(|| {
        std::thread::sleep(RESUME_DEADLINE_OVERRUN);
    });

    let error = child.resume_exact_stopped_before(deadline).unwrap_err();

    assert!(
        format!("{error:#}").contains(super::super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED)
    );
    assert_eq!(
        live.execution_state().unwrap(),
        crate::process::live_identity::ProcessExecutionState::Stopped
    );
    child
        .terminate_exact_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();
    assert_wait_authority_released(process_id);
}

#[tokio::test]
async fn exited_child_is_reaped_when_exact_termination_reports_failure() {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command
        .args([SHELL_COMMAND_ARGUMENT, STDIN_GATE_SCRIPT])
        .stdin(Stdio::piped());
    let mut child = ScopedWaitableChild::spawn_before_deadline(
        &mut command,
        Instant::now() + OPERATION_TIMEOUT,
    )
    .unwrap();
    let process_id = child.process_id();
    let input = RefCell::new(Some(child.take_stdin().unwrap()));
    let _failure = crate::process::install_test_live_signal_failure(move || {
        let mut input = input.borrow_mut().take().unwrap();
        input.write_all(GATE_RELEASE).unwrap();
        drop(input);
        wait_for_exit_without_reaping(process_id);
        Some(SIGNAL_FAILURE_DETAIL.to_owned())
    });

    child
        .terminate_exact_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_wait_authority_released(process_id);
}

#[tokio::test]
async fn live_child_uses_direct_custody_when_exact_termination_reports_failure() {
    let child = spawn_sleep();
    let process_id = child.process_id();
    let _failure =
        crate::process::install_test_live_signal_failure(|| Some(SIGNAL_FAILURE_DETAIL.to_owned()));

    child
        .terminate_exact_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_wait_authority_released(process_id);
}

#[tokio::test]
async fn exact_termination_delivery_is_reaped_before_returning() {
    let child = spawn_sleep();
    let process_id = child.process_id();

    child
        .terminate_exact_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_wait_authority_released(process_id);
}

#[tokio::test]
async fn unavailable_exact_identity_uses_direct_custody_and_reaps() {
    let mut child = spawn_sleep();
    let process_id = child.process_id();
    child.custody_mut().discard_live_identity_for_test();

    child
        .terminate_exact_and_wait_reaped(Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert_wait_authority_released(process_id);
}

fn spawn_sleep() -> ScopedWaitableChild {
    let mut command = Command::new(SLEEP_EXECUTABLE);
    command.arg(SLEEP_DURATION_ARGUMENT);
    ScopedWaitableChild::spawn_before_deadline(&mut command, Instant::now() + OPERATION_TIMEOUT)
        .unwrap()
}

fn wait_until_stopped(live: crate::process::LiveProcessIdentity) {
    let deadline = Instant::now() + OPERATION_TIMEOUT;
    loop {
        if live.execution_state().unwrap()
            == crate::process::live_identity::ProcessExecutionState::Stopped
        {
            return;
        }
        assert!(Instant::now() < deadline, "{STOP_TIMEOUT_DETAIL}");
        std::thread::yield_now();
    }
}

fn wait_for_exit_without_reaping(process_id: u32) {
    let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            information.as_mut_ptr(),
            WAITID_OPTIONS,
        )
    };
    assert_eq!(result, libc::c_int::default());
}

fn assert_wait_authority_released(process_id: u32) {
    let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            information.as_mut_ptr(),
            WAITID_OPTIONS | libc::WNOHANG,
        )
    };
    assert_eq!(result, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
