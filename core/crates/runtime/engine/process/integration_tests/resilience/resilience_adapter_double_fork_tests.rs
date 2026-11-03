use super::captured_targets::test_hook::install_test_signal_hook;
use super::retained_exact_double_fork_support::DoubleForkFixture;
use super::{
    ProcessIdentity, ResilienceFreezePoll, ResilienceFreezingProcessTree,
    ResilienceFrozenProcessTree, ResilienceKillCommitPoll, ResilienceKillPendingProcessTree,
    ResilienceProcessTree,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
#[path = "../../test_support/resilience_adapter_double_fork_constants.rs"]
mod constants;
use constants::*;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const ROOT_TARGET_MISSING_DETAIL: &str = "adapter capture omitted its double-fork root";
const HOOK_NOT_INVOKED_DETAIL: &str = "first SIGSTOP did not trigger the double fork";
const FIRST_FREEZE_COMPLETED_DETAIL: &str = "double-fork race froze on its first observation";
const ESCAPED_IDENTITY_MISSING_DETAIL: &str =
    "setsid leaf did not enter retained adapter identities";
const FREEZE_TIMEOUT_DETAIL: &str = "adapter did not freeze the expanded double-fork closure";
const KILL_COMMIT_FAILURE_DETAIL: &str = "adapter could not commit the double-fork kill";
const ESCAPED_LEAF_SURVIVED_DETAIL: &str = "retained setsid leaf survived adapter cleanup";

#[test]
fn first_sigstop_double_fork_setsid_leaf_is_retained_and_killed() {
    let fixture = Rc::new(RefCell::new(
        DoubleForkFixture::spawn_with_retained_bootstrap_master(false),
    ));
    let root = fixture.borrow().root_identity;
    let proof = fixture.borrow().proof.clone();
    let tree = ResilienceProcessTree::capture_pty(root, proof).unwrap();
    eprintln!(
        "phase={CAPTURED_PHASE} root={root:?} identities={:?}",
        tree.identities()
    );
    let root_index = tree
        .identities()
        .iter()
        .position(|identity| *identity == root)
        .expect(ROOT_TARGET_MISSING_DETAIL);
    let escaped = Rc::new(RefCell::new(None));
    let escaped_from_hook = Rc::clone(&escaped);
    let fixture_from_hook = Rc::clone(&fixture);
    let hook = install_test_signal_hook(libc::SIGSTOP, root_index, move || {
        let leaf = fixture_from_hook.borrow_mut().release_leaf();
        eprintln!("phase={LEAF_RELEASED_PHASE} root={root:?} escaped={leaf:?}");
        *escaped_from_hook.borrow_mut() = Some(leaf);
    });

    let freezing = match tree.begin_force().poll().unwrap() {
        ResilienceFreezePoll::Pending(freezing) => freezing,
        ResilienceFreezePoll::Frozen(_) => panic!("{FIRST_FREEZE_COMPLETED_DETAIL}"),
    };
    drop(hook);
    let leaf = escaped.borrow().expect(HOOK_NOT_INVOKED_DETAIL);
    assert!(leaf.matches(), "{ESCAPED_LEAF_SURVIVED_DETAIL}");
    eprintln!(
        "phase={FIRST_FREEZE_PHASE} root={root:?} escaped={leaf:?} snapshot={}",
        freezing.state_snapshot_for_test()
    );
    let frozen = freeze_with_identity(freezing, leaf);
    eprintln!(
        "phase={FROZEN_PHASE} root={root:?} escaped={leaf:?} identities={:?}",
        frozen.identities()
    );

    assert!(
        frozen.identities().contains(&leaf),
        "{ESCAPED_IDENTITY_MISSING_DETAIL}"
    );
    let pending = commit_kill(frozen, leaf);
    eprintln!("phase={KILL_COMMITTED_PHASE} root={root:?} escaped={leaf:?}");
    drop(pending);
    wait_for_gone(leaf);
}

fn freeze_with_identity(
    mut freezing: ResilienceFreezingProcessTree,
    escaped: ProcessIdentity,
) -> ResilienceFrozenProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        freezing = match freezing.poll() {
            Ok(ResilienceFreezePoll::Pending(next)) => next,
            Ok(ResilienceFreezePoll::Frozen(frozen)) => {
                assert!(
                    frozen.identities().contains(&escaped),
                    "{ESCAPED_IDENTITY_MISSING_DETAIL}"
                );
                return frozen;
            }
            Err(error) => panic!("{FREEZE_TIMEOUT_DETAIL}: {error:?}"),
        };
        assert!(
            Instant::now() < deadline,
            "{FREEZE_TIMEOUT_DETAIL}: escaped={escaped:?}, escaped_live={}, {}",
            escaped.matches(),
            freezing.state_snapshot_for_test()
        );
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn commit_kill(
    mut frozen: ResilienceFrozenProcessTree,
    escaped: ProcessIdentity,
) -> ResilienceKillPendingProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        match frozen.kill() {
            Ok(ResilienceKillCommitPoll::Pending(pending)) => return pending,
            Ok(ResilienceKillCommitPoll::Refreeze(freezing)) => {
                frozen = freeze_with_identity(freezing, escaped);
            }
            Err(error) => panic!("{KILL_COMMIT_FAILURE_DETAIL}: {error:?}"),
        }
        assert!(Instant::now() < deadline, "{KILL_COMMIT_FAILURE_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_gone(identity: ProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline, "{ESCAPED_LEAF_SURVIVED_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
