use super::constants::*;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::{net::TcpStream, time::timeout_at};
use tokio_tungstenite::{
    WebSocketStream, client_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use yolo_web_gateway::{ORIGIN_HEADER, PROTOCOL_HEADER, WEB_PATH, WEB_SUBPROTOCOL};

pub async fn login(address: SocketAddr) -> (WebSocketStream<TcpStream>, Value) {
    let mut request = format!("ws://{address}{WEB_PATH}")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert(ORIGIN_HEADER, HeaderValue::from_static(ORIGIN));
    request
        .headers_mut()
        .insert(PROTOCOL_HEADER, HeaderValue::from_static(WEB_SUBPROTOCOL));
    let socket = TcpStream::connect(address).await.unwrap();
    let (mut browser, _) = timeout_at(deadline(), client_async(request, socket))
        .await
        .unwrap()
        .unwrap();
    browser
        .send(Message::Text(
            serde_json::json!({"token": TOKEN}).to_string().into(),
        ))
        .await
        .unwrap();
    let ready = timeout_at(deadline(), browser.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    (browser, serde_json::from_slice(&ready.into_data()).unwrap())
}

pub async fn expect_closed(browser: &mut WebSocketStream<TcpStream>) {
    assert!(matches!(
        timeout_at(deadline(), browser.next()).await.unwrap(),
        None | Some(Err(_)) | Some(Ok(Message::Close(_)))
    ));
}
