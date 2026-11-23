use super::*;
use crate::protocol::method;
use crate::relay::session_test_support::{
    GENERATION, PendingInput, attached_prefix, closed_resize_channel, empty_attach, options,
    owned_prefix, owned_with_attachment, reconnect_prefix, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, duplex};
use tokio::task::JoinHandle;

const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const BLOCKED_OUTPUT: &[u8] = b"output larger than a one-byte relay sink";
const BLOCKED_OUTPUT_BUFFER_BYTES: usize = 1;
type RelayTask = JoinHandle<Result<RelayExit, RelayError>>;

fn spawn_relay<R, W>(
    host: &FakeHost,
    terminal_id: Uuid,
    input: R,
    output: W,
    cancellation: RelayCancellation,
) -> (RelayTask, tokio::sync::mpsc::UnboundedReceiver<RelayStatus>)
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (status, events) = tokio::sync::mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status_and_cancellation(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        output,
        closed_resize_channel(),
        status,
        cancellation,
    ));
    (relay, events)
}

async fn wait_for_method(host: &FakeHost, expected: &str, count: usize) {
    tokio::time::timeout(TEST_TIMEOUT, async {
        loop {
            let actual = host
                .methods()
                .into_iter()
                .filter(|method| method == expected)
                .count();
            if actual >= count {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn assert_cancelled(task: JoinHandle<Result<RelayExit, RelayError>>) {
    let result = tokio::time::timeout(TEST_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(RelayError::Cancelled)));
}

#[tokio::test]
async fn cancellation_interrupts_pending_prepare() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![Reply::Pending]).await;
    let cancellation = RelayCancellation::default();
    let (relay, _status) = spawn_relay(
        &host,
        terminal_id,
        PendingInput,
        tokio::io::sink(),
        cancellation.clone(),
    );
    wait_for_method(&host, method::HOST_STATUS, 1).await;
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert_eq!(host.methods(), [method::HOST_STATUS]);
}

#[tokio::test]
async fn cancelled_unknown_acquire_is_discovered_and_released() {
    let terminal_id = Uuid::new_v4();
    let acquire_applied = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut replies = attached_prefix(terminal_id, Reply::attachment(empty_attach(), Vec::new()));
    replies.extend([
        Reply::LeaseThenPending(GENERATION, std::sync::Arc::clone(&acquire_applied)),
        Reply::StoredLease,
        Reply::Lease(GENERATION),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let cancellation = RelayCancellation::default();
    let (relay, _status) = spawn_relay(
        &host,
        terminal_id,
        PendingInput,
        tokio::io::sink(),
        cancellation.clone(),
    );
    acquire_applied.notified().await;
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert_eq!(host.methods().last().unwrap(), method::RELEASE_WRITER);
}

#[tokio::test]
async fn cancellation_interrupts_attach_retry_before_writer_acquisition() {
    let terminal_id = Uuid::new_v4();
    let mut replies = reconnect_prefix(terminal_id);
    replies.extend([Reply::Disconnect, Reply::Pending]);
    let host = FakeHost::start(replies).await;
    let cancellation = RelayCancellation::default();
    let (relay, _status) = spawn_relay(
        &host,
        terminal_id,
        PendingInput,
        tokio::io::sink(),
        cancellation.clone(),
    );
    wait_for_method(&host, method::HOST_STATUS, 2).await;
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert!(!host.methods().contains(&method::ACQUIRE_WRITER.into()));
    assert!(!host.methods().contains(&method::RELEASE_WRITER.into()));
}

#[tokio::test]
async fn cancellation_interrupts_blocked_output_and_releases() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_with_attachment(
        terminal_id,
        Reply::attachment(snapshot_attach(BLOCKED_OUTPUT), Vec::new()),
    );
    replies.push(Reply::value(&()));
    let host = FakeHost::start(replies).await;
    let (output, mut blocked_reader) = duplex(BLOCKED_OUTPUT_BUFFER_BYTES);
    let cancellation = RelayCancellation::default();
    let (relay, _status) = spawn_relay(
        &host,
        terminal_id,
        PendingInput,
        output,
        cancellation.clone(),
    );
    blocked_reader.read_u8().await.unwrap();
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert_eq!(host.methods().last().unwrap(), method::RELEASE_WRITER);
}

#[tokio::test]
async fn stalled_release_is_bounded_after_cancellation() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.push(Reply::Pending);
    let host = FakeHost::start(replies).await;
    let cancellation = RelayCancellation::default();
    let (relay, mut status) = spawn_relay(
        &host,
        terminal_id,
        PendingInput,
        tokio::io::sink(),
        cancellation.clone(),
    );
    tokio::time::timeout(TEST_TIMEOUT, status.recv())
        .await
        .unwrap()
        .expect("relay status channel closed");
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert_eq!(host.methods().last().unwrap(), method::RELEASE_WRITER);
}

#[tokio::test]
async fn cancellation_interrupts_blocked_control_mutation_and_releases() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::Pending, Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    let (mut input_sender, input) = duplex(1);
    input_sender.write_all(b"x").await.unwrap();
    let cancellation = RelayCancellation::default();
    let (relay, _status) = spawn_relay(
        &host,
        terminal_id,
        input,
        tokio::io::sink(),
        cancellation.clone(),
    );
    wait_for_method(&host, method::TERMINAL_INPUT, 1).await;
    cancellation.cancel();
    assert_cancelled(relay).await;
    assert_eq!(host.methods().last().unwrap(), method::RELEASE_WRITER);
}
