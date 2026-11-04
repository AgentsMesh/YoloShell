use super::{cleanup_rpc::*, fixtures, input_connection_support::*};
use std::time::Instant;
use yolo_hub_identity::TerminalId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;
use yolo_runtime::protocol::{ErrorBody, ProtocolError, ResponseEnvelope};
const IDLE_MARGIN: std::time::Duration = std::time::Duration::from_millis(1);

#[tokio::test]
async fn consecutive_inputs_reuse_one_connection_and_keep_request_identity() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let second = next_input(&first);
    let peer_first = first.clone();
    let peer_second = second.clone();
    let first_command = Command::Input { request: first };
    let second_command = Command::Input { request: second };
    let peer = async {
        let (mut peer, incoming) = fixture.accept_request().await;
        peer.respond(&delivered(&incoming, &peer_first))
            .await
            .unwrap();
        let incoming = peer.request().await.unwrap();
        peer.respond(&delivered(&incoming, &peer_second))
            .await
            .unwrap();
        fixture.assert_no_pending_connection();
    };
    let client = async {
        assert_delivered(fixture.backend.execute(&first_command).await);
        assert_delivered(fixture.backend.execute(&second_command).await);
    };
    bounded(async { tokio::join!(client, peer) }).await;
}

#[tokio::test]
async fn an_idle_host_connection_is_reopened_before_the_next_input() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let first_peer = first.clone();
    let first_command = Command::Input { request: first };
    let first_task = tokio::spawn({
        let backend = fixture.backend.clone();
        async move { backend.execute(&first_command).await }
    });
    let (mut first_channel, first_incoming) = fixture.accept_request().await;
    first_channel
        .respond(&delivered(&first_incoming, &first_peer))
        .await
        .unwrap();
    assert_delivered(first_task.await.unwrap());

    let slot = fixture
        .backend
        .host_connections
        .slot(fixtures::target().address.terminal_id.as_uuid())
        .await;
    let mut cached = slot.lock().await;
    cached.last_used =
        Instant::now() - yolo_hub_host_connections::INPUT_CONNECTION_IDLE_BUDGET - IDLE_MARGIN;
    drop(cached);

    let next = next_input(&first_peer);
    let next_peer = next.clone();
    let next_command = Command::Input { request: next };
    let next_task = tokio::spawn({
        let backend = fixture.backend.clone();
        async move { backend.execute(&next_command).await }
    });
    let (mut second_channel, second_incoming) = fixture.accept_request().await;
    second_channel
        .respond(&delivered(&second_incoming, &next_peer))
        .await
        .unwrap();
    assert_delivered(next_task.await.unwrap());
}

#[tokio::test]
async fn cancelled_input_retires_connection_and_next_request_is_not_a_replay() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let task = pending(&fixture, &first);
    let (mut old_peer, incoming) = fixture.accept_request().await;
    let _ = delivered(&incoming, &first);
    task.abort();
    assert!(bounded(task).await.unwrap_err().is_cancelled());
    assert!(matches!(
        bounded(old_peer.request()).await,
        Err(ProtocolError::EndOfStream)
    ));

    let next = next_input(&first);
    let peer_next = next.clone();
    let next_command = Command::Input { request: next };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&next_command),
            fixture.reply(|incoming| delivered(incoming, &peer_next)),
        )
    })
    .await;
    assert_delivered(reply);
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn unknown_input_outcome_retires_connection_without_replaying_bytes() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let task = pending(&fixture, &first);
    let (peer, incoming) = fixture.accept_request().await;
    let _ = delivered(&incoming, &first);
    drop(peer);
    assert!(matches!(
        bounded(task).await.unwrap(),
        Err(PublicError::OutcomeUnknown)
    ));

    let next = next_input(&first);
    let peer_next = next.clone();
    let next_command = Command::Input { request: next };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&next_command),
            fixture.reply(|incoming| delivered(incoming, &peer_next)),
        )
    })
    .await;
    assert_delivered(reply);
}

#[tokio::test]
async fn another_terminal_does_not_wait_for_a_blocked_input() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let blocked = pending(&fixture, &first);
    let (mut first_peer, first_incoming) = fixture.accept_request().await;
    let mut other_target = fixtures::target();
    other_target.address.terminal_id = TerminalId::new();
    let other = input(other_target, NEXT_BYTES);
    let peer_other = other.clone();
    let other_command = Command::Input { request: other };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&other_command),
            fixture.reply(|incoming| delivered(incoming, &peer_other)),
        )
    })
    .await;
    assert_delivered(reply);
    assert!(!blocked.is_finished());
    bounded(first_peer.respond(&delivered(&first_incoming, &first)))
        .await
        .unwrap();
    assert_delivered(bounded(blocked).await.unwrap());
}

#[tokio::test]
async fn rejected_input_is_not_replayed_and_a_new_request_reconnects() {
    let fixture = ProtocolFixture::new();
    let first = input(fixtures::target(), FIRST_BYTES);
    let peer_first = first.clone();
    let first_command = Command::Input {
        request: first.clone(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&first_command),
            fixture.reply(|incoming| {
                let _ = delivered(incoming, &peer_first);
                ResponseEnvelope::failure(incoming, ErrorBody::conflict(REFUSED_INPUT))
            }),
        )
    })
    .await;
    assert!(matches!(reply, Err(PublicError::Conflict)));
    let next = next_input(&first);
    let peer_next = next.clone();
    let next_command = Command::Input { request: next };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&next_command),
            fixture.reply(|incoming| delivered(incoming, &peer_next)),
        )
    })
    .await;
    assert_delivered(reply);
}
