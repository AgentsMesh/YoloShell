use super::super::protocol::CodexHookAck;
use super::super::runtime::CodexHookRuntime;
use crate::agent::hooks::{ApplyDisposition, HookEnvelope};
use crate::local_unix::LocalUnixStream;
use serde::Serialize;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ACK_BACKPRESSURE_CHUNK_BYTES: usize = 16 * 1_024;
const SOCKET_SEND_FAILURE: libc::ssize_t = -1;

pub(super) async fn exchange(
    runtime: Arc<CodexHookRuntime>,
    event: &HookEnvelope,
) -> (CodexHookAck, Option<ApplyDisposition>) {
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &event.identity)
            .unwrap();
    let durable = outbox.enqueue(event.clone()).unwrap();
    assert_eq!(&durable, event);
    let result = exchange_uncommitted(runtime, event).await;
    if result.0.accepted {
        outbox
            .acknowledge(event.sequence.0, event.event_id)
            .unwrap();
    }
    result
}

pub(super) async fn exchange_uncommitted(
    runtime: Arc<CodexHookRuntime>,
    event: &HookEnvelope,
) -> (CodexHookAck, Option<ApplyDisposition>) {
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let task = tokio::spawn(super::super::handler::handle(server, runtime));
    write_json_frame(&mut client, event).await;
    let ack = read_ack(&mut client).await;
    let outcome = task.await.unwrap().unwrap();
    let (disposition, response) = outcome.into_parts();
    response.unwrap();
    (ack, disposition)
}

async fn write_json_frame<T: Serialize>(stream: &mut LocalUnixStream, value: &T) {
    let payload = yolo_capnp_codec::encode(value).unwrap();
    write_bytes_frame(stream, &payload).await;
}

pub(super) async fn write_bytes_frame(stream: &mut LocalUnixStream, payload: &[u8]) {
    let length = u32::try_from(payload.len()).unwrap();
    stream.write_all(&length.to_be_bytes()).await.unwrap();
    stream.write_all(payload).await.unwrap();
}

pub(super) async fn read_ack(stream: &mut LocalUnixStream) -> CodexHookAck {
    try_read_ack(stream).await.unwrap()
}

pub(super) async fn try_read_ack(stream: &mut LocalUnixStream) -> anyhow::Result<CodexHookAck> {
    let mut header = [0_u8; super::super::constants::CODEX_FRAME_HEADER_BYTES];
    stream.read_exact(&mut header).await?;
    let length = u32::from_be_bytes(header) as usize;
    let mut payload = vec![0_u8; length];
    stream.read_exact(&mut payload).await?;
    Ok(yolo_capnp_codec::decode(&payload)?)
}

pub(super) fn saturate_ack_channel(stream: &LocalUnixStream) {
    let chunk = [u8::default(); ACK_BACKPRESSURE_CHUNK_BYTES];
    loop {
        let sent = unsafe {
            libc::send(
                stream.as_raw_fd(),
                chunk.as_ptr().cast(),
                chunk.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if sent == SOCKET_SEND_FAILURE {
            let error = std::io::Error::last_os_error();
            assert!(
                error.raw_os_error() == Some(libc::EAGAIN)
                    || error.raw_os_error() == Some(libc::EWOULDBLOCK)
            );
            return;
        }
        assert!(sent > libc::ssize_t::default());
    }
}
