use anyhow::{Context, Result, ensure};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const BARRIER_ENVIRONMENT: &str = "YOLO_TEST_RECONCILE_BARRIER";
const ARM_FILE_NAME: &str = "arm";
const REACHED_FILE_NAME: &str = "reached";
const RELEASE_FILE_NAME: &str = "release";
const MARKER_CONTENT: &[u8] = b"ready";
const BARRIER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CREATE_DIRECTORY_CONTEXT: &str = "create reconcile test barrier directory";
const WRITE_ARM_CONTEXT: &str = "arm reconcile test barrier";
const CLAIM_BARRIER_CONTEXT: &str = "claim reconcile test barrier";
const READ_RELEASE_CONTEXT: &str = "read reconcile test barrier release";
const WRITE_RELEASE_CONTEXT: &str = "release reconcile test barrier";
const ABSOLUTE_ROOT_DETAIL: &str = "reconcile test barrier root must be absolute";
const CONFIGURATION_CONTEXT: &str = "load reconcile test barrier configuration";

/// Test-process coordination after reconciliation acquires a session lock.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ReconcileTestBarrier {
    directory: PathBuf,
}

impl ReconcileTestBarrier {
    pub fn environment_variable() -> &'static str {
        BARRIER_ENVIRONMENT
    }

    pub fn new(root: impl AsRef<Path>, terminal_id: Uuid) -> Result<Self> {
        let root = root.as_ref();
        ensure!(root.is_absolute(), ABSOLUTE_ROOT_DETAIL);
        Ok(Self {
            directory: root.join(terminal_id.to_string()),
        })
    }

    pub fn arm(&self) -> Result<()> {
        std::fs::create_dir_all(&self.directory).context(CREATE_DIRECTORY_CONTEXT)?;
        remove_if_present(&self.reached_path())?;
        remove_if_present(&self.release_path())?;
        std::fs::write(self.arm_path(), MARKER_CONTENT).context(WRITE_ARM_CONTEXT)
    }

    pub fn reached(&self) -> Result<bool> {
        self.reached_path()
            .try_exists()
            .context(CLAIM_BARRIER_CONTEXT)
    }

    pub fn release(&self) -> Result<()> {
        std::fs::write(self.release_path(), MARKER_CONTENT).context(WRITE_RELEASE_CONTEXT)
    }

    pub(crate) fn from_environment(terminal_id: Uuid) -> Result<Option<Self>> {
        std::env::var_os(BARRIER_ENVIRONMENT)
            .map(|root| Self::new(PathBuf::from(root), terminal_id))
            .transpose()
    }

    pub(crate) fn claim(&self) -> Result<bool> {
        match std::fs::rename(self.arm_path(), self.reached_path()) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error).context(CLAIM_BARRIER_CONTEXT),
        }
    }

    pub(crate) fn released(&self) -> Result<bool> {
        self.release_path()
            .try_exists()
            .context(READ_RELEASE_CONTEXT)
    }

    pub(crate) fn arm_path(&self) -> PathBuf {
        self.directory.join(ARM_FILE_NAME)
    }

    pub(crate) fn reached_path(&self) -> PathBuf {
        self.directory.join(REACHED_FILE_NAME)
    }

    pub(crate) fn release_path(&self) -> PathBuf {
        self.directory.join(RELEASE_FILE_NAME)
    }
}

pub(crate) async fn pause_after_session_lock(terminal_id: Uuid) {
    let Some(barrier) =
        ReconcileTestBarrier::from_environment(terminal_id).expect(CONFIGURATION_CONTEXT)
    else {
        return;
    };
    if !barrier.claim().expect(CLAIM_BARRIER_CONTEXT) {
        return;
    }
    while !barrier.released().expect(READ_RELEASE_CONTEXT) {
        tokio::time::sleep(BARRIER_POLL_INTERVAL).await;
    }
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context(CREATE_DIRECTORY_CONTEXT),
    }
}

#[cfg(test)]
#[path = "../../unit_tests/reconcile/reconcile_test_hook_tests.rs"]
mod tests;
