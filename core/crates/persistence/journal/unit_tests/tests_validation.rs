use super::*;
use std::fs::{self, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use tempfile::tempdir;

const CORRUPT_HEADER_BYTE: u8 = b'X';
const INVALID_MESSAGE_PACK_MARKER: u8 = 0xc1;
const MAGIC_TRUNCATION_BYTES: usize = 1;
const SINGLE_PAYLOAD_BYTE: u32 = 1;

#[test]
fn corrupt_output_and_snapshot_are_rejected() {
    let directory = tempdir().unwrap();
    let output = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    Journal::open(&output, epoch)
        .unwrap()
        .append(b"payload")
        .unwrap();
    let mut bytes = fs::read(&output).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(&output, bytes).unwrap();
    assert!(matches!(
        Journal::open(&output, epoch),
        Err(JournalError::Corrupt { .. })
    ));

    let snapshot_path = directory.path().join("screen.snapshot");
    assert!(read_snapshot(&snapshot_path).is_err());
    let snapshot = TerminalSnapshot {
        runtime_epoch: epoch,
        barrier_sequence: 1,
        cols: 80,
        rows: 24,
        data: b"screen".to_vec(),
    };
    write_snapshot(&snapshot_path, &snapshot).unwrap();
    let mut bytes = fs::read(&snapshot_path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(&snapshot_path, bytes).unwrap();
    assert!(read_snapshot(&snapshot_path).is_err());
}

#[test]
fn snapshot_headers_lengths_and_payload_encoding_fail_closed() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("screen.snapshot");
    crate::paths::ensure_private_parent(&path).unwrap();
    write_private(
        &path,
        &SNAPSHOT_MAGIC[..SNAPSHOT_MAGIC.len() - MAGIC_TRUNCATION_BYTES],
    );
    assert!(matches!(
        read_snapshot(&path),
        Err(JournalError::InvalidFormat)
    ));

    write_private(&path, &[CORRUPT_HEADER_BYTE; SNAPSHOT_HEADER_BYTES]);
    assert!(matches!(
        read_snapshot(&path),
        Err(JournalError::InvalidFormat)
    ));

    let mut truncated = SNAPSHOT_MAGIC.to_vec();
    truncated.extend_from_slice(&SINGLE_PAYLOAD_BYTE.to_be_bytes());
    truncated.extend_from_slice(&crc32fast::hash(&[]).to_be_bytes());
    write_private(&path, &truncated);
    assert!(matches!(
        read_snapshot(&path),
        Err(JournalError::InvalidFormat)
    ));

    let invalid_message_pack = [INVALID_MESSAGE_PACK_MARKER];
    let checksum = crc32fast::hash(&invalid_message_pack);
    let mut encoded = SNAPSHOT_MAGIC.to_vec();
    encoded.extend_from_slice(&(invalid_message_pack.len() as u32).to_be_bytes());
    encoded.extend_from_slice(&checksum.to_be_bytes());
    encoded.extend_from_slice(&invalid_message_pack);
    write_private(&path, &encoded);
    assert!(matches!(
        read_snapshot(&path),
        Err(JournalError::SnapshotDecode(_))
    ));
}

#[test]
fn invalid_headers_and_truncated_payloads_are_handled() {
    let directory = tempdir().unwrap();
    let epoch = Uuid::new_v4();
    let invalid = directory.path().join("invalid.log");
    write_private(&invalid, &[b'X'; RECORD_HEADER_BYTES]);
    assert!(matches!(
        Journal::open(&invalid, epoch),
        Err(JournalError::InvalidFormat)
    ));

    let truncated = directory.path().join("truncated.log");
    Journal::open(&truncated, epoch).unwrap();
    let mut file = OpenOptions::new().append(true).open(&truncated).unwrap();
    file.write_all(OUTPUT_MAGIC).unwrap();
    file.write_all(epoch.as_bytes()).unwrap();
    file.write_all(&1_u64.to_be_bytes()).unwrap();
    file.write_all(&5_u32.to_be_bytes()).unwrap();
    file.write_all(&0_u32.to_be_bytes()).unwrap();
    file.write_all(b"x").unwrap();
    drop(file);
    let journal = Journal::open(&truncated, epoch).unwrap();
    assert_eq!(journal.last_sequence(), 0);
    assert_eq!(
        fs::metadata(truncated).unwrap().len(),
        JOURNAL_HEADER_BYTES as u64
    );
}

fn write_private(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

#[test]
fn disk_limit_is_enforced_and_compaction_frees_space() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open_with_max_bytes(&path, epoch, 140).unwrap();
    journal.append(b"first-record-").unwrap();
    journal.append(b"second-record").unwrap();
    assert!(journal.would_exceed_limit(b"third-record-".len()));
    assert!(matches!(
        journal.append(b"third-record-"),
        Err(JournalError::DiskLimitExceeded)
    ));
    journal.compact_through(2).unwrap();
    assert!(!journal.would_exceed_limit(b"third-record-".len()));
    assert_eq!(journal.append(b"third-record-").unwrap().sequence, 3);
}

#[test]
fn replay_respects_byte_limit() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.log");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(b"1234").unwrap();
    journal.append(b"5678").unwrap();
    assert!(journal.replay_after(0, 3).unwrap().is_empty());
    assert_eq!(journal.replay_after(0, 4).unwrap().len(), 1);
    assert_eq!(journal.replay_after(1, 4).unwrap()[0].sequence, 2);
}
