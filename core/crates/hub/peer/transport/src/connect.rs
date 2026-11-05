use crate::{
    Accepted, EnrollmentVerifier, PeerConnection, PeerIo, PeerPins, PeerStream,
    client::{self, ClientHandler},
    connection::Role,
    constants::*,
    server::{self, AuthEvent, ServerHandler},
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
            peer: Some(peer),
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
    match accept_io_with_enrollment(transport, key, pins, None, deadline).await? {
        Accepted::Peer {
            connection,
            incoming,
        } => Ok((connection, incoming)),
        Accepted::Enrollment { .. } => Err(anyhow::anyhow!(AUTH_FAILED)),
    }
}

pub async fn accept_with_enrollment(
    stream: TcpStream,
    key: Arc<PrivateKey>,
    pins: PeerPins,
    verifier: Option<Arc<dyn EnrollmentVerifier>>,
    deadline: Instant,
) -> Result<Accepted> {
    accept_io_with_enrollment(PeerIo::from_tcp(stream)?, key, pins, verifier, deadline).await
}

pub(crate) async fn accept_io_with_enrollment(
    transport: PeerIo,
    key: Arc<PrivateKey>,
    pins: PeerPins,
    verifier: Option<Arc<dyn EnrollmentVerifier>>,
    deadline: Instant,
) -> Result<Accepted> {
    let (stream, transport) = transport.into_parts();
    let enrollment_enabled = verifier.is_some();
    let capacity = Arc::new(Semaphore::new(MAX_PEER_CHANNELS));
    let closed = Arc::new(AtomicBool::new(false));
    let (incoming, incoming_rx) = mpsc::channel(MAX_QUEUED_CHANNELS);
    let (enrollment, enrollment_rx) = mpsc::channel(MAX_QUEUED_CHANNELS);
    let (authenticated, mut events) = mpsc::channel(AUTH_EVENT_CAPACITY);
    let handler = ServerHandler {
        pins,
        authenticated,
        peer: None,
        invite: None,
        verifier,
        incoming,
        enrollment,
        capacity: capacity.clone(),
        closed: closed.clone(),
    };
    let running = timeout_at(
        deadline,
        russh::server::run_stream(server::config(key, enrollment_enabled), stream, handler),
    )
    .await??;
    let event = timeout_at(deadline, events.recv())
        .await?
        .context(AUTH_FAILED)?;
    let role = Role::Server {
        handle: running.handle(),
        running: Mutex::new(Some(running)),
    };
    let peer = match event {
        AuthEvent::Peer(peer) => Some(peer),
        AuthEvent::Enrollment => None,
    };
    let connection = PeerConnection {
        peer,
        role,
        transport,
        capacity,
        closed,
    };
    Ok(match event {
        AuthEvent::Peer(_) => Accepted::Peer {
            connection,
            incoming: incoming_rx,
        },
        AuthEvent::Enrollment => Accepted::Enrollment {
            connection,
            incoming: enrollment_rx,
        },
    })
}
