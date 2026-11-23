use super::super::constants::{CODEX_ACK_IDENTITY, CODEX_ACK_PROTOCOL, CODEX_MAX_FRAME_BYTES};
use super::super::protocol::CodexHookAck;
use super::support::*;
use crate::agent::hooks::ApplyDisposition;
use crate::local_unix::LocalUnixStream;
use tokio::io::AsyncWriteExt;

const MALFORMED_JSON_FRAME: &[u8] = b"{";

#[tokio::test]
async fn applied_event_receives_exact_accepted_ack() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);

    let (ack, publication) = exchange(fixture.runtime, &event).await;

    assert_success_ack(&ack, &event, crate::codex_hooks::ACK_CODE_ACCEPTED);
    assert!(publication.is_some());
}

#[tokio::test]
async fn duplicate_event_receives_exact_replay_ack() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    let (first, _) = exchange(Arc::clone(&fixture.runtime), &event).await;

    let (replay, publication) = exchange(fixture.runtime, &event).await;

    assert_success_ack(&first, &event, crate::codex_hooks::ACK_CODE_ACCEPTED);
    assert_success_ack(&replay, &event, crate::codex_hooks::ACK_CODE_REPLAY);
    assert!(publication.is_some());
}

#[tokio::test]
async fn undurable_sequence_gap_receives_protocol_rejection_ack() {
    let fixture = Fixture::new();
    let start = fixture.start(FIRST_EVENT_SEQUENCE);
    exchange(Arc::clone(&fixture.runtime), &start).await;
    let event = fixture.prompt(GAP_EVENT_SEQUENCE);

    let (ack, publication) = exchange_uncommitted(fixture.runtime, &event).await;

    assert_rejection_ack(&ack, Some(&event), CODEX_ACK_PROTOCOL);
    assert!(publication.is_none());
}

#[tokio::test]
async fn worker_session_mismatch_receives_identity_rejection_ack() {
    let fixture = Fixture::new();
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.identity.terminal_id = Fixture::another_session();

    let (ack, publication) = exchange(fixture.runtime, &event).await;

    assert_rejection_ack(&ack, Some(&event), CODEX_ACK_IDENTITY);
    assert!(publication.is_none());
}

#[tokio::test]
async fn malformed_json_receives_uncorrelated_protocol_rejection() {
    let fixture = Fixture::new();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let task = tokio::spawn(super::super::handler::handle(server, fixture.runtime));
    write_raw_frame(&mut client, MALFORMED_JSON_FRAME).await;

    let ack = read_ack(&mut client).await;
    let outcome = task.await.unwrap().unwrap();
    let (publication, response) = outcome.into_parts();
    response.unwrap();

    assert_rejection_ack(&ack, None, CODEX_ACK_PROTOCOL);
    assert!(publication.is_none());
}

#[tokio::test]
async fn oversized_frame_is_rejected_before_payload_allocation() {
    let fixture = Fixture::new();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    let task = tokio::spawn(super::super::handler::handle(server, fixture.runtime));
    let oversized = CODEX_MAX_FRAME_BYTES
        .checked_add(FRAME_LENGTH_OVERFLOW)
        .unwrap();
    let length = u32::try_from(oversized).unwrap();
    client.write_all(&length.to_be_bytes()).await.unwrap();

    let ack = read_ack(&mut client).await;
    let outcome = task.await.unwrap().unwrap();
    let (publication, response) = outcome.into_parts();
    response.unwrap();

    assert_rejection_ack(&ack, None, CODEX_ACK_PROTOCOL);
    assert!(publication.is_none());
}

#[tokio::test]
async fn ack_timeout_preserves_committed_publication_and_replays() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(FIRST_EVENT_SEQUENCE));
    let runtime = Arc::clone(&fixture.runtime);
    let mut authority = runtime.subscribe_authority();
    let (server, mut client) = LocalUnixStream::pair().unwrap();
    saturate_ack_channel(&server);
    let payload = yolo_capnp_codec::encode(&event).unwrap();
    write_bytes_frame(&mut client, &payload).await;

    let task = tokio::spawn(super::super::handler::handle(server, Arc::clone(&runtime)));
    tokio::time::timeout(
        super::super::constants::CODEX_REQUEST_TIMEOUT,
        authority.changed(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!task.is_finished());
    let outcome = task.await.unwrap().unwrap();
    let (publication, response) = outcome.into_parts();

    assert!(response.is_err());
    assert_eq!(publication.unwrap(), ApplyDisposition::Applied);
    assert!(matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(state)
            if state.last_sequence.0 == FIRST_EVENT_SEQUENCE
    ));
    assert!(runtime.snapshot_path().exists());
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        FIRST_EVENT_SEQUENCE
    );

    let (ack, publication) = exchange_uncommitted(runtime, &event).await;
    assert_success_ack(&ack, &event, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(publication.unwrap(), ApplyDisposition::Replay);
}

fn assert_success_ack(ack: &CodexHookAck, event: &crate::agent::hooks::HookEnvelope, code: &str) {
    assert_eq!(ack.schema_version, crate::codex_hooks::ACK_SCHEMA_VERSION);
    assert!(ack.accepted);
    assert_eq!(ack.event_id, Some(event.event_id));
    assert_eq!(ack.sequence, Some(event.sequence.0));
    assert_eq!(ack.code, code);
}

fn assert_rejection_ack(
    ack: &CodexHookAck,
    event: Option<&crate::agent::hooks::HookEnvelope>,
    code: &str,
) {
    assert_eq!(ack.schema_version, crate::codex_hooks::ACK_SCHEMA_VERSION);
    assert!(!ack.accepted);
    assert_eq!(ack.event_id, event.map(|value| value.event_id));
    assert_eq!(ack.sequence, event.map(|value| value.sequence.0));
    assert_eq!(ack.code, code);
}

async fn write_raw_frame(stream: &mut LocalUnixStream, payload: &[u8]) {
    write_bytes_frame(stream, payload).await;
}

use std::sync::Arc;
