use super::*;
use russh::{
    Channel,
    keys::{PrivateKey, PublicKey},
    server,
};
use std::sync::{Arc, atomic::Ordering};
use tokio::{io::copy_bidirectional, sync::mpsc, task::JoinSet};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::ClientIo;

#[derive(Clone)]
pub struct Shared {
    pub account: PublicKey,
    pub hub_key: Arc<PrivateKey>,
    pub registry: AccessRegistry,
    pub counts: Arc<Counts>,
    pub service: Service,
    pub admitted: mpsc::UnboundedSender<server::Handle>,
}
pub struct Handler {
    pub shared: Shared,
    pub tasks: JoinSet<()>,
}
impl Drop for Handler {
    fn drop(&mut self) {
        self.shared
            .counts
            .closed
            .fetch_add(COUNT_ONE, Ordering::SeqCst);
    }
}
impl server::Handler for Handler {
    type Error = anyhow::Error;
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        Ok(
            if user == USERNAME && key.key_data() == self.shared.account.key_data() {
                server::Auth::Accept
            } else {
                server::Auth::reject()
            },
        )
    }
    async fn channel_open_direct_streamlocal(
        &mut self,
        channel: Channel<server::Msg>,
        socket: &str,
        reply: server::ChannelOpenHandle,
        session: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if socket != SOCKET {
            return Ok(());
        }
        self.shared
            .counts
            .forwards
            .fetch_add(COUNT_ONE, Ordering::SeqCst);
        let _ = self.shared.admitted.send(session.handle());
        reply.accept().await;
        let (mut bridge, ingress) = tokio::io::duplex(BRIDGE_BUFFER);
        let mut outer = channel.into_stream();
        let owner = self.tasks.spawn(async move {
            let _ = copy_bidirectional(&mut bridge, &mut outer).await;
        });
        let io = ClientIo::new(ingress, move || owner.abort());
        let shared = self.shared.clone();
        self.tasks.spawn(async move {
            inner::run(
                io,
                shared.hub_key,
                shared.registry,
                shared.service,
                shared.counts,
            )
            .await;
        });
        Ok(())
    }
}

pub async fn listen(listener: tokio::net::TcpListener, key: Arc<PrivateKey>, shared: Shared) {
    let config = Arc::new(server::Config {
        keys: vec![(*key).clone()],
        ..Default::default()
    });
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { break; };
                shared.counts.connections.fetch_add(COUNT_ONE, Ordering::SeqCst);
                let original = stream.into_std().unwrap();
                let retained = original.try_clone().unwrap();
                let config = config.clone();
                let handler = Handler { shared: shared.clone(), tasks: JoinSet::new() };
                connections.spawn(async move {
                    let _socket = CloseSocket(retained);
                    if let Ok(running) = server::run_stream(config, tokio::net::TcpStream::from_std(original).unwrap(), handler).await {
                        let _ = running.await;
                    }
                });
            },
            Some(_) = connections.join_next() => {},
        }
    }
}
struct CloseSocket(std::net::TcpStream);
impl Drop for CloseSocket {
    fn drop(&mut self) {
        let _ = self.0.shutdown(std::net::Shutdown::Both);
    }
}
