use super::*;
use crate::relay::session_test_support::{
    GENERATION, closed_resize_channel, options, read_only_prefix,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, duplex};

const STATUS_TIMEOUT: Duration = Duration::from_secs(2);
const BUFFERED_INPUT: &[u8] = b"buffered remote input";
const EXPECTED_INPUT_REQUEST_COUNT: usize = 1;

#[tokio::test]
async fn read_only_maintenance_reacquires_control_and_reapplies_size() {
    let terminal_id = Uuid::new_v4();
    let size = TerminalSize::new(93, 31).unwrap();
    let mut replies = read_only_prefix(terminal_id);
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::value(&()),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (resize_sender, resize_events) = mpsc::channel(1);
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let (input_sender, input) = duplex(1);
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id).initial_size(Some(size)),
        input,
        tokio::io::sink(),
        resize_events,
        status_sender,
    ));
    assert!(matches!(
        statuses.recv().await,
        Some(RelayStatus::ReadOnly { .. })
    ));
    assert_eq!(
        tokio::time::timeout(STATUS_TIMEOUT, statuses.recv())
            .await
            .unwrap(),
        Some(RelayStatus::ControlAcquired)
    );
    drop(input_sender);
    drop(resize_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
    assert!(
        host.methods()
            .iter()
            .any(|method| method == crate::protocol::method::RESIZE_TERMINAL)
    );
}

#[tokio::test]
async fn read_only_input_waits_for_reacquired_control_and_is_sent_once() {
    let terminal_id = Uuid::new_v4();
    let mut replies = read_only_prefix(terminal_id);
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (mut input_sender, input) = duplex(BUFFERED_INPUT.len());
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        closed_resize_channel(),
        status_sender,
    ));
    assert!(matches!(
        statuses.recv().await,
        Some(RelayStatus::ReadOnly { .. })
    ));
    input_sender.write_all(BUFFERED_INPUT).await.unwrap();
    assert_eq!(
        tokio::time::timeout(STATUS_TIMEOUT, statuses.recv())
            .await
            .unwrap(),
        Some(RelayStatus::ControlAcquired)
    );
    host.wait_for_method(crate::protocol::method::TERMINAL_INPUT)
        .await;
    let request = host
        .requests()
        .into_iter()
        .find(|request| request.method == crate::protocol::method::TERMINAL_INPUT)
        .unwrap()
        .decode_payload::<crate::worker::InputRequest>()
        .unwrap();
    assert_eq!(request.data, BUFFERED_INPUT);
    assert_eq!(
        host.methods()
            .iter()
            .filter(|method| method.as_str() == crate::protocol::method::TERMINAL_INPUT)
            .count(),
        EXPECTED_INPUT_REQUEST_COUNT
    );
    drop(input_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
}

#[tokio::test]
async fn read_only_eof_waits_for_buffered_input_delivery() {
    let terminal_id = Uuid::new_v4();
    let mut replies = read_only_prefix(terminal_id);
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (mut input_sender, input) = duplex(BUFFERED_INPUT.len());
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        closed_resize_channel(),
        status_sender,
    ));
    assert!(matches!(
        statuses.recv().await,
        Some(RelayStatus::ReadOnly { .. })
    ));
    input_sender.write_all(BUFFERED_INPUT).await.unwrap();
    input_sender.shutdown().await.unwrap();
    assert_eq!(
        tokio::time::timeout(STATUS_TIMEOUT, statuses.recv())
            .await
            .unwrap(),
        Some(RelayStatus::ControlAcquired)
    );
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
    let request = host
        .requests()
        .into_iter()
        .find(|request| request.method == crate::protocol::method::TERMINAL_INPUT)
        .unwrap()
        .decode_payload::<crate::worker::InputRequest>()
        .unwrap();
    assert_eq!(request.data, BUFFERED_INPUT);
}
