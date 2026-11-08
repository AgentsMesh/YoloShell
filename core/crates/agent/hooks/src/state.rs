use std::collections::BTreeSet;
use uuid::Uuid;

pub type ToolSet = BTreeSet<String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    Alive,
    Exited,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Unknown,
    Idle,
    Running,
    Compacting,
    StopCandidate,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionState {
    None,
    /// PermissionRequest precedes the provider's combined hook decision.
    PermissionCandidate,
    /// A provider notification confirms that the permission prompt is displayed.
    Permission,
    UserInput,
    Elicitation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentationState {
    Pending,
    Authoritative,
    Unavailable,
    Unmanaged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StopCandidate {
    pub event_id: Uuid,
    pub turn_id: Option<String>,
    pub continued: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationCandidate {
    Final,
    Clear,
    Resume,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildLifecycle {
    pub active: bool,
    pub turn: TurnState,
    pub attention: AttentionState,
    pub active_tools: ToolSet,
    pub stop_candidate: Option<StopCandidate>,
}
