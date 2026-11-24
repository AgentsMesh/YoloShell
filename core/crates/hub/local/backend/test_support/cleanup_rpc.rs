use super::fixtures::target;
use crate::LocalBackend;
use std::future::Future;
use std::time::Duration;
use tokio::net::UnixListener;
use yolo_hub_terminal_protocol::{CreateShellResult, SecretCapability};
use yolo_runtime::agent::{
    AgentKind,
    provider_contract::{ProjectionPolicies, ProjectionPolicy},
};
use yolo_runtime::protocol::{ProtocolError, RequestEnvelope, ResponseEnvelope, validate_response};
use yolo_runtime_rpc_testing::Peer;

const SOCKET_NAME: &str = "cleanup.sock";
const RPC_TIMEOUT: Duration = Duration::from_secs(5);
const AGENT_STORE: &str = "agent-store";
const PROJECTION_RECORD: &str = "agent-projection.json";
const CONNECTOR_STORE: &str = "connector-store";
const CONNECTOR_KEY_BYTE: u8 = 41;
const CONNECTOR_INDEX_FILE: &str = "index.json";

pub struct ProtocolFixture {
    listener: UnixListener,
    pub backend: LocalBackend,
    _root: tempfile::TempDir,
}

impl ProtocolFixture {
    /// The persisted projection record, to prove a steady catalogue does not
    /// rewrite it.
    pub fn projection_record(&self) -> Vec<u8> {
        let path = self
            ._root
            .path()
            .canonicalize()
            .unwrap()
            .join(AGENT_STORE)
            .join(PROJECTION_RECORD);
        std::fs::read(path).unwrap()
    }

    pub fn invalidate_connector_storage(&self) {
        let path = self
            ._root
            .path()
            .canonicalize()
            .unwrap()
            .join(CONNECTOR_STORE)
            .join(CONNECTOR_INDEX_FILE);
        let value: serde_json::Value = yolo_paths::read_json(&path).unwrap();
        yolo_paths::atomic_write_json(&path, &value).unwrap();
    }

    pub fn with_connectors(mut self) -> Self {
        let directory = yolo_paths::directory::PrivateDirectory::ensure(
            &self
                ._root
                .path()
                .canonicalize()
                .unwrap()
                .join(CONNECTOR_STORE),
        )
        .unwrap();
        let identity = yolo_hub_delegation::PeerIdentity::new(
            self.backend.hub_id(),
            [CONNECTOR_KEY_BYTE; yolo_hub_delegation::ED25519_PUBLIC_KEY_BYTES],
        )
        .unwrap();
        self.backend = self.backend.with_connectors(directory, identity).unwrap();
        self
    }

    pub async fn without_request<F: Future>(&self, future: F) -> F::Output {
        bounded(async {
            tokio::select! {
                biased;
                _ = self.listener.accept() => panic!("unexpected Host request before connector authentication"),
                result = future => result,
            }
        }).await
    }

    pub fn with_agents(mut self) -> Self {
        let directory = yolo_paths::directory::PrivateDirectory::ensure(
            &self._root.path().canonicalize().unwrap().join(AGENT_STORE),
        )
        .unwrap();
        let mut policies = ProjectionPolicies::none();
        policies.insert(AgentKind::Codex, ProjectionPolicy::HOOK_AUTHORITATIVE);
        policies.insert(AgentKind::ClaudeCode, ProjectionPolicy::HOOK_AUTHORITATIVE);
        self.backend = self.backend.with_agents(directory, policies).unwrap();
        self
    }

    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().canonicalize().unwrap();
        let socket = home.join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        // A protocol-only peer verifies adapter payloads, not real custody outcomes.
        let backend = LocalBackend {
            hub: target().address.hub_id,
            client: yolo_client::Client::new(socket),
            host_connections: yolo_hub_host_connections::HostConnections::default(),
            paths: yolo_paths::Paths::new(&home),
            agents: None,
            // Protocol fixtures answer one Host request per call, so no reuse.
            catalogue: std::sync::Arc::new(yolo_hub_host_catalogue::CatalogueCache::new(
                Duration::ZERO,
            )),
            conversations: std::sync::Arc::new(yolo_conversation_service::ConversationReader::new(
                yolo_conversation_service::ConversationSources::new([
                    std::sync::Arc::new(yolo_codex_provider::CodexConversationSource)
                        as std::sync::Arc<dyn yolo_conversation_source::ConversationSource>,
                    std::sync::Arc::new(yolo_claude_provider::ClaudeConversationSource),
                ]),
            )),
            layouts: None,
            connectors: None,
            connector_observation_gate: std::sync::Arc::new(tokio::sync::Mutex::new(())),
            observation_gate: std::sync::Arc::new(tokio::sync::Mutex::new(())),
            file_helper: None,
            file_epoch: yolo_hub_identity::FileRootEpoch::new(),
            files: None,
            launch: None,
            platform: None,
            audit: None,
        };
        Self {
            listener,
            backend,
            _root: root,
        }
    }

    pub async fn reply(&self, response: impl FnOnce(&RequestEnvelope) -> ResponseEnvelope) {
        bounded(async {
            let (mut peer, request) = self.accept_request().await;
            let response = response(&request);
            let valid = validate_response(&request, &response).is_ok();
            let observed = peer.respond(&response).await;
            if valid {
                observed.unwrap();
            } else {
                assert!(matches!(observed, Err(ProtocolError::EndOfStream)));
            }
        })
        .await;
    }

    pub async fn accept_request(&self) -> (Peer, RequestEnvelope) {
        bounded(async {
            let (stream, _) = self.listener.accept().await.unwrap();
            let mut peer = Peer::new(stream);
            let request = peer.request().await.unwrap();
            (peer, request)
        })
        .await
    }

    pub fn assert_no_pending_connection(&self) {
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(
            self.listener.poll_accept(&mut context).is_pending(),
            "{UNEXPECTED_CONNECTOR_REQUEST}"
        );
    }
}

const UNEXPECTED_CONNECTOR_REQUEST: &str =
    "new Connector admission crossed an unresolved Host observation";

pub fn creation() -> CreateShellResult {
    CreateShellResult {
        address: target().address,
        creation_capability: SecretCapability::generate(),
    }
}

pub async fn bounded<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(RPC_TIMEOUT, future).await.unwrap()
}
