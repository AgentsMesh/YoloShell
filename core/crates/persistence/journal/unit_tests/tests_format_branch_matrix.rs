use super::tests_security_support::{private_root, write_private};
use super::*;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const CORRUPT_HEADER_FILE: &str = "nonrecoverable-header";
const RECOVERABLE_HEADER_FILE: &str = "recoverable-header";
const LIMIT_FILE: &str = "limit-matrix";
const FORMAT_PAYLOAD: &[u8] = b"format payload";
const INVALID_MAGIC_BYTE: u8 = b'!';
const HEADER_CORRUPTION_MASK: u8 = u8::MAX;
const TRUNCATED_BYTES: usize = 1;
const FIRST_BYTE_INDEX: usize = 0;

#[test]
fn header_decode_rejects_each_independent_invariant() {
    let valid = JournalHeader::new(JOURNAL_HEADER_BYTES as u64).encode();
    assert_invalid_header(&valid[..valid.len() - TRUNCATED_BYTES]);

    let mut bad_magic = valid;
    bad_magic[..MAGIC_BYTES].fill(INVALID_MAGIC_BYTE);
    assert_invalid_header(&bad_magic);

    let mut bad_checksum = JournalHeader::new(JOURNAL_HEADER_BYTES as u64).encode();
    bad_checksum[GENERATION_START] ^= HEADER_CORRUPTION_MASK;
    assert_invalid_header(&bad_checksum);

    let too_short = JournalHeader {
        generation: Uuid::new_v4(),
        sealed_bytes: JOURNAL_HEADER_BYTES as u64 - TRUNCATED_BYTES as u64,
    };
    assert_invalid_header(&too_short.encode());
}

#[test]
fn header_reads_distinguish_truncation_from_io_failure() {
    let mut empty = tempfile::tempfile().unwrap();
    assert!(matches!(
        header::read_header(&mut empty),
        Err(JournalError::InvalidFormat)
    ));

    let root = private_root();
    let mut directory = File::open(root.path()).unwrap();
    assert!(matches!(
        header::read_header(&mut directory),
        Err(JournalError::Io(_))
    ));
}

#[test]
fn scanner_rejects_boundaries_and_accepts_a_stale_output() {
    let epoch = Uuid::new_v4();
    let mut empty = tempfile::tempfile().unwrap();
    assert!(matches!(
        scan::scan_committed(
            &mut empty,
            JOURNAL_HEADER_BYTES as u64 - TRUNCATED_BYTES as u64,
            epoch,
        ),
        Err(JournalError::InvalidFormat)
    ));

    let mut file = initialized_file();
    let cursor_sequence = INITIAL_RETAINED_SEQUENCE + INITIAL_RETAINED_SEQUENCE;
    write_record(&mut file, RecordKind::Cursor, epoch, cursor_sequence, &[]).unwrap();
    write_record(
        &mut file,
        RecordKind::Output,
        epoch,
        INITIAL_RETAINED_SEQUENCE,
        FORMAT_PAYLOAD,
    )
    .unwrap();
    let committed = file.metadata().unwrap().len();
    let state = scan::scan_committed(&mut file, committed, epoch).unwrap();
    assert!(state.records.is_empty());
    assert_eq!(state.last_sequence, INITIAL_RETAINED_SEQUENCE);
}

#[test]
fn scanner_rejects_invalid_magic_and_both_torn_record_shapes() {
    let epoch = Uuid::new_v4();
    let mut invalid = initialized_file();
    let mut record = record::encode_record(
        RecordKind::Output,
        epoch,
        INITIAL_RETAINED_SEQUENCE,
        FORMAT_PAYLOAD,
    )
    .unwrap();
    record[..MAGIC_BYTES].fill(INVALID_MAGIC_BYTE);
    invalid.write_all(&record).unwrap();
    assert_scan_invalid(&mut invalid, epoch);

    let mut short_header = initialized_file();
    short_header.write_all(OUTPUT_MAGIC).unwrap();
    assert_scan_invalid(&mut short_header, epoch);

    let mut short_payload = initialized_file();
    let encoded = record::encode_record(
        RecordKind::Output,
        epoch,
        INITIAL_RETAINED_SEQUENCE,
        FORMAT_PAYLOAD,
    )
    .unwrap();
    short_payload
        .write_all(&encoded[..encoded.len() - TRUNCATED_BYTES])
        .unwrap();
    assert_scan_invalid(&mut short_payload, epoch);
}

#[test]
fn open_rejects_both_policy_bounds_and_a_nonrecoverable_header() {
    let root = private_root();
    for limit in [
        JOURNAL_HEADER_BYTES as u64 - TRUNCATED_BYTES as u64,
        MAX_JOURNAL_DISK_BYTES + INITIAL_RETAINED_SEQUENCE,
    ] {
        assert!(matches!(
            Journal::open_with_max_bytes(root.path().join(LIMIT_FILE), Uuid::new_v4(), limit),
            Err(JournalError::DiskLimitExceeded)
        ));
    }

    let path = root.path().join(CORRUPT_HEADER_FILE);
    Journal::open(&path, Uuid::new_v4()).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    bytes[FIRST_BYTE_INDEX] = INVALID_MAGIC_BYTE;
    fs::write(&path, bytes).unwrap();
    assert!(matches!(
        Journal::open(&path, Uuid::new_v4()),
        Err(JournalError::InvalidFormat)
    ));
    assert!(cursor_path(&path).exists());
}

#[test]
fn open_reinitializes_a_short_header_only_without_a_cursor() {
    let root = private_root();
    let path = root.path().join(RECOVERABLE_HEADER_FILE);
    write_private(&path, &[INVALID_MAGIC_BYTE]);
    Journal::open(&path, Uuid::new_v4()).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().len(),
        JOURNAL_HEADER_BYTES as u64
    );
}

fn initialized_file() -> File {
    let mut file = tempfile::tempfile().unwrap();
    header::write_header(&mut file, JournalHeader::new(JOURNAL_HEADER_BYTES as u64)).unwrap();
    file
}

fn assert_scan_invalid(file: &mut File, epoch: Uuid) {
    let committed = file.metadata().unwrap().len();
    assert!(matches!(
        scan::scan_committed(file, committed, epoch),
        Err(JournalError::InvalidFormat)
    ));
}

fn assert_invalid_header(bytes: &[u8]) {
    assert!(matches!(
        JournalHeader::decode(bytes),
        Err(JournalError::InvalidFormat)
    ));
}

fn cursor_path(journal: &Path) -> PathBuf {
    let mut name = journal.file_name().map(OsString::from).unwrap();
    name.push(COMMIT_FILE_SUFFIX);
    journal.with_file_name(name)
}
