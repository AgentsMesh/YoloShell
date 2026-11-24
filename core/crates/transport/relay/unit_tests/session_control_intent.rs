use super::*;
use crate::protocol::method;
use crate::relay::session_test_support::{
    GENERATION, attached_prefix, empty_attach, options, other_lease,
};
use crate::relay::test_support::{FakeHost, Reply};
use crate::worker::{InputRequest, ResizeRequest};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, duplex};

const INPUT: &[u8] = b"input restores disabled lease intent";
const RECOVERED_GENERATION: u64 = GENERATION + 2;
const TEST_COLUMNS: u16 = 103;
const TEST_ROWS: u16 = 37;
const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const EXPECTED_INPUT_REQUEST_COUNT: usize = 1;
const STATUS_CHANNEL_CLOSED_DETAIL: &str = "relay status channel closed during recovery";

#[tokio::test]
async fn pending_input_restores_control_intent_after_resize_rejection() {
    let terminal_id = Uuid::new_v4();
    let mut replies = attached_prefix(
        terminal_id,
        Reply::attachment(empty_attach(), Vec::new()),
    );
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
        Reply::value(&Some(other_lease())),
        Reply::Lease(RECOVERED_GENERATION),
        Reply::value(&()),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (mut input_sender, input) = duplex(INPUT.len());
    let (resize_sender, resize_events) = mpsc::channel(1);
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        resize_events,
        status_sender,
    ));

    assert_status(&mut statuses, RelayStatus::ControlAcquired).await;
    resize_sender
        .send(TerminalSize::new(TEST_COLUMNS, TEST_ROWS).unwrap())
        .await
        .unwrap();
    let lost = next_status(&mut statuses).await;
    assert!(matches!(lost, RelayStatus::ControlLost { .. }));
    input_sender.write_all(INPUT).await.unwrap();
    input_sender.shutdown().await.unwrap();
    assert_status(&mut statuses, RelayStatus::ControlAcquired).await;
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);

    let requests = host.requests();
    let inputs = requests
        .iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<InputRequest>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(inputs.len(), EXPECTED_INPUT_REQUEST_COUNT);
    assert_eq!(inputs[0].data, INPUT);
    assert_eq!(inputs[0].generation, RECOVERED_GENERATION);
    let generations = requests
        .iter()
        .filter(|request| request.method == method::RESIZE_TERMINAL)
        .map(|request| request.decode_payload::<ResizeRequest>().unwrap().generation)
        .collect::<Vec<_>>();
    assert_eq!(generations, [GENERATION, RECOVERED_GENERATION]);
    drop(resize_sender);
}

async fn assert_status(
    statuses: &mut mpsc::UnboundedReceiver<RelayStatus>,
    expected: RelayStatus,
) {
    assert_eq!(next_status(statuses).await, expected);
}

async fn next_status(statuses: &mut mpsc::UnboundedReceiver<RelayStatus>) -> RelayStatus {
    tokio::time::timeout(TEST_TIMEOUT, statuses.recv())
        .await
        .unwrap()
        .expect(STATUS_CHANNEL_CLOSED_DETAIL)
}
