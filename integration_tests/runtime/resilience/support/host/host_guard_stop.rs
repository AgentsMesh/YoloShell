use super::*;
use anyhow::{Context, Result, bail};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use yolo_process_test_support::ProcessIdentityObservation;

const HOST_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
const HOST_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);
const HOST_STOP_OUTCOME_MISSING: &str = "test host was already stopped before SIGTERM";
const HOST_SIGTERM_ESCALATED: &str = "Host SIGTERM shutdown escalated to SIGKILL";
const HOST_SIGTERM_STATUS_MISSING: &str = "Host SIGTERM shutdown returned no exit status";
const HOST_SIGTERM_STATUS_FAILED: &str = "Host SIGTERM shutdown was not graceful";
const HOST_SIGNAL_CONTEXT: &str = "signal test Host identity";
const HOST_WAIT_CONTEXT: &str = "wait for test Host exit";
const HOST_EXIT_TIMEOUT_DETAIL: &str = "test Host identity survived forced shutdown";
const HOST_EXIT_UNKNOWN_DETAIL: &str = "test Host identity is indeterminate after forced shutdown";
pub(super) const HOST_STOP_FAILURE_DETAIL: &str = "test Host cleanup failed";

struct HostStopOutcome {
    fallback_sigkill: bool,
    status: Option<ExitStatus>,
}

impl HostGuard {
    pub(crate) fn kill_hard(&mut self) {
        let _ = self.finish(libc::SIGKILL).expect(HOST_STOP_FAILURE_DETAIL);
    }

    pub(crate) fn stop(&mut self) {
        if let Err(error) = self.stop_result() {
            report_stop_failure(error);
        }
    }

    pub(crate) fn stop_result(&mut self) -> Result<()> {
        self.finish(libc::SIGINT).map(|_| ())
    }

    pub(crate) fn stop_with_sigterm(&mut self) {
        let outcome = self
            .finish(libc::SIGTERM)
            .expect(HOST_STOP_FAILURE_DETAIL)
            .expect(HOST_STOP_OUTCOME_MISSING);
        assert!(!outcome.fallback_sigkill, "{HOST_SIGTERM_ESCALATED}");
        let status = outcome.status.expect(HOST_SIGTERM_STATUS_MISSING);
        assert!(
            status.success(),
            "{HOST_SIGTERM_STATUS_FAILED}: {status:?}\n{}",
            self.stderr_tail()
        );
    }

    fn finish(&mut self, signal: i32) -> Result<Option<HostStopOutcome>> {
        let Some(mut child) = self.child.take() else {
            return Ok(None);
        };
        let result = self.finish_child(&mut child, signal);
        if result.is_err() {
            self.child = Some(child);
        }
        result.map(Some)
    }

    fn finish_child(&self, child: &mut Child, signal: i32) -> Result<HostStopOutcome> {
        let requested_sigkill = signal_host(self.identity, signal)? && signal == libc::SIGKILL;
        let mut fallback_sigkill = false;
        let mut status = wait_for_child(child, HOST_EXIT_TIMEOUT).context(HOST_WAIT_CONTEXT)?;
        if status.is_none() {
            fallback_sigkill = signal_host(self.identity, libc::SIGKILL)?;
            status = wait_for_child(child, HOST_EXIT_TIMEOUT).context(HOST_WAIT_CONTEXT)?;
        }
        if status.is_none() {
            status = match self.identity.observe() {
                ProcessIdentityObservation::Gone => Some(child.wait().context(HOST_WAIT_CONTEXT)?),
                ProcessIdentityObservation::Matching => bail!(HOST_EXIT_TIMEOUT_DETAIL),
                ProcessIdentityObservation::Unknown => bail!(HOST_EXIT_UNKNOWN_DETAIL),
            };
        }
        if (requested_sigkill || fallback_sigkill)
            && status.is_some_and(|status| status.signal() == Some(libc::SIGKILL))
        {
            discard_incomplete_coverage_profile(self.identity.pid);
        }
        Ok(HostStopOutcome {
            fallback_sigkill,
            status,
        })
    }
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> std::io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Err(error) => return Err(error),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(HOST_EXIT_POLL_INTERVAL);
            }
            Ok(None) => return Ok(None),
        }
    }
}

fn signal_host(identity: ProcessIdentity, signal: i32) -> Result<bool> {
    match identity.signal(signal) {
        Ok(delivered) => Ok(delivered),
        Err(_) if identity.observe() == ProcessIdentityObservation::Gone => Ok(false),
        Err(error) => Err(error).context(HOST_SIGNAL_CONTEXT),
    }
}

pub(super) fn report_stop_failure(error: anyhow::Error) {
    if std::thread::panicking() {
        tracing::error!(%error, "{HOST_STOP_FAILURE_DETAIL}");
    } else {
        panic!("{HOST_STOP_FAILURE_DETAIL}: {error:#}");
    }
}
