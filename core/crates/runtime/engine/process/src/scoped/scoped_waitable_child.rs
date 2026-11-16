use anyhow::{Context, Result, anyhow, ensure};
use std::io;
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus};
use std::time::{Duration, Instant};

const RESERVE_SETTLEMENT_CONTEXT: &str = "reserve scoped waitable child settlement";
const CAPTURE_CUSTODY_WARNING: &str =
    "scoped waitable child exact identity capture failed; retaining direct wait custody";
const POST_SPAWN_DEADLINE_EXPIRED: &str = "scoped waitable child spawn deadline expired";
pub(super) const WAIT_DEADLINE_EXPIRED: &str = "scoped waitable child exit deadline expired";
const UNSUCCESSFUL_EXIT: &str = "scoped waitable child exited unsuccessfully";
const TERMINATION_REQUEST_WARNING: &str =
    "scoped waitable child direct termination request failed; reaper will retry";
const ACTIVE_CUSTODY_EXPECTATION: &str = "scoped waitable child custody is active";
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct ScopedWaitableChild {
    retained: Option<RetainedChild>,
}

struct RetainedChild {
    custody: super::DetachedChildCustody,
    settlement: super::ReservedDetachedChildSettlement,
}

impl ScopedWaitableChild {
    pub fn spawn_detached_before_deadline(
        command: &mut Command,
        deadline: Instant,
    ) -> Result<Self> {
        super::configure_detached_command(command);
        Self::spawn_before_deadline(command, deadline)
    }

    pub fn spawn_before_deadline(command: &mut Command, deadline: Instant) -> Result<Self> {
        let settlement =
            super::reserve_detached_child_settlement().context(RESERVE_SETTLEMENT_CONTEXT)?;
        let child = super::spawn_waitable_child_before_deadline(command, deadline)?;
        let custody = super::DetachedChildCustody::capture(child).unwrap_or_else(|error| {
            tracing::warn!(%error, message = CAPTURE_CUSTODY_WARNING);
            error.into_custody()
        });
        let scoped = Self {
            retained: Some(RetainedChild {
                custody,
                settlement,
            }),
        };
        ensure!(Instant::now() < deadline, POST_SPAWN_DEADLINE_EXPIRED);
        Ok(scoped)
    }

    pub async fn terminate_and_wait_reaped(self, deadline: Instant) -> Result<()> {
        self.terminate_and_wait_status(deadline).await.map(drop)
    }

    pub async fn terminate_and_wait_status(mut self, deadline: Instant) -> Result<ExitStatus> {
        ensure!(Instant::now() < deadline, WAIT_DEADLINE_EXPIRED);
        let custody = self.custody_mut();
        if let Err(error) = custody.request_direct_termination() {
            tracing::warn!(%error, message = TERMINATION_REQUEST_WARNING);
        }
        let status =
            tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), self.poll_exit())
                .await
                .map_err(|_| anyhow!(WAIT_DEADLINE_EXPIRED))??;
        let completed_before_deadline = Instant::now() < deadline;
        self.retained = None;
        ensure!(completed_before_deadline, WAIT_DEADLINE_EXPIRED);
        Ok(status)
    }

    pub fn process_id(&self) -> u32 {
        self.retained
            .as_ref()
            .expect(ACTIVE_CUSTODY_EXPECTATION)
            .custody
            .process_id()
    }

    /// Identity captured while acquiring this child; unavailable captures stay unproven.
    pub fn captured_identity(&self) -> Option<super::ProcessIdentity> {
        self.custody().identity()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.custody_mut().take_stdout()
    }

    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.custody_mut().take_stdin()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.custody_mut().take_stderr()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.custody_mut().try_wait()
    }

    pub fn retire_reaped(&mut self) -> Result<()> {
        ensure!(self.try_wait()?.is_some(), ACTIVE_CUSTODY_EXPECTATION);
        self.retained = None;
        Ok(())
    }

    pub fn preserve_in_background(mut self) -> Result<()> {
        if self.try_wait()?.is_some() {
            self.retained = None;
            return Err(anyhow!(UNSUCCESSFUL_EXIT));
        }
        let RetainedChild {
            custody,
            settlement,
        } = self.retained.take().expect(ACTIVE_CUSTODY_EXPECTATION);
        settlement.retain_custody(custody);
        Ok(())
    }

    pub async fn wait_for_clean_exit(mut self, deadline: Instant) -> Result<()> {
        ensure!(Instant::now() < deadline, WAIT_DEADLINE_EXPIRED);
        let status =
            tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), self.poll_exit())
                .await
                .map_err(|_| anyhow!(WAIT_DEADLINE_EXPIRED))??;
        let completed_before_deadline = Instant::now() < deadline;
        self.retained = None;
        ensure!(completed_before_deadline, WAIT_DEADLINE_EXPIRED);
        ensure!(status.success(), "{UNSUCCESSFUL_EXIT}: {status}");
        Ok(())
    }

    pub(super) async fn poll_exit(&mut self) -> io::Result<ExitStatus> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(EXIT_POLL_INTERVAL).await;
        }
    }

    pub(super) fn custody_mut(&mut self) -> &mut super::DetachedChildCustody {
        &mut self
            .retained
            .as_mut()
            .expect(ACTIVE_CUSTODY_EXPECTATION)
            .custody
    }

    pub(super) fn custody(&self) -> &super::DetachedChildCustody {
        &self
            .retained
            .as_ref()
            .expect(ACTIVE_CUSTODY_EXPECTATION)
            .custody
    }

    fn settle_child_on_drop(&mut self) {
        let Some(RetainedChild {
            mut custody,
            settlement,
        }) = self.retained.take()
        else {
            return;
        };
        let process_id = custody.process_id();
        let termination = custody.request_direct_termination();
        settlement.terminate_custody(custody);
        if let Err(error) = termination {
            tracing::warn!(%error, process_id, message = TERMINATION_REQUEST_WARNING);
        }
    }
}

impl Drop for ScopedWaitableChild {
    fn drop(&mut self) {
        self.settle_child_on_drop();
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/scoped/scoped_waitable_child_tests.rs"]
mod tests;
