use super::*;
use super::ending_support::{
    CAPTURE_BYTES, await_gate, await_relay, await_session_ending, output_event, tail_exit_event,
};
use crate::lease::constants::{MIN_TTL_MS, code};
use crate::protocol::method;
use crate::relay::constants::CONTROL_RECONNECT_DELAY;
use crate::relay::session_test_support::{
    GENERATION, PendingInput, closed_resize_channel, empty_attach, host_status, options,
    other_lease, running_session,
};
use crate::relay::test_support::{FakeHost, FollowUpGate, Reply};
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, duplex};
use tokio::sync::{Notify, mpsc, oneshot};

const TAIL_OUTPUT: &[u8] = b"tail after ending";
const PENDING_INPUT: &[u8] = b"must never retry";
const RESIZE_COLUMNS: u16 = 101;
const RESIZE_ROWS: u16 = 41;
const QUIET_PERIOD_MULTIPLIER: u32 = 3;

#[derive(Clone, Copy)]
enum Trigger {
    Acquire,
    Takeover,
    Renew,
    Input,
    Resize,
}

impl Trigger {
    fn method(self) -> &'static str {
        match self {
            Self::Acquire => method::ACQUIRE_WRITER,
            Self::Takeover => method::TAKEOVER_WRITER,
            Self::Renew => method::RENEW_WRITER,
            Self::Input => method::TERMINAL_INPUT,
            Self::Resize => method::RESIZE_TERMINAL,
        }
    }

    fn options(self, terminal_id: Uuid) -> RelayOptions {
        let options = options(terminal_id).take_control(matches!(self, Self::Takeover));
        if matches!(self, Self::Resize) {
            options.initial_size(TerminalSize::new(RESIZE_COLUMNS, RESIZE_ROWS))
        } else {
            options
        }
    }

    fn input(self) -> Box<dyn AsyncRead + Unpin + Send> {
        if matches!(self, Self::Input) {
            Box::new(Cursor::new(PENDING_INPUT))
        } else {
            Box::new(PendingInput)
        }
    }
}

#[tokio::test]
async fn all_bound_control_errors_drain_tail_output_until_the_real_exit() {
    for trigger in [
        Trigger::Acquire,
        Trigger::Takeover,
        Trigger::Renew,
        Trigger::Input,
        Trigger::Resize,
    ] {
        assert_ending_flow(trigger).await;
    }
}

async fn assert_ending_flow(trigger: Trigger) {
    let terminal_id = Uuid::new_v4();
    let (gate_reached, gate_reached_rx) = oneshot::channel();
    let gate_resume = Arc::new(Notify::new());
    let events = vec![
        output_event(terminal_id, TAIL_OUTPUT),
        tail_exit_event(terminal_id),
    ];
    let attachment = Reply::gated_attachment(
        empty_attach(),
        events,
        FollowUpGate::new(gate_reached, Arc::clone(&gate_resume)),
    );
    let host = FakeHost::start(replies(terminal_id, trigger, attachment)).await;
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let (output, mut capture) = duplex(CAPTURE_BYTES);
    let mut relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        trigger.options(terminal_id),
        trigger.input(),
        output,
        closed_resize_channel(),
        status_sender,
    ));

    await_gate(gate_reached_rx).await;
    host.wait_for_method(trigger.method()).await;
    let mut observed = await_session_ending(&mut statuses).await;
    let methods_at_ending = host.methods();
    assert_eq!(methods_at_ending, trigger.expected_methods());
    tokio::time::sleep(CONTROL_RECONNECT_DELAY.saturating_mul(QUIET_PERIOD_MULTIPLIER)).await;
    assert_eq!(host.methods(), methods_at_ending);
    assert!(!relay.is_finished());
    gate_resume.notify_one();
    assert_eq!(await_relay(&mut relay).await.unwrap(), RelayExit::SessionExited);
    let mut captured = Vec::new();
    capture.read_to_end(&mut captured).await.unwrap();
    observed.extend(std::iter::from_fn(|| statuses.try_recv().ok()));

    assert_eq!(captured, TAIL_OUTPUT);
    assert_eq!(
        observed
            .iter()
            .filter(|status| **status == RelayStatus::SessionEnding)
            .count(),
        1
    );
    assert_eq!(observed.last(), Some(&RelayStatus::SessionExited));
    assert!(!observed.iter().any(|status| matches!(
        status,
        RelayStatus::ReadOnly { .. } | RelayStatus::ControlLost { .. }
    )));
    assert_eq!(host.methods(), trigger.expected_methods());
}

fn replies(terminal_id: Uuid, trigger: Trigger, attachment: Reply) -> Vec<Reply> {
    let mut replies = vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        attachment,
    ];
    match trigger {
        Trigger::Acquire => {}
        Trigger::Takeover => replies.push(Reply::value(&Some(other_lease()))),
        Trigger::Renew => replies.push(Reply::LeaseWithTtl {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        }),
        Trigger::Input | Trigger::Resize => replies.push(Reply::Lease(GENERATION)),
    }
    replies.push(Reply::error(code::SESSION_NOT_RUNNING));
    replies
}

impl Trigger {
    fn expected_methods(self) -> Vec<String> {
        let mut methods = vec![
            method::HOST_STATUS.into(),
            method::GET_SESSION.into(),
            method::ATTACH_SESSION.into(),
        ];
        match self {
            Self::Acquire => methods.push(method::ACQUIRE_WRITER.into()),
            Self::Takeover => methods.extend([
                method::GET_WRITER_LEASE.into(),
                method::TAKEOVER_WRITER.into(),
            ]),
            Self::Renew => methods.extend([
                method::ACQUIRE_WRITER.into(),
                method::RENEW_WRITER.into(),
            ]),
            Self::Input => methods.extend([
                method::ACQUIRE_WRITER.into(),
                method::TERMINAL_INPUT.into(),
            ]),
            Self::Resize => methods.extend([
                method::ACQUIRE_WRITER.into(),
                method::RESIZE_TERMINAL.into(),
            ]),
        }
        methods
    }
}
