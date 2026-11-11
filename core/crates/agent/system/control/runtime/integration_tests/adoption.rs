use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_control_runtime::*;

#[test]
fn custody_adoption_and_resize_keep_the_instance_and_pending_reservation() {
    let fixture = Fixture::new();
    let mut owner = fixture.initialize();
    let request = Uuid::new_v4();
    let pending = reserve(&mut owner, &fixture.runtime, request);
    let mut adopted = fixture.runtime.clone();
    adopted.controller_generation += STEP;
    adopted.cols += SIZE_STEP;
    adopted.rows += SIZE_STEP;
    owner.verify(&adopted).unwrap();
    assert_eq!(
        owner.lookup(&adopted, request, digest()).unwrap(),
        Some(Receipt::Unknown)
    );
    assert_eq!(
        owner
            .complete(&adopted, pending, Completion::Delivered)
            .unwrap(),
        Receipt::Delivered
    );
    drop(owner);
    let reopened = fixture.reopen(&adopted).unwrap();
    assert_eq!(
        reopened.lookup(&adopted, request, digest()).unwrap(),
        Some(Receipt::Delivered)
    );
}

#[test]
fn a_reopened_owner_cannot_complete_a_reservation_from_the_previous_instance() {
    let fixture = Fixture::new();
    let mut owner = fixture.initialize();
    let request = Uuid::new_v4();
    let pending = reserve(&mut owner, &fixture.runtime, request);
    drop(owner);
    let mut reopened = fixture.reopen(&fixture.runtime).unwrap();
    assert_eq!(
        reopened.complete(&fixture.runtime, pending, Completion::Delivered),
        Err(JournalError::ForeignReservation)
    );
    assert!(matches!(
        reopened
            .reserve(&fixture.runtime, request, digest())
            .unwrap(),
        ReservationResult::Existing(Receipt::Unknown)
    ));
}

#[test]
fn worker_identity_and_generation_regression_are_rejected_without_poisoning() {
    let fixture = Fixture::new();
    let owner = fixture.initialize();
    let mut changed = fixture.runtime.clone();
    changed.worker_id = Uuid::new_v4();
    assert_eq!(owner.verify(&changed), Err(JournalError::ScopeMismatch));
    let mut regressed = fixture.runtime.clone();
    regressed.controller_generation -= STEP;
    assert_eq!(owner.verify(&regressed), Err(JournalError::ScopeMismatch));
    owner.verify(&fixture.runtime).unwrap();
    drop(owner);
    assert!(matches!(
        fixture.reopen(&changed),
        Err(JournalError::ScopeMismatch)
    ));
    fixture
        .reopen(&fixture.runtime)
        .unwrap()
        .verify(&fixture.runtime)
        .unwrap();
}
