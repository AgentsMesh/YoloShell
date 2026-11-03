//! Durable agent-run identity. The store owns naming and revisions; terminal
//! launch and provider binding remain Host/Worker facts.

mod adopt;
mod cancel;
mod error;
mod ops;
mod quota;
mod record;
mod store;

pub use error::StoreError;
pub use record::ReserveAgentRun;
pub use store::AgentRunStore;

#[cfg(test)]
use yolo_hub_agent_run_protocol::*;
#[cfg(test)]
use yolo_hub_identity::*;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
