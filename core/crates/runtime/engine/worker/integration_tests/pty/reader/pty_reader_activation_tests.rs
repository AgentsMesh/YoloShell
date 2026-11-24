use super::*;
use crate::journal::{AppendStage, Journal};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;

const TEST_SHELL: &str = "/bin/sh";
const TEST_COMMAND_ARGUMENT: &str = "-c";
const TEST_OUTPUT_COMMAND: &str = "printf activation-pending-output; IFS= read -r line";
const TEST_OUTPUT: &[u8] = b"activation-pending-output";
const TEST_JOURNAL_FILE: &str = "activation.journal";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_OUTPUT_WAIT_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TEST_OUTPUT_WAIT_TIMEOUT_DETAIL: &str =
    "startup PTY output did not settle before reader activation";

#[test]
fn preparation_keeps_reader_blocked_and_output_exclusive() {
    let (pending, receiver) = PendingPtyReader::for_test();
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let mut output = ExclusivePtyOutput::observed(Arc::clone(&observed));

    let prepared = pending.prepare(&mut output).unwrap();

    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    output.progress().unwrap();
    assert_eq!(observed.load(Ordering::Acquire), 1);
    drop(prepared);
    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
}

#[test]
fn activation_consumes_exclusive_output_before_reader_release() {
    let (pending, receiver) = PendingPtyReader::for_test();
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let output = ExclusivePtyOutput::observed(Arc::clone(&observed));

    let reader_backed = pending.activate(output).ok().unwrap();

    receiver.recv().unwrap();
    reader_backed.recovery().progress().unwrap();
    assert_eq!(observed.load(Ordering::Acquire), 1);
}

#[test]
fn failed_activation_returns_exclusive_output_authority() {
    let pending = PendingPtyReader::disconnected_for_test();
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    let output = ExclusivePtyOutput::observed(Arc::clone(&observed));

    let mut failure = pending.activate(output).err().unwrap();

    assert_eq!(failure.error.to_string(), READER_ACTIVATION_DETAIL);
    failure.output.progress().unwrap();
    assert_eq!(observed.load(Ordering::Acquire), 1);
}

#[test]
fn append_failure_is_recovered_before_reader_activation() {
    let directory = tempfile::tempdir().unwrap();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let journal = Arc::new(std::sync::Mutex::new(
        Journal::open(directory.path().join(TEST_JOURNAL_FILE), Uuid::new_v4()).unwrap(),
    ));
    journal
        .lock()
        .unwrap()
        .inject_append_failure(AppendStage::BeforeWrite);
    let mut output =
        ExclusivePtyOutput::capture(pair.master.as_ref(), Arc::clone(&journal)).unwrap();
    let mut command = CommandBuilder::new(TEST_SHELL);
    command.args([TEST_COMMAND_ARGUMENT, TEST_OUTPUT_COMMAND]);
    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    let deadline = Instant::now() + TEST_OUTPUT_WAIT_TIMEOUT;
    while replay(&journal) != TEST_OUTPUT {
        output.progress().unwrap();
        assert!(
            Instant::now() < deadline,
            "{TEST_OUTPUT_WAIT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(TEST_POLL_INTERVAL);
    }

    let (pending, receiver) = PendingPtyReader::for_test();
    let reader_backed = pending.activate(output).ok().unwrap();

    receiver.recv().unwrap();
    reader_backed.recovery().progress().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(replay(&journal), TEST_OUTPUT);
}

fn replay(journal: &Arc<std::sync::Mutex<Journal>>) -> Vec<u8> {
    journal
        .lock()
        .unwrap()
        .replay_after(u64::default(), TEST_OUTPUT.len())
        .unwrap()
        .into_iter()
        .flat_map(|chunk| chunk.data)
        .collect()
}
