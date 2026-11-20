//! Agent CLI support: live catalogue reads, selectors and run label updates.

mod catalog;
mod constants;
mod reject;
mod render;
mod labels;
mod selector;

pub use catalog::{AgentRow, Selection, agents, resolve, resolve_live, resolve_live_many};
pub use reject::{Reject, rejected};
pub use render::{details, text};
pub use labels::update_labels;
pub use selector::{run_address, run_selector};
