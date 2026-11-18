use super::{constants::*, peer::DirectoryPeer};
use crate::{Federation, registry::Link};
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::{Instant, timeout},
};
use yolo_hub_federation_settings::{Configuration, Peer};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_peer_transport::{PeerConnection, PeerPins, PeerStream};
use yolo_hub_state::HubState;

#[path = "constants.rs"]
mod paths;

pub struct LinkFixture {
    pub node: Arc<Federation>,
    pub link: Arc<Link>,
    pub peer: Arc<PeerConnection>,
    pub responder: Arc<DirectoryPeer>,
    pub incoming: Option<mpsc::Receiver<PeerStream>>,
    pub peer_incoming: Option<mpsc::Receiver<PeerStream>>,
    pub output: PeerStream,
    pub input: PeerStream,
    _root: tempfile::TempDir,
}

impl LinkFixture {
    pub async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let local_path = path.join(LOCAL_DIRECTORY);
        let peer_path = path.join(PEER_DIRECTORY);
        let home = local_path.join(paths::HOME_DIRECTORY);
        let local =
            Arc::new(HubState::open(&local_path.join(paths::STATE_DIRECTORY), &home).unwrap());
        let peer_state = Arc::new(
            HubState::open(
                &peer_path.join(paths::STATE_DIRECTORY),
                &peer_path.join(paths::HOME_DIRECTORY),
            )
            .unwrap(),
        );
        let backend =
            LocalBackend::new(local.hub_id(), &home, &home.join(paths::HOST_SOCKET)).unwrap();
        let node = Federation::new(
            local.clone(),
            backend,
            Configuration {
                peers: vec![Peer {
                    identity: peer_state.peer_identity(),
                    dial: None,
                    consume: true,
                    exports: Vec::new(),
                }],
                ..Configuration::default()
            },
        )
        .await
        .unwrap();
        let listener = TcpListener::bind(TEST_LISTEN_ADDRESS).await.unwrap();
        let (local_socket, peer_socket) = tokio::join!(
            TcpStream::connect(listener.local_addr().unwrap()),
            listener.accept(),
        );
        let deadline = Instant::now() + TEST_TIMEOUT;
        let (local_connection, peer_connection) = tokio::join!(
            yolo_hub_peer_transport::connect(
                local_socket.unwrap(),
                node.key.clone(),
                peer_state.peer_identity(),
                deadline
            ),
            yolo_hub_peer_transport::accept(
                peer_socket.unwrap().0,
                Arc::new(peer_state.transport_key().unwrap()),
                PeerPins::new(vec![local.peer_identity()]).unwrap(),
                deadline
            ),
        );
        let (local_connection, incoming) = local_connection.unwrap();
        let (peer_connection, mut peer_incoming) = peer_connection.unwrap();
        let link = node
            .register(local_connection, local.hub_id())
            .await
            .unwrap();
        let responder = Arc::new(DirectoryPeer::new(peer_state, local.peer_identity()));
        node.book
            .lock()
            .await
            .install(
                link.connection.peer(),
                link.id,
                FIRST_REVISION,
                vec![responder.export.clone()],
                crate::node::now_ms().unwrap(),
            )
            .unwrap();
        let output = link.connection.open_channel(deadline).await.unwrap();
        let input = timeout(TEST_TIMEOUT, peer_incoming.recv())
            .await
            .unwrap()
            .unwrap();
        Self {
            node,
            link,
            peer: Arc::new(peer_connection),
            responder,
            incoming: Some(incoming),
            peer_incoming: Some(peer_incoming),
            output,
            input,
            _root: root,
        }
    }
}
