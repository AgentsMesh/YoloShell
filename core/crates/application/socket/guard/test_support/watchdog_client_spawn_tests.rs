use super::*;
use crate::socket_root_guard::protocol::{ControlCapability, DiscoveryCapability, write_root};
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use yolo_process::{
    ProcessDiscoveryCapability, ProcessIdentity, spawn_detached_with_diagnostic_pipes,
};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SHELL_PROGRAM_NAME: &str = "yolo-watchdog-handshake-test";
pub(super) const SLEEP_EXECUTABLE: &str = "/bin/sleep";
pub(super) const SLEEP_DURATION_ARGUMENT: &str = "120";
pub(super) const SLEEP_DURATION: Duration = Duration::from_secs(120);
pub(super) const STDERR_HOLDER_DURATION_ARGUMENT: &str = "5";
pub(super) const STDERR_HOLDER_DURATION: Duration = Duration::from_secs(5);
pub(super) const STDERR_HOLDER_PID_NAME: &str = "stderr-holder-pid";
pub(super) const EOF_MARKER_NAME: &str = "stdin-closed";
pub(super) const TEST_DIAGNOSTIC: &str = "mismatched watchdog handshake diagnostic";
const MISMATCH_ACCEPTED_DETAIL: &str = "mismatched watchdog handshake was accepted";
const STDERR_HOLDER_SETTLEMENT_DETAIL: &str = "stderr holder did not settle";
const WAITPID_FAILURE: libc::pid_t = -1;
const HANDLE_INCREMENT: u64 = 1;
pub(super) const EOF_SCRIPT: &str =
    "printf '%s' \"$1\"; printf '%s' \"$2\" >&2; read ignored; : > \"$3\"";
pub(super) const FORCE_REAP_SCRIPT: &str =
    "printf '%s' \"$1\"; printf '%s' \"$2\" >&2; exec \"$3\" \"$4\"";
pub(super) const INHERITED_STDERR_SCRIPT: &str = "printf '%s' \"$1\"; \
    printf '%s' \"$2\" >&2; trap '' HUP; \"$3\" \"$4\" & \
    printf '%s' \"$!\" > \"$5\"; trap - HUP; read ignored";

#[derive(Clone, Copy)]
pub(super) enum Mismatch {
    Value,
    Reader,
    Writer,
}

pub(super) struct StderrHolder {
    identity: ProcessIdentity,
    settled: bool,
}

impl StderrHolder {
    pub(super) fn capture(path: &Path) -> Self {
        let pid = std::fs::read_to_string(path)
            .unwrap()
            .parse::<u32>()
            .unwrap();
        Self {
            identity: ProcessIdentity::capture(pid).unwrap(),
            settled: false,
        }
    }

    pub(super) fn settle(mut self) {
        self.stop();
        assert!(self.settled, "{STDERR_HOLDER_SETTLEMENT_DETAIL}");
    }

    fn stop(&mut self) {
        if self.settled {
            return;
        }
        let _ = self.identity.signal(libc::SIGKILL);
        let deadline = Instant::now() + WATCHDOG_FORCE_REAP_TIMEOUT;
        while self.identity.matches() && Instant::now() < deadline {
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
        self.settled = !self.identity.matches();
    }
}

impl Drop for StderrHolder {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(super) fn root_frame(
    discovery: &ProcessDiscoveryCapability,
    mismatch: Mismatch,
    path: &Path,
) -> String {
    let expected_reader = discovery.reader_handle();
    let expected_writer = discovery.writer_handle();
    let protocol_discovery = match mismatch {
        Mismatch::Value => unexpected_discovery(discovery.value()),
        Mismatch::Reader | Mismatch::Writer => {
            DiscoveryCapability::parse(discovery.value()).unwrap()
        }
    };
    let unexpected_handle = distinct_handle(&[expected_reader, expected_writer]);
    let reader = match mismatch {
        Mismatch::Reader => unexpected_handle,
        Mismatch::Value | Mismatch::Writer => expected_reader,
    };
    let writer = match mismatch {
        Mismatch::Writer => unexpected_handle,
        Mismatch::Value | Mismatch::Reader => expected_writer,
    };
    let mut frame = Vec::new();
    write_root(
        &mut frame,
        &ControlCapability::generate(),
        &protocol_discovery,
        reader,
        writer,
        path,
    )
    .unwrap();
    String::from_utf8(frame).unwrap()
}

fn unexpected_discovery(expected: &str) -> DiscoveryCapability {
    loop {
        let candidate = DiscoveryCapability::generate();
        if candidate.value() != expected {
            return candidate;
        }
    }
}

fn distinct_handle(excluded: &[u64]) -> u64 {
    let mut candidate = excluded[0];
    loop {
        candidate = candidate.wrapping_add(HANDLE_INCREMENT);
        if candidate != u64::default() && !excluded.contains(&candidate) {
            return candidate;
        }
    }
}

pub(super) fn spawn_script(
    script: &str,
    frame: &str,
    arguments: &[&str],
) -> (Child, ProcessIdentity) {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command.args([
        SHELL_COMMAND_ARGUMENT,
        script,
        SHELL_PROGRAM_NAME,
        frame,
        TEST_DIAGNOSTIC,
    ]);
    command.args(arguments);
    let child = spawn_detached_with_diagnostic_pipes(command).unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    (child, identity)
}

pub(super) fn rejected(child: Child, discovery: ProcessDiscoveryCapability) -> String {
    match WatchdogClient::complete_spawn(child, discovery) {
        Ok(_client) => panic!("{MISMATCH_ACCEPTED_DETAIL}"),
        Err(error) => format!("{error:#}"),
    }
}

pub(super) fn assert_reaped(identity: ProcessIdentity) {
    assert!(!identity.matches());
    let result = unsafe {
        libc::waitpid(
            identity.pid as libc::pid_t,
            std::ptr::null_mut(),
            libc::WNOHANG,
        )
    };
    assert_eq!(result, WAITPID_FAILURE);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

pub(super) fn path_text(path: &Path) -> &str {
    path.to_str().unwrap()
}
