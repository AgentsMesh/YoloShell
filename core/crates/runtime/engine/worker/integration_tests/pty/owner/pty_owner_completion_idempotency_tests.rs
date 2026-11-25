use super::super::*;
use crate::process;
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use crate::worker::pty_exit_observer::RootExitObserver;
use crate::worker::pty_owner_command_channel::PtyOwnerCommandReceiver;
use crate::worker::pty_owner_handle::{OwnerSettlement, PtyOwnerCommand, PtyOwnerHandle};
use crate::worker::pty_ownership::PtyOwnership;
use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Weak, mpsc};
use std::time::Duration;

const OWNER_CHILD_EXECUTABLE: &str = "/bin/sleep";
const OWNER_CHILD_DURATION: &str = "30";
const OWNER_COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);
const OWNER_LATE_FAILURE_DETAIL: &str = "late owner completion must not replace success";
const OWNER_CHILD_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
const OWNER_CHILD_CLEANUP_POLL: Duration = Duration::from_millis(10);

struct PendingRootObserver;

impl RootExitObserver for PendingRootObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        Ok(None)
    }
}

fn spawn_owner_fixture() -> (PtyOwner, ProcessIdentity) {
    let child = spawn_detached(
        Path::new(OWNER_CHILD_EXECUTABLE),
        &[OWNER_CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let ownership = PtyOwnership::new_for_test(
        Box::new(child) as crate::process::OwnedPtyChild,
        tree,
        crate::worker::pty_ownership::reserve_fault_reaper().unwrap(),
    )
    .unwrap();
    let (_commands, receiver) = mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    (
        ownership.into_owner_for_test(
            Weak::new(),
            Box::new(PendingRootObserver),
            receiver,
            settlement,
        ),
        root,
    )
}

fn command_pair() -> (
    mpsc::Sender<PtyOwnerCommand>,
    mpsc::Receiver<PtyOwnerCommand>,
    tokio::sync::oneshot::Receiver<CommandCompletion>,
    tokio::sync::oneshot::Receiver<CommandCompletion>,
) {
    let (sender, receiver) = mpsc::channel();
    let (first_sender, first_receiver) = tokio::sync::oneshot::channel();
    let (second_sender, second_receiver) = tokio::sync::oneshot::channel();
    sender
        .send(PtyOwnerCommand::Terminate {
            force: false,
            response: first_sender,
        })
        .unwrap();
    sender
        .send(PtyOwnerCommand::Terminate {
            force: true,
            response: second_sender,
        })
        .unwrap();
    (sender, receiver, first_receiver, second_receiver)
}

#[tokio::test]
async fn owner_completion_is_single_assignment_and_drains_each_request_once() {
    let (mut owner, root) = spawn_owner_fixture();
    let (sender, receiver, first, second) = command_pair();
    let settlement: OwnerSettlement = Arc::clone(&owner.settlement);
    owner.commands = Some(PtyOwnerCommandReceiver::from_uncoordinated_receiver(
        receiver,
    ));

    owner.complete_commands_with(Ok(()));
    owner.complete_commands_with(Err(PtyOwnerCommandFailure::Owner(
        OWNER_LATE_FAILURE_DETAIL.to_owned(),
    )));

    assert_eq!(settlement.get(), Some(&Ok(())));
    assert!(
        tokio::time::timeout(OWNER_COMPLETION_TIMEOUT, first)
            .await
            .unwrap()
            .unwrap()
            .is_ok()
    );
    assert!(
        tokio::time::timeout(OWNER_COMPLETION_TIMEOUT, second)
            .await
            .unwrap()
            .unwrap()
            .is_ok()
    );

    let late = PtyOwnerHandle::for_test(sender, settlement)
        .submit_termination(true)
        .unwrap();
    late.wait().await.unwrap();

    settle_owner_fixture(owner, root);
}

#[test]
fn command_responses_are_ready_before_global_publication() {
    let (mut owner, root) = spawn_owner_fixture();
    let (_sender, receiver, mut first, mut second) = command_pair();
    let settlement = Arc::clone(&owner.settlement);
    owner.commands = Some(PtyOwnerCommandReceiver::from_uncoordinated_receiver(
        receiver,
    ));

    owner.complete_commands_before_publication_for_test(Ok(()), || {
        assert!(settlement.get().is_none());
        assert_eq!(first.try_recv().unwrap(), Ok(()));
        assert_eq!(second.try_recv().unwrap(), Ok(()));
    });

    assert_eq!(settlement.get(), Some(&Ok(())));
    settle_owner_fixture(owner, root);
}

fn settle_owner_fixture(owner: PtyOwner, root: ProcessIdentity) {
    root.signal(libc::SIGKILL).unwrap();
    drop(owner);
    wait_for_exit(root);
}

fn wait_for_exit(root: ProcessIdentity) {
    let deadline = std::time::Instant::now() + OWNER_CHILD_CLEANUP_TIMEOUT;
    while root.matches() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(OWNER_CHILD_CLEANUP_POLL);
    }
}
