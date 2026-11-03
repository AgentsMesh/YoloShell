use super::*;
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use crate::worker::pty_force_termination::{ForceOutput, ForceTerminationPoll};
use crate::worker::pty_process_custody::PtyProcessCustodyKind;
use std::path::Path;
use std::time::Duration;

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const HANDOFF_TIMEOUT: Duration = Duration::from_secs(10);
const REAPER_RESERVATION_FAILURE_DETAIL: &str = "reserve held PTY fault reaper";
const HANDOFF_STATUS_MISSING_DETAIL: &str = "fault handoff status is present";
const OWNER_FAULT_DETAIL: &str = "injected owner fault";

#[test]
fn observer_only_survivor_state_survives_fault_handoff() {
    let (mut reservation, held_reaper) =
        FaultReaperReservation::reserve_held_for_test().expect(REAPER_RESERVATION_FAILURE_DETAIL);
    let state = reservation.observe_state_for_test();
    let child = spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.deny_destructive_for_test();
    let mut ownership = PtyOwnership::new_for_test(Box::new(child), tree, reservation).unwrap();
    let mut output = ownership.output_recovery();

    assert!(matches!(
        ownership.poll_force(&mut ForceOutput::Reader(&mut output)),
        Ok(ForceTerminationPoll::Unsupported(_))
    ));
    ownership.enter_observer_only();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    assert!(matches!(
        ownership.settle_after_fault_with_failure(completion, anyhow::anyhow!(OWNER_FAULT_DETAIL)),
        FaultSettlement::Deferred(_)
    ));

    let held = held_reaper.wait_for_handoff();
    assert_eq!(
        held.custody_snapshot_for_test().kind,
        PtyProcessCustodyKind::Survivor
    );
    assert!(held.is_observer_only_for_test());
    let status = state.status().expect(HANDOFF_STATUS_MISSING_DETAIL);
    assert!(status.observer_only);
    assert_eq!(status.last_fault.as_deref(), Some(OWNER_FAULT_DETAIL));

    assert!(root.signal(libc::SIGKILL).unwrap());
    held.resume();
    let outcome = receiver.receive_timeout(HANDOFF_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(
        outcome.fault.as_ref().map(ToString::to_string).as_deref(),
        Some(OWNER_FAULT_DETAIL)
    );
    assert!(!root.matches());
}
