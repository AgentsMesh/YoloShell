use super::test_support::{freeze_with_retry, kill_error_with_retry};
use super::{FreezePoll, FreezingProcessTree, FrozenProcessTree};
use crate::process::captured_targets::{install_test_signal_failure, install_test_signal_noop};
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use std::ffi::OsString;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

#[path = "../../../../test_support/captured_tree/captured_tree_force_discovery_spawn.rs"]
mod discovery_spawn;

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const TREE_COMMAND: &str = "/bin/sleep 30 & wait $!; exec /bin/sleep 30";
const FIRST_TARGET_INDEX: usize = 0;
const DESCENDANT_TARGET_INDEX: usize = 0;
const ROOT_TARGET_INDEX: usize = 1;
const EXPECTED_TREE_TARGETS: usize = 2;
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const REFRESH_FAILURE_DETAIL: &str = "injected force refresh failure";
const STOP_FAILURE_DETAIL: &str = "injected exact stop failure";
const KILL_FAILURE_DETAIL: &str = "injected exact root kill failure";
const EXPECTED_PENDING_DETAIL: &str = "force freeze unexpectedly completed";
const EXPECTED_FROZEN_DETAIL: &str = "second stable stop observation stayed pending";
const EXPECTED_ERROR_DETAIL: &str = "force transition unexpectedly succeeded";
const TARGET_DISCOVERY_DETAIL: &str = "force test descendant was not discovered";
const DESCENDANT_EXIT_DETAIL: &str = "partially killed descendant remained active";

struct ProcessGuard {
    child: Option<Child>,
    identity: ProcessIdentity,
}

impl ProcessGuard {
    fn spawn(executable: &str, arguments: &[&str]) -> (Self, CapturedProcessTree) {
        let arguments = arguments.iter().map(OsString::from).collect::<Vec<_>>();
        let child = spawn_detached(
            Path::new(executable),
            &arguments,
            DetachedStandardError::Null,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let guard = Self {
            child: Some(child),
            identity,
        };
        let mut tree = CapturedProcessTree::capture(identity).unwrap();
        tree.authorize_destructive_for_test();
        (guard, tree)
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGCONT);
        let _ = self.identity.signal(libc::SIGKILL);
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

#[test]
fn refresh_failure_returns_freezing_authority() {
    let (_guard, mut tree) = spawn_sleep();
    let root = tree.root();
    tree.fail_next_refresh(REFRESH_FAILURE_DETAIL);

    let error = force_error(tree.begin_force());

    assert_error_contains(error.error(), REFRESH_FAILURE_DETAIL);
    assert_eq!(error.into_state().root(), root);
}

#[test]
fn sigstop_failure_returns_freezing_authority() {
    let (_guard, tree) = spawn_sleep();
    let root = tree.root();
    let _failure =
        install_test_signal_failure(libc::SIGSTOP, FIRST_TARGET_INDEX, STOP_FAILURE_DETAIL);

    let error = force_error(tree.begin_force());

    assert_error_contains(error.error(), STOP_FAILURE_DETAIL);
    assert_eq!(error.into_state().root(), root);
}

#[test]
fn running_target_cannot_advance_freeze_confirmation() {
    let (_guard, tree) = spawn_sleep();
    let noop = install_test_signal_noop(libc::SIGSTOP, FIRST_TARGET_INDEX);

    let freezing = expect_pending(tree.begin_force());
    drop(noop);
    let freezing = expect_pending(freezing);
    let _frozen = expect_frozen(freezing);
}

#[test]
fn freeze_requires_two_matching_stopped_observations() {
    let (_guard, tree) = spawn_sleep();

    let freezing = expect_pending(tree.begin_force());
    let _frozen = expect_frozen(freezing);
}

#[test]
fn ownership_fault_cannot_mint_a_frozen_state() {
    let (_guard, mut tree) = spawn_sleep();
    tree.mark_ownership_fault();

    let freezing = expect_pending(tree.begin_force());
    let _still_pending = expect_pending(freezing);
}

#[test]
fn partial_sigkill_failure_returns_frozen_authority() {
    let (_guard, mut tree) =
        discovery_spawn::spawn(SHELL_EXECUTABLE, &[SHELL_COMMAND_ARGUMENT, TREE_COMMAND]);
    let identities = wait_for_tree(&mut tree);
    let descendant = identities[DESCENDANT_TARGET_INDEX];
    let root = tree.root();
    let frozen = freeze_with_retry(expect_pending(tree.begin_force()));
    let _failure =
        install_test_signal_failure(libc::SIGKILL, ROOT_TARGET_INDEX, KILL_FAILURE_DETAIL);

    let error = kill_error_with_retry(frozen);
    assert_error_contains(error.error(), KILL_FAILURE_DETAIL);
    let returned = error.into_state();
    assert_eq!(returned.root(), root);
    assert!(root.signal(libc::SIGCONT).unwrap());
    wait_until(DESCENDANT_EXIT_DETAIL, || !descendant.matches());
    assert!(root.matches());
}

fn spawn_sleep() -> (ProcessGuard, CapturedProcessTree) {
    ProcessGuard::spawn(SLEEP_EXECUTABLE, &[SLEEP_DURATION])
}

fn force_error(
    freezing: FreezingProcessTree,
) -> super::ProcessTransitionError<FreezingProcessTree> {
    match freezing.poll() {
        Err(error) => error,
        Ok(_) => panic!("{EXPECTED_ERROR_DETAIL}"),
    }
}

fn expect_pending(freezing: FreezingProcessTree) -> FreezingProcessTree {
    match freezing.poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    }
}

fn expect_frozen(freezing: FreezingProcessTree) -> FrozenProcessTree {
    #[cfg(target_os = "linux")]
    return freeze_with_retry(freezing);
    #[cfg(not(target_os = "linux"))]
    match freezing.poll().unwrap() {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    }
}

fn wait_for_tree(tree: &mut CapturedProcessTree) -> Vec<ProcessIdentity> {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let identities = tree.refresh().unwrap();
        if identities.len() == EXPECTED_TREE_TARGETS {
            return identities;
        }
        assert!(Instant::now() < deadline, "{TARGET_DISCOVERY_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_until(detail: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "{detail}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn assert_error_contains(error: &anyhow::Error, detail: &str) {
    assert!(
        error
            .chain()
            .any(|cause| cause.to_string().contains(detail))
    );
}
