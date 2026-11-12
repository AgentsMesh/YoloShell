use super::{CapturedProcessTree, LiveProcessIdentity, captured_targets};
use crate::process::ProcessIdentity;
use crate::process::live_identity::ProcessExecutionState;
use anyhow::{Result, ensure};

const ADVERSARIAL_TERMINATION_SIGNAL_DETAIL: &str =
    "adversarial process-tree termination requires a termination signal";
const ADVERSARIAL_TERMINATION_SIGNALS: &[i32] = &[libc::SIGKILL, libc::SIGTERM];

impl CapturedProcessTree {
    pub fn capture_with_discovery(root: ProcessIdentity, capability: String) -> Result<Self> {
        Self::capture_with_discovery_internal(root, capability)
    }

    pub fn begin_discovery_cleanup(&mut self) {
        self.begin_discovery_cleanup_internal();
    }

    pub fn discovery_is_settled(&self) -> bool {
        self.discovery_is_settled_internal()
    }

    pub fn discovery_coverage_is_complete(&self) -> bool {
        self.discovery_coverage_is_complete_internal()
    }

    pub fn discovery_publication_ready(&mut self) -> Result<bool> {
        self.discovery_publication_ready_internal()
    }

    pub fn refresh_if_needed(&mut self) -> Result<bool> {
        self.refresh_if_needed_internal()
    }

    pub fn terminate(&mut self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        validate_signal(signal)?;
        self.refresh()?;
        self.terminate_captured(signal)
    }

    pub fn terminate_captured(&self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        validate_signal(signal)?;
        let targets = self.captured_targets();
        let delivered = captured_targets::signal_for_test(&targets, signal)?;
        Ok(captured_targets::durable(delivered))
    }
}

impl ProcessIdentity {
    pub fn is_stably_stopped(self) -> Result<bool> {
        let Some(identity) = LiveProcessIdentity::capture(self)? else {
            return Ok(false);
        };
        Ok(identity.execution_state()? == ProcessExecutionState::Stopped)
    }
}

fn validate_signal(signal: i32) -> Result<()> {
    ensure!(
        ADVERSARIAL_TERMINATION_SIGNALS.contains(&signal),
        ADVERSARIAL_TERMINATION_SIGNAL_DETAIL
    );
    Ok(())
}
