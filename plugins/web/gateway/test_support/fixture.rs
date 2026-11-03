use super::{CLOSE_POLL_INTERVAL, LOOPBACK_BIND, ORIGIN, TIMEOUT, TOKEN, deadline, peer};
use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::UnixListener,
    sync::oneshot,
    task::JoinHandle,
    time::{sleep, timeout_at},
};
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity, FIRST_ACCESS_REVISION,
    OriginAccess,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, HubPin, accept};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;
use yolo_web_gateway::{AccessToken, Gateway, GatewayConfig, ScopedHub, WebOrigin};

pub struct Fixture {
    _root: tempfile::TempDir,
    pub address: SocketAddr,
    pub hub: HubId,
    pub identity: ClientIdentity,
    pub registry: AccessRegistry,
    pub accepted: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    server: JoinHandle<()>,
    gateway: JoinHandle<()>,
}

impl Fixture {
    pub async fn start(lifetime: Duration, capacity: usize) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("public-client.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let (public_key, hub_key) = keys();
        let pin = HubPin {
            hub_id: HubId::new(),
            public_key,
        };
        let (client_public, client_key) = keys();
        let identity = ClientIdentity {
            client_id: uuid::Uuid::new_v4(),
            public_key: client_public,
        };
        let now = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let registry = AccessRegistry::new(AccessConfiguration {
            schema_version: ACCESS_SCHEMA_VERSION,
            entry_hub: pin.hub_id,
            revision: FIRST_ACCESS_REVISION,
            clients: vec![ClientGrant {
                identity,
                origins: vec![OriginAccess {
                    origin: pin.hub_id,
                    rights: [Right::Observe, Right::TerminalRead].into_iter().collect(),
                }],
                expires_at_ms: now + TIMEOUT.as_millis() as u64 * 10,
            }],
        })
        .unwrap();
        let closed = Arc::new(AtomicUsize::new(0));
        let accepted = Arc::new(AtomicUsize::new(0));
        let server = run_peer(
            listener,
            hub_key,
            registry.clone(),
            pin.hub_id,
            accepted.clone(),
            closed.clone(),
        );
        let backend = ScopedHub::new(path, pin, identity, client_key).unwrap();
        let config = GatewayConfig::new(
            LOOPBACK_BIND.parse().unwrap(),
            [WebOrigin::parse(ORIGIN).unwrap()],
            AccessToken::from_hex(TOKEN).unwrap(),
            lifetime,
            capacity,
        )
        .unwrap();
        let gateway = Gateway::bind(config, backend).await.unwrap();
        let address = gateway.local_addr().unwrap();
        let (stop, stopped) = oneshot::channel();
        let gateway = tokio::spawn(async move {
            gateway
                .serve_until(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            _root: root,
            address,
            hub: pin.hub_id,
            identity,
            registry,
            accepted,
            closed,
            stop: Some(stop),
            server,
            gateway,
        }
    }

    pub async fn wait_closed(&self, expected: usize) {
        timeout_at(deadline(), async {
            while self.closed.load(Ordering::SeqCst) < expected {
                sleep(CLOSE_POLL_INTERVAL).await;
            }
        })
        .await
        .unwrap();
    }

    pub async fn shutdown_gateway(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        timeout_at(deadline(), &mut self.gateway)
            .await
            .unwrap()
            .unwrap();
    }

    pub async fn shutdown(mut self) {
        self.shutdown_gateway().await;
        self.server.abort();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.gateway.abort();
        self.server.abort();
        self.registry.close();
    }
}

fn keys() -> ([u8; 32], Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    (
        signing.public_key().as_ref().try_into().unwrap(),
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}

fn run_peer(
    listener: UnixListener,
    key: Arc<PrivateKey>,
    registry: AccessRegistry,
    hub: HubId,
    accepted: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                next = listener.accept() => {
                    let (socket, _) = next.unwrap();
                    let (key, registry, accepted, closed) = (key.clone(), registry.clone(), accepted.clone(), closed.clone());
                    tasks.spawn(async move {
                        let Ok((connection, mut incoming)) = accept(ClientIo::from_unix(socket).unwrap(), key, registry, deadline()).await else { return; };
                        accepted.fetch_add(1, Ordering::SeqCst);
                        let mut channels = tokio::task::JoinSet::new();
                        loop {
                            tokio::select! {
                                _ = connection.closed() => break,
                                next = incoming.recv() => if let Some(channel) = next { channels.spawn(peer::serve(channel, hub)); } else { break; },
                                done = channels.join_next(), if !channels.is_empty() => { let _ = done; },
                            }
                        }
                        channels.abort_all();
                        closed.fetch_add(1, Ordering::SeqCst);
                    });
                },
                done = tasks.join_next(), if !tasks.is_empty() => { let _ = done; },
            }
        }
    })
}
