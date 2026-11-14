use super::super::constants::*;
use super::super::control_output::ControlOutput;
use super::super::mode::remove_internal_environment;
use super::super::protocol::{RootFrame, parse_root};
use super::super::watchdog_diagnostics::WatchdogDiagnostics;
use super::WatchdogClient;
use anyhow::{Context, Error, Result, ensure};
use std::process::{Child, Command};
use yolo_process::{
    ProcessDiscoveryCapability, ProcessIdentity, spawn_detached_with_diagnostic_pipes,
};

impl WatchdogClient {
    pub(in crate::socket_root_guard) fn spawn() -> Result<Self> {
        let process_discovery = ProcessDiscoveryCapability::generate()?;
        let mut command = Command::new(std::env::current_exe()?);
        #[cfg(feature = "adversarial-test-hooks")]
        let hooks = (
            super::super::fault::capture_force_signal_hook(),
            super::super::fault::capture_refresh_handoff_hook(),
            super::super::fault::capture_discovery_arm_hook(),
            super::super::fault::capture_refresh_postcheck_witness(),
            super::super::fault::capture_historical_witness(),
        );
        remove_internal_environment(&mut command);
        command.env(GUARD_MODE_ENVIRONMENT, WATCHDOG_MODE);
        #[cfg(feature = "adversarial-test-hooks")]
        {
            super::super::fault::remove_environment(&mut command);
            super::super::fault::forward_force_signal_hook(&mut command, hooks.0);
            super::super::fault::forward_refresh_handoff_hook(&mut command, hooks.1);
            super::super::fault::forward_discovery_arm_hook(&mut command, hooks.2);
            super::super::fault::forward_refresh_postcheck_witness(&mut command, hooks.3);
            if let Some(witness) = hooks.4 {
                witness.forward(&mut command);
            }
        }
        #[cfg(target_os = "macos")]
        process_discovery.configure_manager_inheritance(&mut command);
        #[cfg(not(target_os = "macos"))]
        process_discovery.configure_child_inheritance(&mut command);
        let child = spawn_detached_with_diagnostic_pipes(command).context(WATCHDOG_CONTEXT)?;
        Self::complete_spawn(child, process_discovery)
    }

    fn complete_spawn(
        mut child: Child,
        process_discovery: ProcessDiscoveryCapability,
    ) -> Result<Self> {
        let identity = match ProcessIdentity::capture(child.id()) {
            Ok(identity) => identity,
            Err(error) => {
                super::super::watchdog_process::settle_failed(child, None, None);
                return Err(error);
            }
        };
        let Some(input) = child.stdin.take() else {
            super::super::watchdog_process::settle_failed(child, None, Some(identity));
            anyhow::bail!(WATCHDOG_PIPE_DETAIL);
        };
        let Some(output) = child.stdout.take() else {
            super::super::watchdog_process::settle_failed(child, Some(input), Some(identity));
            anyhow::bail!(WATCHDOG_PIPE_DETAIL);
        };
        let Some(stderr) = child.stderr.take() else {
            super::super::watchdog_process::settle_failed(child, Some(input), Some(identity));
            anyhow::bail!(WATCHDOG_DIAGNOSTIC_DETAIL);
        };
        let mut diagnostics = match WatchdogDiagnostics::start(stderr) {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                super::super::watchdog_process::settle_failed(child, Some(input), Some(identity));
                return Err(error);
            }
        };
        let mut output = ControlOutput::new(output);
        let root = output
            .read_frame(WATCHDOG_HANDSHAKE_TIMEOUT)
            .and_then(|value| parse_root(&value))
            .and_then(|root| validate_discovery(root, &process_discovery))
            .context(WATCHDOG_HANDSHAKE_DETAIL);
        match root {
            Ok(root) => Ok(Self {
                child: Some(child),
                input: Some(input),
                output,
                diagnostics,
                identity,
                capability: root.capability,
                process_discovery,
                socket_root: root.path,
            }),
            Err(error) => {
                super::super::watchdog_process::settle_failed(child, Some(input), Some(identity));
                Err(with_diagnostics(error, diagnostics.collect()))
            }
        }
    }
}

fn validate_discovery(
    root: RootFrame,
    process_discovery: &ProcessDiscoveryCapability,
) -> Result<RootFrame> {
    ensure!(
        root.discovery.value() == process_discovery.value()
            && root.discovery_reader == process_discovery.reader_handle()
            && root.discovery_writer == process_discovery.writer_handle(),
        WATCHDOG_HANDSHAKE_DETAIL
    );
    Ok(root)
}

pub(super) fn with_diagnostics(error: Error, diagnostics: Result<String>) -> Error {
    match diagnostics {
        Ok(detail) if !detail.is_empty() => {
            error.context(format!("{WATCHDOG_DIAGNOSTIC_DETAIL}: {detail}"))
        }
        Ok(_) => error,
        Err(diagnostic_error) => error.context(format!("{diagnostic_error:#}")),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/watchdog_client_spawn_tests.rs"]
mod tests;
