use super::command_private_spawn::write_private_input;
use super::{DetachedStandardError, spawn_detached_with_private_input};
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "command_private_input_test_support.rs"]
mod support;
use support::{ShortWriter, wait_until_pid_published, wait_until_reaped};

const COPY_EXECUTABLE: &str = "/usr/bin/tee";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const CLOSE_INPUT_SCRIPT: &str = "exec 0<&-; exit 0";
const HOLD_AFTER_CLOSE_SCRIPT: &str = "printf '%s' \"$$\" > \"$1\"; exec 0<&-; while :; do :; done";
const HOLD_WITHOUT_READING_SCRIPT: &str = "printf '%s' \"$$\" > \"$1\"; while :; do :; done";
const SHELL_ARGUMENT_ZERO: &str = "yolo-private-input";
const CHILD_PID_FILE: &str = "private-input-child.pid";
const PRIVATE_INPUT: &[u8] = b"private detached input";
const PIPE_OVERFLOW_BYTES: usize = 1_024 * 1_024;
const PRIVATE_INPUT_TEST_BUDGET: Duration = Duration::from_secs(2);
const PRIVATE_INPUT_TIMEOUT_BUDGET: Duration = Duration::from_millis(50);
const PRIVATE_INPUT_OBSERVATION_POLL: Duration = Duration::from_millis(5);
const SUB_MILLISECOND_POLL_BUDGET: Duration = Duration::from_nanos(999_999);

#[test]
fn detached_private_input_uses_an_anonymous_pipe_and_closes_it() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("captured-input");
    let mut child = spawn_detached_with_private_input(
        Path::new(COPY_EXECUTABLE),
        &[OsString::from(&output)],
        DetachedStandardError::Null,
        PRIVATE_INPUT,
        Instant::now() + PRIVATE_INPUT_TEST_BUDGET,
    )
    .unwrap();

    assert!(child.wait().unwrap().success());
    assert_eq!(std::fs::read(output).unwrap(), PRIVATE_INPUT);
}

#[test]
fn private_input_write_failure_hands_the_child_to_the_reaper() {
    let input = PRIVATE_INPUT
        .iter()
        .copied()
        .cycle()
        .take(PIPE_OVERFLOW_BYTES)
        .collect::<Vec<_>>();
    let error = spawn_detached_with_private_input(
        Path::new(SHELL_EXECUTABLE),
        &[SHELL_COMMAND_ARGUMENT.into(), CLOSE_INPUT_SCRIPT.into()],
        DetachedStandardError::Null,
        &input,
        Instant::now() + PRIVATE_INPUT_TEST_BUDGET,
    )
    .unwrap_err();
    assert!(
        !error
            .to_string()
            .contains(std::str::from_utf8(PRIVATE_INPUT).unwrap())
    );
}

#[test]
fn private_input_failure_terminates_and_reaps_a_noncooperative_child() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join(CHILD_PID_FILE);
    let input = PRIVATE_INPUT
        .iter()
        .copied()
        .cycle()
        .take(PIPE_OVERFLOW_BYTES)
        .collect::<Vec<_>>();

    let error = spawn_detached_with_private_input(
        Path::new(SHELL_EXECUTABLE),
        &[
            SHELL_COMMAND_ARGUMENT.into(),
            HOLD_AFTER_CLOSE_SCRIPT.into(),
            SHELL_ARGUMENT_ZERO.into(),
            pid_file.as_os_str().to_owned(),
        ],
        DetachedStandardError::Null,
        &input,
        Instant::now() + PRIVATE_INPUT_TEST_BUDGET,
    )
    .unwrap_err();
    let pid = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<libc::pid_t>()
        .unwrap();

    assert!(
        !error
            .to_string()
            .contains(std::str::from_utf8(PRIVATE_INPUT).unwrap())
    );
    wait_until_reaped(
        pid as u32,
        PRIVATE_INPUT_TEST_BUDGET,
        PRIVATE_INPUT_OBSERVATION_POLL,
    );
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn expired_delivery_deadline_prevents_child_spawn() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("expired-capture");
    let error = spawn_detached_with_private_input(
        Path::new(COPY_EXECUTABLE),
        &[OsString::from(&output)],
        DetachedStandardError::Null,
        PRIVATE_INPUT,
        Instant::now(),
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(super::command_private_input::PRIVATE_INPUT_DEADLINE_CONTEXT)
    );
    assert!(!output.exists());
}

#[test]
fn sub_millisecond_budget_is_rejected_instead_of_rounded_past_deadline() {
    let error =
        super::command_private_input::poll_milliseconds_for_test(SUB_MILLISECOND_POLL_BUDGET)
            .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(super::command_private_input::PRIVATE_INPUT_DEADLINE_CONTEXT)
    );
}

#[test]
fn private_input_writer_retries_short_writes() {
    let mut writer = ShortWriter::default();
    write_private_input(&mut writer, PRIVATE_INPUT).unwrap();
    assert_eq!(writer.output, PRIVATE_INPUT);
}

include!("command_private_input_deadline_tests.rs");
include!("command_private_input_settlement_tests.rs");
