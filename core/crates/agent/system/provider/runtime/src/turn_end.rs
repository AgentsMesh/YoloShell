use yolo_agent_contract::AgentBinding;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnEndKind {
    Completed,
    Aborted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnEndRecord {
    pub turn_id: String,
    pub kind: TurnEndKind,
}

/// Provider-owned structured turn-ending record. A probe never infers a turn
/// end from PTY activity or free-form transcript text.
pub trait TurnEndProbe: Send + Sync {
    fn read_turn_end(&self, binding: &AgentBinding) -> anyhow::Result<Option<TurnEndRecord>>;
}
