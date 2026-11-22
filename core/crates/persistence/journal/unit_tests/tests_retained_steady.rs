use super::commit_cursor_format::{CursorRecord, encode_cursor};
use super::tests_retained_hooks::inject_parent_sync_failure;
use super::tests_retained_support::{RETAINED_PAYLOAD, RetainedFixture};
use super::tests_security_support::{REPLACEMENT_BYTES, private_root, write_private};
use super::*;
use std::fs;

const STEADY_JOURNAL_FILE: &str = "retained-steady-journal";
const FIRST_DISTINCT_PAYLOAD: &[u8] = b"first retained payload";
const SECOND_DISTINCT_PAYLOAD: &[u8] = b"second retained payload";
const THIRD_DISTINCT_PAYLOAD: &[u8] = b"third retained payload";
const REOPENED_DISTINCT_PAYLOAD: &[u8] = b"reopened retained payload";
const STEADY_SLOT_ENTRY_COUNT: usize = 1;
const EARLIER_RECOVERY_REVISION: u64 = 4;
const NEWER_RECOVERY_REVISION: u64 = EARLIER_RECOVERY_REVISION + CURSOR_REVISION_INCREMENT;

#[test]
fn consecutive_retained_publications_reuse_slots_across_reopen() {
    let root = private_root();
    let path = root.path().join(STEADY_JOURNAL_FILE);
    let epoch = Uuid::new_v4();
    let mut journal = Journal::open(&path, epoch).unwrap();
    let baseline = fs::read_dir(root.path()).unwrap().flatten().count();
    let first_payloads = [
        FIRST_DISTINCT_PAYLOAD,
        SECOND_DISTINCT_PAYLOAD,
        THIRD_DISTINCT_PAYLOAD,
    ];
    for payload in first_payloads {
        append_retained(&mut journal, payload);
    }
    let steady_entries = fs::read_dir(root.path()).unwrap().flatten().count();
    assert_eq!(steady_entries, baseline + STEADY_SLOT_ENTRY_COUNT);
    drop(journal);

    let mut reopened = Journal::open(&path, epoch).unwrap();
    append_retained(&mut reopened, REOPENED_DISTINCT_PAYLOAD);
    let chunks = reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();

    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.data.as_slice())
            .collect::<Vec<_>>(),
        [
            FIRST_DISTINCT_PAYLOAD,
            SECOND_DISTINCT_PAYLOAD,
            THIRD_DISTINCT_PAYLOAD,
            REOPENED_DISTINCT_PAYLOAD,
        ]
    );
    assert_eq!(
        fs::read_dir(root.path()).unwrap().flatten().count(),
        steady_entries
    );
}

#[test]
fn parent_sync_failure_settles_before_same_object_and_reopened_appends() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let baseline = fixture.entry_count();
    inject_parent_sync_failure();
    assert!(fixture.reconcile().is_err());
    fixture.reconcile().unwrap().unwrap();
    append_retained(&mut fixture.journal, SECOND_DISTINCT_PAYLOAD);
    assert_eq!(fixture.entry_count(), baseline + STEADY_SLOT_ENTRY_COUNT);

    let epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, epoch).unwrap();
    append_retained(&mut reopened, THIRD_DISTINCT_PAYLOAD);
    let chunks = reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();

    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.data.as_slice())
            .collect::<Vec<_>>(),
        [
            RETAINED_PAYLOAD,
            SECOND_DISTINCT_PAYLOAD,
            THIRD_DISTINCT_PAYLOAD
        ]
    );
    assert_eq!(fixture.entry_count(), baseline + STEADY_SLOT_ENTRY_COUNT);
    fixture.assert_replacements_unchanged();
}

#[test]
fn a_more_advanced_isolated_cursor_fails_closed() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    inject_parent_sync_failure();
    assert!(fixture.reconcile().is_err());
    let isolated =
        fixture.artifact(crate::paths::test_support::retained_isolation_artifact_prefix());
    let canonical_bytes = fs::read(&fixture.moved_cursor).unwrap();
    let isolated_bytes = fs::read(&isolated).unwrap();
    fs::write(&fixture.moved_cursor, isolated_bytes).unwrap();
    fs::write(isolated, canonical_bytes).unwrap();

    let epoch = fixture.journal.runtime_epoch();
    assert!(Journal::open(&fixture.moved_journal, epoch).is_err());
    fixture.assert_replacements_unchanged();
}

#[test]
fn newer_isolated_revision_fails_closed_even_with_a_smaller_commit() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    inject_parent_sync_failure();
    assert!(fixture.reconcile().is_err());
    let isolated =
        fixture.artifact(crate::paths::test_support::retained_isolation_artifact_prefix());
    let journal_bytes = fs::metadata(&fixture.moved_journal).unwrap().len();
    let sealed_bytes = JOURNAL_HEADER_BYTES as u64;
    assert!(sealed_bytes < journal_bytes);
    let canonical = CursorRecord {
        generation: fixture.attempt.generation,
        committed_bytes: journal_bytes,
        publication_revision: EARLIER_RECOVERY_REVISION,
    };
    let newer_isolated = CursorRecord {
        generation: fixture.attempt.generation,
        committed_bytes: sealed_bytes,
        publication_revision: NEWER_RECOVERY_REVISION,
    };
    fs::write(&fixture.moved_cursor, encode_cursor(canonical)).unwrap();
    fs::write(isolated, encode_cursor(newer_isolated)).unwrap();

    let epoch = fixture.journal.runtime_epoch();
    assert!(Journal::open(&fixture.moved_journal, epoch).is_err());
    fixture.assert_replacements_unchanged();
}

#[test]
fn older_isolated_revision_is_recycled_even_with_a_larger_commit() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    inject_parent_sync_failure();
    assert!(fixture.reconcile().is_err());
    let isolated =
        fixture.artifact(crate::paths::test_support::retained_isolation_artifact_prefix());
    let journal_bytes = fs::metadata(&fixture.moved_journal).unwrap().len();
    let sealed_bytes = JOURNAL_HEADER_BYTES as u64;
    assert!(sealed_bytes < journal_bytes);
    let canonical = CursorRecord {
        generation: fixture.attempt.generation,
        committed_bytes: sealed_bytes,
        publication_revision: NEWER_RECOVERY_REVISION,
    };
    let older_isolated = CursorRecord {
        generation: fixture.attempt.generation,
        committed_bytes: journal_bytes,
        publication_revision: EARLIER_RECOVERY_REVISION,
    };
    fs::write(&fixture.moved_cursor, encode_cursor(canonical)).unwrap();
    fs::write(isolated, encode_cursor(older_isolated)).unwrap();

    let epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, epoch).unwrap();
    assert!(
        reopened
            .replay_after(EMPTY_SEQUENCE, usize::MAX)
            .unwrap()
            .is_empty()
    );
    fixture.assert_replacements_unchanged();
}

#[test]
fn a_new_journal_restores_isolated_cursor_after_replacement_removal() {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterRecordSync);
    let previous_cursor = fs::read(&fixture.moved_cursor).unwrap();
    let cursor = fixture.moved_cursor.clone();
    crate::paths::test_support::set_retained_install_hook(move || {
        write_private(&cursor, REPLACEMENT_BYTES);
        Ok(())
    });
    assert!(fixture.reconcile().is_err());
    fixture.assert_retired_cursor(&previous_cursor);
    fs::remove_file(&fixture.moved_cursor).unwrap();

    let runtime_epoch = fixture.journal.runtime_epoch();
    let mut reopened = Journal::open(&fixture.moved_journal, runtime_epoch).unwrap();
    reopened.append(RETAINED_PAYLOAD).unwrap();
    let chunks = reopened.replay_after(EMPTY_SEQUENCE, usize::MAX).unwrap();

    assert_eq!(chunks.len(), INITIAL_RETAINED_SEQUENCE as usize);
    assert_eq!(chunks[0].data, RETAINED_PAYLOAD);
    fixture.assert_artifacts_bounded();
    fixture.assert_replacements_unchanged();
}

fn append_retained(journal: &mut Journal, payload: &[u8]) {
    let attempt = journal.append_attempt();
    let recovered = journal
        .reconcile_append_retained(attempt, payload)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.data, payload);
}
