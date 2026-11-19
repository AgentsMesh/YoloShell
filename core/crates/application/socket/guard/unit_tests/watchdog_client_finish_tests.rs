use super::*;
use crate::socket_root_guard::control_output::ControlOutput;
use crate::socket_root_guard::protocol::{ControlCapability, write_completion};
use crate::socket_root_guard::watchdog_diagnostics::WatchdogDiagnostics;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use yolo_process::{
    ProcessDiscoveryCapability, ProcessIdentity, spawn_detached_with_diagnostic_pipes,
};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const SHELL_PROGRAM_NAME: &str = "yolo-watchdog-test";
const SCRIPT: &str = "read ignored; printf '%s' \"$1\"; printf '%s' \"$2\" >&2; exit \"$3\"";
const TEST_SOCKET_ROOT: &str = "/private/tmp";
const FAILED_DETAIL: &str = "watchdog reported a captured-tree settlement failure";
const WATCHDOG_STDERR: &str = "watchdog emitted diagnostic evidence";
const EMPTY_OUTPUT: &str = "";
const SUCCESS_EXIT_CODE: i32 = 0;
const NONZERO_EXIT_CODE: i32 = 23;
const EARLY_EXIT_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn finish_combines_control_eof_nonzero_status_and_stderr() {
    let capability = ControlCapability::generate();
    let mut client = scripted_client(
        &capability,
        String::new(),
        WATCHDOG_STDERR,
        NONZERO_EXIT_CODE,
    );

    let error = client.finish().unwrap_err();
    let detail = format!("{error:#}");
    assert!(detail.contains(CONTROL_EOF_DETAIL), "{detail}");
    assert!(detail.contains(&NONZERO_EXIT_CODE.to_string()), "{detail}");
    assert!(detail.contains(WATCHDOG_STDERR), "{detail}");
}

#[test]
fn finish_accepts_authenticated_completion_and_success_status() {
    let capability = ControlCapability::generate();
    let mut frame = Vec::new();
    write_completion(&mut frame, &capability, None).unwrap();
    let mut client = scripted_client(
        &capability,
        String::from_utf8(frame).unwrap(),
        EMPTY_OUTPUT,
        SUCCESS_EXIT_CODE,
    );

    client.finish().unwrap();
}

#[test]
fn failed_completion_detail_survives_status_and_stderr_collection() {
    let capability = ControlCapability::generate();
    let mut frame = Vec::new();
    write_completion(&mut frame, &capability, Some(FAILED_DETAIL)).unwrap();
    let frame = String::from_utf8(frame).unwrap();
    let mut client = scripted_client(&capability, frame, WATCHDOG_STDERR, NONZERO_EXIT_CODE);

    let detail = format!("{:#}", client.finish().unwrap_err());
    assert!(detail.contains(FAILED_DETAIL), "{detail}");
    assert!(detail.contains(&NONZERO_EXIT_CODE.to_string()), "{detail}");
    assert!(detail.contains(WATCHDOG_STDERR), "{detail}");
}

#[test]
fn early_sigkill_status_remains_available_to_finish() {
    let capability = ControlCapability::generate();
    let mut command = Command::new(SLEEP_EXECUTABLE);
    command.arg(SLEEP_DURATION);
    let mut client = client_from_command(command, capability);
    client.child.as_mut().unwrap().kill().unwrap();

    let deadline = Instant::now() + EARLY_EXIT_TIMEOUT;
    let status = loop {
        if let Some(status) = client.exited_early().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "{WATCHDOG_EARLY_EXIT_DETAIL}");
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    };
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert!(client.child.is_some());

    let detail = format!("{:#}", client.finish().unwrap_err());
    assert!(detail.contains(WATCHDOG_EXIT_SIGNAL_DETAIL), "{detail}");
    assert!(detail.contains(&libc::SIGKILL.to_string()), "{detail}");
}

fn scripted_client(
    capability: &ControlCapability,
    completion: String,
    stderr: &str,
    exit_code: i32,
) -> WatchdogClient {
    let mut command = Command::new(SHELL_EXECUTABLE);
    let exit_code = exit_code.to_string();
    command.args([
        SHELL_COMMAND_ARGUMENT,
        SCRIPT,
        SHELL_PROGRAM_NAME,
        &completion,
        stderr,
        &exit_code,
    ]);
    client_from_command(command, capability.clone())
}

fn client_from_command(command: Command, capability: ControlCapability) -> WatchdogClient {
    let mut child = spawn_detached_with_diagnostic_pipes(command).unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let input = child.stdin.take().unwrap();
    let output = ControlOutput::new(child.stdout.take().unwrap());
    let diagnostics = WatchdogDiagnostics::start(child.stderr.take().unwrap()).unwrap();
    WatchdogClient {
        child: Some(child),
        input: Some(input),
        output,
        diagnostics,
        identity,
        capability,
        process_discovery: ProcessDiscoveryCapability::generate().unwrap(),
        socket_root: PathBuf::from(TEST_SOCKET_ROOT),
    }
}
