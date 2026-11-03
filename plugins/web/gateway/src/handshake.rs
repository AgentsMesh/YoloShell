use crate::{
    GatewayConfig, ORIGIN_HEADER, PROTOCOL_HEADER, STREAM_CHUNK_BYTES, WEB_PATH, WEB_SUBPROTOCOL,
    WebOrigin,
};
use anyhow::{Context, Result, ensure};
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    WebSocketStream, accept_hdr_async_with_config,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        http::{HeaderValue, StatusCode},
        protocol::WebSocketConfig,
    },
};

pub(crate) async fn authenticate(
    stream: TcpStream,
    config: &GatewayConfig,
) -> Result<WebSocketStream<TcpStream>> {
    let settings = WebSocketConfig::default()
        .max_message_size(Some(STREAM_CHUNK_BYTES))
        .max_frame_size(Some(STREAM_CHUNK_BYTES));
    let mut socket = accept_hdr_async_with_config(
        stream,
        |request: &Request, mut response: Response| {
            let headers = request.headers();
            let origin = headers
                .get(ORIGIN_HEADER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| WebOrigin::parse(value).ok());
            let protocol = headers
                .get(PROTOCOL_HEADER)
                .and_then(|value| value.to_str().ok());
            if request.uri().path_and_query().map(|path| path.as_str()) != Some(WEB_PATH)
                || headers.get_all(ORIGIN_HEADER).iter().count() != 1
                || origin
                    .as_ref()
                    .is_none_or(|origin| !config.origins.contains(origin))
                || protocol.is_none_or(|value| {
                    !value.split(',').any(|item| item.trim() == WEB_SUBPROTOCOL)
                })
            {
                let mut denied = ErrorResponse::new(Some("web upgrade rejected".into()));
                *denied.status_mut() = StatusCode::FORBIDDEN;
                return Err(denied);
            }
            response
                .headers_mut()
                .insert(PROTOCOL_HEADER, HeaderValue::from_static(WEB_SUBPROTOCOL));
            Ok(response)
        },
        Some(settings),
    )
    .await?;
    let first = socket
        .next()
        .await
        .context("web authentication missing")??;
    ensure!(
        matches!(first, Message::Text(_)),
        "web authentication requires a text document"
    );
    config.token.authenticate(first.into_data().as_ref())?;
    Ok(socket)
}
