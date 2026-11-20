use super::{LIVE_SOCKET, POLL, PROBE_TIMEOUT, deadline};
use crate::{Harness, hub::catalog_gate::CatalogGate};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{Instant, timeout_at},
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation::Federation;
use yolo_hub_federation_settings::{Configuration, PeerAddress};
use yolo_hub_identity::{AgentAddress, HubId};
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer, PeerIo};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;
use yolo_paths::socket_activation::{self, SocketPublication};

/// In-process tests dial the peer socket directly; production uses the broker.
pub struct DirectUnixDialer;

impl PeerDialer for DirectUnixDialer {
    fn dial<'a>(
        &'a self,
        _: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            let stream = timeout_at(
                deadline,
                tokio::net::UnixStream::connect(&address.socket),
            )
            .await??;
            Ok(PeerIo::new(stream, || {}))
        })
    }
}

pub struct RunningNode {
    pub node: Arc<Federation>,
    pub catalog_gate: Arc<CatalogGate>,
    pub socket: PathBuf,
    pub(super) stop: watch::Sender<bool>,
    pub(super) local: Option<JoinHandle<anyhow::Result<()>>>,
    pub(super) network: Option<JoinHandle<anyhow::Result<()>>>,
    pub(super) _publication: SocketPublication,
}
impl RunningNode {
    pub async fn start(harness: &Harness, state: Arc<HubState>, config: Configuration) -> Self {
        let backend = crate::hub::files_runtime::prepared(harness, &state, &config).await;
        Self::start_prepared(state, config, backend).await
    }
    pub async fn client(&self) -> yolo_hub_client::LocalClient {
        yolo_hub_client::Client::connect_local(&self.socket, self.node.hub_id(), deadline())
            .await
            .unwrap()
    }
    pub async fn agents(&self) -> Vec<AgentSnapshot> {
        let mut client = self.client().await;
        let Reply::Agents { agents, next: None } = client
            .call(
                self.node.hub_id(),
                Command::ListAgents { after: None },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        agents
    }
    pub async fn wait_origins(&self, expected: &[HubId]) -> BTreeSet<AgentAddress> {
        let expected: BTreeSet<_> = expected.iter().copied().collect();
        timeout_at(deadline(), async {
            loop {
                let agents = self.agents().await;
                let origins = agents
                    .iter()
                    .map(|agent| agent.cursor().target().address.hub_id)
                    .collect::<BTreeSet<_>>();
                let current = agents.iter().all(|agent| {
                    agent.fields().freshness == yolo_hub_agent_protocol::Freshness::Current
                        && agent.fields().reachability
                            == yolo_hub_agent_protocol::Reachability::Reachable
                });
                if origins == expected && current && agents.len() == expected.len() {
                    return agents
                        .iter()
                        .map(|agent| agent.cursor().target().address)
                        .collect();
                }
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap()
    }
    pub async fn stop(mut self) {
        self.stop.send_replace(true);
        timeout_at(deadline(), self.local.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        timeout_at(deadline(), self.network.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
impl Drop for RunningNode {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        if let Some(task) = self.local.take() {
            task.abort();
        }
        if let Some(task) = self.network.take() {
            task.abort();
        }
    }
}

pub async fn publish(state: &HubState) -> (tokio::net::UnixListener, SocketPublication) {
    socket_activation::publish(
        state.directory().unwrap(),
        &state.peer_socket().unwrap(),
        PROBE_TIMEOUT,
        LIVE_SOCKET,
    )
    .await
    .unwrap()
}
