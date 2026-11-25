#[path = "bootstrap_validation.rs"]
mod bootstrap_validation;
#[path = "client.rs"]
mod client;
#[cfg(target_os = "macos")]
#[path = "preparation/preparation.rs"]
mod preparation;
#[cfg(target_os = "macos")]
#[path = "prepare.rs"]
mod prepare;
#[path = "protocol.rs"]
mod protocol;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "recovery/recovery_client.rs"]
mod recovery_client;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "recovery/recovery_protocol.rs"]
mod recovery_protocol;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[path = "recovery/recovery_server.rs"]
mod recovery_server;
#[cfg(target_os = "macos")]
#[path = "request.rs"]
mod request;
#[path = "runner.rs"]
mod runner;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement.rs"]
mod settlement;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_armed.rs"]
mod settlement_armed;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_backoff.rs"]
mod settlement_backoff;
#[cfg(target_os = "macos")]
#[path = "settlement_containment.rs"]
mod settlement_containment;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_failure.rs"]
mod settlement_failure;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_failure_log.rs"]
mod settlement_failure_log;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_initial.rs"]
mod settlement_initial;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_prepared.rs"]
mod settlement_prepared;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_retirement.rs"]
mod settlement_retirement;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_runtime_file.rs"]
mod settlement_runtime_file;
#[cfg(target_os = "macos")]
#[path = "settlement/settlement_state.rs"]
mod settlement_state;
#[path = "socket.rs"]
mod socket;
#[path = "spawn/spawn.rs"]
mod spawn;
#[path = "spawn/spawn_command.rs"]
mod spawn_command;
#[path = "spawn/spawn_settlement.rs"]
mod spawn_settlement;
#[path = "terminal_reporting.rs"]
mod terminal_reporting;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/guard/test_child_guard.rs"]
mod test_child_guard;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/test_probe.rs"]
pub mod test_probe;
#[cfg(any(test, feature = "host-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/test_processes.rs"]
mod test_processes;

pub use client::prepare_existing;
#[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
pub use recovery_client::recover_authority;
pub use spawn::{spawn_initial, spawn_successor};
#[cfg(any(test, feature = "host-test-hooks"))]
#[doc(hidden)]
pub use test_processes::{
    contains as test_custody_guardian_is_registered,
    terminate_for_home as terminate_test_processes_for_home,
};

pub async fn run(home: std::path::PathBuf, terminal_id: uuid::Uuid) -> anyhow::Result<()> {
    runner::run(home, terminal_id).await
}
