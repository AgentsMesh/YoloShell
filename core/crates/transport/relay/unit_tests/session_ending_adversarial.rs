use super::*;
use super::ending_support::{
    CAPTURE_BYTES, INITIAL_REQUEST_COUNT, RECOVERED_REQUEST_COUNT, assert_recovery_methods,
    await_gate, await_relay, await_session_ending, ending_prefix, output_event, tail_exit_event,
};
use crate::client::ClientError;
use crate::lease::constants::code;
use crate::protocol::{ErrorBody, EventEnvelope, ProtocolError, event};
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, empty_attach, empty_attach_for, host_status, options,
    running_session, session_event,
};
use crate::relay::test_support::{FakeHost, FollowUpGate, Reply};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, duplex};
use tokio::sync::{Notify, mpsc, oneshot};

const TAIL_OUTPUT: &[u8] = b"tail after attachment recovery";
const UNTRUSTED_ERROR_MESSAGE: &str = "untrusted terminal error";

#[derive(Clone, Copy)]
enum Interruption {
    Disconnect,
    OutputGap,
}

#[tokio::test]
async fn ending_interruptions_reattach_without_lease_or_provider_recovery() {
    for interruption in [Interruption::Disconnect, Interruption::OutputGap] {
        assert_recovered(interruption).await;
    }
}

async fn assert_recovered(interruption: Interruption) {
    let terminal_id = Uuid::new_v4();
    let (gate_reached, gate_reached_rx) = oneshot::channel();
    let gate_resume = Arc::new(Notify::new());
    let gate = FollowUpGate::new(gate_reached, Arc::clone(&gate_resume));
    let first_attachment = match interruption {
        Interruption::Disconnect => Reply::gated_closing_attachment(empty_attach(), gate),
        Interruption::OutputGap => Reply::gated_attachment(
            empty_attach(),
            vec![session_event(event::OUTPUT_GAP, terminal_id)],
            gate,
        ),
    };
    let mut replies = ending_prefix(terminal_id, first_attachment);
    if matches!(interruption, Interruption::Disconnect) {
        replies.push(Reply::value(&host_status()));
    }
    replies.push(Reply::attachment(
        empty_attach(),
        vec![
            output_event(terminal_id, TAIL_OUTPUT),
            tail_exit_event(terminal_id),
        ],
    ));
    let host = FakeHost::start(replies).await;
    let (status_sender, mut statuses) = mpsc::unbounded_channel();
    let (output, mut capture) = duplex(CAPTURE_BYTES);
    let mut relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        output,
        closed_resize_channel(),
        status_sender,
    ));

    await_gate(gate_reached_rx).await;
    await_session_ending(&mut statuses).await;
    gate_resume.notify_one();
    assert_eq!(await_relay(&mut relay).await.unwrap(), RelayExit::SessionExited);
    let mut observed = Vec::new();
    capture.read_to_end(&mut observed).await.unwrap();
    assert_eq!(observed, TAIL_OUTPUT);

    let expected_status = if matches!(interruption, Interruption::Disconnect) {
        RECOVERED_REQUEST_COUNT
    } else {
        INITIAL_REQUEST_COUNT
    };
    assert_recovery_methods(&host, RECOVERED_REQUEST_COUNT, expected_status);
}

#[tokio::test]
async fn replacement_epoch_after_ending_fails_closed() {
    let terminal_id = Uuid::new_v4();
    let replacement_epoch = Uuid::new_v4();
    let first = Reply::attachment(
        empty_attach(),
        vec![session_event(event::OUTPUT_GAP, terminal_id)],
    );
    let mut replies = ending_prefix(terminal_id, first);
    replies.push(Reply::attachment(
        empty_attach_for(replacement_epoch),
        Vec::new(),
    ));
    let host = FakeHost::start(replies).await;
    let mut relay = tokio::spawn(relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
    ));
    let result = await_relay(&mut relay).await;
    assert!(matches!(result, Err(RelayError::RuntimeEpochChanged { .. })));
    assert_recovery_methods(&host, RECOVERED_REQUEST_COUNT, INITIAL_REQUEST_COUNT);
}

#[tokio::test]
async fn forged_exit_identity_after_ending_remains_a_protocol_failure() {
    for forged in [ForgedExit::WrongSession, ForgedExit::WrongEpoch] {
        assert_forged_exit(forged).await;
    }
}

#[derive(Clone, Copy)]
enum ForgedExit {
    WrongSession,
    WrongEpoch,
}

async fn assert_forged_exit(forged: ForgedExit) {
    let terminal_id = Uuid::new_v4();
    let event = match forged {
        ForgedExit::WrongSession => session_event(event::SESSION_EXITED, Uuid::new_v4()),
        ForgedExit::WrongEpoch => EventEnvelope::new(
            event::SESSION_EXITED,
            Some(terminal_id.to_string()),
            Some(Uuid::new_v4().to_string()),
            &(),
        )
        .unwrap(),
    };
    let attachment = Reply::attachment(empty_attach(), vec![event]);
    let host = FakeHost::start(ending_prefix(terminal_id, attachment)).await;
    let mut relay = tokio::spawn(relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
    ));
    let result = await_relay(&mut relay).await;
    assert!(matches!(
        result,
        Err(RelayError::Client(ClientError::Protocol(
            ProtocolError::EventMismatch
        )))
    ));
    assert_recovery_methods(&host, INITIAL_REQUEST_COUNT, INITIAL_REQUEST_COUNT);
}

#[tokio::test]
async fn noncanonical_terminal_metadata_is_not_retried_or_swallowed() {
    for error in [
        ErrorBody::new(code::SESSION_NOT_RUNNING, UNTRUSTED_ERROR_MESSAGE, true),
        ErrorBody {
            code: code::SESSION_NOT_RUNNING.into(),
            message: UNTRUSTED_ERROR_MESSAGE.into(),
            retryable: false,
            details: Some(serde_json::Value::Bool(true)),
        },
    ] {
        let terminal_id = Uuid::new_v4();
        let replies = vec![
            Reply::value(&host_status()),
            Reply::value(&running_session(terminal_id)),
            Reply::attachment(empty_attach(), Vec::new()),
            Reply::Error(error),
        ];
        let host = FakeHost::start(replies).await;
        let mut relay = tokio::spawn(relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        ));
        let result = await_relay(&mut relay).await;
        assert!(matches!(result, Err(RelayError::Client(ClientError::Remote { .. }))));
        assert_recovery_methods(&host, INITIAL_REQUEST_COUNT, INITIAL_REQUEST_COUNT);
    }
}
