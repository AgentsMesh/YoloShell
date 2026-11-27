pub(super) mod constants;
mod control_output;
mod custody;
mod direct_child;
pub(super) mod environment;
#[cfg(feature = "adversarial-test-hooks")]
mod fault;
pub(super) mod launcher;
pub(super) mod mode;
pub(super) mod protocol;
mod runner;
mod watchdog_cleanup;
mod watchdog_client;
mod watchdog_diagnostics;
mod watchdog_process;
mod watchdog_server;

use anyhow::Result;
use mode::InvocationMode;
use std::process::ExitCode;

pub(super) fn run() -> Result<ExitCode> {
    match mode::invocation_mode()? {
        InvocationMode::Guard => runner::run_guard(),
        InvocationMode::Launcher => launcher::run_launcher(),
        InvocationMode::Watchdog => watchdog_server::run_watchdog(),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))] #[path = "../integration_tests/socket_root_guard_main_tests.rs"]
mod tests;
