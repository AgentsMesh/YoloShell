use super::{ORIGIN, TOKEN, deadline};
use anyhow::{Result, bail};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::{net::TcpStream, time::timeout_at};
use tokio_tungstenite::{
    WebSocketStream, client_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use yolo_hub_jsonrpc_protocol::Call;
use yolo_web_gateway::{ORIGIN_HEADER, PROTOCOL_HEADER, WEB_PATH, WEB_SUBPROTOCOL};

pub struct Browser {
    pub socket: WebSocketStream<TcpStream>,
    pending: Vec<u8>,
}

impl Browser {
    pub async fn open(address: SocketAddr, origin: Option<&str>) -> Result<Self> {
        let mut request = format!("ws://{address}{WEB_PATH}").into_client_request()?;
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert(ORIGIN_HEADER, HeaderValue::from_str(origin)?);
        }
        request
            .headers_mut()
            .insert(PROTOCOL_HEADER, HeaderValue::from_static(WEB_SUBPROTOCOL));
        let tcp = TcpStream::connect(address).await?;
        let (socket, _) = timeout_at(deadline(), client_async(request, tcp)).await??;
        Ok(Self {
            socket,
            pending: Vec::new(),
        })
    }

    pub async fn login(address: SocketAddr) -> (Self, Value) {
        let mut browser = Self::open(address, Some(ORIGIN)).await.unwrap();
        browser.authenticate(TOKEN).await;
        let ready = timeout_at(deadline(), browser.socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        (browser, serde_json::from_slice(&ready.into_data()).unwrap())
    }

    pub async fn authenticate(&mut self, token: &str) {
        self.socket
            .send(Message::Text(
                serde_json::json!({"token": token}).to_string().into(),
            ))
            .await
            .unwrap();
    }

    pub async fn call(&mut self, call: &Call) {
        // Exercise fragmentation across WebSocket messages, independent of Hub framing.
        for part in yolo_hub_jsonrpc_transport::encode(call).unwrap().chunks(7) {
            self.socket
                .send(Message::Binary(part.to_vec().into()))
                .await
                .unwrap();
        }
    }

    pub async fn json(&mut self) -> Value {
        loop {
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                let document = self.pending.drain(..=end).collect::<Vec<_>>();
                return yolo_hub_jsonrpc_protocol::decode(&document).unwrap();
            }
            self.receive().await.unwrap();
        }
    }

    pub async fn bytes(&mut self, count: usize) -> Vec<u8> {
        while self.pending.len() < count {
            self.receive().await.unwrap();
        }
        self.pending.drain(..count).collect()
    }

    async fn receive(&mut self) -> Result<()> {
        match timeout_at(deadline(), self.socket.next()).await? {
            Some(Ok(Message::Binary(bytes))) => {
                self.pending.extend_from_slice(&bytes);
                Ok(())
            }
            _ => bail!("browser stream ended"),
        }
    }

    pub async fn closed(&mut self) {
        loop {
            match timeout_at(deadline(), self.socket.next()).await.unwrap() {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                _ => {}
            }
        }
    }
}
