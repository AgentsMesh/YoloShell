#[path = "../test_support/node_inventory.rs"]
mod support;
use support::NodeFixture;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunState, AgentRunSummary};
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_identity::{
    AgentRunId, HubId, TerminalAddress, TerminalId, WorkspaceAddress, WorkspaceId,
};
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_rpc_protocol_test_support::fixtures::{list_agent_runs, list_workspaces};
use yolo_hub_workspace_protocol::{WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary};

fn run(hub: HubId) -> AgentRunSummary {
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(Uuid::from_u128(1)).unwrap(),
        },
        revision: 1,
        name: "observed-1".into(),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: "codex".into(),
        workspace: None,
        parent: None,
        terminal: Some(TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(1)).unwrap(),
        }),
        state: AgentRunState::Launched,
        current: None,
        start_token: Uuid::from_u128(2),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn workspace(hub: HubId) -> WorkspaceSummary {
    WorkspaceSummary {
        address: WorkspaceAddress {
            hub_id: hub,
            workspace_id: WorkspaceId::from_uuid(Uuid::from_u128(1)).unwrap(),
        },
        revision: 1,
        name: "peer-workspace".into(),
        root: "/tmp/peer-workspace".into(),
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

#[tokio::test]
async fn public_entry_aggregates_peer_runs_and_workspaces() {
    let fixture = NodeFixture::with_peer_data(|hub| vec![run(hub)], |hub| vec![workspace(hub)]).await;
    let request = Request::new(
        fixture.node.hub_id(),
        Command::ListAgentRuns {
            request: list_agent_runs(),
        },
    )
    .unwrap();
    let Operation::Reply(reply) = fixture.node.dispatch(request.clone()).await.unwrap() else {
        panic!()
    };
    assert!(
        Response::new(&request, fixture.node.hub_id(), reply.clone())
            .validate_for(&request)
            .is_ok()
    );
    let Reply::AgentRuns { list } = reply else {
        panic!()
    };
    assert_eq!(list.runs, vec![run(fixture.origin.identity.hub())]);
    assert_eq!(list.next, None);

    let request = Request::new(
        fixture.node.hub_id(),
        Command::ListWorkspaces {
            request: list_workspaces(),
        },
    )
    .unwrap();
    let Operation::Reply(reply) = fixture.node.dispatch(request.clone()).await.unwrap() else {
        panic!()
    };
    assert!(
        Response::new(&request, fixture.node.hub_id(), reply.clone())
            .validate_for(&request)
            .is_ok()
    );
    let Reply::Workspaces { list } = reply else {
        panic!()
    };
    assert_eq!(
        list.workspaces,
        vec![workspace(fixture.origin.identity.hub())]
    );
    assert_eq!(list.next, None);
}
