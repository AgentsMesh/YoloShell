use super::{FreezePoll, KillCommitPoll};
use crate::process::CapturedProcessTree;
use crate::process::captured_tree_macos_support::LineageFixture;

const NO_PROOF_CONFIRMATION_POLLS: usize = 4;
const UNAUTHORIZED_FREEZE_DETAIL: &str = "coalesced fork evidence minted no-proof kill authority";

#[test]
fn coalesced_root_fork_without_exact_pty_proof_remains_fail_closed() {
    let mut fixture = LineageFixture::spawn();
    let root = fixture.root_identity();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let root_exit = tree.root_exit_proof_for_test(root);
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{UNAUTHORIZED_FREEZE_DETAIL}"),
    };
    let mut frozen = match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{UNAUTHORIZED_FREEZE_DETAIL}"),
    };
    assert!(root.signal(libc::SIGCONT).unwrap());
    let leaf = fixture.release_and_observe_leaf();
    frozen.mark_root_exited_unreaped(root_exit).unwrap();
    let version = frozen.closure_version();
    let mut freezing = match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Refreeze(freezing) => freezing,
        KillCommitPoll::Pending(_) => panic!("{UNAUTHORIZED_FREEZE_DETAIL}"),
    };
    let root_live = freezing.tree.live_root();
    freezing.tree.events.inject_unresolved_fork(root_live);

    for _poll in 0..NO_PROOF_CONFIRMATION_POLLS {
        freezing = match freezing.poll().unwrap() {
            FreezePoll::Pending(freezing) => freezing,
            FreezePoll::Frozen(_) => panic!("{UNAUTHORIZED_FREEZE_DETAIL}"),
        };
    }
    assert!(leaf.matches());
    fixture.reap_root();
}
