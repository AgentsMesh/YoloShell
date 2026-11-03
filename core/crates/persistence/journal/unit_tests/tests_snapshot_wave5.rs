use super::*;
use std::fs::OpenOptions;

const SNAPSHOT_SIZE_INCREMENT: u64 = 1;
const OVERSIZED_BOUNDED_READ_PATH: &str = "oversized-bounded-read";
const INSTALLED_CONTENT: &[u8] = b"installed";
const EXPECTED_CONTENT: &[u8] = b"expected";

#[test]
fn bounded_snapshot_read_rejects_oversized_content() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(OVERSIZED_BOUNDED_READ_PATH);
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.set_len(MAX_SNAPSHOT_DISK_BYTES + SNAPSHOT_SIZE_INCREMENT)
        .unwrap();

    assert!(matches!(
        bounded_read(&mut file),
        Err(JournalError::PayloadTooLarge)
    ));
}

#[test]
fn installed_snapshot_content_must_match_the_encoded_value() {
    assert!(verify_installed_content(EXPECTED_CONTENT, EXPECTED_CONTENT).is_ok());
    assert!(matches!(
        verify_installed_content(INSTALLED_CONTENT, EXPECTED_CONTENT),
        Err(JournalError::Durability(_))
    ));
}
