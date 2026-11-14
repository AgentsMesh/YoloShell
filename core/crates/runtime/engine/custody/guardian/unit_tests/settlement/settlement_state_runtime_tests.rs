use super::*;
use crate::custody_guardian::settlement_armed::test_support::Fixture;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::model::BootstrapDelivery;
use crate::process::ProcessIdentityRelation;
use crate::pty_custody::{CustodyDurableRecord, CustodyGuardianPreparation};

#[test]
fn pending_retry_retains_identity_without_blocking_successor_publication() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let (mut state, preparation, prepared) = configured_state(&fixture);

    state
        .install_pending(
            preparation,
            prepared,
            fixture.snapshot.clone(),
            fixture.runtime.clone(),
            fixture.runtime_identity,
            None,
        )
        .unwrap();

    assert_eq!(
        state.retained_runtime().unwrap(),
        (&fixture.runtime, fixture.runtime_identity)
    );
    RetainedRuntimeFile::capture(
        &fixture.namespace,
        &fixture.paths,
        &fixture.runtime,
        fixture.runtime_identity,
    )
    .unwrap();
}

#[test]
fn pending_retry_rejects_a_same_value_replacement_object() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let (mut state, preparation, prepared) = configured_state(&fixture);
    let replacement = publish_runtime(&fixture, &fixture.runtime);
    assert!(!replacement.same_object(fixture.runtime_identity));

    assert!(
        state
            .install_pending(
                preparation,
                prepared,
                fixture.snapshot.clone(),
                fixture.runtime.clone(),
                replacement,
                None,
            )
            .is_err()
    );
}

#[test]
fn pending_update_requires_and_adopts_the_new_retained_object() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let (mut state, preparation, prepared) = configured_state(&fixture);
    let mut updated = fixture.runtime.clone();
    updated.bootstrap_delivery = BootstrapDelivery::Delivered;
    let replacement = publish_runtime(&fixture, &updated);
    assert!(
        state
            .install_pending(
                preparation,
                prepared.clone(),
                fixture.snapshot.clone(),
                updated.clone(),
                replacement,
                None,
            )
            .is_err()
    );
    let retained =
        RetainedRuntimeFile::capture(&fixture.namespace, &fixture.paths, &updated, replacement)
            .unwrap();

    state
        .install_pending(
            preparation,
            prepared,
            fixture.snapshot.clone(),
            updated.clone(),
            replacement,
            Some(retained),
        )
        .unwrap();
    assert_eq!(state.retained_runtime().unwrap(), (&updated, replacement));
}

fn configured_state(
    fixture: &Fixture,
) -> (
    GuardianState,
    CustodyGuardianPreparation,
    CustodyDurableRecord,
) {
    let preparation = fixture.install().unwrap();
    let (prepared, _) = fixture.current();
    let state = GuardianState::new(
        preparation,
        prepared.clone(),
        fixture.snapshot.clone(),
        fixture.runtime.clone(),
        fixture.runtime_identity,
        fixture.retained_runtime_file(),
    )
    .unwrap();
    (state, preparation, prepared)
}

fn publish_runtime(fixture: &Fixture, record: &RuntimeRecord) -> FileIdentity {
    let guard = SessionDurableMutationGuard::acquire_participant(
        &fixture.paths,
        &fixture.namespace,
        record.terminal_id,
    )
    .unwrap();
    guard
        .write_json_expected_with_fence_identity(
            &fixture.paths.runtime_record(record.terminal_id),
            Some(fixture.runtime_identity),
            record,
            |_| Ok(()),
        )
        .unwrap()
}
