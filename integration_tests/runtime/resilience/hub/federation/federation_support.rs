use super::catalog_gate::CatalogGate;
#[path = "../connector/connector_federation_start.rs"]
mod prepared_start;
use crate::Harness;
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::NonZeroU16,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::watch,
    task::JoinHandle,
    time::{Instant, timeout_at},
};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation::Federation;
use yolo_hub_federation_settings::{Configuration, ExportPolicy, Peer, PeerAddress};
use yolo_hub_identity::{AgentAddress, HubId};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;

pub const GRAPH_TIMEOUT: Duration = Duration::from_secs(180);
pub const POLL: Duration = Duration::from_millis(100);
pub const TEMP_ROOT: &str = "/tmp";
pub const NAME_A: &str = "node-a-agent";
pub const NAME_B: &str = "node-b-agent";
pub const NAME_C: &str = "node-c-agent";
const DYNAMIC_PORT: u16 = 0;
const REEXPORT_ALLOWED: u8 = 1;
const LEAF_EXPORT: u8 = 0;
pub fn deadline() -> Instant {
    Instant::now() + GRAPH_TIMEOUT
}
pub async fn listener() -> TcpListener {
    TcpListener::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        DYNAMIC_PORT,
    ))
    .await
    .unwrap()
}
pub fn exports(origin: HubId, relay: bool) -> ExportPolicy {
    ExportPolicy {
        origin,
        rights: [
            Right::Observe,
            Right::TerminalRead,
            Right::TerminalWrite,
            Right::Lifecycle,
        ]
        .into_iter()
        .collect(),
        remaining_reexports: if relay { REEXPORT_ALLOWED } else { LEAF_EXPORT },
    }
}
pub fn peer(
    identity: PeerIdentity,
    dial: Option<SocketAddr>,
    consume: bool,
    exports: Vec<ExportPolicy>,
) -> Peer {
    Peer {
        identity,
        dial: dial.map(|address| {
            PeerAddress::tcp(
                address.ip().to_string(),
                NonZeroU16::new(address.port()).unwrap(),
            )
        }),
        consume,
        exports,
    }
}
pub struct RunningNode {
    pub node: Arc<Federation>,
    pub catalog_gate: Arc<CatalogGate>,
    pub socket: PathBuf,
    stop: watch::Sender<bool>,
    local: Option<JoinHandle<anyhow::Result<()>>>,
    network: Option<JoinHandle<anyhow::Result<()>>>,
}
impl RunningNode {
    pub async fn start(
        harness: &Harness,
        state: Arc<HubState>,
        config: Configuration,
        listener: TcpListener,
    ) -> Self {
        let backend = super::files_runtime::prepared(harness, &state, &config).await;
        Self::start_prepared(state, config, listener, backend).await
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
