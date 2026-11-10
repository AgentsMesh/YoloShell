use super::tests_retained_hooks::set_parent_sync_hook;
use super::tests_retained_support::{RETAINED_PAYLOAD, RetainedFixture};
use super::tests_security_support::{private_root, write_private};
use super::*;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

const DECOY_FILE: &str = "retained-descriptor-decoy";
const JOURNAL_FILE: &str = "reconcile-branch-journal";
const ALTERNATE_PAYLOAD: &[u8] = b"retained outpux";
const EXTRA_TAIL: &[u8] = b"x";
const LAST_BYTE_FROM_END: i64 = -1;
const REPLACEMENT_TAIL_BYTE: u8 = u8::MAX;
const ONE_BYTE: u64 = 1;

#[test]
fn retained_attempt_metadata_and_bounds_fail_closed() {
    let (_root, mut journal) = fresh_journal();
    let mut attempt = journal.append_attempt();
    attempt.generation = Uuid::new_v4();
    assert_invalid(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    let mut attempt = journal.append_attempt();
    attempt.runtime_epoch = Uuid::new_v4();
    assert_invalid(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    let mut attempt = journal.append_attempt();
    attempt.previous_sequence = INITIAL_RETAINED_SEQUENCE;
    assert_invalid(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    journal.last_sequence = u64::MAX;
    let attempt = journal.append_attempt();
    assert!(matches!(
        journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD),
        Err(JournalError::SequenceExhausted)
    ));

    let (_root, mut journal) = fresh_journal();
    journal.disk_bytes = u64::MAX;
    journal.max_disk_bytes = u64::MAX;
    let attempt = journal.append_attempt();
    assert_invalid(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    journal.disk_bytes = u64::MAX;
    journal.max_disk_bytes = u64::MAX;
    let mut attempt = journal.append_attempt();
    attempt.cursor.record.committed_bytes = journal.disk_bytes;
    assert_disk_limit(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    journal.max_disk_bytes = journal.disk_bytes;
    let attempt = journal.append_attempt();
    assert_disk_limit(journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD));
}

#[test]
fn retained_cursor_and_prefix_inconsistencies_are_rejected() {
    let (_root, mut journal) = fresh_journal();
    let base = journal.disk_bytes;
    journal.commit_cursor.publish(base + ONE_BYTE).unwrap();
    let attempt = journal.append_attempt();
    assert!(
        journal
            .reconcile_append_retained(attempt, RETAINED_PAYLOAD)
            .is_err()
    );

    let (_root, mut journal) = fresh_journal();
    let advanced = journal.disk_bytes + ONE_BYTE;
    journal.commit_cursor.publish(advanced).unwrap();
    journal.disk_bytes = advanced;
    let attempt = journal.append_attempt();
    assert!(
        journal
            .reconcile_append_retained(attempt, RETAINED_PAYLOAD)
            .is_err()
    );

    let (_root, mut journal) = fresh_journal();
    journal.last_sequence = INITIAL_RETAINED_SEQUENCE;
    let attempt = journal.append_attempt();
    assert!(matches!(
        journal.reconcile_append_retained(attempt, RETAINED_PAYLOAD),
        Err(JournalError::Corrupt {
            sequence: EMPTY_SEQUENCE
        })
    ));
}

#[test]
fn retained_descriptor_and_committed_tail_changes_are_fenced() {
    let (root, mut journal) = fresh_journal();
    let decoy = root.path().join(DECOY_FILE);
    write_private(&decoy, &journal.header.encode());
    journal.file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(decoy)
        .unwrap();
    let attempt = journal.append_attempt();
    assert!(
        journal
            .reconcile_append_retained(attempt, RETAINED_PAYLOAD)
            .is_err()
    );

    assert_committed_tail_mutation(|file| {
        file.seek(SeekFrom::End(0))?;
        file.write_all(EXTRA_TAIL)
    });
    assert_committed_tail_mutation(|file| {
        file.seek(SeekFrom::End(LAST_BYTE_FROM_END))?;
        file.write_all(&[REPLACEMENT_TAIL_BYTE])
    });
}

#[test]
fn ordinary_reconcile_rejects_divergent_replay_state() {
    let (_root, mut journal) = fresh_journal();
    let mut attempt = journal.append_attempt();
    attempt.generation = Uuid::new_v4();
    assert_invalid(journal.reconciled_append(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    let mut attempt = journal.append_attempt();
    attempt.runtime_epoch = Uuid::new_v4();
    assert_invalid(journal.reconciled_append(attempt, RETAINED_PAYLOAD));

    let (_root, mut journal) = fresh_journal();
    let attempt = journal.append_attempt();
    journal.last_sequence = INITIAL_RETAINED_SEQUENCE + INITIAL_RETAINED_SEQUENCE;
    assert!(
        journal
            .reconciled_append(attempt, RETAINED_PAYLOAD)
            .is_err()
    );

    let (_root, mut journal) = fresh_journal();
    let attempt = journal.append_attempt();
    journal.last_sequence = INITIAL_RETAINED_SEQUENCE;
    assert!(
        journal
            .reconciled_append(attempt, RETAINED_PAYLOAD)
            .is_err()
    );

    assert_replayed_chunk_rejected(
        |journal| {
            journal.records.front_mut().unwrap().sequence += INITIAL_RETAINED_SEQUENCE;
        },
        RETAINED_PAYLOAD,
    );
    assert_replayed_chunk_rejected(|_| {}, ALTERNATE_PAYLOAD);
}

fn fresh_journal() -> (tempfile::TempDir, Journal) {
    let root = private_root();
    let journal = Journal::open(root.path().join(JOURNAL_FILE), Uuid::new_v4()).unwrap();
    (root, journal)
}

fn assert_committed_tail_mutation(
    mutation: impl FnOnce(&mut std::fs::File) -> std::io::Result<()> + 'static,
) {
    let mut fixture = RetainedFixture::fail_at(AppendStage::AfterCursorCommit);
    let path = fixture.moved_journal.clone();
    set_parent_sync_hook(move || {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        mutation(&mut file)?;
        file.sync_all()?;
        Ok(())
    });
    assert!(fixture.reconcile().is_err());
    fixture.assert_replacements_unchanged();
}

fn assert_replayed_chunk_rejected(mutation: impl FnOnce(&mut Journal), payload: &[u8]) {
    let (_root, mut journal) = fresh_journal();
    let attempt = journal.append_attempt();
    journal.append(RETAINED_PAYLOAD).unwrap();
    mutation(&mut journal);
    assert!(journal.reconciled_append(attempt, payload).is_err());
}

fn assert_invalid(result: Result<Option<OutputChunk>, JournalError>) {
    assert!(matches!(result, Err(JournalError::InvalidFormat)));
}

fn assert_disk_limit(result: Result<Option<OutputChunk>, JournalError>) {
    assert!(matches!(result, Err(JournalError::DiskLimitExceeded)));
}
