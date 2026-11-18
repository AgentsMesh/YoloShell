use super::constants::*;
use super::control_output::ControlOutput;
use super::protocol::{ControlCapability, parse_identity, write_tracked_root};
use super::watchdog_diagnostics::WatchdogDiagnostics;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin};
use yolo_process::{ProcessDiscoveryCapability, ProcessIdentity};

#[path = "watchdog_client_finish.rs"]
mod finish;
#[path = "watchdog_client_spawn.rs"]
mod spawn;

pub(super) struct WatchdogClient {
    child: Option<Child>,
    input: Option<ChildStdin>,
    output: ControlOutput,
    diagnostics: WatchdogDiagnostics,
    identity: ProcessIdentity,
    capability: ControlCapability,
    process_discovery: ProcessDiscoveryCapability,
    socket_root: PathBuf,
}

impl WatchdogClient {
    pub(super) fn socket_root(&self) -> &Path {
        &self.socket_root
    }

    pub(super) fn capability(&self) -> &ControlCapability {
        &self.capability
    }

    pub(super) fn process_discovery(&self) -> &ProcessDiscoveryCapability {
        &self.process_discovery
    }

    pub(super) fn track(&mut self, identity: ProcessIdentity) -> Result<()> {
        let descriptor = self.process_discovery.writer_handle();
        write_tracked_root(
            self.input.as_mut().context(WATCHDOG_PIPE_DETAIL)?,
            &self.capability,
            identity,
            descriptor,
            ProcessIdentity::capture(std::process::id())?,
        )?;
        let acknowledgement = self
            .output
            .read_frame(WATCHDOG_HANDSHAKE_TIMEOUT)
            .and_then(|value| parse_identity(&value, FRAME_TRACKED, &self.capability))
            .map_err(|error| spawn::with_diagnostics(error, self.diagnostics.collect()))
            .context(WATCHDOG_HANDSHAKE_DETAIL)?;
        ensure!(acknowledgement == identity, WATCHDOG_PROTOCOL_DETAIL);
        Ok(())
    }
}
