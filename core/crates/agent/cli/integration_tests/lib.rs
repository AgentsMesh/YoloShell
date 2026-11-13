/// Serializes tests that mutate the process environment.
pub(super) static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

mod ask;
mod control;
mod current;
#[path = "../test_support/fixture.rs"]
mod fixture;
mod harness;
mod interrupt;
#[path = "../test_support/peer.rs"]
mod peer;
mod queries;
mod readiness;
mod runs;
mod selectors;
mod start;
mod start_current;
mod wait;
#[path = "../test_support/wait_agents.rs"]
mod wait_agents;
mod wait_runs;
