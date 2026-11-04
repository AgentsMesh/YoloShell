use super::*;
use std::io::ErrorKind;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::UnixStream,
    time::timeout_at,
};
use yolo_hub_client::{Client, ClientError};
use yolo_hub_client_transport::ClientStream;
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn closed(stream: &mut (impl AsyncRead + Unpin)) {
    let error = timeout_at(deadline(), stream.read_u8())
        .await
        .expect(CLOSE_MUST_NOT_TIMEOUT)
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::UnexpectedEof
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
            | ErrorKind::BrokenPipe
    ));
}

pub fn disconnected(error: ClientError) {
    let yolo_hub_client::ClientError::Transport(error) = error else {
        panic!("{EXPECTED_TRANSPORT_CLOSURE}");
    };
    assert!(matches!(
        error,
        yolo_hub_jsonrpc_transport::TransportError::TruncatedFrame
            | yolo_hub_jsonrpc_transport::TransportError::Io(_)
    ));
}

pub async fn drained_close(stream: &mut (impl AsyncRead + Unpin)) {
    let mut bytes = Vec::new();
    let result = timeout_at(deadline(), stream.read_to_end(&mut bytes))
        .await
        .expect(CLOSE_MUST_NOT_TIMEOUT);
    if let Err(error) = result {
        assert!(matches!(
            error.kind(),
            ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::BrokenPipe
        ));
    }
}

pub async fn client(connection: &ClientConnection, hub: HubId) -> Client<ClientStream> {
    Client::open(
        connection.open_channel(deadline()).await.unwrap(),
        hub,
        deadline(),
    )
    .await
    .unwrap()
}
pub async fn healthy(client: &mut Client<ClientStream>, hub: HubId) {
    assert!(matches!(
        client
            .call(hub, Command::Hello {}, deadline())
            .await
            .unwrap(),
        Reply::Hello { .. }
    ));
}

impl Fixture {
    pub async fn pending_handshake(&self) -> UnixStream {
        let mut stream = timeout_at(deadline(), UnixStream::connect(&self.socket))
            .await
            .unwrap()
            .unwrap();
        // Server identification proves this socket occupies an actual serve task.
        timeout_at(deadline(), stream.read_u8())
            .await
            .unwrap()
            .unwrap();
        stream
    }
    pub async fn eventually_connection(&self) -> ClientConnection {
        let expires = deadline();
        timeout_at(expires, async {
            loop {
                let stream = UnixStream::connect(&self.socket).await.unwrap();
                match yolo_hub_client_transport::connect(
                    ClientIo::from_unix(stream).unwrap(),
                    self.key.clone(),
                    self.pin(),
                    expires,
                )
                .await
                {
                    Ok(connection) => return connection,
                    Err(_) => tokio::time::sleep(CAPACITY_POLL).await,
                }
            }
        })
        .await
        .expect(CAPACITY_MUST_RECOVER)
    }
    pub async fn failed_service(&mut self) {
        let result = timeout_at(deadline(), self.task.take().unwrap())
            .await
            .expect(SERVICE_MUST_EXIT)
            .unwrap();
        assert!(result.is_err());
        assert!(self.registry.current().is_err());
        assert!(!self.socket.exists());
        assert!(UnixStream::connect(&self.socket).await.is_err());
        self.stop.take();
    }
}
