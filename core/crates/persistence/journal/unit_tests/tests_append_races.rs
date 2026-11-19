use super::append::AppendStage;
use super::tests_security_support::*;
use super::*;
use std::fs;

#[test]
fn append_detects_target_replacement_before_write() {
    let root = private_root();
    let path = root.path().join("journal");
    let saved = root.path().join("saved");
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    let result = journal.append_with_test_hook(b"payload", |stage| {
        if stage == AppendStage::BeforeWrite {
            replace_target(&path, &saved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(
        fs::metadata(&saved).unwrap().len(),
        JOURNAL_HEADER_BYTES as u64
    );
}

#[test]
fn append_detects_target_replacement_after_sync_and_becomes_uncertain() {
    let root = private_root();
    let path = root.path().join("journal");
    let saved = root.path().join("saved");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    let result = journal.append_with_test_hook(b"payload", |stage| {
        if stage == AppendStage::AfterRecordSync {
            replace_target(&path, &saved);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(matches!(
        journal.append(b"retry"),
        Err(JournalError::WriteStateUncertain)
    ));
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
    assert!(Journal::open(&saved, epoch).is_err());
    assert!(fs::read(&saved).unwrap().ends_with(b"payload"));
}

#[test]
fn append_detects_parent_replacement_before_write() {
    let root = private_root();
    let parent = root.path().join("terminal");
    set_private_directory(&parent);
    let path = parent.join("journal");
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    let result = journal.append_with_test_hook(b"payload", |stage| {
        if stage == AppendStage::BeforeWrite {
            move_parent(&path);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
}

#[test]
fn append_detects_parent_replacement_after_sync() {
    let root = private_root();
    let parent = root.path().join("terminal");
    set_private_directory(&parent);
    let path = parent.join("journal");
    let mut journal = Journal::open(&path, Uuid::new_v4()).unwrap();
    let result = journal.append_with_test_hook(b"payload", |stage| {
        if stage == AppendStage::AfterRecordSync {
            move_parent(&path);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(matches!(
        journal.append(b"retry"),
        Err(JournalError::WriteStateUncertain)
    ));
    assert_eq!(fs::read(&path).unwrap(), REPLACEMENT_BYTES);
}
