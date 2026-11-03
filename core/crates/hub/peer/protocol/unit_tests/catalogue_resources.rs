use crate::{fixtures::directory::*, support::NOW};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunState, AgentRunSummary};
use yolo_hub_identity::{
    AgentRunId, HubId, TerminalAddress, TerminalId, WorkspaceAddress, WorkspaceId,
};
use yolo_hub_peer_protocol::{Purpose, Signed};
use yolo_hub_workspace_protocol::{
    WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary,
};

fn run(hub: HubId, n: u128, provider: &str) -> AgentRunSummary {
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(Uuid::from_u128(n)).unwrap(),
        },
        revision: 1,
        name: format!("observed-{n}"),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: provider.into(),
        workspace: None,
        parent: None,
        terminal: Some(TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(n)).unwrap(),
        }),
        state: AgentRunState::Launched,
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

#[test]
fn catalogue_runs_and_workspaces_are_origin_bound_sorted_and_typed() {
    let fixture = fixture(None);
    let origin = fixture.origin.identity;
    let hub = origin.hub();
    let mut catalogue = fixture.export.catalogue.payload.clone();
    catalogue.runs = vec![run(hub, 1, "codex"), run(hub, 2, "codex")];
    catalogue.workspaces = vec![workspace(hub, 1)];
    catalogue.validate(origin).unwrap();

    let mut export = fixture.export.clone();
    export.catalogue = Signed::new(catalogue.clone(), Purpose::Catalogue, &fixture.origin).unwrap();
    export
        .verify(fixture.entry.identity, fixture.relay.identity, origin, NOW)
        .unwrap();

    let foreign = HubId::from_uuid(Uuid::from_u128(9_999)).unwrap();
    let mut wrong_run = catalogue.clone();
    wrong_run.runs[0].address.hub_id = foreign;
    assert!(wrong_run.validate(origin).is_err());

    let mut unsorted = catalogue.clone();
    unsorted.runs.swap(0, 1);
    assert!(unsorted.validate(origin).is_err());

    let mut invalid_provider = catalogue.clone();
    invalid_provider.runs[0].provider_id = "Bad Provider".into();
    assert!(invalid_provider.validate(origin).is_err());

    let mut wrong_workspace = catalogue;
    wrong_workspace.workspaces[0].address.hub_id = foreign;
    assert!(wrong_workspace.validate(origin).is_err());
}
