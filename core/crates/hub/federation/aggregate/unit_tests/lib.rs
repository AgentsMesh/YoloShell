use crate::{runs, workspaces};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunAddress, AgentRunFilter, AgentRunState,
    AgentRunSummary, ListAgentRunsFields, ListAgentRunsRequest, ListAgentRunsResult,
};
use yolo_hub_identity::{
    AgentRunId, HubId, TerminalAddress, TerminalId, WorkspaceAddress, WorkspaceId,
};
use yolo_hub_peer_protocol::{CATALOGUE_SCHEMA_VERSION, Catalogue};
use yolo_hub_workspace_protocol::{
    ListWorkspacesFields, ListWorkspacesRequest, ListWorkspacesResult, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
    WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary,
};

fn hub(n: u128) -> HubId {
    HubId::from_uuid(Uuid::from_u128(n)).unwrap()
}

fn run(hub: HubId, n: u128, state: AgentRunState) -> AgentRunSummary {
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(Uuid::from_u128(n)).unwrap(),
        },
        revision: 1,
        name: format!("run-{n}"),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: "codex".into(),
        workspace: None,
        parent: None,
        terminal: Some(TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(n)).unwrap(),
        }),
        state,
        current: None,
        start_token: Uuid::from_u128(n + 1_000),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn workspace(hub: HubId, n: u128) -> WorkspaceSummary {
    WorkspaceSummary {
        address: WorkspaceAddress {
            hub_id: hub,
            workspace_id: WorkspaceId::from_uuid(Uuid::from_u128(n)).unwrap(),
        },
        revision: 1,
        name: format!("ws-{n}"),
        root: format!("/tmp/ws-{n}"),
        kind: WorkspaceKind::Directory,
        parent: None,
        labels: Default::default(),
        member_terminals: 0,
        member_runs: 0,
        capabilities: WorkspaceCapabilities::unsupported("none"),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn catalogue(runs: Vec<AgentRunSummary>, workspaces: Vec<WorkspaceSummary>) -> Catalogue {
    Catalogue {
        schema_version: CATALOGUE_SCHEMA_VERSION,
        revision: 1,
        policy_epoch: 1,
        issued_at_ms: 0,
        expires_at_ms: 1,
        terminals: Vec::new(),
        agents: Vec::new(),
        runs,
        workspaces,
    }
}

fn runs_request(
    after: Option<AgentRunAddress>,
    limit: u16,
    filter: Option<AgentRunFilter>,
) -> ListAgentRunsRequest {
    ListAgentRunsRequest::new(ListAgentRunsFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        after,
        limit,
        filter,
    })
    .unwrap()
}

fn workspaces_request(after: Option<WorkspaceAddress>, limit: u16) -> ListWorkspacesRequest {
    ListWorkspacesRequest::new(ListWorkspacesFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        after,
        limit,
    })
    .unwrap()
}

#[test]
fn runs_merge_respects_the_filter_and_pages_the_union() {
    let local_hub = hub(1);
    let peer_hub = hub(2);
    let local = ListAgentRunsResult {
        runs: vec![
            run(local_hub, 1, AgentRunState::Launched),
            run(local_hub, 2, AgentRunState::Stopped),
        ],
        next: None,
    };
    let peer = catalogue(
        vec![
            run(peer_hub, 3, AgentRunState::Launched),
            run(peer_hub, 4, AgentRunState::Launched),
        ],
        Vec::new(),
    );
    let filter = AgentRunFilter::State {
        is: vec![AgentRunState::Launched],
    };
    let request = runs_request(None, 2, Some(filter.clone()));
    let page = runs(&request, local, std::slice::from_ref(&peer));
    assert_eq!(
        page.runs.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![
            run(local_hub, 1, AgentRunState::Launched).address,
            run(peer_hub, 3, AgentRunState::Launched).address,
        ]
    );
    let next = page.next.unwrap();
    let request = runs_request(Some(next), 2, Some(filter));
    let page = runs(
        &request,
        ListAgentRunsResult {
            runs: Vec::new(),
            next: None,
        },
        std::slice::from_ref(&peer),
    );
    assert_eq!(
        page.runs.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![run(peer_hub, 4, AgentRunState::Launched).address]
    );
    assert_eq!(page.next, None);
}

#[test]
fn runs_report_a_next_page_when_the_local_store_has_more() {
    let hub = hub(1);
    let local_run = run(hub, 1, AgentRunState::Launched);
    let local = ListAgentRunsResult {
        runs: vec![local_run.clone()],
        next: Some(local_run.address),
    };
    let page = runs(&runs_request(None, 8, None), local, &[]);
    assert_eq!(page.runs, vec![local_run.clone()]);
    assert_eq!(page.next, Some(local_run.address));
}

#[test]
fn workspaces_merge_dedupes_and_pages_by_address() {
    let local_hub = hub(1);
    let peer_hub = hub(2);
    let first = workspace(local_hub, 1);
    let second = workspace(peer_hub, 2);
    let local = ListWorkspacesResult {
        workspaces: vec![first.clone()],
        next: None,
    };
    let peer = catalogue(Vec::new(), vec![second.clone(), first.clone()]);
    let request = workspaces_request(None, 1);
    let page = workspaces(&request, local, std::slice::from_ref(&peer));
    assert_eq!(page.workspaces, vec![first.clone()]);
    assert_eq!(page.next, Some(first.address));
    let request = workspaces_request(Some(first.address), 1);
    let local = ListWorkspacesResult {
        workspaces: Vec::new(),
        next: None,
    };
    let page = workspaces(&request, local, std::slice::from_ref(&peer));
    assert_eq!(page.workspaces, vec![second]);
    assert_eq!(page.next, None);
}
