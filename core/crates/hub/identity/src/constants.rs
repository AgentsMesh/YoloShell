pub(crate) const INVALID_IDENTITY: &str = "resource identity must be a non-nil canonical UUID";
pub(crate) const ADDRESS_SCHEME: &str = "yolo";
pub(crate) const TERMINAL_RESOURCE: &str = "terminals";
pub(crate) const AGENT_RESOURCE: &str = "agents";
pub(crate) const WORKSPACE_RESOURCE: &str = "workspaces";

pub mod field {
    pub const HUB_ID: &str = "hub_id";
    pub const TERMINAL_ID: &str = "terminal_id";
    pub const AGENT_ID: &str = "agent_id";
    pub const RUN_ID: &str = "run_id";
    pub const WORKSPACE_ID: &str = "workspace_id";
}
