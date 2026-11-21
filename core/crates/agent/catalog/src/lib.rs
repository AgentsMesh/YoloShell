//! Agent CLI support: live catalogue reads, selectors and run label updates.

mod catalog;
mod constants;
mod reject;
mod render;
mod labels;
mod selector;

pub struct AgentRow {
    pub terminal_name: Option<String>,
    pub snapshot: yolo_hub_agent_protocol::AgentSnapshot,
}

pub struct Selection {
    pub target: yolo_hub_identity::AgentTarget,
    pub observed_runtime: Option<yolo_hub_identity::RuntimeTarget>,
    pub terminal_name: Option<String>,
}

pub use catalog::{agents, resolve, resolve_live, resolve_live_many};
pub use reject::{Reject, rejected};
pub use render::{details, text};
pub use labels::update_labels;
pub use selector::{RunCursor, run_address, run_selector};
