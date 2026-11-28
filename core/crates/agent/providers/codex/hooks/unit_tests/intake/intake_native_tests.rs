use super::intake_test_support::{CONCURRENT_WRITERS, IntakeFixture};
use crate::codex_hooks::CodexHookIntake;
use std::sync::{Arc, Barrier};

const ONE_OCCURRENCE: usize = 1;

#[test]
fn native_retries_retain_the_exact_first_occurrence_identity() {
    let fixture = IntakeFixture::new();
    let intake = fixture.intake();
    let mut event = fixture.envelope();
    event.event_id = uuid::Uuid::new_v4();
    assert!(intake.persist_explicit_occurrence(event.clone()).unwrap() == event);
    let original = intake
        .find_exact_occurrence(&event)
        .unwrap()
        .unwrap()
        .durable_identity()
        .unwrap();
    assert!(intake.persist_explicit_occurrence(event.clone()).unwrap() == event);
    let repeated = intake
        .find_exact_occurrence(&event)
        .unwrap()
        .unwrap()
        .durable_identity()
        .unwrap();
    assert!(original == repeated);
    assert_eq!(intake.scan().unwrap().occurrences.len(), ONE_OCCURRENCE);
}

#[test]
fn concurrent_native_retries_commit_one_occurrence() {
    let fixture = IntakeFixture::new();
    let mut event = fixture.envelope();
    event.event_id = uuid::Uuid::new_v4();
    let barrier = Arc::new(Barrier::new(CONCURRENT_WRITERS));
    std::thread::scope(|scope| {
        for _ in 0..CONCURRENT_WRITERS {
            let event = event.clone();
            let path = fixture.artifacts.intake_directory();
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                let intake = CodexHookIntake::for_identity(path, &event.identity).unwrap();
                barrier.wait();
                intake.persist_explicit_occurrence(event).unwrap();
            });
        }
    });
    assert_eq!(
        fixture.intake().scan().unwrap().occurrences.len(),
        ONE_OCCURRENCE
    );
}

#[test]
fn native_event_id_reuse_with_different_payload_is_rejected() {
    let fixture = IntakeFixture::new();
    let mut event = fixture.envelope();
    event.event_id = uuid::Uuid::new_v4();
    fixture
        .intake()
        .persist_explicit_occurrence(event.clone())
        .unwrap();
    event.turn_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(fixture.intake().persist_explicit_occurrence(event).is_err());
}

#[test]
fn native_occurrence_requires_real_event_identity_and_matching_invocation() {
    let fixture = IntakeFixture::new();
    assert!(
        fixture
            .intake()
            .persist_explicit_occurrence(fixture.envelope())
            .is_err()
    );
    let mut event = fixture.envelope();
    event.event_id = uuid::Uuid::new_v4();
    event.identity.invocation_id = uuid::Uuid::new_v4();
    assert!(fixture.intake().persist_explicit_occurrence(event).is_err());
}
