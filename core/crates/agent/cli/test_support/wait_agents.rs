use super::fixture::Fixture;
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, Lifecycle, ObservationCursor,
};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, RuntimeEpoch, RuntimeTarget, TerminalAddress,
    TerminalId,
};
use yolo_hub_terminal_protocol::TerminalSummary;

/// A second live agent on the same Hub, so one wait can name both.
pub fn second_live_agent(fixture: &Fixture) -> (TerminalSummary, AgentSnapshot) {
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: fixture.hub,
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    let address = TerminalAddress {
        hub_id: fixture.hub,
        terminal_id: TerminalId::new(),
    };
    let mut fields = fixture
        .agent(Activity::WaitingForInput, Lifecycle::Running {})
        .fields()
        .clone();
    fields.cursor = ObservationCursor::new(target, uuid::Uuid::new_v4(), 1).unwrap();
    fields.terminal = RuntimeTarget {
        address,
        runtime_epoch: RuntimeEpoch::new(),
    };
    let agent = AgentSnapshot::new(fields).unwrap();
    let mut terminal = fixture.terminal("flow-2").fields().clone();
    terminal.address = address;
    if let Some(runtime) = terminal.runtime.as_mut() {
        runtime.target.address = address;
    }
    terminal.agent = Some(target);
    (TerminalSummary::new(terminal).unwrap(), agent)
}
