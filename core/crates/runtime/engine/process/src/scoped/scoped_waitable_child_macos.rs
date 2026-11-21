use super::ScopedWaitableChild;
use anyhow::{Result, anyhow, ensure};
use std::time::Instant;

const EXACT_IDENTITY_UNAVAILABLE: &str = "scoped waitable child exact identity is unavailable";
const STOPPED_STATE_REQUIRED: &str = "scoped waitable child is not exactly stopped";
const RESUME_NOT_DELIVERED: &str = "scoped waitable child exact resume was not delivered";
const TERMINATION_FALLBACK_WARNING: &str =
    "scoped waitable child exact termination failed; using direct wait custody";

impl ScopedWaitableChild {
    fn exact_live_identity(&self) -> Result<crate::process::LiveProcessIdentity> {
        self.custody()
            .live_identity()
            .ok_or_else(|| anyhow!(EXACT_IDENTITY_UNAVAILABLE))
    }

    pub fn observe_exact_exec(&self) -> Result<super::MacosExecObserver> {
        super::MacosExecObserver::register(self.exact_live_identity()?)
    }

    fn exact_stopped_identity(&self) -> Result<crate::process::LiveProcessIdentity> {
        let live = self.exact_live_identity()?;
        ensure!(
            live.execution_state()?
                == crate::process::live_identity::ProcessExecutionState::Stopped,
            STOPPED_STATE_REQUIRED
        );
        Ok(live)
    }

    pub fn resume_exact_stopped_before(&self, deadline: Instant) -> Result<()> {
        ensure!(
            Instant::now() < deadline,
            super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED
        );
        let live = self.exact_stopped_identity()?;
        ensure!(
            Instant::now() < deadline,
            super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED
        );
        ensure!(
            crate::process::signal_authority::signal_exact_process(
                &live,
                crate::process::signal_authority::ExactProcessSignal::ResumeSuspendedBefore(
                    deadline,
                ),
            )?,
            RESUME_NOT_DELIVERED
        );
        Ok(())
    }

    pub async fn terminate_exact_and_wait_reaped(mut self, deadline: Instant) -> Result<()> {
        ensure!(
            Instant::now() < deadline,
            super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED
        );
        if self.try_wait()?.is_some() {
            return self.retire_before(deadline);
        }
        let live = match self.exact_live_identity() {
            Ok(live) => live,
            Err(error) => {
                tracing::warn!(%error, message = TERMINATION_FALLBACK_WARNING);
                return self.terminate_and_wait_reaped(deadline).await;
            }
        };
        let delivery = crate::process::signal_authority::signal_exact_process(
            &live,
            crate::process::signal_authority::ExactProcessSignal::ForceTermination,
        );
        if !matches!(&delivery, Ok(true)) {
            if self.try_wait()?.is_some() {
                return self.retire_before(deadline);
            }
            match delivery {
                Ok(false) => tracing::warn!(message = TERMINATION_FALLBACK_WARNING),
                Err(error) => tracing::warn!(%error, message = TERMINATION_FALLBACK_WARNING),
                Ok(true) => unreachable!(),
            }
            return self.terminate_and_wait_reaped(deadline).await;
        }
        tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), self.poll_exit())
            .await
            .map_err(|_| anyhow!(super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED))??;
        self.retire_before(deadline)
    }

    fn retire_before(&mut self, deadline: Instant) -> Result<()> {
        let completed_before_deadline = Instant::now() < deadline;
        self.retire_reaped()?;
        ensure!(
            completed_before_deadline,
            super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED
        );
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/scoped/scoped_waitable_child_macos_tests.rs"]
mod tests;
