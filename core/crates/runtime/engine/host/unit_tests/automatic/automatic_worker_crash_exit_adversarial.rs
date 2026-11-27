use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_EMPTY_JOURNAL: &[u8] = b"";
#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_EMPTY_JOURNAL_BYTES: u64 = 0;
#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_SEQUENCE_OFFSET: u64 = 1;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_requires_existing_strict_journal_evidence() {
    let missing = AutomaticCrashFixture::new();
    missing.settle_custody();
    missing.remove_journal_evidence();
    missing.write_exit(
        ExitReason::WorkerCrash,
        missing.runtime.runtime_epoch,
        AUTOMATIC_EXIT_TEST_EMPTY_SEQUENCE,
    );
    assert!(
        missing
            .host
            .ensure_automatic_worker_crash_exit_inner(&missing.manifest)
            .await
            .is_err()
    );

    let corrupt = AutomaticCrashFixture::new();
    let sequence = corrupt.append_output();
    corrupt.settle_custody();
    corrupt.write_exit(
        ExitReason::WorkerCrash,
        corrupt.runtime.runtime_epoch,
        sequence,
    );
    let journal = corrupt.host.paths.journal(corrupt.manifest.terminal_id);
    write_private_bytes(&journal, AUTOMATIC_EXIT_TEST_EMPTY_JOURNAL);

    assert!(
        corrupt
            .host
            .ensure_automatic_worker_crash_exit_inner(&corrupt.manifest)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::metadata(journal).unwrap().len(),
        AUTOMATIC_EXIT_TEST_EMPTY_JOURNAL_BYTES
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_requires_existing_commit_cursor_without_repair() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence,
    );
    let journal_path = fixture.host.paths.journal(fixture.manifest.terminal_id);
    let cursor_path = crate::journal::retained_cursor_path_for_test(&journal_path);
    std::fs::remove_file(&cursor_path).unwrap();

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    assert!(!cursor_path.exists());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_must_match_retained_journal_sequence() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence + AUTOMATIC_EXIT_TEST_SEQUENCE_OFFSET,
    );

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
}
