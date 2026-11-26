use super::{RootExitObserver, RootWaitAuthority};
use crate::process::{CapturedProcessTree, ProcessIdentity, UnreapedRootExit};
use crate::worker::constants::STARTUP_CHILD_EXIT_TIMEOUT;
use crate::worker::pty_fault_reaper::FaultReaperReservation;
use crate::worker::pty_owner::PtyOwner;
use crate::worker::pty_owner_handle::PtyOwnerCommandFailure;
use crate::worker::pty_ownership::{PtyOwnership, ReapEvidence};
use anyhow::Result;
use std::process::Command;
use std::sync::{Arc, mpsc};

const SHELL_PATH: &str = "/bin/sh";
const SHELL_ARGUMENT: &str = "-c";
const RUN_UNTIL_SIGNALED: &str = "while :; do :; done";
const STALE_START_TIME_OFFSET: u64 = 1;
const OWNER_COMPLETION_CAPACITY: usize = 1;
const OWNER_TIMEOUT_MULTIPLIER: u32 = 2;
const OWNER_DID_NOT_SETTLE_DETAIL: &str = "owner did not settle mismatched root-exit proof";
const OWNER_FAULT_TYPE_MISMATCH_DETAIL: &str =
    "mismatched root-exit proof did not publish an owner fault";
const FAULT_REAP_DID_NOT_SETTLE_DETAIL: &str =
    "fault reaper did not settle mismatched root-exit proof";
const FAULT_REAPER_PANICKED_DETAIL: &str = "fault reaper panicked during proof fault cleanup";
const FAULT_REAP_MISSING_FAILURE_DETAIL: &str =
    "fault reaper lost the initiating mismatched-proof failure";
const ROOT_EXIT_STATUS_MISSING_DETAIL: &str =
    "fault reaper did not retain the exact root exit status";
const EARLY_ROOT_EXIT_DETAIL: &str = "owner handoff completed fault cleanup synchronously";

struct MismatchedProofObserver {
    proof: Option<UnreapedRootExit>,
}

impl RootExitObserver for MismatchedProofObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &RootWaitAuthority,
    ) -> Result<Option<UnreapedRootExit>> {
        Ok(self.proof.take())
    }
}

#[test]
fn mismatched_root_exit_proof_enters_bounded_fault_cleanup() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(SHELL_PATH).args([SHELL_ARGUMENT, RUN_UNTIL_SIGNALED]),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let stale_root = ProcessIdentity {
        start_time: root.start_time.wrapping_add(STALE_START_TIME_OFFSET),
        ..root
    };
    let proof = tree.root_exit_proof_for_test(stale_root);
    let (reaper, held_reaper) = FaultReaperReservation::reserve_held_for_test().unwrap();
    let ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let (commands, receiver) = mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner_settlement = Arc::clone(&settlement);
    let (finished, completion) = mpsc::sync_channel(OWNER_COMPLETION_CAPACITY);

    let owner = std::thread::spawn(move || {
        ownership
            .into_owner_for_test(
                std::sync::Weak::new(),
                Box::new(MismatchedProofObserver { proof: Some(proof) }),
                receiver,
                owner_settlement,
            )
            .run();
        let _ = finished.send(());
    });

    let timeout = STARTUP_CHILD_EXIT_TIMEOUT.saturating_mul(OWNER_TIMEOUT_MULTIPLIER);
    completion
        .recv_timeout(timeout)
        .expect(OWNER_DID_NOT_SETTLE_DETAIL);
    owner.join().unwrap();
    drop(commands);

    let owner_fault = match settlement.get() {
        Some(Err(PtyOwnerCommandFailure::Owner(detail))) => Some(detail.clone()),
        _ => None,
    };
    let mut held_reap = held_reaper.wait_for_handoff();
    let root_was_live_at_handoff = root.matches();
    let reap_completion = held_reap.observe_completion_for_test();
    let reaper = std::thread::spawn(move || held_reap.resume());
    let outcome = reap_completion
        .receive_timeout(timeout)
        .expect(FAULT_REAP_DID_NOT_SETTLE_DETAIL);
    reaper.join().expect(FAULT_REAPER_PANICKED_DETAIL);

    let owner_fault = owner_fault.expect(OWNER_FAULT_TYPE_MISMATCH_DETAIL);
    assert!(root_was_live_at_handoff, "{EARLY_ROOT_EXIT_DETAIL}");
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    let reap_fault = outcome
        .fault
        .as_ref()
        .map(ToString::to_string)
        .expect(FAULT_REAP_MISSING_FAILURE_DETAIL);
    assert!(reap_fault.contains(&owner_fault), "{reap_fault}");
    assert!(
        outcome.status.is_some(),
        "{ROOT_EXIT_STATUS_MISSING_DETAIL}"
    );
    assert!(!root.matches());
}
