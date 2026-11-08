#![cfg(test)]

use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{
    ActiveMacosSecondaryCustodyFixture, ProcessIdentity, ProcessIdentityRelation,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use portable_pty::PtySize;
use std::time::{Duration, Instant};

#[path = "settlement_failure_residual_writer.rs"]
mod residual_writer;

const TEST_CAPABILITY: &str = "namespace-loss-active-settlement-order";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_WORKER_PID: u32 = 48_002;
const TEST_WORKER_START: u64 = 58_002;
const WAIT_FAILURE: libc::pid_t = -1;
const WAIT_PENDING: libc::pid_t = 0;
pub(super) const TEST_TIMEOUT: Duration =
    crate::session_timing::GUARDIAN_NAMESPACE_LOSS_SETTLEMENT_TIMEOUT;
pub(super) const TEST_POLL_INTERVAL: Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;

pub(super) struct Fixture {
    pub(super) custody: crate::process::MacosSecondaryCustody,
    pub(super) runtime: CustodyRuntimeIdentity,
    child: GuardedChild,
    residual: residual_writer::ResidualWriter,
}

impl Fixture {
    pub(super) fn new(
        worker_relation: ProcessIdentityRelation,
    ) -> (Self, crate::process::ProcessIdentityRelationGuard) {
        let active = ActiveMacosSecondaryCustodyFixture::new(pty_size(), TEST_CAPABILITY);
        let mut command = super::stopped_child::command();
        active.configure_exec_inheritance(&mut command);
        let child = GuardedChild::spawn(move || crate::process::spawn_waitable_test_child(command));
        let child_identity = child.identity();
        let residual = residual_writer::ResidualWriter::spawn(&active);
        let runtime = runtime(child_identity);
        let custody = active.capture(&runtime);
        let worker = crate::process::install_test_process_identity_relation(
            TEST_WORKER_PID,
            TEST_WORKER_START,
            worker_relation,
        );
        let mut fixture = Self {
            custody,
            runtime,
            child,
            residual,
        };
        fixture.wait_for_stop();
        (fixture, worker)
    }

    pub(super) fn assert_child_pending(&mut self) {
        assert!(self.child.try_wait().unwrap().is_none());
    }

    pub(super) fn assert_residual_matching(&self) {
        self.residual.assert_matching();
    }

    pub(super) fn wait_for_child_exit(&mut self) -> std::process::ExitStatus {
        wait_for_exit(&mut self.child)
    }

    pub(super) fn terminate_child(&mut self) -> std::process::ExitStatus {
        child_identity(&self.runtime)
            .enforce_exact_test_cleanup()
            .unwrap();
        self.wait_for_child_exit()
    }

    pub(super) fn wait_for_residual_exit(&mut self) -> std::process::ExitStatus {
        self.residual.wait_for_exit()
    }

    pub(super) fn terminate_residual(&mut self) -> std::process::ExitStatus {
        self.residual.terminate()
    }

    fn wait_for_stop(&mut self) {
        let pid = self.runtime.child_pid as libc::pid_t;
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            let mut status = libc::c_int::default();
            let observed =
                unsafe { libc::waitpid(pid, &mut status, libc::WUNTRACED | libc::WNOHANG) };
            if observed == pid {
                assert!(libc::WIFSTOPPED(status));
                return;
            }
            if observed == WAIT_FAILURE {
                assert_eq!(
                    std::io::Error::last_os_error().kind(),
                    std::io::ErrorKind::Interrupted
                );
            } else {
                assert_eq!(observed, WAIT_PENDING);
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(TEST_POLL_INTERVAL);
        }
    }
}

fn wait_for_exit(child: &mut GuardedChild) -> std::process::ExitStatus {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn runtime(child: ProcessIdentity) -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: crate::process::boot_id(),
    }
}

fn child_identity(runtime: &CustodyRuntimeIdentity) -> ProcessIdentity {
    ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TEST_ROWS,
        cols: TEST_COLUMNS,
        pixel_width: TEST_PIXEL_SIZE,
        pixel_height: TEST_PIXEL_SIZE,
    }
}
