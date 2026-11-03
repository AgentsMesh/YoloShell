use super::append::AppendStage;
use super::compact::CompactStage;
use super::tests_security_support::*;
use super::*;
use std::fs;
use std::io;

const INJECTED_CRASH: &str = "injected crash boundary";

#[test]
fn full_length_corrupt_uncommitted_tail_is_discarded() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(b"durable").unwrap();
    let committed_bytes = fs::metadata(&path).unwrap().len();
    let result = journal.append_with_test_hook(b"torn", |stage| {
        if stage == AppendStage::AfterRecordSync {
            flip_tail(&path);
            return Err(injected_crash());
        }
        Ok(())
    });
    assert!(result.is_err());
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.last_sequence(), 1);
    assert_eq!(fs::metadata(&path).unwrap().len(), committed_bytes);
    assert_eq!(
        reopened.replay_after(0, usize::MAX).unwrap()[0].data,
        b"durable"
    );
    assert_eq!(reopened.append(b"next").unwrap().sequence, 2);
}

#[test]
fn committed_tail_corruption_is_never_silently_truncated() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    Journal::open(&path, epoch)
        .unwrap()
        .append(b"committed")
        .unwrap();
    let committed_bytes = fs::metadata(&path).unwrap().len();
    flip_tail(&path);
    assert!(matches!(
        Journal::open(&path, epoch),
        Err(JournalError::Corrupt { sequence: 1 })
    ));
    assert_eq!(fs::metadata(path).unwrap().len(), committed_bytes);
}

#[test]
fn cursor_commit_claims_an_uncertain_append_without_sequence_reuse() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(b"one").unwrap();
    let result = journal.append_with_test_hook(b"two", |stage| {
        if stage == AppendStage::AfterCursorCommit {
            return Err(injected_crash());
        }
        Ok(())
    });
    assert!(result.is_err());
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.last_sequence(), 2);
    assert_eq!(
        reopened
            .replay_after(0, usize::MAX)
            .unwrap()
            .into_iter()
            .map(|chunk| chunk.data)
            .collect::<Vec<_>>(),
        vec![b"one".to_vec(), b"two".to_vec()]
    );
    assert_eq!(reopened.append(b"three").unwrap().sequence, 3);
}

#[test]
fn compaction_generation_repair_requires_a_valid_sealed_file() {
    let root = private_root();
    let path = root.path().join("journal");
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    journal.append(b"one").unwrap();
    journal.append(b"two").unwrap();
    let result = journal.compact_with_test_hook(1, |stage| {
        if stage == CompactStage::Installed {
            return Err(injected_crash());
        }
        Ok(())
    });
    assert!(result.is_err());
    drop(journal);

    let reopened = Journal::open(&path, epoch).unwrap();
    assert_eq!(reopened.retained_from_sequence(), 2);
    assert_eq!(reopened.last_sequence(), 2);

    let mut journal = reopened;
    let result = journal.compact_with_test_hook(1, |stage| {
        if stage == CompactStage::Installed {
            return Err(injected_crash());
        }
        Ok(())
    });
    assert!(result.is_err());
    drop(journal);
    flip_tail(&path);
    assert!(matches!(
        Journal::open(&path, epoch),
        Err(JournalError::Corrupt { .. })
    ));
}

#[test]
fn missing_or_corrupt_commit_cursor_cannot_discard_committed_output() {
    let root = private_root();
    let path = root.path().join("journal");
    let cursor = cursor_path(&path);
    let epoch = Uuid::new_v4();
    Journal::open(&path, epoch)
        .unwrap()
        .append(b"committed")
        .unwrap();
    let cursor_bytes = fs::read(&cursor).unwrap();
    fs::remove_file(&cursor).unwrap();
    assert!(Journal::open(&path, epoch).is_err());
    write_private(&cursor, &cursor_bytes);
    flip_tail(&cursor);
    assert!(Journal::open(&path, epoch).is_err());
}

fn cursor_path(journal: &Path) -> std::path::PathBuf {
    let mut name = journal.file_name().unwrap().to_os_string();
    name.push(COMMIT_FILE_SUFFIX);
    journal.with_file_name(name)
}

fn flip_tail(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= u8::MAX;
    fs::write(path, bytes).unwrap();
}

fn injected_crash() -> JournalError {
    io::Error::other(INJECTED_CRASH).into()
}
