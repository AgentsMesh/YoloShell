use crate::{
    PeerConnection, PeerIo, PeerPins, PeerStream,
    client::{self, ClientHandler},
    connection::Role,
    constants::*,
    server::{self, ServerHandler},
};
use anyhow::{Context, Result, ensure};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::{
    net::TcpStream,
    sync::{Mutex, Semaphore, mpsc},
    time::{Instant, timeout_at},
};
use yolo_hub_delegation::PeerIdentity;

pub async fn connect(
    stream: TcpStream,
    key: Arc<PrivateKey>,
    peer: PeerIdentity,
    deadline: Instant,
) -> Result<(PeerConnection, mpsc::Receiver<PeerStream>)> {
    connect_io(PeerIo::from_tcp(stream)?, key, peer, deadline).await
}

pub async fn connect_io(
    transport: PeerIo,
    key: Arc<PrivateKey>,
    peer: PeerIdentity,
    deadline: Instant,
) -> Result<(PeerConnection, mpsc::Receiver<PeerStream>)> {
    let (stream, transport) = transport.into_parts();
    let capacity = Arc::new(Semaphore::new(MAX_PEER_CHANNELS));
    let closed = Arc::new(AtomicBool::new(false));
    let (incoming, receiver) = mpsc::channel(MAX_QUEUED_CHANNELS);
    let handler = ClientHandler {
        peer,
        incoming,
        capacity: capacity.clone(),
        closed: closed.clone(),
    };
    let mut handle = timeout_at(
        deadline,
        russh::client::connect_stream(client::config(), stream, handler),
    )
    .await??;
    let result = timeout_at(
        deadline,
        handle.authenticate_publickey(PEER_USER, PrivateKeyWithHashAlg::new(key, None)),
    )
    .await??;
    ensure!(result.success(), AUTH_FAILED);
    Ok((
        PeerConnection {
            peer,
            role: Role::Client(Mutex::new(Some(handle))),
            transport,
            capacity,
            closed,
        },
        receiver,
    ))
}

pub async fn accept(
    stream: TcpStream,
    key: Arc<PrivateKey>,
    pins: PeerPins,
    deadline: Instant,
) -> Result<(PeerConnection, mpsc::Receiver<PeerStream>)> {
    accept_io(PeerIo::from_tcp(stream)?, key, pins, deadline).await
}

pub async fn accept_io(
    transport: PeerIo,
    key: Arc<PrivateKey>,
    pins: PeerPins,
    deadline: Instant,
) -> Result<(PeerConnection, mpsc::Receiver<PeerStream>)> {
    let (stream, transport) = transport.into_parts();
    let capacity = Arc::new(Semaphore::new(MAX_PEER_CHANNELS));
    let closed = Arc::new(AtomicBool::new(false));
    let (incoming, receiver) = mpsc::channel(MAX_QUEUED_CHANNELS);
    let (authenticated, mut authenticated_peer) = mpsc::channel(AUTH_EVENT_CAPACITY);
    let handler = ServerHandler {
        pins,
        authenticated,
        peer: None,
        incoming,
        capacity: capacity.clone(),
        closed: closed.clone(),
    };
    let running = timeout_at(
        deadline,
        russh::server::run_stream(server::config(key), stream, handler),
    )
    .await??;
    let peer = timeout_at(deadline, authenticated_peer.recv())
        .await?
        .context(AUTH_FAILED)?;
    let role = Role::Server {
        handle: running.handle(),
        running: Mutex::new(Some(running)),
    };
    Ok((
        PeerConnection {
            peer,
            role,
            transport,
            capacity,
            closed,
        },
        receiver,
    ))
}
