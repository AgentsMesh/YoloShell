use super::*;
use crate::client::ClientError;
use crate::protocol::error_code::INTERNAL as INTERNAL_CODE;
use crate::protocol::{EventEnvelope, ProtocolError, event, method};
use crate::relay::constants::TERMINAL_RESET;
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, empty_attach, exit_event, options, owned_with_attachment,
    reconnect_prefix, session_event, snapshot_attach,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::time::Duration;
use tokio::io::{AsyncReadExt, duplex};

const CAPTURE_BYTES: usize = 128;
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(2);
const SNAPSHOT: &[u8] = b"recovered screen";

async fn recover_failed_attach(first: Reply) {
    let terminal_id = Uuid::new_v4();
    let mut replies = reconnect_prefix(terminal_id);
    replies.push(first);
    replies.extend(reconnect_prefix(terminal_id));
    replies.extend([
        Reply::attachment(empty_attach(), vec![exit_event(terminal_id)]),
        Reply::Lease(crate::relay::session_test_support::GENERATION),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (status, mut statuses) = tokio::sync::mpsc::unbounded_channel();
    let exit = tokio::time::timeout(
        RECOVERY_TIMEOUT,
        relay_session_with_status(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
            status,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(exit, RelayExit::SessionExited);
    let statuses: Vec<_> = std::iter::from_fn(|| statuses.try_recv().ok()).collect();
    assert_eq!(statuses.last(), Some(&RelayStatus::SessionExited));
    assert_eq!(
        host.methods()
            .iter()
            .filter(|name| name.as_str() == method::ATTACH_SESSION)
            .count(),
        2
    );
}

#[tokio::test]
async fn attach_retries_transport_and_explicit_retryable_failures() {
    recover_failed_attach(Reply::Disconnect).await;
    recover_failed_attach(Reply::retryable_error(
        crate::protocol::error_code::WORKER_UNREACHABLE,
    ))
    .await;
}

#[tokio::test]
async fn attached_transport_loss_recovers_host_session_and_writer_identity() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_with_attachment(terminal_id, Reply::closing_attachment(empty_attach()));
    replies.extend(reconnect_prefix(terminal_id));
    replies.extend([
        Reply::StoredLease,
        Reply::Lease(crate::relay::session_test_support::GENERATION),
        Reply::attachment(empty_attach(), vec![exit_event(terminal_id)]),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let exit = tokio::time::timeout(
        RECOVERY_TIMEOUT,
        relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(exit, RelayExit::SessionExited);
    assert!(host.methods().contains(&method::GET_WRITER_LEASE.into()));
}

#[tokio::test]
async fn output_gap_reattaches_with_a_terminal_reset_before_snapshot() {
    let terminal_id = Uuid::new_v4();
    let mut replies = owned_with_attachment(
        terminal_id,
        Reply::attachment(
            empty_attach(),
            vec![session_event(event::OUTPUT_GAP, terminal_id)],
        ),
    );
    replies.extend([
        Reply::attachment(snapshot_attach(SNAPSHOT), vec![exit_event(terminal_id)]),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (output, mut capture) = duplex(CAPTURE_BYTES);
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        output,
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::SessionExited);
    let mut bytes = Vec::new();
    capture.read_to_end(&mut bytes).await.unwrap();
    assert_eq!(bytes, [TERMINAL_RESET, SNAPSHOT].concat());
}

#[tokio::test]
async fn attachment_rejects_wrong_session_and_malformed_output() {
    let expected = Uuid::new_v4();
    let wrong = Uuid::new_v4();
    let wrong_event = session_event("ignored", wrong);
    let malformed = EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(expected.to_string()),
        None,
        &(),
    )
    .unwrap();
    for event in [wrong_event, malformed] {
        let mut replies =
            owned_with_attachment(expected, Reply::attachment(empty_attach(), vec![event]));
        replies.push(Reply::value(&()));
        let host = FakeHost::start(replies).await;
        let error = relay_session(
            RelayRuntime::existing(host.client()),
            options(expected),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            RelayError::Client(ClientError::Protocol(ProtocolError::EventMismatch))
        ));
    }
}

#[tokio::test]
async fn non_retryable_attach_error_prevents_writer_acquisition() {
    let terminal_id = Uuid::new_v4();
    let mut replies = reconnect_prefix(terminal_id);
    replies.push(Reply::error(INTERNAL_CODE));
    let host = FakeHost::start(replies).await;
    let (status, mut statuses) = tokio::sync::mpsc::unbounded_channel();
    let result = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
        status,
    )
    .await;
    assert!(matches!(result, Err(RelayError::Client(_))));
    assert!(
        !std::iter::from_fn(|| statuses.try_recv().ok())
            .any(|status| status == RelayStatus::SessionExited)
    );
    assert!(!host.methods().contains(&method::ACQUIRE_WRITER.into()));
    assert!(!host.methods().contains(&method::RELEASE_WRITER.into()));
}
