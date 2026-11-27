use crate::*;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunFilter, AgentRunSelector, GetAgentRunFields, ListAgentRunsFields,
    UpdateAgentRunFields,
};
use yolo_hub_identity::TerminalId;
use yolo_hub_terminal_protocol::PublicError;

mod adopt;
mod filters;
mod parent;
mod quota;
mod reconcile;
mod store;

fn fixture() -> (tempfile::TempDir, HubId, AgentRunStore) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let store = AgentRunStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    (root, hub, store)
}

fn reserve(store: &mut AgentRunStore, name: &str, token: Uuid) -> AgentRunSummary {
    store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: name.into(),
            provider_id: "codex".into(),
            workspace: None,
            parent: None,
            start_token: token,
        })
        .unwrap()
}

fn terminal() -> TerminalAddress {
    TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    }
}

fn get(store: &AgentRunStore, selector: AgentRunSelector) -> Option<AgentRunSummary> {
    let request = GetAgentRunRequest::new(GetAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        selector,
    })
    .unwrap();
    store.get(&request).unwrap()
}

fn update(
    store: &mut AgentRunStore,
    address: AgentRunAddress,
    revision: u64,
    name: Option<&str>,
    aliases: Option<Vec<String>>,
) -> Result<AgentRunSummary, StoreError> {
    let request = UpdateAgentRunRequest::new(UpdateAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address,
        expected_revision: revision,
        name: name.map(Into::into),
        aliases,
        labels: None,
    })
    .unwrap();
    store.update(&request)
}
