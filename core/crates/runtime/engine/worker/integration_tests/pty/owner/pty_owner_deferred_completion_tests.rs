use super::super::*;
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use crate::worker::pty_exit_observer::WaitidRootExitObserver;
use crate::worker::pty_fault_reaper::FaultReaperReservation;
use crate::worker::pty_owner_handle::PtyOwnerCommandFailure;
use crate::worker::pty_ownership::PtyOwnership;
use crate::worker::termination_timing::CapabilitySettlementDeferred;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::Duration;

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const OWNER_RETURN_TIMEOUT: Duration = Duration::from_secs(10);
const OWNER_THREAD_BLOCKED: &str = "PTY owner blocked on deferred reap completion";

#[test]
fn ownership_fault_returns_before_reserved_reaper_resumes() {
    let child = spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let (reservation, held) = FaultReaperReservation::reserve_held_for_test().unwrap();
    let ownership = PtyOwnership::new_for_test(Box::new(child), tree, reservation).unwrap();
    let (_commands, receiver) = mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let observed_settlement = Arc::clone(&settlement);
    let (returned, observation) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let mut owner = ownership.into_owner_for_test(
            std::sync::Weak::new(),
            Box::new(WaitidRootExitObserver::new(root)),
            receiver,
            Arc::clone(&settlement),
        );
        owner.finish_cleanup_deferred(CapabilitySettlementDeferred);
        returned.send(owner.failure.is_none()).unwrap();
    });

    let held = held.wait_for_handoff();
    let owner_has_no_failure = observation
        .recv_timeout(OWNER_RETURN_TIMEOUT)
        .expect(OWNER_THREAD_BLOCKED);
    assert!(matches!(
        observed_settlement.get(),
        Some(Err(PtyOwnerCommandFailure::CleanupDeferred(_)))
    ));
    assert!(owner_has_no_failure);
    held.resume();
    thread.join().unwrap();

    assert!(!root.matches());
}
