//! Pure presentation of independent terminal, transport, input, and agent facts.
mod agent;
mod labels;
mod phase;
mod projection;

pub use agent::active_agent;
pub use labels::*;
pub use phase::{TerminalPhase, terminal_phase};
pub use projection::{ConnectionState, InputAccess, TerminalProjection};
