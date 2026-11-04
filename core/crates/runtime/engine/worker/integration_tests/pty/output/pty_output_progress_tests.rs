use super::*;
use crate::journal::Journal;
use crate::process::ProcessClosureVersion;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;

const TEST_TERMINAL_ROWS: u16 = 24;
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_SHELL: &str = "/bin/sh";
const TEST_COMMAND_ARGUMENT: &str = "-c";
const TEST_OUTPUT: &[u8] = b"reader-backed-output";
const TEST_OUTPUT_COMMAND: &str = "printf reader-backed-output";
const READ_FAILURE: libc::ssize_t = -1;
const TEST_OUTPUT_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_OUTPUT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TEST_JOURNAL_FILE: &str = "output-progress.journal";

#[test]
fn observed_reader_recovery_reports_output_progress() {
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let output = ReaderBackedPtyOutput::observed(Arc::clone(&observed));
    let mut recovery = output.recovery();

    recovery.progress().unwrap();

    assert_eq!(observed.load(Ordering::Acquire), 1);
}

#[test]
fn released_master_recovery_is_a_bounded_noop() {
    let output = ReaderBackedPtyOutput::after_master_release();
    let mut recovery = output.recovery();

    recovery.progress().unwrap();
}

#[test]
fn released_output_without_bound_authority_defers_tracked_settlement() {
    let output = ReaderBackedPtyOutput::after_master_release();

    assert!(!output.supports_tracked_settlement());
}

#[test]
fn exclusive_recovery_reports_an_empty_pty_drained() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path())).unwrap();

    assert_eq!(output.progress().unwrap(), OutputProgressState::Quiescent);
}

#[test]
fn reader_backed_recovery_never_consumes_or_flushes_output() {
    let directory = tempfile::tempdir().unwrap();
    let pair = open_test_pty();
    let output = ExclusivePtyOutput::capture(pair.master.as_ref(), test_journal(directory.path()))
        .unwrap()
        .into_reader_backed_for_test();
    let mut command = CommandBuilder::new(TEST_SHELL);
    command.args([TEST_COMMAND_ARGUMENT, TEST_OUTPUT_COMMAND]);
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);

    output.recovery().progress().unwrap();

    let received = read_test_output(pair.master.as_ref());
    assert_eq!(received, TEST_OUTPUT);
    child.wait().unwrap();
}

#[test]
fn released_and_observed_completion_cannot_issue_kernel_eof_authority() {
    let version = ProcessClosureVersion::default();
    let mut released = ReaderBackedPtyOutput::after_master_release().recovery();
    assert_eq!(released.progress().unwrap(), OutputProgressState::Complete);
    assert!(
        released
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );

    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let lifecycle = Arc::new(ReaderLifecycle::default());
    lifecycle.mark_eof_observed();
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(observed, lifecycle);
    let mut recovery = output.recovery();
    assert_eq!(recovery.progress().unwrap(), OutputProgressState::Complete);
    assert!(
        recovery
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );
}

fn test_journal(directory: &std::path::Path) -> SharedJournal {
    Arc::new(std::sync::Mutex::new(
        Journal::open(directory.join(TEST_JOURNAL_FILE), Uuid::new_v4()).unwrap(),
    ))
}

fn open_test_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_TERMINAL_ROWS,
            cols: TEST_TERMINAL_COLUMNS,
            pixel_width: u16::default(),
            pixel_height: u16::default(),
        })
        .unwrap()
}

fn read_test_output(master: &dyn MasterPty) -> [u8; TEST_OUTPUT.len()] {
    let deadline = Instant::now() + TEST_OUTPUT_WAIT_TIMEOUT;
    let mut received = [u8::default(); TEST_OUTPUT.len()];
    let mut offset = 0_usize;
    while offset < received.len() {
        let read = unsafe {
            libc::read(
                master.as_raw_fd().unwrap(),
                received[offset..].as_mut_ptr().cast(),
                received.len() - offset,
            )
        };
        if read != READ_FAILURE {
            offset += usize::try_from(read).unwrap();
            continue;
        }
        let error = std::io::Error::last_os_error();
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_OUTPUT_POLL_INTERVAL);
    }
    received
}

#[cfg(target_os = "macos")]
#[path = "pty_output_ctrl_s_tests.rs"]
mod ctrl_s;
#[cfg(target_os = "macos")]
#[path = "pty_output_eof_macos_tests.rs"]
mod eof_macos;
#[path = "pty_output_exclusive_eof_recovery_tests.rs"]
mod exclusive_eof_recovery;
#[cfg(target_os = "macos")]
#[path = "pty_output_reader_eof_macos_tests.rs"]
mod reader_eof_macos;
#[cfg(target_os = "macos")]
#[path = "pty_output_reader_probe_macos_tests.rs"]
mod reader_probe_macos;
#[cfg(target_os = "macos")]
#[path = "pty_output_reader_silent_reopen_macos_tests.rs"]
mod reader_silent_reopen_macos;
#[cfg(target_os = "macos")]
#[path = "pty_output_seal_invalidation_macos_tests.rs"]
mod seal_invalidation_macos;
#[cfg(target_os = "macos")]
#[path = "pty_output_sealed_terminal_invalidation_macos_tests.rs"]
mod sealed_terminal_invalidation_macos;
#[cfg(target_os = "macos")]
#[path = "../../../test_support/integration/pty/pty_output_sealed_terminal_invalidation_support.rs"]
mod sealed_terminal_invalidation_support;
