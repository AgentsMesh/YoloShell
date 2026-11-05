use super::admission::runtime_activated;
use super::test_support::Fixture;
use crate::model::BootstrapDelivery;
use crate::process::ProcessIdentityRelation;

#[path = "../../test_support/unit/settlement_armed_activation_test_support.rs"]
pub(super) mod support;
use support::{
    publish_runtime, rewrite_runtime_in_place, successor_generation, write_activated_checkpoint,
};

const GENERATION_ADVANCE_COUNT: u64 = 2;

#[test]
fn bootstrap_finalization_completes_initial_guardian_activation() {
    for delivery in [
        BootstrapDelivery::NotRequired,
        BootstrapDelivery::Delivered,
        BootstrapDelivery::Survivor,
    ] {
        let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
        let mut current = fixture.runtime.clone();
        current.bootstrap_delivery = delivery;
        let current_identity = publish_runtime(&fixture, &current);
        write_activated_checkpoint(&fixture, &current);

        assert!(
            runtime_activated(
                &fixture.paths,
                &fixture.runtime,
                fixture.runtime_identity,
                &current,
                current_identity,
                current.controller_generation,
            )
            .unwrap(),
            "{delivery:?}"
        );
    }
}

#[test]
fn exact_successor_generation_and_bootstrap_update_are_accepted() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let mut current = fixture.runtime.clone();
    current.bootstrap_delivery = BootstrapDelivery::Delivered;
    current.controller_generation = successor_generation(&fixture.runtime);
    let current_identity = publish_runtime(&fixture, &current);
    write_activated_checkpoint(&fixture, &current);

    assert!(
        runtime_activated(
            &fixture.paths,
            &fixture.runtime,
            fixture.runtime_identity,
            &current,
            current_identity,
            current.controller_generation,
        )
        .unwrap()
    );
}

#[test]
fn generation_ahead_of_the_exact_candidate_is_rejected() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let mut current = fixture.runtime.clone();
    current.bootstrap_delivery = BootstrapDelivery::Delivered;
    current.controller_generation = fixture
        .runtime
        .controller_generation
        .checked_add(GENERATION_ADVANCE_COUNT)
        .unwrap();
    let current_identity = publish_runtime(&fixture, &current);

    assert!(
        runtime_activated(
            &fixture.paths,
            &fixture.runtime,
            fixture.runtime_identity,
            &current,
            current_identity,
            successor_generation(&fixture.runtime),
        )
        .is_err()
    );
}

#[test]
fn process_identity_drift_is_rejected() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let mut current = fixture.runtime.clone();
    current.worker_id = uuid::Uuid::new_v4();
    current.bootstrap_delivery = BootstrapDelivery::Delivered;
    let current_identity = publish_runtime(&fixture, &current);

    assert!(
        runtime_activated(
            &fixture.paths,
            &fixture.runtime,
            fixture.runtime_identity,
            &current,
            current_identity,
            current.controller_generation,
        )
        .is_err()
    );
}

#[test]
fn terminal_bootstrap_state_cannot_change_again() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let mut retained = fixture.runtime.clone();
    retained.bootstrap_delivery = BootstrapDelivery::Delivered;
    let retained_identity = publish_runtime(&fixture, &retained);
    let mut current = retained.clone();
    current.bootstrap_delivery = BootstrapDelivery::Survivor;
    let current_identity = publish_runtime(&fixture, &current);

    assert!(
        runtime_activated(
            &fixture.paths,
            &retained,
            retained_identity,
            &current,
            current_identity,
            current.controller_generation,
        )
        .is_err()
    );
}

#[test]
fn same_value_runtime_inode_aba_is_rejected() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let replacement_identity = publish_runtime(&fixture, &fixture.runtime);
    assert_ne!(replacement_identity, fixture.runtime_identity);

    assert!(
        runtime_activated(
            &fixture.paths,
            &fixture.runtime,
            fixture.runtime_identity,
            &fixture.runtime,
            replacement_identity,
            fixture.runtime.controller_generation,
        )
        .is_err()
    );
}

#[test]
fn successor_runtime_rewritten_in_place_is_rejected() {
    let fixture = Fixture::new_with_distinct_child(ProcessIdentityRelation::Gone);
    let mut current = fixture.runtime.clone();
    current.bootstrap_delivery = BootstrapDelivery::Delivered;
    current.controller_generation = successor_generation(&fixture.runtime);
    let current_identity = rewrite_runtime_in_place(&fixture, &current);

    assert!(
        runtime_activated(
            &fixture.paths,
            &fixture.runtime,
            fixture.runtime_identity,
            &current,
            current_identity,
            current.controller_generation,
        )
        .is_err()
    );
}
