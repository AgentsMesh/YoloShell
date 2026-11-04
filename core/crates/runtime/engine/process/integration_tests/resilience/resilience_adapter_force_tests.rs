use super::captured_targets::install_test_signal_failure;
use super::captured_targets::test_hook::install_test_signal_hook;
use super::pty_session_proof::fail_next_reopen_seal_open_for_test;
use super::resilience_adapter_test_support::{
    AdapterTreeFixture, freeze, wait_gone, wait_running, wait_stopped,
};
use super::retained_exact_double_fork_support::DoubleForkFixture;
use super::{ResilienceFreezePoll, ResilienceProcessTree};
use std::cell::RefCell;
use std::rc::Rc;

const STOP_FAILURE_DETAIL: &str = "injected adapter partial SIGSTOP failure";
const KILL_FAILURE_DETAIL: &str = "injected adapter partial SIGKILL failure";
const FREEZE_ERROR_DETAIL: &str = "adapter freeze unexpectedly succeeded";
const KILL_ERROR_DETAIL: &str = "adapter kill unexpectedly succeeded";
const PENDING_OBSERVATION_DETAIL: &str = "first frozen observation did not stay pending";
const REFRESH_PANIC_DETAIL: &str = "injected adapter refresh panic";
const EXPANDED_ROOT_TARGET_INDEX: usize = 1;

#[test]
fn partial_sigstop_failure_drop_resumes_every_survivor() {
    let (fixture, tree) = AdapterTreeFixture::spawn();
    let failure_target = fixture.failure_target();
    let stopped_targets = fixture
        .identities()
        .iter()
        .copied()
        .filter(|identity| *identity != failure_target)
        .collect::<Vec<_>>();
    let failure = install_test_signal_failure(
        libc::SIGSTOP,
        fixture.failure_target_index(),
        STOP_FAILURE_DETAIL,
    );

    let error = freeze_error(tree);

    assert_error_contains(error.error(), STOP_FAILURE_DETAIL);
    wait_stopped(&stopped_targets);
    wait_running(&[failure_target]);
    drop(failure);
    drop(error.into_state());
    wait_running(fixture.identities());
}

#[test]
fn cancellation_at_pending_frozen_observation_resumes_every_target() {
    let (fixture, tree) = AdapterTreeFixture::spawn();
    for identity in fixture.identities() {
        assert!(identity.signal(libc::SIGSTOP).unwrap());
    }
    wait_stopped(fixture.identities());

    let pending = match tree.begin_force().poll().unwrap() {
        ResilienceFreezePoll::Pending(pending) => pending,
        ResilienceFreezePoll::Frozen(_) => panic!("{PENDING_OBSERVATION_DETAIL}"),
    };

    drop(pending);
    wait_running(fixture.identities());
}

#[test]
fn panic_after_refresh_stops_a_new_target_resumes_the_expanded_closure() {
    let fixture = Rc::new(RefCell::new(
        DoubleForkFixture::spawn_with_retained_bootstrap_master(false),
    ));
    let root = fixture.borrow().root_identity;
    let proof = fixture.borrow().proof.clone();
    let tree = ResilienceProcessTree::capture_pty(root, proof).unwrap();
    let leaf = Rc::new(RefCell::new(None));
    let leaf_from_hook = Rc::clone(&leaf);
    let fixture_from_hook = Rc::clone(&fixture);
    let root_index = tree
        .identities()
        .iter()
        .position(|target| *target == root)
        .unwrap();
    let first_hook = install_test_signal_hook(libc::SIGSTOP, root_index, move || {
        *leaf_from_hook.borrow_mut() = Some(fixture_from_hook.borrow_mut().release_leaf());
    });
    let freezing = match tree.begin_force().poll().unwrap() {
        ResilienceFreezePoll::Pending(freezing) => freezing,
        ResilienceFreezePoll::Frozen(_) => panic!("{PENDING_OBSERVATION_DETAIL}"),
    };
    drop(first_hook);
    let leaf = leaf.borrow().expect(PENDING_OBSERVATION_DETAIL);
    let panic_hook = install_test_signal_hook(libc::SIGSTOP, EXPANDED_ROOT_TARGET_INDEX, || {
        panic!("{REFRESH_PANIC_DETAIL}")
    });

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = freezing.poll();
    }));

    assert!(panic.is_err(), "{REFRESH_PANIC_DETAIL}");
    drop(panic_hook);
    wait_running(&[root, leaf]);
}

#[test]
fn partial_sigkill_failure_drop_kills_every_retained_target() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let failure = install_test_signal_failure(
        libc::SIGKILL,
        fixture.failure_target_index(),
        KILL_FAILURE_DETAIL,
    );
    let frozen = freeze(tree.begin_force());

    let error = match frozen.kill() {
        Err(error) => error,
        Ok(_) => panic!("{KILL_ERROR_DETAIL}"),
    };

    assert_error_contains(error.error(), KILL_FAILURE_DETAIL);
    wait_stopped(&[fixture.failure_target()]);
    drop(failure);
    drop(error.into_state());
    fixture.reap_root();
    wait_gone(fixture.identities());
}

#[test]
fn reopen_seal_acquisition_failure_drop_resumes_every_target() {
    let fixture = DoubleForkFixture::spawn_with_retained_bootstrap_master(false);
    let root = fixture.root_identity;
    let tree = ResilienceProcessTree::capture_pty(root, fixture.proof.clone()).unwrap();
    let frozen = freeze(tree.begin_force());
    fail_next_reopen_seal_open_for_test();

    let error = match frozen.kill() {
        Err(error) => error,
        Ok(_) => panic!("{KILL_ERROR_DETAIL}"),
    };

    drop(error.into_state());
    wait_running(&[root]);
}

#[test]
fn armed_frozen_drop_kills_every_retained_target_without_kill_transition() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let mut frozen = freeze(tree.begin_force());

    frozen.arm_kill_cleanup();
    drop(frozen);

    fixture.reap_root();
    wait_gone(fixture.identities());
}

fn freeze_error(
    tree: ResilienceProcessTree,
) -> super::ResilienceTransitionError<super::ResilienceFreezingProcessTree> {
    match tree.begin_force().poll() {
        Err(error) => error,
        Ok(_) => panic!("{FREEZE_ERROR_DETAIL}"),
    }
}

fn assert_error_contains(error: &anyhow::Error, detail: &str) {
    assert!(
        error
            .chain()
            .any(|cause| cause.to_string().contains(detail))
    );
}
