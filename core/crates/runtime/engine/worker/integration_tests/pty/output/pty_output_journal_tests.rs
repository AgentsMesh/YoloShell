use super::*;
use crate::journal::AppendStage;
use std::io::{self, Read, Write};
use uuid::Uuid;

const TEST_JOURNAL_FILE: &str = "exclusive-output.journal";
const TEST_INPUT_FILE: &str = "exclusive-output.input";
const TEST_PAYLOAD_BYTE: u8 = b'x';
const TEST_TAIL_BYTES: usize = 1;
const TEST_EXTRA_RECOVERY_FAILURE: usize = 1;
const TEST_OUTPUT_SEQUENCE: u64 = 1;
const TEST_RETAINED_INSTALL_FAILURE: &str = "injected startup output retained install failure";
const TEST_NATIVE_EOF_ERROR: i32 = libc::EIO;
const TEST_GENERIC_READ_ERROR: i32 = libc::EBADF;

#[test]
fn read_budget_reports_pending_until_a_later_call_reaches_eof() {
    let directory = tempfile::tempdir().unwrap();
    let payload_bytes = OUTPUT_READ_BUFFER_BYTES
        .saturating_mul(OUTPUT_PROGRESS_CHUNK_BUDGET)
        .saturating_add(TEST_TAIL_BYTES);
    let payload = vec![TEST_PAYLOAD_BYTE; payload_bytes];
    let (journal, mut output, input) = fixture(&directory, &payload);

    assert_eq!(
        output.progress(&input).unwrap(),
        super::super::OutputProgressState::Pending
    );
    assert_eq!(
        output.progress(&input).unwrap(),
        super::super::OutputProgressState::Complete
    );
    assert_eq!(replay(&journal, payload_bytes), payload);
}

#[test]
fn append_attempt_and_bytes_survive_reconcile_failure_without_duplication() {
    let directory = tempfile::tempdir().unwrap();
    let payload = vec![TEST_PAYLOAD_BYTE; OUTPUT_READ_BUFFER_BYTES];
    let (journal, mut output, input) = fixture(&directory, &payload);
    {
        let mut journal = journal.lock().unwrap();
        journal.inject_append_failure(AppendStage::AfterRecordSync);
        journal.inject_reconcile_failures(
            crate::worker::JOURNAL_APPEND_RECOVERY_ATTEMPTS
                .saturating_add(TEST_EXTRA_RECOVERY_FAILURE),
        );
    }
    crate::paths::test_support::set_retained_install_hook(|| {
        Err(std::io::Error::other(TEST_RETAINED_INSTALL_FAILURE).into())
    });

    assert!(output.progress(&input).is_err());
    assert_eq!(
        output.progress(&input).unwrap(),
        super::super::OutputProgressState::Complete
    );
    assert_eq!(replay(&journal, payload.len()), payload);
    assert_eq!(
        journal.lock().unwrap().last_sequence(),
        TEST_OUTPUT_SEQUENCE
    );
}

#[test]
fn native_eof_and_generic_read_failures_are_distinguished() {
    let directory = tempfile::tempdir().unwrap();
    let (_, mut eof_output, _) = fixture(&directory, &[]);
    let mut eof = ErrorReader(TEST_NATIVE_EOF_ERROR);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    assert_eq!(
        eof_output.progress_with(&mut eof).unwrap(),
        super::super::OutputProgressState::Complete
    );
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    assert!(eof_output.progress_with(&mut eof).is_err());

    let (_, mut failed_output, _) = fixture(&directory, &[]);
    let mut failed = ErrorReader(TEST_GENERIC_READ_ERROR);
    let error = failed_output.progress_with(&mut failed).unwrap_err();
    assert!(error.to_string().contains(OUTPUT_READ_CONTEXT));
}

struct ErrorReader(i32);

impl Read for ErrorReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from_raw_os_error(self.0))
    }
}

fn fixture(
    directory: &tempfile::TempDir,
    payload: &[u8],
) -> (SharedJournal, ExclusiveJournalOutput, File) {
    let journal = Arc::new(Mutex::new(
        Journal::open(directory.path().join(TEST_JOURNAL_FILE), Uuid::new_v4()).unwrap(),
    ));
    let input_path = directory.path().join(TEST_INPUT_FILE);
    let mut input = File::create(&input_path).unwrap();
    input.write_all(payload).unwrap();
    drop(input);
    let input = File::open(input_path).unwrap();
    (
        Arc::clone(&journal),
        ExclusiveJournalOutput::new(journal),
        input,
    )
}

fn replay(journal: &SharedJournal, byte_limit: usize) -> Vec<u8> {
    journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), byte_limit)
        .unwrap()
        .into_iter()
        .flat_map(|chunk| chunk.data)
        .collect()
}
