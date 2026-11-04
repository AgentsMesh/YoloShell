use super::intake_test_support::*;
use crate::codex_hooks::intake_record::CodexIntakeRecord;
use uuid::Uuid;

const FIRST_TIME: u64 = 30;
const SECOND_TIME: u64 = 10;
const THIRD_TIME: u64 = 20;
const OCCURRENCE_BASE: u128 = 100;

#[test]
fn corrupt_occurrence_is_quarantined_without_blocking_valid_tail() {
    let fixture = IntakeFixture::new();
    let intake = fixture.intake();
    let corrupt = record(&fixture, FIRST_TIME, OCCURRENCE_BASE);
    let corrupt_name = corrupt.pending_file_name();
    let mut value = serde_json::to_value(corrupt).unwrap();
    value[crate::codex_hooks::constants::test_values::UNKNOWN_INTAKE_FIELD] =
        serde_json::json!(true);
    super::super::intake_storage::create(intake.directory(), &intake.path(&corrupt_name), &value)
        .unwrap();
    intake.persist(fixture.envelope()).unwrap();

    let first = intake.scan().unwrap();
    assert_eq!(first.quarantined, 1);
    assert_eq!(first.occurrences.len(), 1);
    let second = fixture.intake().scan().unwrap();
    assert_eq!(second.quarantined, 1);
    assert_eq!(second.occurrences.len(), 1);
}

#[test]
fn cold_scan_orders_occurrences_by_monotonic_time_then_identity() {
    let fixture = IntakeFixture::new();
    let writer = fixture.intake();
    for (time, offset) in [
        (FIRST_TIME, OCCURRENCE_BASE),
        (SECOND_TIME, OCCURRENCE_BASE + 1),
        (THIRD_TIME, OCCURRENCE_BASE + 2),
    ] {
        let record = record(&fixture, time, offset);
        super::super::intake_storage::create(
            writer.directory(),
            &writer.path(&record.pending_file_name()),
            &record,
        )
        .unwrap();
    }
    drop(writer);
    let cold = fixture.intake().scan().unwrap();
    let times: Vec<_> = cold
        .occurrences
        .iter()
        .map(|occurrence| occurrence.ordering_key().0)
        .collect();
    assert_eq!(times, vec![SECOND_TIME, THIRD_TIME, FIRST_TIME]);
}

fn record(fixture: &IntakeFixture, time: u64, occurrence: u128) -> CodexIntakeRecord {
    CodexIntakeRecord::for_test(fixture.envelope(), Uuid::from_u128(occurrence), time).unwrap()
}
