use super::*;
use crate::process::{
    ActiveMacosSecondaryCustodyFixture, ProcessIdentity, ProcessIdentityRelation,
    ProcessIdentityRelationGuard,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use portable_pty::PtySize;
use std::time::{Duration, Instant};

const TEST_CAPABILITY: &str = "initial-guardian-settlement-test";
const TEST_CHILD_COMMAND: &str = "/bin/sleep";
const TEST_CHILD_DURATION: &str = "30";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_WORKER_PID: u32 = 48_001;
const TEST_WORKER_START: u64 = 58_001;
const CLOSURE_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn current_worker_retains_initial_guardian_descriptors() {
    let (mut fixture, _worker) = Fixture::new(ProcessIdentityRelation::Matching);

    assert!(!settle_processes(&fixture.custody, &fixture.runtime).unwrap());
    assert!(fixture.child.try_wait().unwrap().is_none());
    assert!(!fixture.custody.closure_settled().unwrap());
}

#[test]
fn unknown_worker_retains_initial_guardian_descriptors() {
    let (mut fixture, _worker) = Fixture::new(ProcessIdentityRelation::Unknown);

    assert!(settle_processes(&fixture.custody, &fixture.runtime).is_err());
    assert!(fixture.child.try_wait().unwrap().is_none());
    assert!(!fixture.custody.closure_settled().unwrap());
}

#[test]
fn gone_worker_starts_active_settlement_and_proves_child_closure() {
    let (mut fixture, _worker) = Fixture::new(ProcessIdentityRelation::Gone);

    let mut settled = settle_processes(&fixture.custody, &fixture.runtime).unwrap();
    let status = fixture.child.wait().unwrap();
    assert!(!status.success());
    let deadline = Instant::now() + crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT;
    while !settled {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CLOSURE_POLL_INTERVAL);
        settled = settle_processes(&fixture.custody, &fixture.runtime).unwrap();
    }
    assert!(fixture.custody.closure_settled().unwrap());
}

struct Fixture {
    custody: MacosSecondaryCustody,
    runtime: CustodyRuntimeIdentity,
    child: std::process::Child,
}

impl Fixture {
    fn new(relation: ProcessIdentityRelation) -> (Self, ProcessIdentityRelationGuard) {
        let active = ActiveMacosSecondaryCustodyFixture::new(
            PtySize {
                rows: TEST_ROWS,
                cols: TEST_COLUMNS,
                pixel_width: TEST_PIXEL_SIZE,
                pixel_height: TEST_PIXEL_SIZE,
            },
            TEST_CAPABILITY,
        );
        let mut command = std::process::Command::new(TEST_CHILD_COMMAND);
        command.arg(TEST_CHILD_DURATION);
        active.configure_exec_inheritance(&mut command);
        let child = crate::process::spawn_waitable_test_child(command);
        let child_identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
        let runtime = runtime(child_identity);
        let custody = active.capture(&runtime);
        let worker = crate::process::install_test_process_identity_relation(
            TEST_WORKER_PID,
            TEST_WORKER_START,
            relation,
        );
        (
            Self {
                custody,
                runtime,
                child,
            },
            worker,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
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
