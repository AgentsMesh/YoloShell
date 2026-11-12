use yolo_hub_identity::{AgentAddress, AgentId, HubId, TerminalAddress, TerminalId};

pub trait Resource: Copy + Ord {
    type LocalId;
    fn origin(self) -> HubId;
    fn local(hub: HubId, id: Self::LocalId) -> Self;
}

impl Resource for AgentAddress {
    type LocalId = AgentId;
    fn origin(self) -> HubId {
        self.hub_id
    }
    fn local(hub_id: HubId, agent_id: AgentId) -> Self {
        Self { hub_id, agent_id }
    }
}
impl Resource for TerminalAddress {
    type LocalId = TerminalId;
    fn origin(self) -> HubId {
        self.hub_id
    }
    fn local(hub_id: HubId, terminal_id: TerminalId) -> Self {
        Self {
            hub_id,
            terminal_id,
        }
    }
}
impl Resource for HubId {
    type LocalId = ();
    fn origin(self) -> HubId {
        self
    }
    fn local(hub: HubId, _: ()) -> Self {
        hub
    }
}
