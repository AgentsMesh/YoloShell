use super::*;
use russh::{client, keys::PublicKey, server};
use std::net::Shutdown;

pub struct ClientHandler(pub HubPin);
impl client::Handler for ClientHandler {
    type Error = anyhow::Error;
    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(key
            .key_data()
            .ed25519()
            .is_some_and(|key| key.0 == self.0.public_key))
    }
}

pub struct ServerHandler;
impl server::Handler for ServerHandler {
    type Error = anyhow::Error;
    async fn auth_publickey(
        &mut self,
        _: &str,
        _: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        Ok(server::Auth::Accept)
    }
}

pub struct Server {
    pub running: server::RunningSession<ServerHandler>,
    socket: std::net::TcpStream,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}
pub async fn malicious_server() -> (ClientConnection, Server) {
    let (_, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let (client, server) = sockets().await;
    let socket = server.into_std().unwrap();
    let owned = socket.try_clone().unwrap();
    let config = Arc::new(server::Config {
        keys: vec![(*hub_key).clone()],
        ..Default::default()
    });
    let (connected, accepted) = tokio::join!(
        yolo_hub_client_transport::connect(
            ClientIo::from_tcp(client).unwrap(),
            client_key,
            pin,
            deadline()
        ),
        server::run_stream(config, TcpStream::from_std(socket).unwrap(), ServerHandler),
    );
    (
        connected.unwrap(),
        Server {
            running: accepted.unwrap(),
            socket: owned,
        },
    )
}
