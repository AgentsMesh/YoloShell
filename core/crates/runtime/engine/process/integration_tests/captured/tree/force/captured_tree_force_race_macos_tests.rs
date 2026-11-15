use super::test_support::{freeze_with_retry, kill_with_retry};
use super::{FreezePoll, KillPoll};
use crate::process::captured_targets::test_hook::install_test_signal_hook;
use crate::process::{CapturedProcessTree, PtyLineageFixture};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ROOT_TARGET_INDEX: usize = 0;
const PRE_KILL_FORK_CAPABILITY: &str = "force-pre-kill-fork";
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const RECOVERY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const EXPECTED_PENDING_DETAIL: &str = "force transaction completed before the race was injected";
const EXPECTED_RECOVERY_PENDING_DETAIL: &str = "escaped producer was reported gone before recovery";
const RECOVERY_TIMEOUT_DETAIL: &str = "escaped producer survived kill-pending recovery";
const EXPECTED_REFREEZE_DETAIL: &str = "new producer did not reopen process freeze";
const EXPECTED_PUBLICATION_FENCE_DETAIL: &str =
    "new producer bypassed its watcher publication fence";
const EXPECTED_CAPTURED_PRODUCER_DETAIL: &str =
    "new producer was not retained while its watcher publication was pending";

#[test]
fn producer_created_before_first_sigkill_is_recovered_by_kill_pending() {
    let fixture = Arc::new(Mutex::new(PtyLineageFixture::spawn(
        PRE_KILL_FORK_CAPABILITY,
    )));
    let root = fixture.lock().unwrap().root_identity();
    let tree = fixture.lock().unwrap().take_tree();
    let root_exit = tree.root_exit_proof_for_test(root);
    let frozen = expect_frozen(tree);
    let escaped = Arc::new(Mutex::new(None));
    let escaped_from_hook = Arc::clone(&escaped);
    let fixture_from_hook = Arc::clone(&fixture);
    let _hook = install_test_signal_hook(libc::SIGKILL, ROOT_TARGET_INDEX, move || {
        assert!(root.signal(libc::SIGCONT).unwrap());
        let leaf = fixture_from_hook.lock().unwrap().release_and_capture_leaf();
        *escaped_from_hook.lock().unwrap() = Some(leaf);
    });

    let mut pending = kill_with_retry(frozen);
    fixture
        .lock()
        .unwrap()
        .wait_for_root_exit_and_leaf_reparent();
    pending.mark_root_exited_unreaped(root_exit).unwrap();
    let leaf = escaped.lock().unwrap().expect(EXPECTED_PENDING_DETAIL);
    let pending = match pending.poll().unwrap() {
        KillPoll::Pending(pending) => pending,
        KillPoll::Refreeze(_) | KillPoll::Gone(_) => panic!("{EXPECTED_PUBLICATION_FENCE_DETAIL}"),
    };
    let snapshot = pending.state_snapshot_for_test();
    assert!(
        snapshot.captured.contains(&leaf),
        "{EXPECTED_CAPTURED_PRODUCER_DETAIL}: {snapshot:?}"
    );
    assert!(
        !snapshot.publication_quiescent && snapshot.event_reconciliation_pending,
        "{EXPECTED_PUBLICATION_FENCE_DETAIL}: {snapshot:?}"
    );
    let freezing = expect_refreeze(pending);
    assert!(leaf.matches(), "{EXPECTED_RECOVERY_PENDING_DETAIL}");
    let mut pending = kill_with_retry(expect_refrozen(freezing));
    fixture.lock().unwrap().wait_for_leaf_exit();
    let deadline = Instant::now() + RECOVERY_TIMEOUT;
    let completion = loop {
        pending = match pending.poll() {
            Ok(KillPoll::Pending(pending)) => pending,
            Ok(KillPoll::Gone(completion)) => break completion,
            Ok(KillPoll::Refreeze(_)) => panic!("{EXPECTED_REFREEZE_DETAIL}"),
            Err(error) => panic!("{RECOVERY_TIMEOUT_DETAIL}: {error:?}"),
        };
        assert!(Instant::now() < deadline, "{RECOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(RECOVERY_POLL_INTERVAL);
    };
    assert!(!leaf.matches(), "{EXPECTED_RECOVERY_PENDING_DETAIL}");
    assert_eq!(
        completion.root(),
        root,
        "{EXPECTED_RECOVERY_PENDING_DETAIL}"
    );
    fixture.lock().unwrap().reap_root();
}

fn expect_refreeze(mut pending: super::KillPendingProcessTree) -> super::FreezingProcessTree {
    let deadline = Instant::now() + RECOVERY_TIMEOUT;
    loop {
        pending = match pending.poll().unwrap() {
            KillPoll::Refreeze(freezing) => return freezing,
            KillPoll::Pending(pending) => pending,
            KillPoll::Gone(_) => panic!("{EXPECTED_REFREEZE_DETAIL}"),
        };
        assert!(Instant::now() < deadline, "{RECOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(RECOVERY_POLL_INTERVAL);
    }
}

fn expect_refrozen(mut freezing: super::FreezingProcessTree) -> super::FrozenProcessTree {
    let deadline = Instant::now() + RECOVERY_TIMEOUT;
    loop {
        freezing = match freezing.poll().unwrap() {
            FreezePoll::Pending(freezing) => freezing,
            FreezePoll::Frozen(frozen) => return frozen,
        };
        assert!(Instant::now() < deadline, "{RECOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(RECOVERY_POLL_INTERVAL);
    }
}

fn expect_frozen(tree: CapturedProcessTree) -> super::FrozenProcessTree {
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    };
    freeze_with_retry(freezing)
}
