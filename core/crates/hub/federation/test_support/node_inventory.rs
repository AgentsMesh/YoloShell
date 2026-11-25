#[path = "constants.rs"]
mod constants;
use crate::Federation;
use constants::*;
use std::sync::Arc;
use yolo_hub_delegation::{DelegationChain, SignedGrant};
use yolo_hub_agent_run_protocol::AgentRunSummary;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::HubId;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_protocol::{
    CATALOGUE_SCHEMA_VERSION, Catalogue, ExportedOrigin, FIRST_REVISION,
    MAX_CATALOGUE_LIFETIME_MS, Purpose, Signed,
};
use yolo_hub_peer_protocol_test_support::support::{Identity, child_claims, root_claims};
use yolo_hub_state::HubState;
use yolo_hub_workspace_protocol::WorkspaceSummary;

pub struct NodeFixture {
    pub node: Arc<Federation>,
    pub origin: Identity,
    pub relay: Identity,
    pub link: uuid::Uuid,
    pub host_socket: std::path::PathBuf,
    _root: tempfile::TempDir,
}

impl NodeFixture {
    pub async fn new() -> Self {
        Self::with_resource_access(&[]).await
    }
    pub async fn with_resource_access(commands: &[yolo_hub_rpc_protocol::Command]) -> Self {
        Self::build(commands, |_| Vec::new(), |_| Vec::new()).await
    }
    pub async fn with_peer_data(
        runs: impl FnOnce(HubId) -> Vec<AgentRunSummary>,
        workspaces: impl FnOnce(HubId) -> Vec<WorkspaceSummary>,
    ) -> Self {
        Self::build(&[], runs, workspaces).await
    }
    async fn build(
        commands: &[yolo_hub_rpc_protocol::Command],
        runs: impl FnOnce(HubId) -> Vec<AgentRunSummary>,
        workspaces: impl FnOnce(HubId) -> Vec<WorkspaceSummary>,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().canonicalize().unwrap();
        let home = path.join(HOME_DIRECTORY);
        let host_socket = home.join(HOST_SOCKET);
        let state = Arc::new(HubState::open(&path.join(STATE_DIRECTORY), &home).unwrap());
        let backend = LocalBackend::new(state.hub_id(), &home, &host_socket).unwrap();
        let (origin, relay) = (Identity::new(), Identity::new());
        let config = Configuration {
            trusted_origins: vec![origin.identity, relay.identity],
            file_roots: vec![],
            ..Configuration::default()
        };
        let node = Federation::new(state.clone(), backend, config)
            .await
            .unwrap();
        let now = crate::node::now_ms().unwrap();
        let mut claims = root_claims(&origin, &relay);
        for command in commands {
            claims.rights = claims.rights.union(
                &yolo_hub_peer_protocol::required_rights(command).expect("remote command rights"),
            );
        }
        claims.not_before_ms = now;
        claims.expires_at_ms = now + MAX_CATALOGUE_LIFETIME_MS;
        let root_grant = SignedGrant::issue(claims, &origin).unwrap();
        let mut claims = child_claims(&root_grant, &relay);
        claims.recipient = state.peer_identity();
        let child = SignedGrant::issue(claims, &relay).unwrap();
        let catalogue = Catalogue {
            schema_version: CATALOGUE_SCHEMA_VERSION,
            revision: FIRST_REVISION,
            policy_epoch: root_grant.fields().claims.policy_epoch,
            issued_at_ms: now,
            expires_at_ms: now + MAX_CATALOGUE_LIFETIME_MS,
            terminals: Vec::new(),
            agents: Vec::new(),
            runs: runs(origin.identity.hub()),
            workspaces: workspaces(origin.identity.hub()),
        };
        let export = ExportedOrigin {
            catalogue: Signed::new(catalogue, Purpose::Catalogue, &origin).unwrap(),
            authority: DelegationChain::new(vec![root_grant, child]).unwrap(),
            reachable: true,
        };
        let link = uuid::Uuid::new_v4();
        {
            let mut book = node.book.lock().await;
            book.connect(relay.identity, link).unwrap();
            book.install(relay.identity, link, FIRST_REVISION, vec![export], now)
                .unwrap();
        }
        Self {
            node,
            origin,
            relay,
            link,
            host_socket,
            _root: root,
        }
    }
}
