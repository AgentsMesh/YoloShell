use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity, RootWaitAuthority};
use crate::worker::pty_force_termination::PtyForceTermination;
use crate::worker::pty_output_progress::ReaderBackedPtyOutput;
use crate::worker::pty_process_custody::PtyProcessCustody;
use std::path::Path;
use std::process::Command;

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const OWNERSHIP_CREATION_FAILURE: &str = "create ownership for emergency escrow test";

pub(super) fn live_custody() -> (PtyOwnershipCustody, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(CHILD_EXECUTABLE)).arg(CHILD_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    (
        PtyOwnershipCustody {
            authority: Some(authority),
            processes: Some(PtyProcessCustody::tracking(tree)),
            force: PtyForceTermination::new(),
            output_progress: super::super::FaultOutput::Reader(ReaderBackedPtyOutput::for_test()),
            root_exit_proof: None,
            access: crate::worker::pty_ownership::PtyCustodyAccess::Control,
            missing_eof: crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        },
        root,
    )
}

pub(super) fn live_ownership(
    reaper: FaultReaperReservation,
) -> (crate::worker::pty_ownership::PtyOwnership, ProcessIdentity) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(CHILD_EXECUTABLE)).arg(CHILD_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let ownership =
        crate::worker::pty_ownership::PtyOwnership::new_for_test(Box::new(child), tree, reaper)
            .expect(OWNERSHIP_CREATION_FAILURE);
    (ownership, root)
}
