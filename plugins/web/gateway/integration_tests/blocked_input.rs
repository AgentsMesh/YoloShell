use crate::{INPUT_QUEUE_CAPACITY, STREAM_CHUNK_BYTES, bridge, support::*};
use futures::SinkExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout_at,
};
use tokio_tungstenite::{WebSocketStream, accept_async, client_async, tungstenite::Message};

struct BlockedInput {
    browser: WebSocketStream<TcpStream>,
    hub: DuplexStream,
    bridge: JoinHandle<anyhow::Result<()>>,
}

impl BlockedInput {
    async fn open() -> Self {
        let listener = TcpListener::bind(LOOPBACK_BIND).await.unwrap();
        let address = listener.local_addr().unwrap();
        let tcp = TcpStream::connect(address).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        let (browser, server) = tokio::join!(
            client_async(format!("ws://{address}/"), tcp),
            accept_async(server),
        );
        let (gateway, hub) = duplex(PAUSED_HUB_CAPACITY);
        let mut server = server.unwrap();
        let bridge = tokio::spawn(async move {
            let mut gateway = gateway;
            bridge::bridge(&mut server, &mut gateway).await
        });
        Self {
            browser: browser.unwrap().0,
            hub,
            bridge,
        }
    }

    async fn block_writer(&mut self) {
        self.browser
            .send(Message::Binary(vec![b'x'; STREAM_CHUNK_BYTES].into()))
            .await
            .unwrap();
        // Observe an actual forwarded byte before sending Close. The remaining
        // message cannot fit in this live one-byte duplex until the Hub reads.
        assert_eq!(
            timeout_at(deadline(), self.hub.read_u8())
                .await
                .unwrap()
                .unwrap(),
            b'x'
        );
    }
}

impl Drop for BlockedInput {
    fn drop(&mut self) {
        self.bridge.abort();
    }
}

#[tokio::test]
async fn hub_output_eof_retires_the_bridge_even_while_the_browser_is_idle() {
    let mut fixture = BlockedInput::open().await;
    fixture.hub.shutdown().await.unwrap();
    timeout_at(deadline(), &mut fixture.bridge)
        .await
        .expect("Hub EOF did not retire the bridge")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn browser_close_cancels_a_hub_write_without_waiting_for_the_hub_to_read() {
    let mut fixture = BlockedInput::open().await;
    fixture.block_writer().await;
    fixture.browser.send(Message::Close(None)).await.unwrap();
    timeout_at(deadline(), &mut fixture.bridge)
        .await
        .expect("Close was blocked behind Hub input")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn input_queue_overload_retires_connection_instead_of_dropping_or_replaying_bytes() {
    let mut fixture = BlockedInput::open().await;
    fixture.block_writer().await;
    for _ in 0..=INPUT_QUEUE_CAPACITY {
        let _ = fixture
            .browser
            .send(Message::Binary(vec![b'y'; STREAM_CHUNK_BYTES].into()))
            .await;
    }
    let error = timeout_at(deadline(), &mut fixture.bridge)
        .await
        .expect("input saturation did not retire the connection")
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("input queue exhausted"));
    let mut delivered = Vec::new();
    timeout_at(deadline(), fixture.hub.read_to_end(&mut delivered))
        .await
        .unwrap()
        .unwrap();
    assert!(delivered.iter().all(|byte| *byte == b'x'));
    assert!(delivered.len() <= PAUSED_HUB_CAPACITY);
}
