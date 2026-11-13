use super::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use yolo_relay::{RelayExit, RelayOptions, TerminalSize, relay_session};

const IO_CAPACITY: usize = 64 * 1024;
const MARKER: &[u8] = b"relay-round-trip-ready";
const READ_ONLY_INPUT_CAPACITY: usize = 1;
const READ_ONLY_MARKER: &[u8] = b"relay-read-only-recovered";
const READ_ONLY_COMMAND: &[u8] = b"printf 'relay-read-only-recovered\\n'\n";
const CURRENT_OWNER: &str = "current-owner";
const OTHER_HOLDER: &str = "other-holder";

#[path = "relay_dimensions.rs"]
mod dimensions;
#[path = "relay/timing.rs"]
mod timing;
use timing::{POLL_INTERVAL, RELAY_TIMEOUT};

async fn read_relay_until<R: AsyncRead + Unpin>(reader: &mut R, needle: &[u8]) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + RELAY_TIMEOUT;
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    while !received
        .windows(needle.len())
        .any(|window| window == needle)
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "relay output missing {needle:?}");
        let length = timeout(remaining, reader.read(&mut buffer))
            .await
            .unwrap()
            .unwrap();
        assert_ne!(length, 0, "relay output closed before marker: {received:?}");
        received.extend_from_slice(&buffer[..length]);
    }
    received
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_streams_io_resizes_detaches_and_reports_natural_exit() {
    let harness = Harness::start().await;
    let session = harness.create_cat("relay-lifecycle").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let mut dimensions =
        dimensions::RuntimeDimensions::capture(&harness.paths(), &session).unwrap();
    let initial_size = TerminalSize::new(100, 35).unwrap();
    let resized = TerminalSize::new(101, 36).unwrap();
    let options = RelayOptions::new(id, "relay-owner")
        .unwrap()
        .initial_size(Some(initial_size));
    let (mut input_writer, input_reader) = tokio::io::duplex(IO_CAPACITY);
    let (output_writer, mut output_reader) = tokio::io::duplex(IO_CAPACITY);
    let (resize_sender, resize_events) = mpsc::channel(4);
    let relay = tokio::spawn(relay_session(
        harness.client(),
        options,
        input_reader,
        output_writer,
        resize_events,
    ));
    resize_sender.send(initial_size).await.unwrap();
    resize_sender.send(resized).await.unwrap();
    drop(resize_sender);
    dimensions
        .wait(resized, tokio::time::Instant::now() + RELAY_TIMEOUT)
        .await
        .unwrap();

    input_writer
        .write_all(b"printf 'relay-round-trip-ready\\n'\n")
        .await
        .unwrap();
    let output = read_relay_until(&mut output_reader, MARKER).await;
    assert!(output.windows(MARKER.len()).any(|window| window == MARKER));
    drop(input_writer);
    assert_eq!(
        timeout(RELAY_TIMEOUT, relay)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        RelayExit::InputClosed
    );
    let view = harness.client().get_session(id).await.unwrap();
    assert_eq!(view.runtime.unwrap().status, TerminalRuntimeStatus::Running);
    assert!(
        harness
            .client()
            .get_writer_lease(target)
            .await
            .unwrap()
            .is_none()
    );

    let (mut exit_writer, exit_reader) = tokio::io::duplex(IO_CAPACITY);
    let (exit_output, _exit_reader) = tokio::io::duplex(IO_CAPACITY);
    let (_resize_sender, resize_events) = mpsc::channel(1);
    let relay = tokio::spawn(relay_session(
        harness.client(),
        RelayOptions::new(id, "relay-owner").unwrap(),
        exit_reader,
        exit_output,
        resize_events,
    ));
    exit_writer.write_all(b"exit\n").await.unwrap();
    assert_eq!(
        timeout(RELAY_TIMEOUT, relay)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        RelayExit::SessionExited
    );
    wait_for_exit(&harness.client(), id).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_conflict_is_read_only_without_stopping_the_session() {
    let harness = Harness::start().await;
    let session = harness.create_cat("relay-fencing").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let current = harness
        .client()
        .acquire_writer(target, CURRENT_OWNER, None)
        .await
        .unwrap();
    let (mut input_writer, input_reader) = tokio::io::duplex(READ_ONLY_INPUT_CAPACITY);
    let (output_writer, mut output_reader) = tokio::io::duplex(IO_CAPACITY);
    let (_resize_sender, resize_events) = mpsc::channel(1);
    let mut relay = tokio::spawn(relay_session(
        harness.client(),
        RelayOptions::new(id, OTHER_HOLDER).unwrap(),
        input_reader,
        output_writer,
        resize_events,
    ));
    input_writer.write_all(READ_ONLY_COMMAND).await.unwrap();
    assert!(timeout(POLL_INTERVAL, &mut relay).await.is_err());
    let lease = harness
        .client()
        .get_writer_lease(target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lease.holder_id, CURRENT_OWNER);
    harness
        .client()
        .release_writer(&current, target)
        .await
        .unwrap();
    let output = read_relay_until(&mut output_reader, READ_ONLY_MARKER).await;
    assert!(
        output
            .windows(READ_ONLY_MARKER.len())
            .any(|window| window == READ_ONLY_MARKER)
    );
    input_writer.shutdown().await.unwrap();
    let exit = timeout(RELAY_TIMEOUT, relay)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    assert!(
        harness
            .client()
            .get_writer_lease(target)
            .await
            .unwrap()
            .is_none()
    );
    let view = harness.client().get_session(id).await.unwrap();
    assert_eq!(view.runtime.unwrap().status, TerminalRuntimeStatus::Running);
}
