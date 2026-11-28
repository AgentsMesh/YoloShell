use super::conflict_support::*;
use super::support::*;
use super::*;
use std::sync::Arc;

const CONFLICT_FIELDS: &[ConflictField] = &[
    ConflictField::Token,
    ConflictField::ProcessId,
    ConflictField::ProcessStart,
    ConflictField::Cwd,
    ConflictField::DataHome,
    ConflictField::Hook,
    ConflictField::Real,
];

#[test]
fn every_native_authority_field_conflict_is_rejected_by_the_durable_boundary() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let inserted = authority.install(&router).unwrap();

    for field in CONFLICT_FIELDS {
        let changed = fixture.changed_record(&authority, *field);
        assert!(!inserted.entry().matches_record(&changed));
        assert!(authority.storage.publish_for_test(&changed).is_err());
    }
    assert!(authority.future_version_record().is_err());
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);
}

#[test]
fn runtime_owner_mismatch_is_rejected_before_first_insertion() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let other = fixture.authority();
    let router = fixture.router();
    let error = router
        .install_or_replay(authority.durable(), |_| Ok(Arc::clone(&other.runtime)))
        .err()
        .unwrap();
    assert_eq!(error, CodexRouterError::Runtime);
    assert_eq!(router.len().unwrap(), usize::default());
}

#[test]
fn durable_admission_from_another_worker_home_is_rejected() {
    let fixture = Fixture::new();
    let foreign = Fixture::with_scope(fixture.terminal_id, fixture.runtime_epoch);
    let authority = foreign.authority();
    let router = fixture.router();
    let error = router
        .install_or_replay(authority.durable(), |_| Ok(Arc::clone(&authority.runtime)))
        .err()
        .unwrap();
    assert_eq!(error, CodexRouterError::Admission);
    assert_eq!(router.len().unwrap(), usize::default());
}

#[test]
fn full_router_rejects_before_invoking_the_runtime_builder() {
    let fixture = Fixture::new();
    let first = fixture.authority();
    let second = fixture.authority();
    let rejected = fixture.authority();
    let router = fixture.router_with_capacity(TEST_ROUTER_CAPACITY);
    let _first_lease = first.install(&router).unwrap();
    let _second_lease = second.install(&router).unwrap();
    let builder_calls = std::sync::atomic::AtomicUsize::new(usize::default());
    let result = router.install_or_replay(rejected.durable(), |_| {
        builder_calls.fetch_add(SINGLE_ENTRY_COUNT, std::sync::atomic::Ordering::SeqCst);
        Ok(Arc::clone(&rejected.runtime))
    });
    assert_eq!(result.err().unwrap(), CodexRouterError::Capacity);
    assert_eq!(
        builder_calls.load(std::sync::atomic::Ordering::SeqCst),
        usize::default()
    );
    assert_eq!(router.len().unwrap(), TEST_ROUTER_CAPACITY);
}
