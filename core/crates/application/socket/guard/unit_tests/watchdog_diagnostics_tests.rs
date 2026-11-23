use super::*;
use std::io::{Cursor, Read};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const DIAGNOSTIC_BYTE: u8 = b'x';
const DIAGNOSTIC_OVERFLOW_BYTES: usize = CONTROL_READ_BUFFER_BYTES;
const LINE_TERMINATED_DETAIL: &[u8] = b"watchdog diagnostic\r\n";
const REJECTED_READ_DETAIL: &str = "injected diagnostic read failure";
const REPEATED_COLLECTION_ATTEMPTS: usize = 2;

#[test]
fn bounded_reader_drains_input_and_marks_truncation() {
    let input =
        vec![DIAGNOSTIC_BYTE; MAXIMUM_WATCHDOG_DIAGNOSTIC_BYTES + DIAGNOSTIC_OVERFLOW_BYTES];
    let captured = read_bounded(Cursor::new(input)).unwrap();
    assert_eq!(captured.bytes.len(), MAXIMUM_WATCHDOG_DIAGNOSTIC_BYTES);
    assert!(captured.truncated);
    assert!(render(captured).ends_with(WATCHDOG_OUTPUT_TRUNCATED_SUFFIX));
}

#[test]
fn renderer_removes_only_trailing_line_terminators() {
    let captured = CapturedDiagnostic {
        bytes: LINE_TERMINATED_DETAIL.to_vec(),
        truncated: false,
    };
    assert_eq!(render(captured), "watchdog diagnostic");
}

#[test]
fn reader_failure_and_thread_panic_remain_diagnostic() {
    assert!(read_bounded(RejectReads).is_err());
    let reader = std::thread::spawn(|| -> std::io::Result<CapturedDiagnostic> {
        panic!("{WATCHDOG_DIAGNOSTIC_THREAD_DETAIL}")
    });
    let mut diagnostics = WatchdogDiagnostics {
        reader: Some(reader),
        cancelled: Arc::new(AtomicBool::new(false)),
        collected: None,
    };
    for _attempt in 0..REPEATED_COLLECTION_ATTEMPTS {
        let detail = format!("{:#}", diagnostics.collect().unwrap_err());
        assert!(detail.contains(WATCHDOG_DIAGNOSTIC_THREAD_DETAIL));
    }
}

struct RejectReads;

impl Read for RejectReads {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other(REJECTED_READ_DETAIL))
    }
}
