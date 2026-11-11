#![cfg(test)]

use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_QUIET_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const HELPER_TEST: &str =
    "custody_guardian::settlement_failure::containment_tests::escaped_pty_holder_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_PTY_HOLDER_ROLE";
const READY_ENV: &str = "YOLO_GUARDIAN_PTY_HOLDER_READY";
const PTY_PATH_ENV: &str = "YOLO_GUARDIAN_PTY_HOLDER_PATH";
const HOLDER_ROLE: &str = "holder";
const READY_FILE: &str = "holder.ready";
const EMPTY: &[u8] = b"";
const SETSID_FAILURE: libc::pid_t = -1;
const HELPER_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const HELPER_EXIT_TIMEOUT: std::time::Duration =
    crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;
const HELPER_POLL: std::time::Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const HELPER_TIMEOUT_DETAIL: &str = "escaped PTY holder helper did not converge";
const PTY_OPEN_FLAGS: libc::c_int = libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_NOFOLLOW;

pub(super) struct EscapedPtyHolder {
    child: GuardedChild,
    _directory: tempfile::TempDir,
    pty_path: PathBuf,
}

impl EscapedPtyHolder {
    pub(super) fn spawn(pty_path: &Path) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let child = GuardedChild::spawn(|| {
            crate::process::spawn_waitable_test_child(holder_command(&ready, pty_path))
        });
        let holder = Self {
            child,
            _directory: directory,
            pty_path: pty_path.to_path_buf(),
        };
        wait_for(&ready);
        assert_eq!(
            holder.identity().observe(),
            ProcessIdentityObservation::Matching
        );
        holder
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.child.identity()
    }

    pub(super) fn pty_path(&self) -> &Path {
        &self.pty_path
    }

    pub(super) fn wait_for_exit(&mut self) {
        assert!(self.wait_for_exit_before(Instant::now() + HELPER_EXIT_TIMEOUT));
    }

    fn wait_for_exit_before(&mut self, deadline: Instant) -> bool {
        self.child.is_complete()
            || self
                .child
                .wait_before(deadline, HELPER_POLL)
                .is_ok_and(|status| status.is_some())
    }
}

impl Drop for EscapedPtyHolder {
    fn drop(&mut self) {
        let _ = self.child.request_cleanup();
        let _ = self.wait_for_exit_before(Instant::now() + HELPER_EXIT_TIMEOUT);
    }
}

pub(super) fn run_role() {
    if std::env::var(ROLE_ENV).as_deref() == Ok(HOLDER_ROLE) {
        run_holder();
    }
}

fn run_holder() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    let _pty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(PTY_OPEN_FLAGS)
        .open(required_path(PTY_PATH_ENV))
        .unwrap();
    std::fs::write(required_path(READY_ENV), EMPTY).unwrap();
    loop {
        std::thread::park();
    }
}

fn holder_command(ready: &Path, pty_path: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_QUIET_ARGUMENT,
        ])
        .env(ROLE_ENV, HOLDER_ROLE)
        .env(READY_ENV, ready)
        .env(PTY_PATH_ENV, pty_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
        std::thread::sleep(HELPER_POLL);
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
