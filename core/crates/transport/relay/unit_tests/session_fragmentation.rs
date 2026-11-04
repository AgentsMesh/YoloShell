use super::*;
use crate::lease::constants::MIN_TTL_MS;
use crate::protocol::{EventEnvelope, event, method};
use crate::relay::session_test_support::{
    GENERATION, empty_attach, exit_event, options, reconnect_prefix, test_runtime_epoch,
};
use crate::relay::test_support::{FakeHost, RenewalReply, Reply};
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, duplex};
use tokio::sync::{Notify, mpsc, oneshot};

const EVENT_PAYLOAD_BYTES: usize = 4096;
const EVENT_TRANSPORT_FRAGMENT_BYTES: usize = 1;
const INPUT_BUFFER_BYTES: usize = 32;
const INPUT: &[u8] = b"fragmented-event-input";
const EARLY_RELAY_EXIT_DETAIL: &str = "relay exited before the fragmented event was sent";
const TERMINAL_COLUMNS: u16 = 103;
const TERMINAL_ROWS: u16 = 43;
const RESIZE_CHANNEL_CAPACITY: usize = 1;

#[tokio::test]
async fn fragmented_event_survives_input_resize_and_lease_maintenance() {
    fragmented_event_with_controls(false).await;
}

#[tokio::test]
async fn fragmented_event_survives_lease_maintenance_before_input_and_resize() {
    fragmented_event_with_controls(true).await;
}

async fn fragmented_event_with_controls(renew_before_controls: bool) {
    let terminal_id = Uuid::new_v4();
    let (fragment_sent, fragment_received) = oneshot::channel();
    let (attachment_ready, ready) = oneshot::channel();
    let resume_event = Arc::new(Notify::new());
    let event = EventEnvelope::new(
        event::OUTPUT_GAP,
        Some(terminal_id.to_string()),
        Some(test_runtime_epoch().to_string()),
        &vec![u8::default(); EVENT_PAYLOAD_BYTES],
    )
    .unwrap();
    let fragment_bytes = EVENT_TRANSPORT_FRAGMENT_BYTES;
    let mut replies = reconnect_prefix(terminal_id);
    replies.extend([
        Reply::fragmented_attachment(
            empty_attach(),
            event,
            ready,
            fragment_bytes,
            fragment_sent,
            Arc::clone(&resume_event),
        ),
        Reply::LeaseWithTtl {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        },
        Reply::InputDelivered,
        Reply::value(&()),
        Reply::attachment(empty_attach(), vec![exit_event(terminal_id)]),
        Reply::value(&()),
    ]);
    let host = FakeHost::start_renewing(
        replies,
        RenewalReply {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        },
    )
    .await;
    let (mut input_sender, input) = duplex(INPUT_BUFFER_BYTES);
    let (resize_sender, resize_events) = mpsc::channel(RESIZE_CHANNEL_CAPACITY);
    let mut relay = tokio::spawn(relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        resize_events,
    ));

    host.wait_for_method(method::ACQUIRE_WRITER).await;
    attachment_ready.send(()).unwrap();
    tokio::select! {
        received = fragment_received => {
            assert!(
                received.is_ok(),
                "{EARLY_RELAY_EXIT_DETAIL}: finished={}; methods={:?}",
                relay.is_finished(),
                host.methods(),
            );
        }
        result = &mut relay => {
            panic!("{EARLY_RELAY_EXIT_DETAIL}: {result:?}; methods={:?}", host.methods())
        }
    }
    if renew_before_controls {
        host.wait_for_method(method::RENEW_WRITER).await;
    }
    input_sender.write_all(INPUT).await.unwrap();
    host.wait_for_method(method::TERMINAL_INPUT).await;
    resize_sender
        .send(TerminalSize::new(TERMINAL_COLUMNS, TERMINAL_ROWS).unwrap())
        .await
        .unwrap();
    host.wait_for_method(method::RESIZE_TERMINAL).await;
    host.wait_for_method(method::RENEW_WRITER).await;
    resume_event.notify_one();

    let exit = relay.await.unwrap().unwrap();
    assert_eq!(exit, RelayExit::SessionExited);
    assert_eq!(host.methods().last().unwrap(), method::RELEASE_WRITER);
}
