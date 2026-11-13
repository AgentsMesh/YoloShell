use super::*;
use crate::process::{DetachedStandardError, ProcessIdentity, spawn_detached};
use crate::worker::pty_exit_observer::WaitidRootExitObserver;
use crate::worker::pty_force_termination::{
    ForceOutput, ForceTerminationPoll, PtyForceTermination,
};
use crate::worker::pty_output_progress::ReaderBackedPtyOutput;
use crate::worker::pty_process_custody::{PtyProcessCustodyKind, PtyProcessCustodySnapshot};
use std::path::Path;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const HANDOFF_TIMEOUT: Duration = Duration::from_secs(10);
const TRANSITION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TRANSITION_TIMEOUT_DETAIL: &str = "typed PTY custody transition timed out";
const ROOT_EXIT_TIMEOUT_DETAIL: &str = "typed PTY custody root-exit proof timed out";
const FORCE_COMPLETED_EARLY_DETAIL: &str = "PTY force transaction completed before target state";
const HANDOFF_NOT_DEFERRED_DETAIL: &str = "PTY custody handoff was not deferred to the reaper";
const ROOT_SIGNAL_MISSED_DETAIL: &str = "exact root signal did not reach its process";
const ROOT_STATUS_MISSING_DETAIL: &str = "fault reaper did not retain root exit status";
const REAPER_RESERVATION_FAILURE_DETAIL: &str = "reserve observed PTY fault reaper";

#[test]
fn freezing_custody_survives_root_exit_and_fault_handoff() {
    assert_typed_handoff(PtyProcessCustodyKind::Freezing);
}

#[test]
fn frozen_custody_survives_root_exit_and_fault_handoff() {
    assert_typed_handoff(PtyProcessCustodyKind::Frozen);
}

#[test]
fn killing_custody_survives_root_exit_and_fault_handoff() {
    assert_typed_handoff(PtyProcessCustodyKind::Killing);
}

#[test]
fn gone_pending_custody_survives_fault_handoff() {
    assert_typed_handoff(PtyProcessCustodyKind::GonePending);
}

#[test]
fn gone_reported_custody_survives_fault_handoff() {
    assert_typed_handoff(PtyProcessCustodyKind::GoneReported);
}

fn assert_typed_handoff(target: PtyProcessCustodyKind) {
    let (reaper, observations) = FaultReaperReservation::reserve_observed_for_test()
        .expect(REAPER_RESERVATION_FAILURE_DETAIL);
    let child = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.authorize_destructive_for_test();
    let output = ReaderBackedPtyOutput::for_test();
    let mut ownership =
        PtyOwnership::new_for_test_with_output(Box::new(child), tree, reaper, output).unwrap();
    drive_to_target(&mut ownership, target, root);
    let expected = PtyProcessCustodySnapshot { kind: target, root };
    assert_eq!(custody_snapshot(&ownership), expected);
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    assert!(
        matches!(
            ownership.settle_after_fault(completion),
            FaultSettlement::Deferred(_)
        ),
        "{HANDOFF_NOT_DEFERRED_DETAIL}"
    );
    assert_eq!(
        observations.recv_timeout(HANDOFF_TIMEOUT).unwrap(),
        expected
    );
    let outcome = receiver.receive_timeout(HANDOFF_TIMEOUT).unwrap();
    let status = outcome.status.expect(ROOT_STATUS_MISSING_DETAIL);
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert!(outcome.fault.is_none());
    assert!(!root.matches());
}

fn drive_to_target(
    ownership: &mut PtyOwnership,
    target: PtyProcessCustodyKind,
    root: ProcessIdentity,
) {
    let exit_state = match target {
        PtyProcessCustodyKind::Freezing => PtyProcessCustodyKind::Freezing,
        PtyProcessCustodyKind::Frozen => PtyProcessCustodyKind::Frozen,
        PtyProcessCustodyKind::Killing
        | PtyProcessCustodyKind::GonePending
        | PtyProcessCustodyKind::GoneReported
        | PtyProcessCustodyKind::MasterClosePending => PtyProcessCustodyKind::Killing,
        PtyProcessCustodyKind::Tracking => unreachable!(),
        PtyProcessCustodyKind::Survivor => unreachable!(),
    };
    let mut force = PtyForceTermination::new();
    let mut output = ownership.output_recovery();
    drive_force(ownership, &mut force, &mut output, exit_state);
    if exit_state != PtyProcessCustodyKind::Killing {
        assert!(
            root.signal(libc::SIGKILL).unwrap(),
            "{ROOT_SIGNAL_MISSED_DETAIL}"
        );
    }
    observe_and_mark_root_exit(ownership, root);
    assert_eq!(custody_snapshot(ownership).kind, exit_state);
    drive_force(ownership, &mut force, &mut output, target);
}

fn drive_force(
    ownership: &mut PtyOwnership,
    force: &mut PtyForceTermination,
    output: &mut crate::worker::pty_output_progress::PtyOutputRecovery,
    target: PtyProcessCustodyKind,
) {
    let deadline = Instant::now() + HANDOFF_TIMEOUT;
    while custody_snapshot(ownership).kind != target {
        assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
        let progress = force.poll(
            ownership.process_custody_mut(),
            &mut ForceOutput::Reader(output),
        );
        match progress {
            Ok(ForceTerminationPoll::Pending) => {}
            Ok(ForceTerminationPoll::Complete) if target == PtyProcessCustodyKind::GoneReported => {
            }
            Ok(ForceTerminationPoll::Complete) => panic!("{FORCE_COMPLETED_EARLY_DETAIL}"),
            Ok(ForceTerminationPoll::Unsupported(error)) => panic!("{error}"),
            Err(error) => panic!("{error:#}"),
        }
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}

fn observe_and_mark_root_exit(ownership: &mut PtyOwnership, root: ProcessIdentity) {
    let deadline = Instant::now() + HANDOFF_TIMEOUT;
    let mut observer = WaitidRootExitObserver::new(root);
    loop {
        if let Some(proof) = ownership.poll_root_exit(&mut observer).unwrap() {
            ownership.mark_root_exited_unreaped(proof).unwrap();
            return;
        }
        assert!(Instant::now() < deadline, "{ROOT_EXIT_TIMEOUT_DETAIL}");
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}

fn custody_snapshot(ownership: &PtyOwnership) -> PtyProcessCustodySnapshot {
    ownership
        .custody()
        .processes
        .as_ref()
        .unwrap()
        .snapshot_for_test()
}

#[path = "../missing/pty_missing_eof_handoff_tests.rs"]
mod missing_eof_handoff_tests;
