use super::compact::CompactStage;
use super::tests_security_support::*;
use super::*;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn compaction_detects_parent_replacement_before_install() {
    let root = private_root();
    let parent = root.path().join("terminal");
    set_private_directory(&parent);
    let path = parent.join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(ORIGINAL_BYTES).unwrap();
    let result = journal.compact_with_test_hook(1, |stage| {
        if stage == CompactStage::TemporarySynced {
            move_parent(&path);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
}

#[test]
fn compaction_detects_target_replacement_after_install() {
    let root = private_root();
    let path = root.path().join("journal");
    let saved = root.path().join("saved");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(ORIGINAL_BYTES).unwrap();
    let result = journal.compact_with_test_hook(1, |stage| {
        if stage == CompactStage::Installed {
            replace_target(&path, &saved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(
        Journal::open(&saved, epoch)
            .unwrap()
            .retained_from_sequence(),
        2
    );
}

#[test]
fn compaction_never_follows_a_replaced_temporary_name() {
    let root = private_root();
    let path = root.path().join("journal");
    let outside = root.path().join("outside");
    write_private(&outside, ORIGINAL_BYTES);
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    journal.append(b"payload").unwrap();
    let result = journal.compact_with_test_hook(1, |stage| {
        if stage == CompactStage::TemporarySynced {
            let temporary = temporary_path(root.path(), COMPACT_TEMPORARY_PREFIX);
            fs::remove_file(&temporary)?;
            symlink(&outside, temporary)?;
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&outside).unwrap(), ORIGINAL_BYTES);
    assert_eq!(journal.last_sequence(), 1);
}

#[test]
fn maximum_sequence_is_committed_once_and_never_reused() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    write_record(&mut journal.file, RecordKind::Cursor, epoch, u64::MAX, &[]).unwrap();
    journal.file.sync_all().unwrap();
    let committed_bytes = journal.file.metadata().unwrap().len();
    journal.commit_cursor.publish(committed_bytes).unwrap();
    drop(journal);

    let mut journal = Journal::open(&path, epoch).unwrap();
    assert_eq!(journal.append(b"last").unwrap().sequence, u64::MAX);
    assert!(matches!(
        journal.append(b"never"),
        Err(JournalError::SequenceExhausted)
    ));
    journal.compact_through(u64::MAX).unwrap();
    assert_eq!(journal.last_sequence(), u64::MAX);
    assert!(matches!(
        journal.append(b"still-never"),
        Err(JournalError::SequenceExhausted)
    ));
}
