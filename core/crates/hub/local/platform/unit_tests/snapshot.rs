use crate::snapshot::annotate_current;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunAddress, AgentRunState, AgentRunSummary, FIRST_AGENT_RUN_REVISION,
};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentRunId, AgentTarget, ExecutionId, HubId, TerminalAddress, TerminalId,
};

fn run(hub: HubId, terminal: TerminalId) -> AgentRunSummary {
    AgentRunSummary {
        address: AgentRunAddress {
            hub_id: hub,
            run_id: AgentRunId::from_uuid(Uuid::new_v4()).unwrap(),
        },
        revision: FIRST_AGENT_RUN_REVISION,
        name: format!("run-{terminal}"),
        aliases: Vec::new(),
        labels: Default::default(),
        provider_id: "codex".into(),
        workspace: None,
        parent: None,
        terminal: Some(TerminalAddress {
            hub_id: hub,
            terminal_id: terminal,
        }),
        state: AgentRunState::Launched,
        current: None,
        start_token: Uuid::new_v4(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

#[test]
fn snapshot_maps_runs_to_their_observed_terminal_execution() {
    let hub = HubId::new();
    let terminal = TerminalId::new();
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: hub,
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    let mut runs = vec![run(hub, terminal), run(hub, TerminalId::new())];
    annotate_current(&mut runs, &[(terminal, target)]);
    assert_eq!(runs[0].current, Some(target));
    assert_eq!(runs[1].current, None);
}
