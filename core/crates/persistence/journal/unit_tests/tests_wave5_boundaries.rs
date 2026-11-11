use super::*;
use std::fs::File;

const APPEND_PAYLOAD: &[u8] = b"append-boundary";
const APPEND_WRITE_FAILURE_PATH: &str = "append-write-failure";
const APPEND_LENGTH_OVERFLOW_PATH: &str = "append-length-overflow";
const OVERSIZED_PAYLOAD_INCREMENT: usize = 1;

#[test]
fn append_surfaces_record_write_failure_and_becomes_uncertain() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(APPEND_WRITE_FAILURE_PATH);
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    journal.file = File::open(&path).unwrap();

    assert!(matches!(
        journal.append(APPEND_PAYLOAD),
        Err(JournalError::Io(_))
    ));
    assert!(matches!(
        journal.append(APPEND_PAYLOAD),
        Err(JournalError::WriteStateUncertain)
    ));
}

#[test]
fn append_length_validation_rejects_the_u32_wire_overflow() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(APPEND_LENGTH_OVERFLOW_PATH);
    let journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    let oversized = usize::try_from(u32::MAX).unwrap() + OVERSIZED_PAYLOAD_INCREMENT;

    assert!(matches!(
        journal.validate_append_length(oversized),
        Err(JournalError::PayloadTooLarge)
    ));
}
