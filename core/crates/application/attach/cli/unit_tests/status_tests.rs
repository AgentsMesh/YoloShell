use super::*;
use std::os::fd::AsRawFd;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_runtime::local_unix::LocalUnixStream as UnixStream;
use yolo_hub_relay_control_protocol::{decode_status, Status, StatusKind, MAX_HOLDER_BYTES};

const TEST_TIMEOUT: Duration = Duration::from_secs(1);
const WRITE_TEST_TIMEOUT: Duration = Duration::from_secs(3);
const BLOCKING_FRAME_COUNT: usize = 1024;
const SOCKET_BUFFER_BYTES: libc::c_int = 1024;
const HOLDER: &str = "socket-owner";
const BLOCKING_HOLDER_CHARACTER: &str = "x";
const HUMAN_STATUS_BUFFER_BYTES: usize = 64;

#[tokio::test]
async fn socket_reporter_emits_one_typed_binary_message_per_status() {
    let (peer, stream) = UnixStream::pair().unwrap();
    let reporter = StatusReporter::Socket(stream);
    let (sender, events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { reporter.run(events).await });
    sender
        .send(RelayStatus::ReadOnly {
            holder_id: Some(HOLDER.into()),
        })
        .unwrap();
    drop(sender);

    let value = receive(&mut { peer }).await;
    assert_eq!(value.kind, StatusKind::ReadOnly);
    assert_eq!(value.holder_id.as_deref(), Some(HOLDER));
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn socket_reporter_emits_canonical_session_ending_record() {
    let (peer, stream) = UnixStream::pair().unwrap();
    let reporter = StatusReporter::Socket(stream);
    let (sender, events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { reporter.run(events).await });
    sender.send(RelayStatus::SessionEnding).unwrap();
    drop(sender);

    let value = receive(&mut { peer }).await;
    assert_eq!(value.kind, StatusKind::SessionEnding);
    assert_eq!(value.holder_id, None);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn inherited_status_peer_loss_is_fail_closed() {
    let (peer, stream) = UnixStream::pair().unwrap();
    let reporter = StatusReporter::Socket(stream);
    drop(peer);
    let (_sender, events) = mpsc::unbounded_channel();
    let result = tokio::time::timeout(TEST_TIMEOUT, reporter.run(events))
        .await
        .unwrap();
    assert!(result.unwrap_err().to_string().contains(STATUS_PEER_CLOSED));
}

#[tokio::test]
async fn human_reporter_drains_statuses_without_a_socket() {
    let reporter = StatusReporter::from_inherited_fd(None).unwrap();
    let (sender, events) = mpsc::unbounded_channel();
    sender.send(RelayStatus::ControlAcquired).unwrap();
    drop(sender);
    reporter.run(events).await.unwrap();
}

#[tokio::test]
async fn human_reporter_propagates_output_failure_without_panicking() {
    let (output, input) = tokio::io::duplex(HUMAN_STATUS_BUFFER_BYTES);
    drop(input);
    let (sender, mut events) = mpsc::unbounded_channel();
    sender.send(RelayStatus::ControlAcquired).unwrap();
    drop(sender);

    let error = run_human(output, &mut events).await.unwrap_err();

    assert!(error.to_string().contains(HUMAN_STATUS_WRITE_CONTEXT));
}

#[tokio::test]
async fn socket_reporter_rejects_unexpected_peer_input() {
    let (mut peer, stream) = UnixStream::pair().unwrap();
    let reporter = StatusReporter::Socket(stream);
    let (_sender, events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { reporter.run(events).await });

    peer.write_all(&[NDJSON_DELIMITER]).await.unwrap();
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(STATUS_PEER_INPUT));
}

async fn reporter_with_pending_write() -> (UnixStream, tokio::task::JoinHandle<anyhow::Result<()>>)
{
    let (mut peer, stream) = UnixStream::pair().unwrap();
    limit_send_buffer(&stream);
    let reporter = StatusReporter::Socket(stream);
    let (sender, events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { reporter.run(events).await });
    for _ in 0..BLOCKING_FRAME_COUNT { sender
        .send(RelayStatus::ReadOnly {
            holder_id: Some(BLOCKING_HOLDER_CHARACTER.repeat(MAX_HOLDER_BYTES)),
        })
        .unwrap(); }
    let mut first_byte = [0_u8; 1];
    tokio::time::timeout(TEST_TIMEOUT, peer.read_exact(&mut first_byte))
        .await
        .unwrap()
        .unwrap();
    (peer, task)
}

#[tokio::test]
async fn socket_reporter_rejects_peer_input_during_a_pending_write() {
    let (mut peer, task) = reporter_with_pending_write().await;
    peer.write_all(&[NDJSON_DELIMITER]).await.unwrap();
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(STATUS_PEER_INPUT));
}

#[tokio::test]
async fn socket_reporter_rejects_peer_eof_during_a_pending_write() {
    let (mut peer, task) = reporter_with_pending_write().await;
    peer.shutdown().await.unwrap();
    let error = task.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(STATUS_PEER_CLOSED));
}

#[tokio::test]
async fn socket_reporter_preserves_session_ending_before_exit() {
    let (peer, stream) = UnixStream::pair().unwrap();
    let reporter = StatusReporter::Socket(stream);
    let (sender, events) = mpsc::unbounded_channel();
    sender.send(RelayStatus::SessionEnding).unwrap();
    sender.send(RelayStatus::SessionExited).unwrap();
    drop(sender);
    let task = tokio::spawn(async move { reporter.run(events).await });
    let mut peer = peer;
    assert_eq!(receive(&mut peer).await.kind, StatusKind::SessionEnding);
    assert_eq!(receive(&mut peer).await.kind, StatusKind::SessionExited);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn socket_reporter_times_out_when_peer_never_reads() {
    let (_peer, stream) = UnixStream::pair().unwrap();
    limit_send_buffer(&stream);
    let reporter = StatusReporter::Socket(stream);
    let (sender, events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { reporter.run(events).await });
    for _ in 0..BLOCKING_FRAME_COUNT { sender
        .send(RelayStatus::ReadOnly {
            holder_id: Some(BLOCKING_HOLDER_CHARACTER.repeat(MAX_HOLDER_BYTES)),
        })
        .unwrap(); }

    let result = tokio::time::timeout(WRITE_TEST_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains(STATUS_WRITE_TIMEOUT_MESSAGE)
    );
}

fn limit_send_buffer(stream: &UnixStream) {
    let size = SOCKET_BUFFER_BYTES;
    // The pointer is valid for the duration of setsockopt and has the declared byte length.
    let result = unsafe {
        libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_SNDBUF,
            std::ptr::from_ref(&size).cast(),
            std::mem::size_of_val(&size) as libc::socklen_t,
        )
    };
    assert_eq!(result, 0);
}

async fn receive(peer: &mut UnixStream) -> Status {
    let length = peer.read_u32().await.unwrap();
    let mut bytes = vec![0; length as usize];
    peer.read_exact(&mut bytes).await.unwrap();
    decode_status(&bytes).unwrap()
}
