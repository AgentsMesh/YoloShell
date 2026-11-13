use super::*;
use crate::journal::Journal;
use crate::worker::pty_missing_eof::{MissingEofDeadline, MissingEofOutput, OutputAbandonment};
use portable_pty::{PtySize, native_pty_system};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};
use uuid::Uuid;

const TEST_TERMINAL_ROWS: u16 = 24;
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_JOURNAL_FILE: &str = "missing-eof-authority.journal";

#[test]
fn raw_kernel_eof_authority_blocks_output_abandonment() {
    let mut output = quiescent_output();
    output.kernel_eof = Some(KernelPtyEofIdentity::from_kernel_read());

    assert!(matches!(
        output.settle_missing_eof(expired()).unwrap(),
        OutputAbandonment::Retry
    ));
    assert!(!output.output_abandoned);
}

#[test]
fn committed_kernel_eof_marker_blocks_output_abandonment() {
    let mut output = quiescent_output();
    output.kernel_eof_committed = Some(KernelPtyEofIdentity::from_kernel_read());

    assert!(matches!(
        output.settle_missing_eof(expired()).unwrap(),
        OutputAbandonment::Retry
    ));
    assert!(!output.output_abandoned);
}

#[test]
fn abandoned_native_output_rejects_residual_kernel_eof_authority() {
    let directory = tempfile::tempdir().unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_TERMINAL_ROWS,
            cols: TEST_TERMINAL_COLUMNS,
            pixel_width: u16::default(),
            pixel_height: u16::default(),
        })
        .unwrap();
    let journal = Arc::new(std::sync::Mutex::new(
        Journal::open(directory.path().join(TEST_JOURNAL_FILE), Uuid::new_v4()).unwrap(),
    ));
    let mut output = ExclusivePtyOutput::capture(pair.master.as_ref(), journal).unwrap();
    let version = output.bind_kernel_eof_for_test(pair.master.as_ref());
    output.output_abandoned = true;
    output.kernel_eof = Some(KernelPtyEofIdentity::from_kernel_read());

    assert!(
        output
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );
}

fn quiescent_output() -> ExclusivePtyOutput {
    ExclusivePtyOutput::quiescent_for_test(Arc::new(AtomicUsize::default()))
}

fn expired() -> crate::worker::pty_missing_eof::MissingEofExpired {
    MissingEofDeadline::with_timeout(Duration::ZERO)
        .observe(OutputProgressState::Quiescent, Instant::now())
        .unwrap()
        .unwrap()
}
