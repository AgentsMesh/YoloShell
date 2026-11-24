use anyhow::{Context, Result, ensure};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const BARRIER_ENVIRONMENT: &str = "YOLO_TEST_TERMINATION_BARRIER";
const ARM_FILE_NAME: &str = "arm";
const REACHED_FILE_NAME: &str = "reached";
const RELEASE_FILE_NAME: &str = "release";
const MARKER_CONTENT: &[u8] = b"ready";
const BARRIER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CREATE_DIRECTORY_CONTEXT: &str = "create termination test barrier directory";
const WRITE_ARM_CONTEXT: &str = "arm termination test barrier";
const CLAIM_BARRIER_CONTEXT: &str = "claim termination test barrier";
const READ_RELEASE_CONTEXT: &str = "read termination test barrier release";
const WRITE_RELEASE_CONTEXT: &str = "release termination test barrier";
const ABSOLUTE_ROOT_DETAIL: &str = "termination test barrier root must be absolute";

/// Test-process coordination for the durable tombstone boundary.
///
/// The hook is dormant unless its environment variable is explicitly passed
/// to a test Host. It is public only so cross-process resilience tests can arm
/// and observe the real Worker binary.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct TerminationTestBarrier {
    directory: PathBuf,
}

impl TerminationTestBarrier {
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

    fn from_environment(terminal_id: Uuid) -> Result<Option<Self>> {
        std::env::var_os(BARRIER_ENVIRONMENT)
            .map(|root| Self::new(PathBuf::from(root), terminal_id))
            .transpose()
    }

    fn claim(&self) -> Result<bool> {
        match std::fs::rename(self.arm_path(), self.reached_path()) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error).context(CLAIM_BARRIER_CONTEXT),
        }
    }

    fn released(&self) -> Result<bool> {
        self.release_path()
            .try_exists()
            .context(READ_RELEASE_CONTEXT)
    }

    fn holds_owner(&self) -> Result<bool> {
        let armed = self
            .arm_path()
            .try_exists()
            .context(CLAIM_BARRIER_CONTEXT)?;
        Ok((armed || self.reached()?) && !self.released()?)
    }

    fn arm_path(&self) -> PathBuf {
        self.directory.join(ARM_FILE_NAME)
    }

    fn reached_path(&self) -> PathBuf {
        self.directory.join(REACHED_FILE_NAME)
    }

    fn release_path(&self) -> PathBuf {
        self.directory.join(RELEASE_FILE_NAME)
    }
}

pub(super) async fn pause_after_tombstone(terminal_id: Uuid) -> Result<()> {
    let Some(barrier) = TerminationTestBarrier::from_environment(terminal_id)? else {
        return Ok(());
    };
    if !barrier.claim()? {
        return Ok(());
    }
    while !barrier.released()? {
        tokio::time::sleep(BARRIER_POLL_INTERVAL).await;
    }
    Ok(())
}

pub(super) fn owner_is_held(terminal_id: Uuid) -> Result<bool> {
    TerminationTestBarrier::from_environment(terminal_id)?
        .map(|barrier| barrier.holds_owner())
        .unwrap_or(Ok(false))
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context(CREATE_DIRECTORY_CONTEXT),
    }
}

#[cfg(all(test, feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/termination_test_hook_tests.rs"]
mod tests;
