use super::*;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};

const COMPACT_PAYLOAD: &[u8] = b"compact-boundary";
const CORRUPT_RETAINED_PATH: &str = "corrupt-retained";
const SEALED_SIZE_MISMATCH_PATH: &str = "sealed-size-mismatch";
const BEFORE_REOPEN_PATH: &str = "before-reopen";
const SAVED_INSTALL_PATH: &str = "saved-install";
const REPLACEMENT_INSTALL_PATH: &str = "replacement-install";
const SEALED_SIZE_MISMATCH: u64 = 1;

#[test]
fn compaction_rechecks_the_retained_payload_checksum() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(CORRUPT_RETAINED_PATH);
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(COMPACT_PAYLOAD).unwrap();
    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    let payload_offset = u64::try_from(JOURNAL_HEADER_BYTES + RECORD_HEADER_BYTES).unwrap();
    writer.seek(SeekFrom::Start(payload_offset)).unwrap();
    writer.write_all(&[COMPACT_PAYLOAD[0] ^ u8::MAX]).unwrap();
    writer.sync_all().unwrap();
    journal.identity = FileIdentity::inspect_file_bounded(
        &journal.file,
        &path,
        journal.max_disk_bytes,
    )
    .unwrap();

    assert!(matches!(
        journal.compact_through(EMPTY_SEQUENCE),
        Err(JournalError::Corrupt { .. })
    ));
}

#[test]
fn compact_builder_rejects_a_sealed_size_mismatch() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(SEALED_SIZE_MISMATCH_PATH);
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    let _lock = journal
        .entry
        .lock_and_cleanup(COMPACT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let mut temporary = journal
        .entry
        .temporary(COMPACT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)
        .unwrap();
    let sealed_bytes =
        u64::try_from(JOURNAL_HEADER_BYTES + RECORD_HEADER_BYTES).unwrap() + SEALED_SIZE_MISMATCH;

    let error = journal
        .build_and_install(
            &mut temporary,
            INITIAL_RETAINED_SEQUENCE,
            sealed_bytes,
            &[],
            &mut |_| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(error, JournalError::InvalidFormat));
}

#[test]
fn compaction_rejects_missing_and_replaced_installs_before_reopen() {
    assert_before_reopen_rejected(false);
    assert_before_reopen_rejected(true);
}

fn assert_before_reopen_rejected(replace: bool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(BEFORE_REOPEN_PATH);
    let saved = directory.path().join(SAVED_INSTALL_PATH);
    let replacement = directory.path().join(REPLACEMENT_INSTALL_PATH);
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(COMPACT_PAYLOAD).unwrap();

    let result = journal.compact_with_test_hook(EMPTY_SEQUENCE, |stage| {
        if stage == CompactStage::BeforeReopen {
            if replace {
                fs::copy(&path, &replacement)?;
                fs::rename(&path, &saved)?;
                fs::rename(&replacement, &path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    });

    assert!(result.is_err());
}
