//! One-shot trusted plugin startup/action runner. Yolo never supervises the daemon.

mod context;
mod log;
mod runner;
mod runtime;

pub use context::LifecycleContext;
pub use runner::{
    LifecycleError, LifecycleOutcome, run_action, run_action_with, run_startup, run_startup_with,
};
