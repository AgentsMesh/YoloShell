use super::test_support::Fixture;

const ONE_EVENT: usize = 1;

#[test]
fn native_registration_recovers_first_event_without_begin_or_synthetic_snapshot() {
    let fixture = Fixture::new();
    assert!(!fixture.artifacts.snapshot_path().exists());
    super::prime(&fixture.paths, &fixture.admission).unwrap();
    let pending = fixture.outbox().pending().unwrap();
    assert_eq!(pending.len(), ONE_EVENT);
    let mut expected = fixture.admission.record().first_event().clone();
    expected.sequence = crate::agent::hooks::EventSequence(crate::agent::hooks::FIRST_SEQUENCE);
    assert!(pending.first() == Some(&expected));
    assert!(!fixture.artifacts.snapshot_path().exists());
}

#[test]
fn repeated_native_first_event_materialization_is_idempotent() {
    let fixture = Fixture::new();
    super::prime(&fixture.paths, &fixture.admission).unwrap();
    let original = fixture.outbox().load().unwrap();
    super::prime(&fixture.paths, &fixture.admission).unwrap();
    assert!(fixture.outbox().load().unwrap() == original);
}

#[test]
fn native_first_event_materialization_cannot_follow_replaced_admission_file() {
    let fixture = Fixture::new();
    crate::paths::atomic_write_json(fixture.admission.path(), fixture.admission.record()).unwrap();
    assert!(super::prime(&fixture.paths, &fixture.admission).is_err());
    assert!(!fixture.artifacts.snapshot_path().exists());
}

#[test]
fn native_registration_settles_the_first_outbox_head_after_snapshot_recovery() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    assert_eq!(
        fixture.outbox().load().unwrap().acknowledged_sequence(),
        crate::agent::hooks::ZERO_SEQUENCE
    );
    runtime
        .complete_native_registration(&fixture.admission)
        .unwrap();
    let settled = fixture.outbox().load().unwrap();
    assert_eq!(
        settled.acknowledged_sequence(),
        crate::agent::hooks::FIRST_SEQUENCE
    );
    assert!(fixture.outbox().pending().unwrap().is_empty());
    runtime
        .complete_native_registration(&fixture.admission)
        .unwrap();
    assert!(fixture.outbox().load().unwrap() == settled);
}

#[test]
fn native_registration_cannot_ack_without_its_exact_snapshot() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    std::fs::remove_file(runtime.snapshot_path()).unwrap();
    assert!(
        runtime
            .complete_native_registration(&fixture.admission)
            .is_err()
    );
    assert_eq!(
        fixture.outbox().load().unwrap().acknowledged_sequence(),
        crate::agent::hooks::ZERO_SEQUENCE
    );
}
