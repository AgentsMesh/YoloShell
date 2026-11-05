use super::super::{
    broker,
    constants::*,
    fixture::{Fixture, deadline},
    lifecycle_constants::*,
};
use std::sync::Arc;
use tokio::{
    net::UnixListener,
    task::JoinHandle,
    time::{timeout, timeout_at},
};
use yolo_hub_federation_settings::{PeerAddress, SshProfileRef};
use yolo_hub_peer_dialer::dial_peer;
use yolo_hub_peer_link::{accept, connect};
use yolo_hub_peer_protocol::{FIRST_REVISION, PEER_PROTOCOL_VERSION, PeerReply, PeerRequest};
use yolo_hub_peer_rpc::{Client, Handler, Response, RpcFuture};
use yolo_hub_state::HubState;
use yolo_remote_broker::BrokerPaths;
use uuid::Uuid;

struct Echo;
impl Handler for Echo {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        Box::pin(async move {
            let PeerRequest::Discover { request_id, .. } = request else {
                return Err(yolo_hub_peer_rpc::RpcError::Invalid);
            };
            Ok(Response {
                reply: PeerReply::Directory {
                    request_id,
                    revision: FIRST_REVISION,
                    origins: Vec::new(),
                },
                updates: None,
            })
        })
    }
}

fn discover() -> PeerRequest {
    PeerRequest::Discover {
        request_id: Uuid::new_v4(),
        version: PEER_PROTOCOL_VERSION,
        allowed_origins: Vec::new(),
    }
}

async fn responds(client: &Client) -> bool {
    client.request(discover(), deadline()).await.is_ok()
}

pub struct Tunnels {
    clients: Vec<Client>,
    tasks: Vec<JoinHandle<()>>,
    _brokers: Vec<(BrokerPaths, UnixListener)>,
    _peer: Arc<HubState>,
}
impl Tunnels {
    pub async fn open(fixture: &Fixture) -> Self {
        let peer = Arc::new(
            HubState::open(
                &fixture.root.path().join(PEER_STATE_DIRECTORY),
                &fixture.root.path().join(PEER_HOST_DIRECTORY),
            )
            .unwrap(),
        );
        let brokers = (0..TUNNEL_COUNT)
            .map(|index| {
                broker::bind(
                    &fixture
                        .root
                        .path()
                        .join(format!("{BROKER_DIRECTORY}-{index}")),
                )
            })
            .collect::<Vec<_>>();
        let profiles = brokers
            .iter()
            .enumerate()
            .map(|(index, (paths, _))| {
                let mut profile = fixture.profile(paths.state_dir());
                profile.profile = SshProfileRef::new(
                    format!("{PROFILE_ID}-{index}"),
                    DIGEST_CHARACTER.repeat(DIGEST_BYTES),
                )
                .unwrap();
                profile
            })
            .collect::<Vec<_>>();
        fixture.apply(profiles.clone()).await;
        let mut clients = Vec::new();
        let mut tasks = Vec::new();
        for (profile, (_, listener)) in profiles.into_iter().zip(&brokers) {
            let address = PeerAddress::ssh(profile.profile.profile_id().to_owned(), SOCKET.into()).unwrap();
            let remote = peer.peer_identity();
            let (opened, accepted) = tokio::join!(
                dial_peer(fixture.runtime.as_ref(), remote, &address, deadline()),
                timeout_at(deadline(), broker::upgrade(listener, remote)),
            );
            let local = fixture.state.peer_identity();
            let stream = accepted.unwrap();
            let server_peer = peer.clone();
            let handler = Arc::new(Echo);
            tasks.push(tokio::spawn(async move {
                let (_, stream) = accept(stream, remote, &*server_peer, &[local], deadline())
                    .await
                    .unwrap();
                let _ = yolo_hub_peer_rpc::serve(stream, handler).await;
            }));
            let client = connect(
                opened.unwrap(),
                local,
                fixture.state.as_ref(),
                remote,
                deadline(),
            )
            .await
            .unwrap();
            assert!(responds(&client).await);
            clients.push(client);
        }
        Self {
            clients,
            tasks,
            _brokers: brokers,
            _peer: peer,
        }
    }
    /// Tunnel shutdown is synchronous; RPC closure becomes observable asynchronously.
    pub async fn require_closed(&mut self) {
        timeout(CLOSURE_OBSERVATION, async {
            loop {
                let mut live = false;
                for client in &self.clients {
                    if responds(client).await {
                        live = true;
                        break;
                    }
                }
                if !live {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        for task in &mut self.tasks {
            timeout(CLOSURE_OBSERVATION, task)
                .await
                .expect("peer RPC server did not observe tunnel closure")
                .expect("peer RPC server task failed");
        }
    }
    pub async fn finish(&mut self) {
        self.tasks.clear();
        self.clients.clear();
    }
}
