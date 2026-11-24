#![cfg(test)]

use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{
    ActiveMacosSecondaryCustodyFixture, MacosProcessCapabilityObservation, ProcessIdentity,
    ProcessIdentityRelation,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use portable_pty::{PtySize, SlavePty};
use std::time::Instant;

const TEST_CAPABILITY: &str = "namespace-loss-premature-release";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_WORKER_PID: u32 = 48_003;
const TEST_WORKER_START: u64 = 58_003;
const WAIT_FAILURE: libc::pid_t = -1;
const WAIT_PENDING: libc::pid_t = 0;

pub(super) struct PrematureFixture {
    pub(super) custody: crate::process::MacosSecondaryCustody,
    pub(super) runtime: CustodyRuntimeIdentity,
    child: GuardedChild,
    inherited_slave: Option<Box<dyn SlavePty + Send>>,
}

impl PrematureFixture {
    pub(super) fn new(
        worker_relation: ProcessIdentityRelation,
    ) -> (Self, crate::process::ProcessIdentityRelationGuard) {
        let (fixture, worker, holder) = Self::build(worker_relation, false);
        assert!(holder.is_none());
        (fixture, worker)
    }

    pub(super) fn with_escaped_holder(
        worker_relation: ProcessIdentityRelation,
    ) -> (
        Self,
        crate::process::ProcessIdentityRelationGuard,
        super::escaped_holder::EscapedPtyHolder,
    ) {
        let (fixture, worker, holder) = Self::build(worker_relation, true);
        (fixture, worker, holder.unwrap())
    }

    fn build(
        worker_relation: ProcessIdentityRelation,
        escape_holder: bool,
    ) -> (
        Self,
        crate::process::ProcessIdentityRelationGuard,
        Option<super::escaped_holder::EscapedPtyHolder>,
    ) {
        let active = ActiveMacosSecondaryCustodyFixture::new(pty_size(), TEST_CAPABILITY);
        let pty_path = active.pty_path();
        let holder =
            escape_holder.then(|| super::escaped_holder::EscapedPtyHolder::spawn(&pty_path));
        let child = GuardedChild::spawn(|| {
            crate::process::spawn_waitable_test_child(super::stopped_child::command())
        });
        let child_identity = child.identity();
        let runtime = runtime(child_identity);
        let (custody, inherited_slave) = active.capture_retaining_slave(&runtime);
        let worker = crate::process::install_test_process_identity_relation(
            TEST_WORKER_PID,
            TEST_WORKER_START,
            worker_relation,
        );
        let mut fixture = Self {
            custody,
            runtime,
            child,
            inherited_slave: Some(inherited_slave),
        };
        fixture.wait_for_stop();
        fixture.wait_for_premature_release();
        (fixture, worker, holder)
    }

    pub(super) fn terminate_child(&mut self) -> std::process::ExitStatus {
        child_identity(&self.runtime)
            .enforce_exact_test_cleanup()
            .unwrap();
        self.wait_for_child_exit()
    }

    pub(super) fn take_inherited_slave(&mut self) -> Box<dyn SlavePty + Send> {
        self.inherited_slave.take().unwrap()
    }

    fn wait_for_child_exit(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + super::fixture::TEST_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(super::fixture::TEST_POLL_INTERVAL);
        }
    }

    fn wait_for_premature_release(&self) {
        let deadline = Instant::now() + super::fixture::TEST_TIMEOUT;
        while self.custody.process_capability_observation()
            != Some(MacosProcessCapabilityObservation::PrematurelyReleased)
        {
            assert!(Instant::now() < deadline);
            std::thread::sleep(super::fixture::TEST_POLL_INTERVAL);
        }
    }

    fn wait_for_stop(&mut self) {
        let pid = self.runtime.child_pid as libc::pid_t;
        let deadline = Instant::now() + super::fixture::TEST_TIMEOUT;
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
            std::thread::sleep(super::fixture::TEST_POLL_INTERVAL);
        }
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
