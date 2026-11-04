use crate::worker::tests::*;
const JOURNAL_MATRIX_OUTPUT: &[u8] = b"journal-matrix";
const JOURNAL_MATRIX_DIRECT_OUTPUT: &[u8] = b"direct-journal-matrix";
const JOURNAL_MATRIX_BARRIER_SEQUENCE: u64 = 0;
const JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH: usize = 0;
const JOURNAL_MATRIX_CAPACITY_INCREMENT: usize = 1;
const JOURNAL_MATRIX_SMALL_LIMIT: u64 = 512;
const JOURNAL_MATRIX_FIRST_FILE: &str = "first.journal";
const JOURNAL_MATRIX_SECOND_FILE: &str = "second.journal";

#[test]
fn reconcile_journal_covers_healthy_recovered_and_failed_states() {
    let fixture = Fixture::new();
    let mut journal = fixture.state.journal.lock().unwrap();
    reconcile_journal(&mut journal).unwrap();

    make_journal_uncertain(&mut journal);
    reconcile_journal(&mut journal).unwrap();
    assert!(!journal.requires_reconcile());

    make_journal_uncertain(&mut journal);
    journal.inject_reconcile_failure();
    assert!(reconcile_journal(&mut journal).is_err());
    assert!(journal.requires_reconcile());
}

#[test]
fn compact_for_output_recovers_uncertainty_and_propagates_reconcile_failure() {
    let fixture = Fixture::new();
    let mut journal = fixture.state.journal.lock().unwrap();
    compact_journal_for_output(
        &mut journal,
        JOURNAL_MATRIX_BARRIER_SEQUENCE,
        JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH,
    )
    .unwrap();

    make_journal_uncertain(&mut journal);
    compact_journal_for_output(
        &mut journal,
        JOURNAL_MATRIX_BARRIER_SEQUENCE,
        JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH,
    )
    .unwrap();

    make_journal_uncertain(&mut journal);
    journal.inject_reconcile_failure();
    assert!(
        compact_journal_for_output(
            &mut journal,
            JOURNAL_MATRIX_BARRIER_SEQUENCE,
            JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH,
        )
        .is_err()
    );
}

#[test]
fn compact_for_output_bounds_retryable_path_loss() {
    let fixture = Fixture::new();
    let journal_path = fixture.state.paths.journal(fixture.terminal_id());
    let mut journal = fixture.state.journal.lock().unwrap();
    std::fs::remove_file(journal_path).unwrap();
    assert!(
        compact_journal_for_output(
            &mut journal,
            JOURNAL_MATRIX_BARRIER_SEQUENCE,
            JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH,
        )
        .is_err()
    );
}

#[test]
fn direct_append_recovers_without_preallocated_attempt_custody() {
    let fixture = Fixture::new();
    let mut journal = fixture.state.journal.lock().unwrap();
    journal.inject_append_failure(crate::journal::AppendStage::AfterRecordSync);
    let mut pending_attempt = None;

    let chunk = append_journal_output(
        &mut journal,
        JOURNAL_MATRIX_DIRECT_OUTPUT,
        &mut pending_attempt,
    )
    .unwrap();

    assert_eq!(chunk.data, JOURNAL_MATRIX_DIRECT_OUTPUT);
    assert!(pending_attempt.is_none());
    assert!(!journal.requires_reconcile());
}

fn make_journal_uncertain(journal: &mut Journal) {
    journal.inject_append_failure(crate::journal::AppendStage::AfterRecordSync);
    assert!(journal.append(JOURNAL_MATRIX_OUTPUT).is_err());
    assert!(journal.requires_reconcile());
}

#[test]
fn append_rejects_non_retryable_disk_limit_without_consuming_attempt_custody() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = Journal::open_with_max_bytes(
        root.path().join(JOURNAL_MATRIX_FIRST_FILE),
        Uuid::new_v4(),
        JOURNAL_MATRIX_SMALL_LIMIT,
    )
    .unwrap();
    let payload = vec![
        u8::default();
        journal.remaining_payload_capacity() + JOURNAL_MATRIX_CAPACITY_INCREMENT
    ];
    let mut pending_attempt = None;

    let error = append_journal_output(&mut journal, &payload, &mut pending_attempt).unwrap_err();

    assert!(matches!(
        error.downcast_ref::<JournalError>(),
        Some(JournalError::DiskLimitExceeded)
    ));
    assert!(pending_attempt.is_some());
    assert!(!journal.requires_reconcile());
}

#[test]
fn pending_reconcile_propagates_a_non_retryable_attempt_identity_mismatch() {
    let root = tempfile::tempdir().unwrap();
    let first = Journal::open(root.path().join(JOURNAL_MATRIX_FIRST_FILE), Uuid::new_v4()).unwrap();
    let attempt = first.append_attempt();
    let mut second =
        Journal::open(root.path().join(JOURNAL_MATRIX_SECOND_FILE), Uuid::new_v4()).unwrap();
    let mut recoveries = usize::default();

    let error =
        reconcile_pending_append(&mut second, attempt, JOURNAL_MATRIX_OUTPUT, &mut recoveries)
            .unwrap_err();

    assert!(matches!(error, JournalError::InvalidFormat));
    assert_eq!(recoveries, usize::default());
}

#[test]
fn uncertain_compaction_retries_when_reconcile_cannot_restore_payload_capacity() {
    let root = tempfile::tempdir().unwrap();
    let mut journal = Journal::open_with_max_bytes(
        root.path().join(JOURNAL_MATRIX_FIRST_FILE),
        Uuid::new_v4(),
        JOURNAL_MATRIX_SMALL_LIMIT,
    )
    .unwrap();
    make_journal_uncertain(&mut journal);
    let payload_len = journal.remaining_payload_capacity() + JOURNAL_MATRIX_CAPACITY_INCREMENT;

    compact_journal_for_output(&mut journal, JOURNAL_MATRIX_BARRIER_SEQUENCE, payload_len).unwrap();

    assert!(!journal.requires_reconcile());
    assert!(journal.would_exceed_limit(payload_len));
}

#[test]
fn compaction_propagates_a_non_retryable_disk_limit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(JOURNAL_MATRIX_FIRST_FILE);
    let epoch = Uuid::new_v4();
    drop(Journal::open(&path, epoch).unwrap());
    let header_only_limit = std::fs::metadata(&path).unwrap().len();
    let mut journal = Journal::open_with_max_bytes(&path, epoch, header_only_limit).unwrap();

    let error = compact_journal_for_output(
        &mut journal,
        JOURNAL_MATRIX_BARRIER_SEQUENCE,
        JOURNAL_MATRIX_EMPTY_PAYLOAD_LENGTH,
    )
    .unwrap_err();

    assert!(matches!(
        error.downcast_ref::<JournalError>(),
        Some(JournalError::DiskLimitExceeded)
    ));
    assert!(!journal.requires_reconcile());
}
