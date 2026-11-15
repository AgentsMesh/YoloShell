#![cfg(test)]

use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_QUIET_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedPtyChild;
use crate::process::ProcessIdentity;
use portable_pty::{CommandBuilder, SlavePty};
use std::path::{Path, PathBuf};
use std::time::Instant;

const HELPER_TEST: &str = "custody_guardian::settlement_failure::containment_tests::\
late_holder::late_holder_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_LATE_PTY_HOLDER_ROLE";
const READY_ENV: &str = "YOLO_GUARDIAN_LATE_PTY_HOLDER_READY";
const HOLDER_ROLE: &str = "late-holder";
const READY_FILE: &str = "holder.ready";
const EMPTY: &[u8] = b"";
const HOLDER_READY_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const HOLDER_TIMEOUT: std::time::Duration =
    crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;
const HOLDER_POLL: std::time::Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const HOLDER_TIMEOUT_DETAIL: &str = "late PTY holder helper did not settle";

pub(super) struct LatePtyHolder {
    child: GuardedPtyChild,
    _directory: tempfile::TempDir,
}

impl LatePtyHolder {
    pub(super) fn spawn(slave: Box<dyn SlavePty + Send>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let mut command = CommandBuilder::new(std::env::current_exe().unwrap());
        command.args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_QUIET_ARGUMENT,
        ]);
        command.env(ROLE_ENV, HOLDER_ROLE);
        command.env(READY_ENV, &ready);
        let child = GuardedPtyChild::spawn(move || slave.spawn_command(command).unwrap());
        let holder = Self {
            child,
            _directory: directory,
        };
        wait_for_ready(&ready);
        holder
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.child.identity()
    }

    pub(super) fn wait_for_exit(&mut self) {
        assert!(self.wait_for_exit_before(Instant::now() + HOLDER_TIMEOUT));
    }

    fn wait_for_exit_before(&mut self, deadline: Instant) -> bool {
        self.child.is_complete()
            || self
                .child
                .wait_before(deadline, HOLDER_POLL)
                .is_ok_and(|status| status.is_some())
    }
}

impl Drop for LatePtyHolder {
    fn drop(&mut self) {
        let _ = self.child.request_cleanup();
        let _ = self.wait_for_exit_before(Instant::now() + HOLDER_TIMEOUT);
    }
}

#[test]
fn late_holder_helper() {
    if std::env::var(ROLE_ENV).as_deref() != Ok(HOLDER_ROLE) {
        return;
    }
    std::fs::write(required_path(READY_ENV), EMPTY).unwrap();
    loop {
        std::thread::park();
    }
}

fn wait_for_ready(path: &Path) {
    let deadline = Instant::now() + HOLDER_READY_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HOLDER_TIMEOUT_DETAIL}");
        std::thread::sleep(HOLDER_POLL);
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
