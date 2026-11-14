use super::commit_cursor_format::decode_cursor;
use super::tests_retained_hooks::{clear_staging_hook, set_post_install_hook, set_staging_hook};
use super::tests_retained_support::{RETAINED_PAYLOAD, RetainedFixture};
use super::tests_security_support::private_root;
use super::*;
use std::fs::{self, OpenOptions};

const CLONE_INTERRUPTION: &str = "injected retained clone interruption";
const SLOT_WRITE_INTERRUPTION: &str = "injected retained slot write interruption";
const ROLLBACK_INTERRUPTION: &str = "injected retained rollback validation failure";
const PARTIAL_CURSOR_SLOT_BYTES: u64 = (COMMIT_CURSOR_BYTES / 2) as u64;
const ROLLBACK_RETRY_PAYLOAD: &[u8] = b"rollback retry payload";
const FIRST_CHUNK_INDEX: usize = 0;
const COMPACTED_CURSOR_JOURNAL_FILE: &str = "compacted-retained-cursor";
const COMPACTED_PAYLOAD: &[u8] = b"compacted payload";
const POST_COMPACTION_PAYLOAD: &[u8] = b"post compaction retained payload";

#[test]
fn atomic_clone_with_empty_second_slot_recovers_after_reopen() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    crate::paths::test_support::set_retained_clone_hook(|| {
        Err(std::io::Error::other(CLONE_INTERRUPTION).into())
    });
    assert!(fixture.reconcile().is_err());

    let epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, epoch).unwrap();
    let chunk = reopened
        .reconcile_append_retained(fixture.attempt, RETAINED_PAYLOAD)
        .unwrap()
        .unwrap();

    assert_eq!(chunk.data, RETAINED_PAYLOAD);
    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn partial_second_slot_recovers_after_reopen() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    set_staging_hook(|| Err(std::io::Error::other(SLOT_WRITE_INTERRUPTION).into()));
    assert!(fixture.reconcile().is_err());
    clear_staging_hook();
    let retired = fixture.artifact(crate::paths::test_support::retired_artifact_prefix());
    let file = OpenOptions::new().write(true).open(retired).unwrap();
    file.set_len(COMMIT_CURSOR_BYTES as u64 + PARTIAL_CURSOR_SLOT_BYTES)
        .unwrap();
    file.sync_all().unwrap();

    let epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, epoch).unwrap();
    let chunk = reopened
        .reconcile_append_retained(fixture.attempt, RETAINED_PAYLOAD)
        .unwrap()
        .unwrap();

    assert_eq!(chunk.data, RETAINED_PAYLOAD);
    fixture.assert_exactly_once();
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn rollback_revision_keeps_the_smaller_canonical_commit_active() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    set_post_install_hook(|| Err(std::io::Error::other(ROLLBACK_INTERRUPTION).into()));
    assert!(fixture.reconcile().is_err());

    let epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, epoch).unwrap();
    assert!(
        reopened
            .replay_after(EMPTY_SEQUENCE, usize::MAX)
            .unwrap()
            .is_empty()
    );
    let chunk = reopened
        .reconcile_append_retained(fixture.attempt, ROLLBACK_RETRY_PAYLOAD)
        .unwrap()
        .unwrap();

    assert_eq!(chunk.data, ROLLBACK_RETRY_PAYLOAD);
    let chunks = reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();
    assert_eq!(chunks.len(), INITIAL_RETAINED_SEQUENCE as usize);
    assert_eq!(chunks[FIRST_CHUNK_INDEX].data, ROLLBACK_RETRY_PAYLOAD);
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn empty_replacement_slot_fails_closed_without_overwrite() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    crate::paths::test_support::set_retained_clone_hook(|| {
        Err(std::io::Error::other(CLONE_INTERRUPTION).into())
    });
    assert!(fixture.reconcile().is_err());
    let retired = fixture.artifact(crate::paths::test_support::retired_artifact_prefix());
    fs::write(&retired, []).unwrap();

    assert!(fixture.reconcile().is_err());

    assert!(fs::read(retired).unwrap().is_empty());
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

#[test]
fn old_generation_slot_is_migrated_after_compaction() {
    let root = private_root();
    let path = root.path().join(COMPACTED_CURSOR_JOURNAL_FILE);
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    let compacted_attempt = journal.append_attempt();
    let compacted = journal
        .reconcile_append_retained(compacted_attempt, COMPACTED_PAYLOAD)
        .unwrap()
        .unwrap();
    journal.compact_through(compacted.sequence).unwrap();
    let retired = fs::read_dir(root.path())
        .unwrap()
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(crate::paths::test_support::retired_artifact_prefix())
        })
        .unwrap();
    let retired_bytes = fs::read(retired.path()).unwrap();
    let retired_record = decode_cursor(&retired_bytes[..COMMIT_CURSOR_BYTES]).unwrap();
    assert_ne!(retired_record.generation, journal.header.generation);
    let attempt = journal.append_attempt();

    let chunk = journal
        .reconcile_append_retained(attempt, POST_COMPACTION_PAYLOAD)
        .unwrap()
        .unwrap();

    assert_eq!(chunk.data, POST_COMPACTION_PAYLOAD);
    let chunks = journal.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();
    assert_eq!(chunks.len(), INITIAL_RETAINED_SEQUENCE as usize);
    assert_eq!(chunks[FIRST_CHUNK_INDEX].data, POST_COMPACTION_PAYLOAD);
}
