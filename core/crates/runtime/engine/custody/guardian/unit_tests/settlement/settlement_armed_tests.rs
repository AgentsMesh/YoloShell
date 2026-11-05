use super::{GuardianState, Resolution, resolve, restoration};
use crate::process::ProcessIdentityRelation;
use std::cell::Cell;

use super::test_support::Fixture;

#[test]
fn current_host_does_not_contend_for_session_lifecycle_lock() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let preparation = fixture.install().unwrap();
    let (prepared, _) = fixture.current();
    let mut state = GuardianState::new(
        preparation,
        prepared.clone(),
        fixture.snapshot.clone(),
        fixture.runtime.clone(),
        fixture.runtime_identity,
        fixture.retained_runtime_file(),
    )
    .unwrap();
    let (armed, armed_identity) = crate::custody_guardian::preparation::arm(
        &fixture.namespace,
        &fixture.paths,
        &prepared,
        preparation,
    )
    .unwrap();
    let _lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.runtime.terminal_id)
        .unwrap()
        .unwrap();

    assert!(matches!(
        resolve(
            &fixture.namespace,
            &fixture.paths,
            &mut state,
            &armed,
            armed_identity,
        ),
        Ok(Resolution::AwaitingActivation)
    ));
    assert_eq!(fixture.current(), (armed, armed_identity));
}

#[test]
fn post_rename_parent_sync_failure_reconciles_restored_prepared_identity() {
    let fixture = Fixture::new(ProcessIdentityRelation::Gone);
    let preparation = fixture.install().unwrap();
    let (prepared, prepared_identity) = fixture.current();
    assert_eq!(prepared_identity, preparation.custody_file_identity());
    let (armed, armed_identity) = crate::custody_guardian::preparation::arm(
        &fixture.namespace,
        &fixture.paths,
        &prepared,
        preparation,
    )
    .unwrap();
    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &fixture.paths,
        &fixture.namespace,
        fixture.runtime.terminal_id,
    )
    .unwrap();
    let callback_ran = Cell::new(false);

    let restored_identity = restoration::restore_with_commit_observer(
        &guard,
        &fixture.paths,
        &armed,
        armed_identity,
        &prepared,
        &fixture.runtime,
        fixture.runtime_identity,
        || {
            assert!(!callback_ran.replace(true));
            crate::paths::test_support::fail_next_directory_sync();
        },
    )
    .unwrap();

    assert!(callback_ran.get());
    assert_ne!(restored_identity, prepared_identity);
    assert_eq!(fixture.current(), (prepared, restored_identity));
    assert_eq!(
        fixture
            .namespace
            .read_json_with_identity(&fixture.paths.runtime_record(fixture.runtime.terminal_id))
            .unwrap(),
        (fixture.runtime, fixture.runtime_identity)
    );
}
