use crate::support::*;
use futures::{SinkExt, StreamExt};
use tokio::time::timeout_at;
use tokio_tungstenite::tungstenite::Message;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::Call;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{AttachRequest, OutputCursor};

async fn saturated(fixture: &Fixture) -> Browser {
    let (mut browser, _) = Browser::login(fixture.address).await;
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: fixture.hub,
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let request = AttachRequest::new(
        target,
        Some(OutputCursor {
            target,
            after_sequence: 0,
        }),
    )
    .unwrap();
    browser
        .call(&Call::new(fixture.hub, Command::Attach { request }).unwrap())
        .await;
    let _ = browser.json().await;
    // Let the source saturate the bounded WebSocket/TCP buffers without reading.
    tokio::time::sleep(OUTPUT_SATURATION_WAIT).await;
    browser
}

#[tokio::test]
async fn disconnect_interrupts_output_backpressure_and_retires_only_its_connection() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let browser = saturated(&fixture).await;
    drop(browser);
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn revocation_interrupts_output_backpressure_without_waiting_for_session_expiry() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let mut browser = saturated(&fixture).await;
    fixture.registry.close();
    browser.closed().await;
    fixture.wait_closed(1).await;
    drop(browser);
    fixture.shutdown().await;
}

#[tokio::test]
async fn malformed_login_and_oversized_websocket_frames_do_not_keep_channels() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let mut browser = Browser::open(fixture.address, Some(ORIGIN)).await.unwrap();
    browser
        .socket
        .send(Message::Text(
            format!("{{\"token\":\"{TOKEN}\",\"token\":\"{TOKEN}\"}}").into(),
        ))
        .await
        .unwrap();
    browser.closed().await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    let _ = browser
        .socket
        .send(Message::Binary(
            vec![0; yolo_web_gateway::STREAM_CHUNK_BYTES + 1].into(),
        ))
        .await;
    browser.closed().await;
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn idle_websocket_ping_is_answered_without_a_hub_rpc() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    let payload = b"still-here".to_vec();
    browser
        .socket
        .send(Message::Ping(payload.clone().into()))
        .await
        .unwrap();
    let pong = timeout_at(deadline(), browser.socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(pong, Message::Pong(payload.into()));
    drop(browser);
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}
