use super::resilience_adapter_test_support::{AdapterTreeFixture, freeze, wait_gone, wait_stopped};
use super::{ResilienceFrozenProcessTree, ResilienceTransitionError};
use std::panic::{AssertUnwindSafe, catch_unwind};

const FENCE_ERROR_DETAIL: &str = "injected destructive commit-fence failure";
const FENCE_PANIC_DETAIL: &str = "injected destructive commit-fence panic";
const KILL_ERROR_DETAIL: &str = "adapter kill unexpectedly succeeded";
const CLEANUP_NOT_ARMED_DETAIL: &str = "destructive fence did not arm kill cleanup";

#[derive(Clone, Copy)]
enum TestKillBasis {
    Exact,
    Ordinary,
}

#[test]
fn exact_fence_error_drop_kills_without_resuming() {
    assert_fence_error_kills_on_drop(TestKillBasis::Exact);
}

#[test]
fn ordinary_fence_error_drop_kills_without_resuming() {
    assert_fence_error_kills_on_drop(TestKillBasis::Ordinary);
}

#[test]
fn exact_fence_panic_drop_kills_without_resuming() {
    assert_fence_panic_kills_on_drop(TestKillBasis::Exact);
}

#[test]
fn ordinary_fence_panic_drop_kills_without_resuming() {
    assert_fence_panic_kills_on_drop(TestKillBasis::Ordinary);
}

fn assert_fence_error_kills_on_drop(basis: TestKillBasis) {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let frozen = frozen_with_basis(tree.begin_force(), basis);
    wait_stopped(fixture.identities());

    let error =
        match frozen.kill_with_destructive_commit_fence(|| anyhow::bail!(FENCE_ERROR_DETAIL)) {
            Err(error) => error,
            Ok(_) => panic!("{KILL_ERROR_DETAIL}"),
        };

    assert_error_contains(&error, FENCE_ERROR_DETAIL);
    let state = error.into_state();
    assert!(state.kill_cleanup_is_armed(), "{CLEANUP_NOT_ARMED_DETAIL}");
    wait_stopped(fixture.identities());
    drop(state);
    fixture.reap_root();
    wait_gone(fixture.identities());
}

fn assert_fence_panic_kills_on_drop(basis: TestKillBasis) {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let frozen = frozen_with_basis(tree.begin_force(), basis);
    wait_stopped(fixture.identities());

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = frozen.kill_with_destructive_commit_fence(|| -> anyhow::Result<()> {
            panic!("{FENCE_PANIC_DETAIL}")
        });
    }));

    assert!(panic.is_err(), "{FENCE_PANIC_DETAIL}");
    fixture.reap_root();
    wait_gone(fixture.identities());
}

fn frozen_with_basis(
    freezing: super::ResilienceFreezingProcessTree,
    basis: TestKillBasis,
) -> ResilienceFrozenProcessTree {
    let mut frozen = freeze(freezing);
    if matches!(basis, TestKillBasis::Ordinary) {
        frozen.downgrade_kill_basis_to_ordinary_for_test();
    }
    frozen
}

fn assert_error_contains(
    error: &ResilienceTransitionError<ResilienceFrozenProcessTree>,
    detail: &str,
) {
    assert!(
        error
            .error()
            .chain()
            .any(|cause| cause.to_string().contains(detail))
    );
}
