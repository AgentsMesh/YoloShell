use super::test_support::kill_with_retry;
use super::{FreezePoll, KillPoll};
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const GONE_TIMEOUT: Duration = Duration::from_secs(5);
const GONE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROVEN_ROOT_RETIRING_COUNT: usize = 1;
const EXPECTED_PENDING_DETAIL: &str = "first freeze observation unexpectedly completed";
#[cfg(not(target_os = "linux"))]
const EXPECTED_FROZEN_DETAIL: &str = "second freeze observation stayed pending";
const FIRST_EMPTY_COMPLETED_DETAIL: &str = "first empty observation completed gone proof";
const GONE_TIMEOUT_DETAIL: &str = "proven exited root prevented gone completion";
const UNEXPECTED_GONE_ERROR_DETAIL: &str = "gone polling returned an unexpected error";

#[test]
fn proven_exited_reaped_root_without_capability_reaches_gone() {
    let (mut child, tree) = spawn_tree();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    };
    #[cfg(target_os = "linux")]
    let frozen = super::test_support::freeze_with_retry(freezing);
    #[cfg(not(target_os = "linux"))]
    let frozen = match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    };
    let mut pending = kill_with_retry(frozen);
    pending.mark_root_exited_unreaped(proof).unwrap();
    child.wait().unwrap();
    pending = match pending.poll().unwrap() {
        KillPoll::Pending(pending) => pending,
        KillPoll::Gone(_) => panic!("{FIRST_EMPTY_COMPLETED_DETAIL}"),
        KillPoll::Refreeze(_) => panic!("{FIRST_EMPTY_COMPLETED_DETAIL}"),
    };
    assert_eq!(pending.tree.retiring_count(), PROVEN_ROOT_RETIRING_COUNT);
    assert!(pending.tree.descendant_observation_is_settled());
    let deadline = Instant::now() + GONE_TIMEOUT;
    loop {
        pending = match pending.poll() {
            Ok(KillPoll::Pending(pending)) => pending,
            Ok(KillPoll::Gone(completion)) => {
                assert_eq!(completion.root(), root);
                return;
            }
            Ok(KillPoll::Refreeze(_)) => panic!("{UNEXPECTED_GONE_ERROR_DETAIL}"),
            Err(error) => panic!("{UNEXPECTED_GONE_ERROR_DETAIL}: {error:?}"),
        };
        assert!(Instant::now() < deadline, "{GONE_TIMEOUT_DETAIL}");
        std::thread::sleep(GONE_POLL_INTERVAL);
    }
}

fn spawn_tree() -> (Child, CapturedProcessTree) {
    let child = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let mut tree = CapturedProcessTree::capture(identity).unwrap();
    tree.authorize_destructive_for_test();
    (child, tree)
}
