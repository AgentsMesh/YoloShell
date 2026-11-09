use super::*;
use crate::worker::pty_ownership::ReapEvidence;
use crate::worker::pty_process_custody::{PtyProcessCustodyKind, PtyProcessCustodySnapshot};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Weak, mpsc};
use std::time::{Duration, Instant};

const PENDING_TERMINATION_COUNT: usize = 2;
const OWNER_BRANCH_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const OWNER_BRANCH_SLEEP_DURATION: &str = "30";
const OWNER_BRANCH_REFRESH_DELAY: Duration = Duration::from_secs(1);
const OWNER_BRANCH_CLEANUP_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const OWNER_BRANCH_TERMINATION_FAILURE: &str = "injected owner force-termination refresh failure";
const OWNER_BRANCH_CLEANUP_TIMEOUT_DETAIL: &str =
    "owned fault recovery did not publish exact cleanup completion";

struct PendingRootObserver;

impl crate::worker::pty_exit_observer::RootExitObserver for PendingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &crate::process::RootWaitAuthority,
    ) -> anyhow::Result<Option<crate::process::UnreapedRootExit>> {
        Ok(None)
    }
}

#[tokio::test]
async fn unsupported_termination_completes_every_pending_request() {
    let mut pending = Vec::new();
    let mut completions = Vec::new();
    for _ in 0..PENDING_TERMINATION_COUNT {
        let (sender, receiver) = oneshot::channel();
        pending.push(sender);
        completions.push(receiver);
    }

    PtyOwner::complete_unsupported(
        &mut pending,
        crate::process::DestructiveSessionUnavailable::new(),
    );

    assert!(pending.is_empty());
    for completion in completions {
        assert!(matches!(
            completion.await.unwrap(),
            Err(PtyOwnerCommandFailure::DestructiveUnavailable(_))
        ));
    }
}

#[test]
fn force_termination_failure_enters_owned_fault_recovery() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(OWNER_BRANCH_SLEEP_EXECUTABLE)).arg(OWNER_BRANCH_SLEEP_DURATION),
    );
    let root = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(root).unwrap();
    tree.authorize_destructive_for_test();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let (reservation, held_reaper) =
        crate::worker::pty_fault_reaper::FaultReaperReservation::reserve_held_for_test().unwrap();
    let mut ownership =
        crate::worker::pty_ownership::PtyOwnership::new_for_test(child, tree, reservation).unwrap();
    ownership
        .tree_mut()
        .fail_next_refresh_for_test(OWNER_BRANCH_TERMINATION_FAILURE);
    let (commands, receiver) = mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let (response, completion) = oneshot::channel();
    commands
        .send(PtyOwnerCommand::Terminate {
            force: true,
            response,
        })
        .unwrap();
    let mut owner = ownership.into_owner_for_test(
        Weak::new(),
        Box::new(PendingRootObserver),
        receiver,
        Arc::clone(&settlement),
    );
    owner.next_refresh = Instant::now() + OWNER_BRANCH_REFRESH_DELAY;

    owner.run();

    let command_error = completion.blocking_recv().unwrap().unwrap_err();
    assert!(command_error.contains(OWNER_BRANCH_TERMINATION_FAILURE));
    let settlement_error = settlement.get().unwrap().as_ref().unwrap_err();
    assert!(settlement_error.contains(OWNER_BRANCH_TERMINATION_FAILURE));
    let mut held_reap = held_reaper.wait_for_handoff();
    assert_eq!(
        held_reap.custody_snapshot_for_test(),
        PtyProcessCustodySnapshot {
            kind: PtyProcessCustodyKind::Freezing,
            root,
        }
    );
    assert!(root.matches());
    let cleanup_completion = held_reap.observe_completion_for_test();
    let reaper = std::thread::spawn(move || held_reap.resume());
    let outcome = cleanup_completion
        .receive_timeout(OWNER_BRANCH_CLEANUP_TIMEOUT)
        .expect(OWNER_BRANCH_CLEANUP_TIMEOUT_DETAIL);
    reaper.join().unwrap();

    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert_eq!(outcome.status.unwrap().signal(), Some(libc::SIGKILL));
    assert!(
        outcome
            .fault
            .as_ref()
            .unwrap()
            .chain()
            .any(|cause| cause.to_string().contains(OWNER_BRANCH_TERMINATION_FAILURE))
    );
    assert!(matches!(
        cleanup_completion.receive_timeout(Duration::ZERO),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(!root.matches());
}
