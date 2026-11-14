use crate::fixtures::Fixture;
use std::collections::BTreeMap;
use yolo_hub_identity::{AgentTarget, TerminalAddress, TerminalId, WorkspaceAddress, WorkspaceId};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_hub_workspace_protocol::{WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary};

fn summary(
    fixture: &Fixture,
    address: TerminalAddress,
    workspace: WorkspaceAddress,
    agent: Option<AgentTarget>,
) -> TerminalSummary {
    let mut fields = fixture.session.fields().clone();
    fields.address = address;
    fields.workspace = Some(workspace);
    fields.agent = agent;
    if let Some(runtime) = fields.runtime.as_mut() {
        runtime.target = yolo_hub_identity::RuntimeTarget {
            address,
            runtime_epoch: runtime.target.runtime_epoch,
        };
    }
    TerminalSummary::new(fields).unwrap()
}

#[test]
fn workspace_filter_scopes_terminals_and_agents() {
    let fixture = Fixture::new();
    let hub = fixture.hub.hub_id;
    let first = WorkspaceAddress {
        hub_id: hub,
        workspace_id: WorkspaceId::new(),
    };
    let second = WorkspaceAddress {
        hub_id: hub,
        workspace_id: WorkspaceId::new(),
    };
    let first_address = fixture.session.fields().address;
    let second_address = TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::new(),
    };
    let terminals: BTreeMap<_, _> = [
        (first_address, summary(&fixture, first_address, first, fixture.session.fields().agent)),
        (second_address, summary(&fixture, second_address, second, None)),
    ]
    .into_iter()
    .collect();
    let mut inventory = fixture.inventory();
    assert_eq!(inventory.visible_terminals("", &terminals).len(), 2);
    assert_eq!(inventory.visible_agents("", &terminals).len(), 1);

    inventory.workspace = Some(first);
    let label = inventory.workspace_status(&terminals);
    assert!(label.contains("Workspace filter:"));
    let visible = inventory.visible_terminals("", &terminals);
    assert_eq!(visible.len(), 1);
    let agents = inventory.visible_agents("", &terminals).len();
    assert_eq!(agents, usize::from(visible[0] == first_address));

    inventory.workspace = None;
    assert_eq!(inventory.visible_terminals("", &terminals).len(), 2);

    inventory.replace_workspaces(
        [(first, "checkout"), (second, "scratch")]
            .into_iter()
            .map(|(workspace, name)| WorkspaceSummary {
                address: workspace,
                revision: 1,
                name: name.into(),
                root: "/tmp/checkout".into(),
                kind: WorkspaceKind::Directory,
                parent: None,
                labels: BTreeMap::new(),
                member_terminals: 1,
                member_runs: 0,
                capabilities: WorkspaceCapabilities::unsupported("test"),
                created_at_ms: 0,
                updated_at_ms: 0,
            })
            .collect(),
    );
    inventory.workspace = Some(first);
    let label = inventory.workspace_status(&terminals);
    assert_eq!(label, "Workspace filter: checkout (1 terminals)");
}
