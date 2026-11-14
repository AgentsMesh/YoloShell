#![cfg(test)]

use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_QUIET_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Instant;

const HELPER_TEST: &str = "custody_guardian::settlement_failure::containment_tests::\
queued_holder::queued_holder_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_ROLE";
const READY_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_READY";
const TRIGGER_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_TRIGGER";
const RECEIVED_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_RECEIVED";
const PARENT_PID_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_PARENT_PID";
const PARENT_START_ENV: &str = "YOLO_GUARDIAN_QUEUED_PTY_HOLDER_PARENT_START";
const HOLDER_ROLE: &str = "queued-holder";
const READY_FILE: &str = "holder.ready";
const TRIGGER_FILE: &str = "holder.trigger";
const RECEIVED_FILE: &str = "holder.received";
const EMPTY: &[u8] = b"";
const RIGHT_MARKER: u8 = b'Q';
const SENDER_EXPECTATION: &str = "queued PTY holder sender is unavailable";
const HOLDER_TIMEOUT_DETAIL: &str = "queued PTY holder helper did not converge";
const HOLDER_TIMEOUT: std::time::Duration =
    crate::session_timing::WORKER_TERMINATION_PROTOCOL_TIMEOUT;
const HOLDER_POLL: std::time::Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;

pub(super) struct QueuedPtyHolder {
    child: GuardedChild,
    sender: Option<UnixStream>,
    trigger: PathBuf,
    received: PathBuf,
    _directory: tempfile::TempDir,
}

impl QueuedPtyHolder {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let trigger = directory.path().join(TRIGGER_FILE);
        let received = directory.path().join(RECEIVED_FILE);
        let (sender, receiver) = UnixStream::pair().unwrap();
        let receiver: OwnedFd = receiver.into();
        let parent = ProcessIdentity::capture(std::process::id()).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                HELPER_TEST,
                TEST_HARNESS_QUIET_ARGUMENT,
            ])
            .env(ROLE_ENV, HOLDER_ROLE)
            .env(READY_ENV, &ready)
            .env(TRIGGER_ENV, &trigger)
            .env(RECEIVED_ENV, &received)
            .env(PARENT_PID_ENV, parent.pid.to_string())
            .env(PARENT_START_ENV, parent.start_time.to_string())
            .stdin(Stdio::from(receiver))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = GuardedChild::spawn(move || crate::process::spawn_waitable_test_child(command));
        let holder = Self {
            child,
            sender: Some(sender),
            trigger,
            received,
            _directory: directory,
        };
        wait_for_file(&ready);
        holder
    }

    pub(super) fn queue(&mut self, descriptor: &File) {
        let sender = self.sender.take().expect(SENDER_EXPECTATION);
        crate::local_unix::test_support::send_right(
            sender.as_raw_fd(),
            descriptor.as_raw_fd(),
            RIGHT_MARKER,
        );
    }

    pub(super) fn materialize(&self) {
        std::fs::write(&self.trigger, EMPTY).unwrap();
        wait_for_file(&self.received);
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.child.identity()
    }

    pub(super) fn wait_for_exit(&mut self) -> ExitStatus {
        self.child
            .wait_before(Instant::now() + HOLDER_TIMEOUT, HOLDER_POLL)
            .unwrap()
            .expect(HOLDER_TIMEOUT_DETAIL)
    }
}

impl Drop for QueuedPtyHolder {
    fn drop(&mut self) {
        if !self.child.is_complete() {
            let _ = self.child.request_cleanup();
            let _ = self
                .child
                .wait_before(Instant::now() + HOLDER_TIMEOUT, HOLDER_POLL);
        }
    }
}

#[test]
fn queued_holder_waits_for_trigger_while_its_parent_is_alive() {
    let holder = QueuedPtyHolder::spawn();
    std::thread::sleep(HOLDER_TIMEOUT.saturating_add(HOLDER_TIMEOUT));
    assert_eq!(
        holder.identity().observe(),
        ProcessIdentityObservation::Matching
    );
}

#[test]
fn queued_holder_helper() {
    if std::env::var(ROLE_ENV).as_deref() != Ok(HOLDER_ROLE) {
        return;
    }
    let parent = ProcessIdentity {
        pid: std::env::var(PARENT_PID_ENV).unwrap().parse().unwrap(),
        start_time: std::env::var(PARENT_START_ENV).unwrap().parse().unwrap(),
    };
    std::fs::write(required_path(READY_ENV), EMPTY).unwrap();
    if !wait_for_trigger(&required_path(TRIGGER_ENV), parent) {
        return;
    }
    let _right =
        crate::local_unix::macos::receive_exactly_one_right(libc::STDIN_FILENO, RIGHT_MARKER)
            .unwrap();
    std::fs::write(required_path(RECEIVED_ENV), EMPTY).unwrap();
    while parent.observe() != ProcessIdentityObservation::Gone {
        std::thread::sleep(HOLDER_POLL);
    }
}

// Queued rights follow the test parent's lifetime, not its readiness deadline.
fn wait_for_trigger(path: &Path, parent: ProcessIdentity) -> bool {
    loop {
        match parent.observe() {
            ProcessIdentityObservation::Gone => return false,
            ProcessIdentityObservation::Matching if path.exists() => return true,
            _ => std::thread::sleep(HOLDER_POLL),
        }
    }
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + HOLDER_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HOLDER_TIMEOUT_DETAIL}");
        std::thread::sleep(HOLDER_POLL);
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
