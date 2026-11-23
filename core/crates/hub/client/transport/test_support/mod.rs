use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Instant,
};
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity,
    ED25519_PUBLIC_KEY_BYTES, FIRST_ACCESS_REVISION, OriginAccess,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{
    AcceptedConnection, ClientConnection, ClientIo, HubPin, IncomingChannel,
};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;

mod constants;
pub mod raw;
pub mod signer;
pub use constants::*;

pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
pub fn keys() -> ([u8; ED25519_PUBLIC_KEY_BYTES], Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    (
        signing.public_key().as_ref().try_into().unwrap(),
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}
pub fn configuration(
    entry_hub: HubId,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
) -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub,
        revision: FIRST_ACCESS_REVISION,
        clients: vec![ClientGrant {
            identity: ClientIdentity {
                client_id: uuid::Uuid::new_v4(),
                public_key,
            },
            origins: vec![OriginAccess {
                origin: entry_hub,
                rights: [Right::TerminalRead].into_iter().collect(),
            }],
            expires_at_ms: now_ms() + u64::try_from(GRANT_LIFETIME.as_millis()).unwrap(),
        }],
    }
}
pub async fn sockets() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, DYNAMIC_PORT)))
        .await
        .unwrap();
    let (connected, accepted) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    (connected.unwrap(), accepted.unwrap().0)
}
pub async fn pair() -> (
    ClientConnection,
    AcceptedConnection,
    mpsc::Receiver<IncomingChannel>,
    AccessRegistry,
) {
    let (client_public, client_key) = keys();
    let (hub_public, hub_key) = keys();
    let pin = HubPin {
        hub_id: HubId::new(),
        public_key: hub_public,
    };
    let registry = AccessRegistry::new(configuration(pin.hub_id, client_public)).unwrap();
    let (client, server) = sockets().await;
    let (connected, accepted) = tokio::join!(
        yolo_hub_client_transport::connect(
            ClientIo::from_tcp(client).unwrap(),
            client_key,
            pin,
            deadline()
        ),
        yolo_hub_client_transport::accept(
            ClientIo::from_tcp(server).unwrap(),
            hub_key,
            registry.clone(),
            deadline()
        ),
    );
    let (accepted, incoming) = accepted.unwrap();
    let connected = connected.unwrap();
    assert_eq!(connected.hub(), pin);
    assert_eq!(accepted.admission().entry_hub(), pin.hub_id);
    (connected, accepted, incoming, registry)
}
