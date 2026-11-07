use super::*;
use super::ending_support::{CAPTURE_BYTES, await_relay, output_event, tail_exit_event};
use crate::client::ClientError;
use crate::lease::{LeaseEventPayload, LeaseReason};
use crate::protocol::{EventEnvelope, ProtocolError, event, method};
use crate::relay::constants::error_code;
use crate::relay::session_test_support::{
    OTHER_HOLDER, PendingInput, closed_resize_channel, empty_attach, host_status, options,
    other_lease, running_session, test_runtime_epoch,
};
use crate::relay::test_support::{FakeHost, Reply};
use tokio::io::{AsyncReadExt, duplex};
use tokio::sync::mpsc;

const TAIL_OUTPUT: &[u8] = b"observer tail after ending";

#[tokio::test]
async fn authenticated_session_ended_revocation_ends_a_read_only_observer() {
    let terminal_id = Uuid::new_v4();
    let events = vec![
        revocation(terminal_id, test_runtime_epoch()),
        output_event(terminal_id, TAIL_OUTPUT),
        tail_exit_event(terminal_id),
    ];
    let host = FakeHost::start(observer_replies(terminal_id, events)).await;
    let (status, mut statuses) = mpsc::unbounded_channel();
    let (output, mut capture) = duplex(CAPTURE_BYTES);
    let mut relay = tokio::spawn(relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        output,
        closed_resize_channel(),
        status,
    ));

    assert_eq!(await_relay(&mut relay).await.unwrap(), RelayExit::SessionExited);
    let mut captured = Vec::new();
    capture.read_to_end(&mut captured).await.unwrap();
    let observed: Vec<_> = std::iter::from_fn(|| statuses.try_recv().ok()).collect();

    assert_eq!(captured, TAIL_OUTPUT);
    assert_eq!(
        observed,
        [
            RelayStatus::ReadOnly {
                holder_id: Some(OTHER_HOLDER.into())
            },
            RelayStatus::SessionEnding,
            RelayStatus::SessionExited,
        ]
    );
    assert_eq!(host.methods(), observer_methods());
}

#[tokio::test]
async fn forged_session_ended_revocation_cannot_end_an_observer() {
    let terminal_id = Uuid::new_v4();
    for forged in [
        revocation(Uuid::new_v4(), test_runtime_epoch()),
        revocation(terminal_id, Uuid::new_v4()),
    ] {
        let host = FakeHost::start(observer_replies(terminal_id, vec![forged])).await;
        let mut relay = tokio::spawn(relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        ));

        assert!(matches!(
            await_relay(&mut relay).await,
            Err(RelayError::Client(ClientError::Protocol(
                ProtocolError::EventMismatch
            )))
        ));
        assert_eq!(host.methods(), observer_methods());
    }
}

fn observer_replies(terminal_id: Uuid, events: Vec<EventEnvelope>) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::attachment(empty_attach(), events),
        Reply::error(error_code::HELD_BY_OTHER),
        Reply::value(&Some(other_lease())),
    ]
}

fn revocation(terminal_id: Uuid, runtime_epoch: Uuid) -> EventEnvelope {
    let payload = LeaseEventPayload::new(other_lease(), Some(LeaseReason::SessionEnded));
    EventEnvelope::new(
        event::WRITER_LEASE_REVOKED,
        Some(terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &payload,
    )
    .unwrap()
}

fn observer_methods() -> Vec<String> {
    [
        method::HOST_STATUS,
        method::GET_SESSION,
        method::ATTACH_SESSION,
        method::ACQUIRE_WRITER,
        method::GET_WRITER_LEASE,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
