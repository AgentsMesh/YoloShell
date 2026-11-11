use super::*;
use crate::protocol::{event, method};
use crate::relay::session_test_support::{
    GENERATION, closed_resize_channel, empty_attach, exit_event, options, other_lease,
    reconnect_prefix, session_event,
};
use crate::relay::test_support::{FakeHost, FollowUpGate, Reply};
use crate::worker::InputRequest;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::{Notify, mpsc, oneshot};

const INPUT: &[u8] = b"input retained across attachment recovery";
const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const EXPECTED_INPUT_REQUEST_COUNT: usize = 1;
const INPUT_BUFFER_TOO_SMALL_DETAIL: &str = "relay input buffer is smaller than the test input";
const STATUS_CHANNEL_CLOSED_DETAIL: &str = "relay status channel closed before control recovery";

#[derive(Clone, Copy)]
enum Interruption {
    OutputGap,
    TransportDisconnect,
}

struct ObservedInput {
    data: Option<&'static [u8]>,
    observed: Option<oneshot::Sender<()>>,
}

impl AsyncRead for ObservedInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let Some(data) = self.data.take() else {
            return Poll::Pending;
        };
        if buffer.remaining() < data.len() {
            return Poll::Ready(Err(std::io::Error::other(INPUT_BUFFER_TOO_SMALL_DETAIL)));
        }
        buffer.put_slice(data);
        if let Some(observed) = self.observed.take() {
            let _ = observed.send(());
        }
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn pending_input_survives_output_gap_and_is_sent_once() {
    assert_pending_input_recovery(Interruption::OutputGap).await;
}

#[tokio::test]
async fn pending_input_survives_transport_disconnect_and_is_sent_once() {
    assert_pending_input_recovery(Interruption::TransportDisconnect).await;
}

async fn assert_pending_input_recovery(interruption: Interruption) {
    let terminal_id = Uuid::new_v4();
    let (first_reached, first_reached_rx) = oneshot::channel();
    let first_resume = Arc::new(Notify::new());
    let first_gate = FollowUpGate::new(first_reached, Arc::clone(&first_resume));
    let first_attachment = match interruption {
        Interruption::OutputGap => Reply::gated_attachment(
            empty_attach(),
            vec![session_event(event::OUTPUT_GAP, terminal_id)],
            first_gate,
        ),
        Interruption::TransportDisconnect => {
            Reply::gated_closing_attachment(empty_attach(), first_gate)
        }
    };
    let (final_reached, final_reached_rx) = oneshot::channel();
    let final_resume = Arc::new(Notify::new());
    let final_attachment = Reply::gated_attachment(
        empty_attach(),
        vec![exit_event(terminal_id)],
        FollowUpGate::new(final_reached, Arc::clone(&final_resume)),
    );
    let replies = recovery_replies(terminal_id, interruption, first_attachment, final_attachment);
    let host = FakeHost::start(replies).await;
    let (input_observed, input_observed_rx) = oneshot::channel();
    let input = ObservedInput {
        data: Some(INPUT),
        observed: Some(input_observed),
    };
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
    await_gate(first_reached_rx).await;
    await_gate(input_observed_rx).await;
    first_resume.notify_one();
    await_gate(final_reached_rx).await;
    await_control(&mut statuses).await;
    host.wait_for_method(method::TERMINAL_INPUT).await;
    assert_single_input(&host);
    final_resume.notify_one();
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::SessionExited);
    assert_single_input(&host);
}

fn recovery_replies(
    terminal_id: Uuid,
    interruption: Interruption,
    first_attachment: Reply,
    final_attachment: Reply,
) -> Vec<Reply> {
    let mut replies = reconnect_prefix(terminal_id);
    replies.extend([
        first_attachment,
        Reply::error(crate::relay::constants::error_code::HELD_BY_OTHER),
        Reply::value(&Some(other_lease())),
    ]);
    if matches!(interruption, Interruption::TransportDisconnect) {
        replies.extend(reconnect_prefix(terminal_id));
        replies.push(Reply::value(&Some(other_lease())));
    }
    replies.extend([
        final_attachment,
        Reply::Lease(GENERATION),
        Reply::InputDelivered,
        Reply::value(&()),
    ]);
    replies
}

async fn await_gate(receiver: oneshot::Receiver<()>) {
    tokio::time::timeout(TEST_TIMEOUT, receiver)
        .await
        .unwrap()
        .unwrap();
}

async fn await_control(statuses: &mut mpsc::UnboundedReceiver<RelayStatus>) {
    tokio::time::timeout(TEST_TIMEOUT, async {
        loop {
            match statuses.recv().await {
                Some(RelayStatus::ControlAcquired) => return,
                Some(_) => {}
                None => panic!("{STATUS_CHANNEL_CLOSED_DETAIL}"),
            }
        }
    })
    .await
    .unwrap();
}

fn assert_single_input(host: &FakeHost) {
    let inputs: Vec<_> = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .collect();
    assert_eq!(inputs.len(), EXPECTED_INPUT_REQUEST_COUNT);
    let input = inputs
        .first()
        .unwrap()
        .decode_payload::<InputRequest>()
        .unwrap();
    assert_eq!(input.data, INPUT);
}
