use super::append::AppendStage;
use super::compact::CompactStage;
use super::snapshot::{SnapshotWriteStage, write_snapshot_with_test_hook};
use super::tests_security_support::*;
use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

#[test]
fn snapshot_rejects_encoded_limits_and_installed_content_change() {
    let root = private_root();
    let path = root.path().join("snapshot");
    for length in [MAX_SNAPSHOT_DISK_BYTES, MAX_SNAPSHOT_DISK_BYTES + 1] {
        let length = usize::try_from(length).unwrap();
        let snapshot = sample_snapshot(&vec![0_u8; length]);
        assert!(matches!(
            write_snapshot(&path, &snapshot),
            Err(JournalError::PayloadTooLarge)
        ));
    }

    let result = write_snapshot_with_test_hook(&path, &sample_snapshot(b"new"), |stage| {
        if stage == SnapshotWriteStage::Installed {
            let mut bytes = fs::read(&path)?;
            *bytes.last_mut().unwrap() ^= u8::MAX;
            fs::write(&path, bytes)?;
        }
        Ok(())
    });
    assert!(result.is_err());
}

#[test]
fn compaction_cleans_hook_failure_and_rejects_size_and_corrupt_source() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    assert!(
        journal
            .compact_with_test_hook(EMPTY_SEQUENCE, |stage| {
                assert_eq!(stage, CompactStage::TemporarySynced);
                Err(JournalError::InvalidFormat)
            })
            .is_err()
    );
    assert!(fs::read_dir(root.path()).unwrap().flatten().all(|entry| {
        !entry
            .file_name()
            .to_string_lossy()
            .starts_with(COMPACT_TEMPORARY_PREFIX)
    }));
    journal.append(b"healthy").unwrap();

    let too_small = root.path().join("small");
    let mut small = Journal::open_with_max_bytes(
        &too_small,
        epoch,
        u64::try_from(RECORD_HEADER_BYTES - 1).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        small.compact_through(EMPTY_SEQUENCE),
        Err(JournalError::DiskLimitExceeded)
    ));

    let corrupt = root.path().join("corrupt");
    let mut journal = Journal::open(&corrupt, epoch).unwrap();
    journal.append(b"payload").unwrap();
    flip_last_byte(&corrupt);
    assert!(journal.compact_through(EMPTY_SEQUENCE).is_err());
}

#[test]
fn journal_open_rejects_cursor_payload_zero_sequence_and_gap() {
    let root = private_root();
    let epoch = Uuid::new_v4();
    let cases = [
        (
            RecordKind::Cursor,
            INITIAL_RETAINED_SEQUENCE,
            b"x".as_slice(),
        ),
        (RecordKind::Output, EMPTY_SEQUENCE, b"x".as_slice()),
        (
            RecordKind::Output,
            INITIAL_RETAINED_SEQUENCE + 1,
            b"x".as_slice(),
        ),
    ];
    for (index, (kind, sequence, payload)) in cases.into_iter().enumerate() {
        let path = root.path().join(format!("invalid-{index}"));
        Journal::open(&path, epoch)
            .unwrap()
            .append(payload)
            .unwrap();
        let mut bytes = fs::read(&path).unwrap();
        let header = JOURNAL_HEADER_BYTES;
        bytes[header..header + MAGIC_BYTES].copy_from_slice(match kind {
            RecordKind::Output => OUTPUT_MAGIC,
            RecordKind::Cursor => CURSOR_MAGIC,
        });
        bytes[header + SEQUENCE_START..header + SEQUENCE_END]
            .copy_from_slice(&sequence.to_be_bytes());
        fs::write(&path, bytes).unwrap();
        assert!(Journal::open(&path, epoch).is_err());
    }
}

#[test]
fn append_rejects_descriptor_changes_and_replay_rechecks_checksum() {
    let root = private_root();
    let unsafe_path = root.path().join("unsafe");
    let mut unsafe_journal = Journal::open(&unsafe_path, Uuid::new_v4()).unwrap();
    fs::set_permissions(&unsafe_path, fs::Permissions::from_mode(UNSAFE_FILE_MODE)).unwrap();
    assert!(unsafe_journal.append(b"rejected").is_err());

    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    let result = journal.append_with_test_hook(b"payload", |stage| {
        if stage == AppendStage::AfterRecordSync {
            OpenOptions::new()
                .append(true)
                .open(&path)?
                .write_all(b"x")?;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(matches!(
        journal.append(b"retry"),
        Err(JournalError::WriteStateUncertain)
    ));

    let mut reopened = Journal::open(&path, epoch).unwrap();
    reopened.append(b"durable").unwrap();
    flip_last_byte(&path);
    assert!(reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).is_err());
}

fn flip_last_byte(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= u8::MAX;
    fs::write(path, bytes).unwrap();
}
