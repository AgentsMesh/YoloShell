use crate::{
    ACCEPT_RETRY_DELAY, CONNECT_TIMEOUT, GatewayConfig, HANDSHAKE_TIMEOUT, READY_TYPE, ScopedHub,
    bridge, handshake,
};
use anyhow::{Result, ensure};
use futures::SinkExt;
use serde::Serialize;
use std::{future::Future, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::JoinSet,
    time::{Instant, timeout, timeout_at},
};
use tokio_tungstenite::tungstenite::Message;
use yolo_builtin_client_contract::{CapabilityNegotiation, ClientApiContract, negotiate};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};

pub struct Gateway {
    listener: TcpListener,
    config: Arc<GatewayConfig>,
    backend: Arc<ScopedHub>,
}

impl Gateway {
    pub async fn bind(config: GatewayConfig, backend: ScopedHub) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(config.bind).await?,
            config: Arc::new(config),
            backend: Arc::new(backend),
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn serve_until(self, shutdown: impl Future<Output = ()>) -> Result<()> {
        tokio::pin!(shutdown);
        let capacity = Arc::new(Semaphore::new(self.config.capacity));
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                completed = connections.join_next(), if !connections.is_empty() => { let _ = completed; },
                accepted = self.listener.accept() => {
                    let (socket, _) = match accepted {
                        Ok(accepted) => accepted,
                        Err(error) if accept_error_is_transient(&error) => {
                            // A reset connection must not abort every
                            // established session. The listener itself is
                            // still healthy, so retry after a short pause.
                            tracing::warn!(%error, "gateway accept failed; retrying");
                            tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                            continue;
                        }
                        Err(error) => return Err(error.into()),
                    };
                    let Ok(permit) = capacity.clone().try_acquire_owned() else { continue; };
                    let config = self.config.clone();
                    let backend = self.backend.clone();
                    connections.spawn(async move {
                        let _permit = permit;
                        // Per-connection failure must not retire sibling clients or log credentials.
                        let _ = serve(socket, config, backend).await;
                    });
                }
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
        Ok(())
    }
}

/// Only a failure attributable to one queued connection is retried. A broken
/// listener (bad descriptor, exhausted descriptor table, wrong socket state)
/// must reach the supervisor instead of leaving a live gateway that can never
/// accept again.
pub fn accept_error_is_transient(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::TimedOut
    )
}

#[derive(Serialize)]
struct Ready<'a> {
    kind: &'static str,
    client_id: uuid::Uuid,
    hello: &'a yolo_hub_rpc_protocol::Hello,
    capabilities: CapabilityNegotiation,
    lifetime_ms: u64,
}

async fn serve(
    socket: TcpStream,
    config: Arc<GatewayConfig>,
    backend: Arc<ScopedHub>,
) -> Result<()> {
    socket.set_nodelay(true)?;
    let mut web = timeout(HANDSHAKE_TIMEOUT, handshake::authenticate(socket, &config)).await??;
    let expires = Instant::now() + config.lifetime;
    // The deadline includes authentication to the scoped Hub and slow browser sends.
    timeout_at(expires, async {
        let (client, connection) = backend
            .open((Instant::now() + CONNECT_TIMEOUT).min(expires))
            .await?;
        let api = ClientApiContract::new(HUB_PROTOCOL_VERSION, [Method::Hello], [], []);
        let capabilities = negotiate(&api, client.hello());
        ensure!(
            capabilities.can_start(),
            "Hub does not support web client startup"
        );
        let ready = Ready {
            kind: READY_TYPE,
            client_id: backend.identity.client_id,
            hello: client.hello(),
            capabilities,
            lifetime_ms: config.lifetime.as_millis().try_into()?,
        };
        web.send(Message::Text(serde_json::to_string(&ready)?.into()))
            .await?;
        let mut stream = client.into_channel()?.into_stream()?;
        tokio::select! {
            result = bridge::bridge(&mut web, &mut stream) => result,
            _ = connection.closed() => Ok(()),
        }
    })
    .await?
}
