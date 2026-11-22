use super::*;
use crate::protocol::method;
use crate::relay::session_test_support::{
    GENERATION, HOLDER, OTHER_HOLDER, closed_resize_channel, other_lease, owned_prefix,
    read_only_prefix,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::io::Cursor;
use std::time::Duration;
use tokio::io::duplex;

const IO_BUFFER_BYTES: usize = 64;
const TEST_TIMEOUT: Duration = Duration::from_secs(1);
const INPUT: &[u8] = b"terminal input";
const RETRIED_INPUT_REQUEST_COUNT: usize = 2;
const RECOVERED_GENERATION: u64 = GENERATION + 2;

fn options(terminal_id: Uuid) -> RelayOptions {
    RelayOptions::new(terminal_id, HOLDER).unwrap()
}

#[tokio::test]
async fn writer_eof_applies_initial_size_and_releases_ownership() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::value(&()), Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id).initial_size(TerminalSize::new(100, 40)),
        tokio::io::empty(),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
            method::ACQUIRE_WRITER,
            method::RESIZE_TERMINAL,
            method::RELEASE_WRITER,
        ]
    );
}

#[tokio::test]
async fn read_only_relay_preserves_input_and_reports_the_current_holder() {
    let terminal_id = Uuid::new_v4();
    let mut replies = read_only_prefix(terminal_id);
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (status, mut events) = mpsc::unbounded_channel();
    let exit = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        Cursor::new(INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
        status,
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    assert_eq!(
        events.try_recv().unwrap(),
        RelayStatus::ReadOnly {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );
    assert_eq!(events.try_recv().unwrap(), RelayStatus::ControlAcquired);
    let request = host
        .requests()
        .into_iter()
        .find(|request| request.method == method::TERMINAL_INPUT)
        .unwrap()
        .decode_payload::<crate::worker::InputRequest>()
        .unwrap();
    assert_eq!(request.data, INPUT);
}

#[tokio::test]
async fn successful_input_uses_the_lease_generation_before_eof() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::InputDelivered, Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        Cursor::new(INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    let input = host
        .requests()
        .into_iter()
        .find(|request| request.method == method::TERMINAL_INPUT)
        .unwrap()
        .decode_payload::<crate::worker::InputRequest>()
        .unwrap();
    assert_eq!(input.generation, GENERATION);
    assert_eq!(input.data, INPUT);
}

#[tokio::test]
async fn ownership_rejection_preserves_input_until_reacquisition() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
        Reply::value(&Some(other_lease())),
        Reply::Lease(RECOVERED_GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (status, mut events) = mpsc::unbounded_channel();
    let exit = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        Cursor::new(INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
        status,
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    assert_eq!(events.try_recv().unwrap(), RelayStatus::ControlAcquired);
    assert_eq!(
        events.try_recv().unwrap(),
        RelayStatus::ControlLost {
            holder_id: Some(OTHER_HOLDER.into())
        }
    );
    assert_eq!(events.try_recv().unwrap(), RelayStatus::ControlAcquired);
    let inputs: Vec<_> = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<crate::worker::InputRequest>().unwrap())
        .collect();
    assert_eq!(inputs.len(), RETRIED_INPUT_REQUEST_COUNT);
    let first = inputs.first().unwrap();
    let retry = inputs.last().unwrap();
    assert_eq!(first.input_id, retry.input_id);
    assert!(inputs.iter().all(|input| input.data == INPUT));
    assert_eq!(first.generation, GENERATION);
    assert_eq!(retry.generation, RECOVERED_GENERATION);
}

#[tokio::test]
async fn resize_stream_deduplicates_sizes_and_closes_without_ending_the_relay() {
    let terminal_id = Uuid::new_v4();
    let size = TerminalSize::new(101, 41).unwrap();
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::value(&()), Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    let (resize_sender, resize_events) = mpsc::channel(2);
    resize_sender.send(size).await.unwrap();
    resize_sender.send(size).await.unwrap();
    drop(resize_sender);
    let (input_sender, input) = duplex(IO_BUFFER_BYTES);
    let relay = tokio::spawn(relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        resize_events,
    ));
    tokio::time::timeout(TEST_TIMEOUT, async {
        while !host.methods().contains(&method::RESIZE_TERMINAL.into()) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(input_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
    assert_eq!(
        host.methods()
            .iter()
            .filter(|method_name| method_name.as_str() == method::RESIZE_TERMINAL)
            .count(),
        1
    );
}
