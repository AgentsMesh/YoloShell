use super::*;
use crate::worker::constants::{BOOTSTRAP_TEST_EXACT_ARGUMENT, BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const HELPER_ENTRY: &str = "worker::termination_test_hook::tests::pause_after_tombstone_helper";
const HELPER_SESSION_ENVIRONMENT: &str = "YOLO_TEST_TERMINATION_SESSION";
const RELATIVE_TEST_ROOT: &str = "relative-test-root";
const STORAGE_BLOCKER_FILE: &str = "storage-blocker";
const REACHED_TIMEOUT: Duration = Duration::from_secs(2);
const REACHED_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[test]
fn barrier_claim_is_one_shot_and_rearm_clears_completion() {
    let root = tempfile::tempdir().unwrap();
    let barrier = TerminationTestBarrier::new(root.path(), Uuid::new_v4()).unwrap();

    barrier.arm().unwrap();
    assert!(barrier.holds_owner().unwrap());
    assert!(!barrier.reached().unwrap());
    assert!(!barrier.released().unwrap());
    assert!(barrier.claim().unwrap());
    assert!(barrier.holds_owner().unwrap());
    assert!(barrier.reached().unwrap());
    assert!(!barrier.claim().unwrap());
    barrier.release().unwrap();
    assert!(!barrier.holds_owner().unwrap());
    assert!(barrier.released().unwrap());

    barrier.arm().unwrap();
    assert!(!barrier.reached().unwrap());
    assert!(!barrier.released().unwrap());
    assert!(TerminationTestBarrier::new(RELATIVE_TEST_ROOT, Uuid::new_v4()).is_err());
    assert!(!owner_is_held(Uuid::new_v4()).unwrap());
}

#[test]
fn barrier_storage_failures_preserve_operation_context() {
    let root = tempfile::tempdir().unwrap();
    let barrier = TerminationTestBarrier::new(root.path(), Uuid::new_v4()).unwrap();
    assert!(barrier.release().is_err());
    assert!(remove_if_present(root.path()).is_err());

    let blocker = root.path().join(STORAGE_BLOCKER_FILE);
    std::fs::write(&blocker, MARKER_CONTENT).unwrap();
    let blocked = TerminationTestBarrier::new(&blocker, Uuid::new_v4()).unwrap();
    assert!(blocked.arm().is_err());
    assert!(blocked.reached().is_err());
    assert!(blocked.released().is_err());
    assert!(blocked.claim().is_err());
}

#[test]
fn pause_after_tombstone_waits_for_release_and_claims_only_once() {
    let root = tempfile::tempdir().unwrap();
    let terminal_id = Uuid::new_v4();
    let barrier = TerminationTestBarrier::new(root.path(), terminal_id).unwrap();
    barrier.arm().unwrap();
    let mut helper = HelperChild::spawn(root.path(), terminal_id);

    wait_until(|| barrier.reached().unwrap());
    assert!(helper.child_mut().try_wait().unwrap().is_none());
    barrier.release().unwrap();
    assert!(helper.wait().success());

    let mut duplicate = HelperChild::spawn(root.path(), terminal_id);
    assert!(duplicate.wait().success());
}

#[test]
fn pause_after_tombstone_helper() {
    let Some(terminal_id) = std::env::var_os(HELPER_SESSION_ENVIRONMENT) else {
        return;
    };
    let terminal_id = Uuid::parse_str(&terminal_id.to_string_lossy()).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(pause_after_tombstone(terminal_id))
        .unwrap();
}

struct HelperChild(Option<Child>);

impl HelperChild {
    fn spawn(root: &Path, terminal_id: Uuid) -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    BOOTSTRAP_TEST_EXACT_ARGUMENT,
                    HELPER_ENTRY,
                    BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
                ])
                .env(BARRIER_ENVIRONMENT, root)
                .env(HELPER_SESSION_ENVIRONMENT, terminal_id.to_string()),
        );
        Self(Some(child))
    }

    fn child_mut(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.0.take().unwrap().wait().unwrap()
    }
}

impl Drop for HelperChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            if let Ok(status) = child.wait() {
                yolo_coverage_test_support::discard_reaped_kill_profile(child.id(), status).unwrap();
            }
        }
    }
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + REACHED_TIMEOUT;
    while !predicate() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(REACHED_POLL_INTERVAL);
    }
}
