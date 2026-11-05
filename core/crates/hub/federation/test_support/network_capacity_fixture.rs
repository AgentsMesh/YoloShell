use super::{constants::*, peer::DirectoryPeer};
use crate::{Federation, registry::Link};
use std::sync::Arc;
use tokio::time::Instant;
use yolo_hub_federation_settings::{Configuration, Peer, PeerAddress};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_link::{Admission, accept, connect};
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_state::HubState;

#[path = "constants.rs"]
mod paths;

pub struct LinkFixture {
    pub node: Arc<Federation>,
    pub link: Arc<Link>,
    pub responder: Arc<DirectoryPeer>,
    pub server: tokio::task::JoinHandle<()>,
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
                    dial: Some(
                        PeerAddress::ssh(
                            "in-process",
                            peer_state.peer_socket().unwrap(),
                        )
                        .unwrap(),
                    ),
                    consume: true,
                    exports: Vec::new(),
                }],
                ..Configuration::default()
            },
        )
        .await
        .unwrap();
        let responder = Arc::new(DirectoryPeer::new(peer_state.clone(), local.peer_identity()));
        let (local_stream, peer_stream) = tokio::io::duplex(DUPLEX_BYTES);
        let deadline = Instant::now() + TEST_TIMEOUT;
        let local_identity = local.peer_identity();
        let remote_identity = peer_state.peer_identity();
        let served = responder.clone();
        let server_state = peer_state.clone();
        let server = tokio::spawn(async move {
            let (_, stream) = accept(
                peer_stream,
                remote_identity,
                server_state.as_ref(),
                Admission::Only(&[local_identity]),
                deadline,
            )
            .await
            .unwrap();
            let _ = yolo_hub_peer_rpc::serve(stream, served).await;
        });
        let client = connect(
            local_stream,
            local_identity,
            local.as_ref(),
            remote_identity,
            deadline,
        )
        .await
        .unwrap();
        let link = node
            .register(remote_identity, client)
            .await
            .unwrap();
        node.book
            .lock()
            .await
            .install(
                link.peer,
                link.id,
                FIRST_REVISION,
                vec![responder.export.clone()],
                crate::node::now_ms().unwrap(),
            )
            .unwrap();
        Self {
            node,
            link,
            responder,
            server,
            _root: root,
        }
    }
}
