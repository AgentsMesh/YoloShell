//! Pure executable/argv contract. No services, process spawning or environment reads.
mod argument_constants;
mod role;
mod selection;

include!("generated/process_role_constants.rs");

pub use argument_constants::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT};
pub use role::ProcessRole;
pub use selection::{Invocation, SelectionError};
