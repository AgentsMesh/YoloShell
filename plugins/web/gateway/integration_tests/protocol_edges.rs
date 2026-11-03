use crate::support::*;
use futures::SinkExt;
use std::sync::atomic::Ordering;
use tokio::{net::TcpStream, time::timeout_at};
use tokio_tungstenite::{
    client_async,
    tungstenite::{
        Error, Message,
        client::IntoClientRequest,
        http::{HeaderValue, StatusCode},
    },
};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_web_gateway::{ORIGIN_HEADER, PROTOCOL_HEADER, WEB_PATH, WEB_SUBPROTOCOL};

#[tokio::test]
async fn upgrade_rejects_wrong_paths_and_subprotocols_before_accepting_credentials() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    for (path, protocol) in [
        ("/other", Some(WEB_SUBPROTOCOL)),
        ("/hub?token=forbidden", Some(WEB_SUBPROTOCOL)),
        (WEB_PATH, None),
        (WEB_PATH, Some("other.protocol")),
    ] {
        let mut request = format!("ws://{}{path}", fixture.address)
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert(ORIGIN_HEADER, HeaderValue::from_static(ORIGIN));
        if let Some(protocol) = protocol {
            request
                .headers_mut()
                .insert(PROTOCOL_HEADER, HeaderValue::from_str(protocol).unwrap());
        }
        let socket = TcpStream::connect(fixture.address).await.unwrap();
        let result = timeout_at(deadline(), client_async(request, socket))
            .await
            .unwrap();
        assert!(
            matches!(result, Err(Error::Http(response)) if response.status() == StatusCode::FORBIDDEN)
        );
    }
    assert_eq!(fixture.accepted.load(Ordering::SeqCst), 0);
    fixture.shutdown().await;
}

#[tokio::test]
async fn pong_preserves_rpc_but_text_after_login_retires_the_scoped_channel() {
    let fixture = Fixture::start(LIFETIME, 2).await;
    let (mut browser, _) = Browser::login(fixture.address).await;
    browser
        .socket
        .send(Message::Pong(Vec::new().into()))
        .await
        .unwrap();
    let call = Call::new(
        fixture.hub,
        Command::List {
            include_archived: false,
            after: None,
        },
    )
    .unwrap();
    browser.call(&call).await;
    let answer: Answer = serde_json::from_value(browser.json().await).unwrap();
    assert!(matches!(
        answer.reply(&call).unwrap(),
        Reply::Terminals { .. }
    ));
    browser
        .socket
        .send(Message::Text("cannot-change-stream-mode".into()))
        .await
        .unwrap();
    browser.closed().await;
    fixture.wait_closed(1).await;
    fixture.shutdown().await;
}

#[tokio::test]
async fn gateway_shutdown_joins_live_channels_before_the_backend_is_stopped() {
    let mut fixture = Fixture::start(LIFETIME, 2).await;
    let (mut first, _) = Browser::login(fixture.address).await;
    let (mut second, _) = Browser::login(fixture.address).await;
    fixture.shutdown_gateway().await;
    first.closed().await;
    second.closed().await;
    fixture.wait_closed(2).await;
}
