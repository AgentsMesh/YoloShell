use super::{MarkerExt, StartupHandoffMarker};
use anyhow::{Context, Result, bail, ensure};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};
use yolo_runtime::paths::{atomic_write_json, read_json};
use yolo_runtime::process::ProcessIdentity;
use yolo_runtime::worker::startup_handoff_test_hook::{
    COMPLETION_ACK_MARKER_FILE, RELEASE_MARKER_FILE,
};

const WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WATCHDOG_GRACE: Duration = Duration::from_secs(3);
const WORKER_STDERR_LIMIT_BYTES: u64 = 64 * 1024;
const MARKER_TIMEOUT_DETAIL: &str = "timed out waiting for startup handoff marker";
const MARKER_WORKER_EXIT_DETAIL: &str = "session worker exited before startup handoff marker";
const WORKER_EXIT_TIMEOUT_DETAIL: &str = "session worker did not exit before its deadline";
const WORKER_EARLY_EXIT_DETAIL: &str = "session worker exited before cleanup release";
const WORKER_IDENTITY_DETAIL: &str = "session worker marker identity mismatch";
const WORKER_REAP_CONTEXT: &str = "reap exact session worker";
const WORKER_STDERR_READ_FAILURE_DETAIL: &str = "session worker stderr could not be read";

pub(super) struct WorkerWatchdog {
    child: Child,
    identity: ProcessIdentity,
    exact_child: Option<ProcessIdentity>,
    failure_marker: PathBuf,
    exited: bool,
}

impl WorkerWatchdog {
    pub(super) fn new(child: Child, failure_marker: PathBuf) -> Result<Self> {
        let identity = ProcessIdentity::capture(child.id())?;
        Ok(Self {
            child,
            identity,
            exact_child: None,
            failure_marker,
            exited: false,
        })
    }

    pub(super) fn observe_child(&mut self, marker: &StartupHandoffMarker) -> Result<()> {
        ensure!(
            marker.worker_pid == self.identity.pid
                && marker.worker_start_time == self.identity.start_time,
            WORKER_IDENTITY_DETAIL
        );
        self.exact_child = Some(marker.child_identity());
        Ok(())
    }

    pub(super) fn wait_marker(
        &mut self,
        path: &Path,
        timeout: Duration,
    ) -> Result<StartupHandoffMarker> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(marker) = read_json(path) {
                return Ok(marker);
            }
            if let Some(status) = self.child.try_wait()? {
                self.exited = true;
                let stderr = self.read_stderr();
                bail!(
                    "{MARKER_WORKER_EXIT_DETAIL}: status={status}; marker={}; stderr={stderr:?}",
                    path.display()
                );
            }
            if Instant::now() >= deadline {
                bail!("{MARKER_TIMEOUT_DETAIL}: {}", path.display());
            }
            std::thread::sleep(WATCHDOG_POLL_INTERVAL);
        }
    }

    pub(super) fn assert_running(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            self.exited = true;
            bail!(
                "{WORKER_EARLY_EXIT_DETAIL}: {status}; stderr: {}",
                self.read_stderr()
            );
        }
        Ok(())
    }

    pub(super) fn wait_for_exit(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.exited = true;
                return Ok(status);
            }
            if Instant::now() >= deadline {
                bail!(WORKER_EXIT_TIMEOUT_DETAIL);
            }
            std::thread::sleep(WATCHDOG_POLL_INTERVAL);
        }
    }

    fn read_stderr(&mut self) -> String {
        let mut detail = Vec::new();
        if let Some(mut stderr) = self.child.stderr.take() {
            if let Err(error) = stderr
                .by_ref()
                .take(WORKER_STDERR_LIMIT_BYTES)
                .read_to_end(&mut detail)
            {
                return format!("{WORKER_STDERR_READ_FAILURE_DETAIL}: {error}");
            }
        }
        String::from_utf8_lossy(&detail).into_owned()
    }

    fn release_if_observed(&self) {
        let Ok(marker) = read_json::<StartupHandoffMarker>(&self.failure_marker) else {
            return;
        };
        let Some(directory) = self.failure_marker.parent() else {
            return;
        };
        let _ = atomic_write_json(&directory.join(RELEASE_MARKER_FILE), &marker);
        let _ = atomic_write_json(&directory.join(COMPLETION_ACK_MARKER_FILE), &marker);
    }
}

impl Drop for WorkerWatchdog {
    fn drop(&mut self) {
        if self.exited {
            return;
        }
        self.release_if_observed();
        let deadline = Instant::now() + WATCHDOG_GRACE;
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(WATCHDOG_POLL_INTERVAL);
        }
        if let Some(identity) = self.exact_child {
            let _ = yolo_process_test_support::send_signal(identity.pid, libc::SIGKILL);
        }
        let _ = yolo_process_test_support::send_signal(self.identity.pid, libc::SIGKILL);
        let _ = self.child.wait().context(WORKER_REAP_CONTEXT);
    }
}

#[cfg(test)]
#[path = "startup_handoff_process_watchdog_tests.rs"]
mod tests;
