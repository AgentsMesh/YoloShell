use super::*;
use std::sync::Arc;
use uuid::Uuid;

#[path = "router_build_gate_adversarial_tests.rs"]
mod build_gate_adversarial_tests;
#[path = "router_build_gate_isolation_tests.rs"]
mod build_gate_isolation_tests;
#[path = "../../../test_support/unit/codex_hook/router/router_concurrency_support.rs"]
mod concurrency_support;
#[path = "router_concurrency_tests.rs"]
mod concurrency_tests;
#[path = "../../../test_support/unit/codex_hook/router/router_conflict_support.rs"]
mod conflict_support;
#[path = "router_conflict_tests.rs"]
mod conflict_tests;
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/integration/codex_hook/router_process_support.rs"]
mod process_support;
#[path = "router_publication_tests.rs"]
mod publication_tests;
#[path = "../../../test_support/unit/codex_hook/router/router_test_support.rs"]
pub(in crate::worker::codex_hook) mod support;
use support::*;

#[test]
fn insertion_strongly_owns_runtime_and_resolves_only_the_exact_identity() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let runtime_count = Arc::strong_count(&authority.runtime);
    let inserted = authority.install(&router).unwrap();
    assert!(!inserted.is_replay());
    assert_eq!(
        Arc::strong_count(&authority.runtime),
        runtime_count + SINGLE_ENTRY_COUNT
    );
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);

    let entry = inserted.entry();
    assert_eq!(entry.key().runtime_epoch(), fixture.runtime_epoch);
    assert_eq!(
        entry.key().invocation_id(),
        authority.identity.invocation_id
    );
    assert!(Arc::ptr_eq(entry.runtime(), &authority.runtime));
    assert!(entry.matches_record(&authority.record));
    let resolved = router.resolve(&authority.identity).unwrap();
    assert!(Arc::ptr_eq(entry, &resolved));

    let mut wrong_token = authority.identity.clone();
    wrong_token.hook_token = Uuid::new_v4();
    assert_eq!(
        router.resolve(&wrong_token).err().unwrap(),
        CodexRouterError::Conflict
    );
    let mut old_epoch = authority.identity.clone();
    old_epoch.runtime_epoch = Uuid::new_v4();
    assert_eq!(
        router.resolve(&old_epoch).err().unwrap(),
        CodexRouterError::Scope
    );
}

#[test]
fn exact_begin_replay_returns_the_original_entry_without_replacing_runtime() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let first = authority.install(&router).unwrap();
    let replay = router
        .install_or_replay(authority.durable(), |_| unreachable!())
        .unwrap();
    assert!(replay.is_replay());
    assert!(Arc::ptr_eq(first.entry(), replay.entry()));
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);
}

#[test]
fn distinct_invocations_share_the_session_router_without_aliasing() {
    let fixture = Fixture::new();
    let first = fixture.authority();
    let second = fixture.authority();
    let router = fixture.router();
    let _first_lease = first.install(&router).unwrap();
    let _second_lease = second.install(&router).unwrap();
    assert_eq!(router.len().unwrap(), DISTINCT_ENTRY_COUNT);
    assert!(Arc::ptr_eq(
        router.resolve(&first.identity).unwrap().runtime(),
        &first.runtime,
    ));
    assert!(Arc::ptr_eq(
        router.resolve(&second.identity).unwrap().runtime(),
        &second.runtime,
    ));
}

#[test]
fn retirement_requires_explicit_commit_and_a_dropped_permit_is_retryable() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let rejected = fixture.authority();
    let router = fixture.router_with_capacity(SINGLE_ENTRY_COUNT);
    let inserted = authority.install(&router).unwrap();
    let entry = Arc::clone(inserted.entry());
    inserted.settle().unwrap();
    let permit = ready_retirement(router.begin_retirement(&entry));
    assert!(Arc::ptr_eq(permit.entry(), &entry));
    assert!(matches!(
        router.begin_retirement(&entry).unwrap(),
        CodexRetirementReadiness::Pending
    ));
    assert_eq!(
        router
            .install_or_replay(authority.durable(), |_| unreachable!())
            .err()
            .unwrap(),
        CodexRouterError::Conflict
    );
    assert_eq!(
        router.lease_committed(&authority.identity).err().unwrap(),
        CodexRouterError::Conflict
    );
    assert_eq!(
        rejected.install(&router).err().unwrap(),
        CodexRouterError::Capacity
    );
    drop(permit);
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);
    ready_retirement(router.begin_retirement(&entry))
        .commit()
        .unwrap();
    assert_eq!(router.len().unwrap(), usize::default());
}

#[test]
fn stale_permit_identity_cannot_retire_a_replacement_with_the_same_key() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let inserted = authority.install(&router).unwrap();
    let stale = Arc::clone(inserted.entry());
    inserted.settle().unwrap();
    ready_retirement(router.begin_retirement(&stale))
        .commit()
        .unwrap();
    let replacement = authority.install(&router).unwrap();
    let current = Arc::clone(replacement.entry());
    replacement.settle().unwrap();

    assert!(matches!(
        router.begin_retirement(&stale).unwrap(),
        CodexRetirementReadiness::Missing
    ));
    let event = router.lease_committed(&authority.identity).unwrap();
    assert!(Arc::ptr_eq(event.entry(), &current));
}

#[test]
fn cold_recovery_reconstructs_the_durable_admission_capability() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    drop(authority.durable());
    let recovered = DurableCodexAdmission::recover(authority.storage.clone())
        .unwrap()
        .unwrap();
    let router = fixture.router();
    let lease = router
        .install_or_replay(recovered, |_| Ok(Arc::clone(&authority.runtime)))
        .unwrap();
    assert!(!lease.is_replay());
    assert!(Arc::ptr_eq(lease.entry().runtime(), &authority.runtime));
}

#[test]
fn nil_router_scope_is_rejected() {
    let fixture = Fixture::new();
    assert!(
        CodexSessionRouter::new(fixture.paths.clone(), Uuid::nil(), fixture.runtime_epoch).is_err()
    );
    assert!(CodexSessionRouter::new(fixture.paths, fixture.terminal_id, Uuid::nil()).is_err());
}
