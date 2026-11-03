use super::{
    FencedDescendants, ProcessScanFuture, ProcessScanObservation, ProcessScanner,
    ProcessSnapshotOutcome, ProcessSnapshotReceipt, read_process_snapshot,
};
use crate::process::{ProcessIdentity, ScopedWaitableChild};
use anyhow::{Context, Result, anyhow, ensure};
use std::path::PathBuf;
use std::time::Instant;

#[path = "isolated_process_scanner_command.rs"]
mod command;

const DEADLINE_EXPIRED: &str = "isolated process scan deadline expired";
const EXECUTABLE_INVALID: &str = "isolated process scanner executable is empty";
const HELPER_REPORTED_FAILURE: &str = "isolated process scanner helper reported failure";
const PARENT_SOCKET_CONTEXT: &str = "create isolated process scanner socket";
const PARENT_SOCKET_FLAGS_CONTEXT: &str = "configure isolated process scanner parent socket";
const PARENT_SOCKET_RUNTIME_CONTEXT: &str = "register isolated process scanner parent socket";
const ROOT_INVALID: &str = "isolated process scanner root identity is invalid";

pub struct IsolatedProcessScanner {
    executable: PathBuf,
    environment_keys: Vec<String>,
    #[cfg(test)]
    behavior: Option<&'static str>,
    #[cfg(test)]
    spawned_process: Option<std::sync::Arc<std::sync::atomic::AtomicU32>>,
}

impl IsolatedProcessScanner {
    pub fn new(executable: PathBuf, environment_keys: Vec<String>) -> Result<Self> {
        ensure!(!executable.as_os_str().is_empty(), EXECUTABLE_INVALID);
        Ok(Self {
            executable,
            environment_keys,
            #[cfg(test)]
            behavior: None,
            #[cfg(test)]
            spawned_process: None,
        })
    }

    async fn scan_inner(
        &self,
        root: ProcessIdentity,
        deadline: Instant,
    ) -> Result<ProcessScanObservation> {
        validate_request(root, deadline)?;
        let (parent_socket, child_socket) =
            std::os::unix::net::UnixStream::pair().context(PARENT_SOCKET_CONTEXT)?;
        parent_socket
            .set_nonblocking(true)
            .context(PARENT_SOCKET_FLAGS_CONTEXT)?;
        let mut reader = crate::local_unix::LocalUnixStream::from_std(parent_socket)
            .context(PARENT_SOCKET_RUNTIME_CONTEXT)?;
        let mut helper =
            command::prepare(&self.executable, root, child_socket, &self.environment_keys)?;
        #[cfg(test)]
        if let Some(behavior) = self.behavior {
            helper.env(super::process_snapshot_fault::BEHAVIOR_ENV, behavior);
        }
        let child = ScopedWaitableChild::spawn_before_deadline(&mut helper, deadline)?;
        #[cfg(test)]
        if let Some(spawned_process) = &self.spawned_process {
            spawned_process.store(child.process_id(), std::sync::atomic::Ordering::Release);
        }
        // Command retains the configured descriptors after spawn, which would suppress EOF.
        drop(helper);
        let receipt = tokio::time::timeout_at(
            tokio::time::Instant::from_std(deadline),
            read_process_snapshot(&mut reader, root, self.environment_keys.clone()),
        )
        .await
        .map_err(|_| anyhow!(DEADLINE_EXPIRED))??;
        child.wait_for_clean_exit(deadline).await?;
        observation(receipt)
    }

    #[cfg(test)]
    pub(super) fn with_behavior(mut self, behavior: &'static str) -> Self {
        self.behavior = Some(behavior);
        self
    }

    #[cfg(test)]
    fn with_spawn_observer(
        mut self,
        observer: std::sync::Arc<std::sync::atomic::AtomicU32>,
    ) -> Self {
        self.spawned_process = Some(observer);
        self
    }
}

impl ProcessScanner for IsolatedProcessScanner {
    fn scan<'a>(&'a self, root: ProcessIdentity, deadline: Instant) -> ProcessScanFuture<'a> {
        Box::pin(self.scan_inner(root, deadline))
    }
}

fn validate_request(root: ProcessIdentity, deadline: Instant) -> Result<()> {
    ensure!(root.pid != 0 && root.start_time != 0, ROOT_INVALID);
    ensure!(Instant::now() < deadline, DEADLINE_EXPIRED);
    Ok(())
}

fn observation(receipt: ProcessSnapshotReceipt) -> Result<ProcessScanObservation> {
    let descendants = match receipt.outcome {
        ProcessSnapshotOutcome::Stable(processes) => FencedDescendants::Stable(processes),
        ProcessSnapshotOutcome::RootChanged => FencedDescendants::RootChanged,
        ProcessSnapshotOutcome::Failed(detail) => {
            return Err(anyhow!(detail)).context(HELPER_REPORTED_FAILURE);
        }
    };
    Ok(ProcessScanObservation {
        descendants,
        started_at_ms: receipt.started_at_ms,
        completed_at_ms: receipt.completed_at_ms,
    })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/isolated_process_scanner_lifecycle_tests.rs"]
mod lifecycle_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/isolated_process_scanner_test_support.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/isolated_process_scanner_tests.rs"]
mod tests;
