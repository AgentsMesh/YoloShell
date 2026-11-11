//! Read-only reconciliation of durable creation intents. No method creates a
//! shell, finalizes it, claims a writer, or binds a pane.
mod actions;
mod attempt;
mod completion;
mod constants;
mod rejection;
mod state;
mod status;

pub use constants::RETRY_DELAY;
pub use rejection::BlockedReason;
pub use state::CreationRecovery;
pub use status::RecoveryStatus;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
