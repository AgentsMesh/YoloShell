use super::test_support::{freeze_with_retry, kill_with_retry};
use super::{FreezePoll, FreezingProcessTree, KillCommitPoll, KillPendingProcessTree, KillPoll};
use crate::process::PtyLineageFixture;
use crate::process::captured_targets::install_test_signal_failure;
use crate::process::captured_tree_macos_wait::wait_for_exit_unreaped;
use crate::process::{
    CapturedProcessTree, DetachedStandardError, ProcessIdentity, macos_kernel, spawn_detached,
};
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const RESUMED_ESCAPE_CAPABILITY: &str = "force-resumed-escape";
const SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(5);
const SETTLEMENT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const RESET_FAILURE_DETAIL: &str = "injected gone-confirmation refresh failure";
const REFREEZE_FAILURE_DETAIL: &str = "injected refreeze exact stop failure";
const SETTLEMENT_TIMEOUT_DETAIL: &str = "killed process tree did not settle empty";
const EXPECTED_PENDING_DETAIL: &str = "first force confirmation unexpectedly completed";
const EXPECTED_FROZEN_DETAIL: &str = "second exact stop observation stayed pending";
const EXPECTED_GONE_DETAIL: &str = "second settled empty observation stayed pending";
const EXPECTED_RESET_ERROR_DETAIL: &str = "gone reset refresh unexpectedly succeeded";
const EXPECTED_REFREEZE_DETAIL: &str = "resumed root did not invalidate its output fence";
const EXPECTED_REFREEZE_ERROR_DETAIL: &str = "refreeze unexpectedly succeeded";

pub(super) struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    pub(super) fn spawn() -> (Self, CapturedProcessTree) {
        let child = spawn_detached(
            Path::new(SLEEP_EXECUTABLE),
            &[SLEEP_DURATION.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(identity).unwrap();
        (Self { child: Some(child) }, tree)
    }

    pub(super) fn reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.wait().unwrap();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
        self.reap();
    }
}

#[test]
fn audit_token_stop_reaches_sstop_and_typed_gone_proof() {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let before = macos_kernel::metadata(root.pid).unwrap().unwrap();
    let freezing = expect_pending(tree.begin_force());
    let stopped = macos_kernel::metadata(root.pid).unwrap().unwrap();
    assert_eq!(stopped.unique_id, before.unique_id);
    assert_eq!(stopped.status, libc::SSTOP);
    let frozen = match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    };
    let mut pending = kill_with_retry(frozen);
    wait_for_exit_unreaped(root);
    pending.mark_root_exited_unreaped(proof).unwrap();
    settle_empty(&mut pending);
    let mut pending = expect_kill_pending(pending);
    pending.tree.fail_next_refresh(RESET_FAILURE_DETAIL);
    let error = match pending.poll() {
        Err(error) => error,
        Ok(_) => panic!("{EXPECTED_RESET_ERROR_DETAIL}"),
    };
    assert!(
        error
            .error()
            .chain()
            .any(|cause| cause.to_string().contains(RESET_FAILURE_DETAIL))
    );
    let pending = error.into_state();
    assert_eq!(pending.root(), root);
    let pending = expect_kill_pending(pending);
    let (_foreign_child, foreign_tree) = ChildGuard::spawn();
    let foreign_root = foreign_tree.root();
    match pending.poll().unwrap() {
        KillPoll::Gone(completion) => {
            assert_eq!(completion.root(), root);
            assert_ne!(completion.root(), foreign_root);
            assert!(foreign_root.matches());
            let (reported, ()) = completion
                .report(|authority| {
                    assert_eq!(authority.root(), root);
                    Ok(())
                })
                .unwrap();
            let reported = reported.retire_direct_for_test();
            assert!(reported.session_is_retired());
        }
        KillPoll::Pending(_) => panic!("{EXPECTED_GONE_DETAIL}"),
        KillPoll::Refreeze(_) => panic!("{EXPECTED_GONE_DETAIL}"),
    }
    child.reap();
}

#[test]
fn kill_revalidates_a_resumed_root_and_its_setsid_escape() {
    let mut fixture = PtyLineageFixture::spawn(RESUMED_ESCAPE_CAPABILITY);
    let root = fixture.root_identity();
    let tree = fixture.take_tree();
    let proof = tree.root_exit_proof_for_test(root);
    let mut frozen = freeze_with_retry(expect_pending(tree.begin_force()));

    assert!(root.signal(libc::SIGCONT).unwrap());
    let escaped = fixture.release_and_observe_leaf();
    assert!(escaped.matches());
    frozen.mark_root_exited_unreaped(proof).unwrap();

    let _pending = kill_with_retry(frozen);
    fixture.wait_for_leaf_exit();
    fixture.reap_root();
}

#[test]
fn refreeze_error_returns_freezing_authority() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let frozen = expect_frozen(expect_pending(tree.begin_force()));
    assert!(root.signal(libc::SIGCONT).unwrap());
    let failure =
        install_test_signal_failure(libc::SIGSTOP, usize::default(), REFREEZE_FAILURE_DETAIL);
    let version = frozen.closure_version();
    let freezing = match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Refreeze(freezing) => freezing,
        KillCommitPoll::Pending(_) => panic!("{EXPECTED_REFREEZE_DETAIL}"),
    };
    let error = match freezing.poll() {
        Err(error) => error,
        Ok(_) => panic!("{EXPECTED_REFREEZE_ERROR_DETAIL}"),
    };
    assert!(
        error
            .error()
            .chain()
            .any(|cause| cause.to_string().contains(REFREEZE_FAILURE_DETAIL))
    );
    let freezing = error.into_state();
    assert_eq!(freezing.root(), root);
    drop(failure);
    let frozen = expect_frozen(expect_pending(freezing));
    let _pending = kill_with_retry(frozen);
}

fn expect_pending(freezing: FreezingProcessTree) -> FreezingProcessTree {
    match freezing.poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    }
}

fn expect_frozen(freezing: FreezingProcessTree) -> super::FrozenProcessTree {
    match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    }
}

fn expect_kill_pending(pending: KillPendingProcessTree) -> KillPendingProcessTree {
    match pending.poll().unwrap() {
        KillPoll::Pending(pending) => pending,
        KillPoll::Gone(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
        KillPoll::Refreeze(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    }
}

fn settle_empty(pending: &mut KillPendingProcessTree) {
    let deadline = Instant::now() + SETTLEMENT_TIMEOUT;
    loop {
        pending.tree.refresh().unwrap();
        if pending.tree.descendant_observation_is_settled()
            && pending.tree.live_identities().unwrap().is_empty()
        {
            return;
        }
        assert!(Instant::now() < deadline, "{SETTLEMENT_TIMEOUT_DETAIL}");
        std::thread::sleep(SETTLEMENT_POLL_INTERVAL);
    }
}
