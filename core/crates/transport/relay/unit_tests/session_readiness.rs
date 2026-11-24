use super::*;
use crate::lease::constants::{MIN_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::method;
use crate::relay::constants::TERMINAL_RESET;
use crate::relay::session_test_support::{
    GENERATION, attached_prefix, closed_resize_channel, empty_attach, options, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, FollowUpGate, Reply};
use crate::worker::ResizeRequest;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, duplex};
use tokio::sync::{Notify, oneshot};

const SNAPSHOT: &[u8] = b"snapshot blocked past the lease renewal deadline";
const RESYNCHRONIZED_OUTPUT: &[&[u8]] = &[TERMINAL_RESET, SNAPSHOT];
const OUTPUT_BUFFER_BYTES: usize = size_of::<u8>();
const FIRST_BYTE_COUNT: usize = size_of::<u8>();
const DEADLINE_MARGIN_MS: u64 = 100;
const STATUS_TIMEOUT: Duration = Duration::from_secs(3);
const RECOVERED_GENERATION: u64 = GENERATION + 2;
const TEST_COLUMNS: u16 = 97;
const TEST_ROWS: u16 = 33;

#[tokio::test]
async fn blocked_snapshot_renews_before_control_is_announced() {
    let terminal_id = Uuid::new_v4();
    let mut replies = attached_prefix(
        terminal_id,
        Reply::attachment(snapshot_attach(SNAPSHOT), Vec::new()),
    );
    replies.extend([
        Reply::LeaseWithTtl {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        },
        Reply::LeaseWithTtl {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        },
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (input_sender, input) = duplex(OUTPUT_BUFFER_BYTES);
    let (output, mut output_reader) = duplex(OUTPUT_BUFFER_BYTES);
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        output,
        closed_resize_channel(),
        status_sender,
    ));

    let first = tokio::time::timeout(STATUS_TIMEOUT, output_reader.read_u8())
        .await
        .unwrap()
        .unwrap();
    let mut observed = vec![first];
    let renewal_delay_ms = MIN_TTL_MS / RENEW_AFTER_DIVISOR + DEADLINE_MARGIN_MS;
    tokio::time::sleep(Duration::from_millis(renewal_delay_ms)).await;
    assert!(statuses.try_recv().is_err());
    let expected = RESYNCHRONIZED_OUTPUT.concat();
    observed.resize(expected.len(), u8::default());
    tokio::time::timeout(
        STATUS_TIMEOUT,
        output_reader.read_exact(&mut observed[FIRST_BYTE_COUNT..]),
    )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed, expected);

    host.wait_for_method(method::RENEW_WRITER).await;
    assert_eq!(
        tokio::time::timeout(STATUS_TIMEOUT, statuses.recv())
            .await
            .unwrap(),
        Some(RelayStatus::ControlAcquired)
    );
    drop(input_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
}

#[tokio::test]
async fn resize_replays_after_generation_change_before_control_is_announced() {
    let terminal_id = Uuid::new_v4();
    let size = TerminalSize::new(TEST_COLUMNS, TEST_ROWS).unwrap();
    let (resize_reached, resize_reached_rx) = oneshot::channel();
    let resize_resume = Arc::new(Notify::new());
    let resize_gate = FollowUpGate::new(resize_reached, Arc::clone(&resize_resume));
    let mut replies = attached_prefix(
        terminal_id,
        Reply::attachment(empty_attach(), Vec::new()),
    );
    replies.extend([
        Reply::Lease(GENERATION),
        Reply::error(crate::relay::constants::error_code::NOT_HOLDER),
        Reply::value(&Option::<crate::model::WriterLease>::None),
        Reply::Lease(RECOVERED_GENERATION),
        Reply::gated_value(&(), resize_gate),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (input_sender, input) = duplex(OUTPUT_BUFFER_BYTES);
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id).initial_size(Some(size)),
        input,
        tokio::io::sink(),
        closed_resize_channel(),
        status_sender,
    ));

    tokio::time::timeout(STATUS_TIMEOUT, resize_reached_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        statuses.try_recv(),
        Ok(RelayStatus::ControlLost { .. })
    ));
    assert!(statuses.try_recv().is_err());
    resize_resume.notify_one();
    assert_eq!(
        tokio::time::timeout(STATUS_TIMEOUT, statuses.recv())
            .await
            .unwrap(),
        Some(RelayStatus::ControlAcquired)
    );
    drop(input_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
    let generations = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::RESIZE_TERMINAL)
        .map(|request| request.decode_payload::<ResizeRequest>().unwrap().generation)
        .collect::<Vec<_>>();
    assert_eq!(generations, [GENERATION, RECOVERED_GENERATION]);
}
