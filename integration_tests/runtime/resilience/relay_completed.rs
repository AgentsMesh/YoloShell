use super::*;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::sync::mpsc;
use yolo_relay::{RelayExit, RelayOptions, RelayStatus, TerminalSize, relay_session_with_status};
use yolo_runtime::host::reconcile_test_hook::ReconcileTestBarrier;
use yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE;

#[path = "relay_completed_support.rs"]
mod support;
use support::{
    assert_get_only_completed_adoption, spawn_completed_reconcile_host,
    wait_for_completed_reconcile_barrier,
};

const COMPLETED_RELAY_COMMAND: &[u8] = b"printf 'completed-relay-output\\n'; exit 29\n";
const COMPLETED_RELAY_SESSION: &str = "completed-relay";
const COMPLETED_RELAY_OUTPUT: &[u8] = b"completed-relay-output";
const COMPLETED_RELAY_EXIT_CODE: i32 = 29;
const COMPLETED_RELAY_HOLDER: &str = "completed-relay-holder";
const COMPLETED_RELAY_INPUT_PANIC: &str = "completed relay read terminal input";
const COMPLETED_RELAY_BUFFER_BYTES: usize = 64 * 1024;
const COMPLETED_RELAY_TIMEOUT: Duration = Duration::from_secs(5);
const COMPLETED_RELAY_COLUMNS: u16 = 113;
const COMPLETED_RELAY_ROWS: u16 = 47;
const COMPLETED_RESIZE_CHANNEL_CAPACITY: usize = 1;
const COMPLETED_RECONCILE_BARRIER_DIRECTORY: &str = "completed-reconcile-barrier";

struct PanicInput;

impl AsyncRead for PanicInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        _buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        panic!("{COMPLETED_RELAY_INPUT_PANIC}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_relay_survives_host_replacement_without_mutating_runtime() {
    let mut harness = Harness::start().await;
    let session = harness.create_cat(COMPLETED_RELAY_SESSION).await;
    let terminal_id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let runtime = session.runtime.as_ref().unwrap();
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    let original_record: RuntimeRecord =
        read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, COMPLETED_RELAY_HOLDER, None)
        .await
        .unwrap();
    harness
        .client()
        .input(
            target,
            lease.generation,
            &lease.writer_capability,
            COMPLETED_RELAY_COMMAND.to_vec(),
        )
        .await
        .unwrap();
    wait_for_exit(&harness.client(), terminal_id).await;
    wait_for_identity_gone(child).await;
    let completed = harness.client().get_session(terminal_id).await.unwrap();
    let completed_runtime = completed.runtime.as_ref().unwrap();
    let exit = completed_runtime.exit.as_ref().unwrap();
    assert_eq!(
        completed.completed_attachment.unwrap().runtime_epoch,
        target.runtime_epoch
    );
    assert_eq!(exit.runtime_epoch, target.runtime_epoch);
    assert_eq!(exit.code, Some(COMPLETED_RELAY_EXIT_CODE));
    assert_eq!(exit.reason, yolo_runtime::model::ExitReason::Natural);
    assert!(worker.matches());
    let barrier_root = harness
        .home
        .path()
        .join(COMPLETED_RECONCILE_BARRIER_DIRECTORY);
    let barrier = ReconcileTestBarrier::new(&barrier_root, terminal_id).unwrap();
    barrier.arm().unwrap();
    harness.kill_host_hard();
    harness
        .replace_host(spawn_completed_reconcile_host(
            harness.home.path(),
            &barrier_root,
        ))
        .await;
    wait_for_completed_reconcile_barrier(&barrier).await;
    assert!(worker.matches());
    assert_get_only_completed_adoption(&harness, terminal_id, &barrier).await;

    let requested_size = TerminalSize::new(COMPLETED_RELAY_COLUMNS, COMPLETED_RELAY_ROWS).unwrap();
    let (output_writer, mut output_reader) = tokio::io::duplex(COMPLETED_RELAY_BUFFER_BYTES);
    let (resize_sender, resize_events) = mpsc::channel(COMPLETED_RESIZE_CHANNEL_CAPACITY);
    resize_sender.send(requested_size).await.unwrap();
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = relay_session_with_status(
        harness.client(),
        RelayOptions::new(terminal_id, COMPLETED_RELAY_HOLDER)
            .unwrap()
            .after_sequence(FORCE_SNAPSHOT_AFTER_SEQUENCE)
            .initial_size(Some(requested_size))
            .take_control(true),
        PanicInput,
        output_writer,
        resize_events,
        status_sender,
    );
    assert_eq!(
        timeout(COMPLETED_RELAY_TIMEOUT, relay)
            .await
            .unwrap()
            .unwrap(),
        RelayExit::SessionExited
    );
    drop(resize_sender);

    let mut output = Vec::new();
    output_reader.read_to_end(&mut output).await.unwrap();
    assert!(
        output
            .windows(COMPLETED_RELAY_OUTPUT.len())
            .any(|window| window == COMPLETED_RELAY_OUTPUT)
    );
    assert_eq!(
        [statuses.try_recv().unwrap(), statuses.try_recv().unwrap()],
        [RelayStatus::SessionEnding, RelayStatus::SessionExited]
    );
    assert!(statuses.try_recv().is_err());

    let retained_record: RuntimeRecord =
        read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
    assert!(retained_record.has_same_process_identity(&original_record));
    assert_eq!(retained_record.cols, original_record.cols);
    assert_eq!(retained_record.rows, original_record.rows);
    assert!(worker.matches());
}
