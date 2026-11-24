use super::inventory_test_support::TestProcess;
use super::{
    CapturedProcessTree, DestructiveSessionUnavailable, FreezePoll, SurvivorPoll, SurvivorReady,
};
use crate::process::PtySessionProof;

const FIRST_EMPTY_COMPLETED_DETAIL: &str = "first empty survivor observation completed";
const LIVE_SURVIVOR_COMPLETED_DETAIL: &str = "live survivor observation completed";
const SECOND_EMPTY_PENDING_DETAIL: &str = "second empty survivor observation stayed pending";
const FREEZE_STARTED_CONVERSION_DETAIL: &str =
    "destructive freeze converted to termination-unsupported custody";
const TEST_CAPABILITY: &str = "unsupported-observer-scope";
const TEST_TTY_PATH: &str = "/dev/null";
const INJECTED_REFRESH_FAILURE: &str = "unsupported observer unexpectedly refreshed candidates";
const EXACT_OBSERVER_REPORTED_GONE_DETAIL: &str =
    "exact-only PTY observer forged tracked-producers-gone evidence";

#[test]
fn survivor_poll_requires_consecutive_empty_observations() {
    let mut process = TestProcess::spawn();
    let tree = CapturedProcessTree::capture(process.identity).unwrap();
    let root_exit = tree.root_exit_proof_for_test(process.identity);
    let proof = DestructiveSessionUnavailable::new();
    let mut survivor = tree.into_termination_unsupported(&proof);
    assert_eq!(survivor.root(), process.identity);
    survivor.mark_ownership_fault();

    survivor = pending(survivor, false, LIVE_SURVIVOR_COMPLETED_DETAIL);
    process.terminate();
    survivor.mark_root_exited_unreaped(root_exit).unwrap();
    survivor = pending(survivor, true, FIRST_EMPTY_COMPLETED_DETAIL);
    let ready = survivor.prepare_poll().unwrap();
    let gone = match survivor.commit_poll(ready) {
        SurvivorPoll::Gone(gone) => gone,
        SurvivorPoll::Pending(_) => panic!("{SECOND_EMPTY_PENDING_DETAIL}"),
    };
    assert_eq!(gone.root(), process.identity);
}

#[test]
fn freeze_can_only_convert_before_destructive_signaling_starts() {
    let proof = DestructiveSessionUnavailable::new();
    let process = TestProcess::spawn();
    let tree = CapturedProcessTree::capture(process.identity).unwrap();
    let survivor = tree
        .begin_force()
        .into_termination_unsupported(&proof)
        .unwrap();
    assert_eq!(survivor.root(), process.identity);
    drop(process);

    let process = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(process.identity).unwrap();
    tree.authorize_destructive_for_test();
    let freezing = match tree.begin_force().poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{FREEZE_STARTED_CONVERSION_DETAIL}"),
    };
    assert!(freezing.into_termination_unsupported(&proof).is_err());
}

#[test]
#[cfg(target_os = "macos")]
fn pty_scope_unsupported_retires_proof_and_observes_exact_identities_only() {
    let mut process = TestProcess::spawn();
    let proof = PtySessionProof::capture(TEST_TTY_PATH.into(), TEST_CAPABILITY.to_owned()).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
    let root_exit = tree.root_exit_proof_for_test(process.identity);
    tree.fail_next_refresh(INJECTED_REFRESH_FAILURE);
    let unavailable = DestructiveSessionUnavailable::new();
    let mut survivor = tree.into_termination_unsupported(&unavailable);

    assert!(survivor.is_exact_observer_only_for_test());
    assert!(!survivor.retains_pty_proof_for_test());
    assert!(survivor.prepare_poll().is_ok());
    process.terminate();
    survivor.mark_root_exited_unreaped(root_exit).unwrap();
    survivor = pending(survivor, true, EXACT_OBSERVER_REPORTED_GONE_DETAIL);
    survivor = pending(survivor, true, EXACT_OBSERVER_REPORTED_GONE_DETAIL);
    assert!(matches!(
        survivor.commit_poll(SurvivorReady::Gone),
        SurvivorPoll::Pending(_)
    ));
}

#[test]
#[cfg(not(target_os = "macos"))]
fn non_macos_pty_scope_keeps_its_platform_custody_proof() {
    let process = TestProcess::spawn();
    let proof = PtySessionProof::capture(TEST_TTY_PATH.into(), TEST_CAPABILITY.to_owned()).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(process.identity, proof).unwrap();
    tree.fail_next_refresh(INJECTED_REFRESH_FAILURE);
    let unavailable = DestructiveSessionUnavailable::new();
    let mut survivor = tree.into_termination_unsupported(&unavailable);

    assert!(!survivor.is_exact_observer_only_for_test());
    assert!(survivor.retains_pty_proof_for_test());
    assert!(survivor.prepare_poll().is_err());
}

#[test]
fn process_discovery_unsupported_keeps_its_proof_and_refresh_behavior() {
    let process = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        process.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    tree.fail_next_refresh(INJECTED_REFRESH_FAILURE);
    let unavailable = DestructiveSessionUnavailable::new();
    let mut survivor = tree.into_termination_unsupported(&unavailable);

    assert!(!survivor.is_exact_observer_only_for_test());
    assert!(survivor.retains_pty_proof_for_test());
    assert!(survivor.prepare_poll().is_err());
}

fn pending(
    mut survivor: super::TerminationUnsupportedProcessTree,
    expected_empty: bool,
    detail: &str,
) -> super::TerminationUnsupportedProcessTree {
    let ready = survivor.prepare_poll().unwrap();
    match &ready {
        SurvivorReady::Pending(empty) => assert_eq!(*empty, expected_empty, "{detail}"),
        SurvivorReady::Gone => panic!("{detail}"),
    }
    match survivor.commit_poll(ready) {
        SurvivorPoll::Pending(survivor) => survivor,
        SurvivorPoll::Gone(_) => panic!("{detail}"),
    }
}
