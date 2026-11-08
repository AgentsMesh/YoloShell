use super::append::AppendStage;
use super::compact::CompactStage;
use super::snapshot::{
    SnapshotReadStage, SnapshotWriteStage, read_snapshot_with_test_hook,
    write_snapshot_with_test_hook,
};
use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;

const PAYLOAD: &[u8] = b"instance-matrix-payload";
const PREVIOUS_SNAPSHOT_PAYLOAD: &[u8] = b"previous-snapshot";
const FAILURE: JournalError = JournalError::InvalidFormat;
const APPEND_PATH: &str = "append";
const COMPACT_PATH: &str = "compact";
const SAVED_PATH: &str = "saved";
const SNAPSHOT_PATH: &str = "terminal-state";
const SNAPSHOT_READ_PATH: &str = "terminal-state-read";
const FIRST_BYTE_INDEX: usize = 0;
const SNAPSHOT_BARRIER: u64 = 7;
const SNAPSHOT_COLUMNS: u16 = 80;
const SNAPSHOT_ROWS: u16 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendAction {
    Success,
    FailBeforeWrite,
    FailAfterRecordSync,
    FailAfterCursorCommit,
}

#[test]
fn one_append_instance_covers_success_and_each_transaction_boundary() {
    for action in [
        AppendAction::Success,
        AppendAction::FailBeforeWrite,
        AppendAction::FailAfterRecordSync,
        AppendAction::FailAfterCursorCommit,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(APPEND_PATH);
        let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
        let result = journal.append_with_test_hook(PAYLOAD, |stage| {
            let failed = matches!(
                (action, stage),
                (AppendAction::FailBeforeWrite, AppendStage::BeforeWrite)
                    | (
                        AppendAction::FailAfterRecordSync,
                        AppendStage::AfterRecordSync
                    )
                    | (
                        AppendAction::FailAfterCursorCommit,
                        AppendStage::AfterCursorCommit
                    )
            );
            if failed { Err(FAILURE) } else { Ok(()) }
        });
        assert_eq!(result.is_ok(), action == AppendAction::Success);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompactAction {
    Success,
    FailTemporary,
    FailInstalled,
    FailBeforeReopen,
    ChangeSource,
    RemoveInstalled,
    ReplaceBeforeReopen,
}

#[test]
fn one_compaction_instance_covers_commit_failures_and_identity_races() {
    for action in [
        CompactAction::Success,
        CompactAction::FailTemporary,
        CompactAction::FailInstalled,
        CompactAction::FailBeforeReopen,
        CompactAction::ChangeSource,
        CompactAction::RemoveInstalled,
        CompactAction::ReplaceBeforeReopen,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(COMPACT_PATH);
        let saved = root.path().join(SAVED_PATH);
        let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
        journal.append(PAYLOAD).unwrap();
        let result =
            journal.compact_with_test_hook(EMPTY_SEQUENCE, |stage| match (action, stage) {
                (CompactAction::FailTemporary, CompactStage::TemporarySynced)
                | (CompactAction::FailInstalled, CompactStage::Installed)
                | (CompactAction::FailBeforeReopen, CompactStage::BeforeReopen) => Err(FAILURE),
                (CompactAction::ChangeSource, CompactStage::TemporarySynced) => {
                    OpenOptions::new()
                        .append(true)
                        .open(&path)?
                        .write_all(PAYLOAD)?;
                    Ok(())
                }
                (CompactAction::RemoveInstalled, CompactStage::Installed) => {
                    fs::remove_file(&path)?;
                    Ok(())
                }
                (CompactAction::ReplaceBeforeReopen, CompactStage::BeforeReopen) => {
                    fs::rename(&path, &saved)?;
                    fs::copy(&saved, &path)?;
                    Ok(())
                }
                _ => Ok(()),
            });
        assert_eq!(result.is_ok(), action == CompactAction::Success);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SnapshotAction {
    Success,
    FailTemporary,
    FailInstalled,
    ChangeOriginal,
    ChangeInstalled,
}

#[test]
fn one_snapshot_write_instance_covers_commit_and_content_failures() {
    for action in [
        SnapshotAction::Success,
        SnapshotAction::FailTemporary,
        SnapshotAction::FailInstalled,
        SnapshotAction::ChangeOriginal,
        SnapshotAction::ChangeInstalled,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(SNAPSHOT_PATH);
        write_snapshot(&path, &snapshot(PREVIOUS_SNAPSHOT_PAYLOAD)).unwrap();
        let result = write_snapshot_with_test_hook(&path, &snapshot(PAYLOAD), |stage| {
            match (action, stage) {
                (SnapshotAction::FailTemporary, SnapshotWriteStage::TemporarySynced)
                | (SnapshotAction::FailInstalled, SnapshotWriteStage::Installed) => Err(FAILURE),
                (SnapshotAction::ChangeOriginal, SnapshotWriteStage::TemporarySynced)
                | (SnapshotAction::ChangeInstalled, SnapshotWriteStage::Installed) => {
                    fs::write(&path, PAYLOAD)?;
                    Ok(())
                }
                _ => Ok(()),
            }
        });
        assert_eq!(result.is_ok(), action == SnapshotAction::Success);
    }
}

#[test]
fn one_snapshot_read_instance_covers_stable_and_changed_content() {
    for change in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(SNAPSHOT_READ_PATH);
        write_snapshot(&path, &snapshot(PAYLOAD)).unwrap();
        let result = read_snapshot_with_test_hook(&path, |_stage: SnapshotReadStage| {
            if change {
                let mut bytes = fs::read(&path)?;
                bytes[FIRST_BYTE_INDEX] ^= u8::MAX;
                fs::write(&path, bytes)?;
            }
            Ok(())
        });
        assert_eq!(result.is_ok(), !change);
    }
}

fn snapshot(data: &[u8]) -> TerminalSnapshot {
    TerminalSnapshot {
        runtime_epoch: Uuid::new_v4(),
        barrier_sequence: SNAPSHOT_BARRIER,
        cols: SNAPSHOT_COLUMNS,
        rows: SNAPSHOT_ROWS,
        data: data.to_vec(),
    }
}
