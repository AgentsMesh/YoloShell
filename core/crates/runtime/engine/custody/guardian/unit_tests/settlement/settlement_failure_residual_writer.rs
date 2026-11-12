#![cfg(test)]

use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_QUIET_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{
    ActiveMacosSecondaryCustodyFixture, ProcessIdentity, ProcessIdentityRelation,
    process_identity_relation,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HELPER_TEST: &str = "custody_guardian::settlement_failure::containment_tests::fixture::\
residual_writer::residual_writer_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_RESIDUAL_WRITER_ROLE";
const READY_ENV: &str = "YOLO_GUARDIAN_RESIDUAL_WRITER_READY";
const HOLDER_ROLE: &str = "residual-writer";
const READY_FILE: &str = "writer.ready";
const EMPTY: &[u8] = b"";
const HELPER_TIMEOUT_DETAIL: &str = "residual writer helper did not become ready";

pub(super) struct ResidualWriter {
    _directory: tempfile::TempDir,
    child: GuardedChild,
    identity: ProcessIdentity,
}

impl ResidualWriter {
    pub(super) fn spawn(writer: &ActiveMacosSecondaryCustodyFixture) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let mut command = command(&ready);
        writer.configure_exec_inheritance(&mut command);
        let child = GuardedChild::spawn(move || crate::process::spawn_waitable_test_child(command));
        let identity = child.identity();
        let writer = Self {
            _directory: directory,
            child,
            identity,
        };
        wait_for_ready(&ready);
        writer
    }

    pub(super) fn assert_matching(&self) {
        assert_eq!(
            process_identity_relation(self.identity.pid, self.identity.start_time),
            ProcessIdentityRelation::Matching
        );
    }

    pub(super) fn wait_for_exit(&mut self) -> std::process::ExitStatus {
        let deadline = std::time::Instant::now() + super::TEST_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(super::TEST_POLL_INTERVAL);
        }
    }

    pub(super) fn terminate(&mut self) -> std::process::ExitStatus {
        self.identity.enforce_exact_test_cleanup().unwrap();
        self.wait_for_exit()
    }
}

#[test]
fn residual_writer_helper() {
    if std::env::var(ROLE_ENV).as_deref() != Ok(HOLDER_ROLE) {
        return;
    }
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    std::fs::write(required_path(READY_ENV), EMPTY).unwrap();
    loop {
        std::thread::park();
    }
}

fn command(ready: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_QUIET_ARGUMENT,
        ])
        .env(ROLE_ENV, HOLDER_ROLE)
        .env(READY_ENV, ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn wait_for_ready(path: &Path) {
    let deadline = std::time::Instant::now() + super::TEST_TIMEOUT;
    while !path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "{HELPER_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(super::TEST_POLL_INTERVAL);
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
