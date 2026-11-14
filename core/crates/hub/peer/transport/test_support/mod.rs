use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Instant,
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_hub_peer_transport::{PeerConnection, PeerPins, PeerStream};

pub const TEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);
pub const PAYLOAD: &[u8] = b"peer\0terminal\r\n\x1b[31mdata";
const DYNAMIC_PORT: u16 = 0;

pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn keys() -> (PeerIdentity, Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    let identity = PeerIdentity::new(
        HubId::new(),
        signing.public_key().as_ref().try_into().unwrap(),
    )
    .unwrap();
    (
        identity,
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}

pub async fn sockets() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        DYNAMIC_PORT,
    ))
    .await
    .unwrap();
    let address = listener.local_addr().unwrap();
    let (connected, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
    (connected.unwrap(), accepted.unwrap().0)
}

pub async fn pair() -> (
    (PeerConnection, mpsc::Receiver<PeerStream>),
    (PeerConnection, mpsc::Receiver<PeerStream>),
) {
    let (left_id, left_key) = keys();
    let (right_id, right_key) = keys();
    let (left, right) = sockets().await;
    let (left, right) = tokio::join!(
        yolo_hub_peer_transport::connect(left, left_key, right_id, deadline()),
        yolo_hub_peer_transport::accept(
            right,
            right_key,
            PeerPins::new(vec![left_id]).unwrap(),
            deadline()
        ),
    );
    let (left, right) = (left.unwrap(), right.unwrap());
    assert_eq!(left.0.peer(), right_id);
    assert_eq!(right.0.peer(), left_id);
    (left, right)
}
