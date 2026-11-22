use crate::process::PtyMasterFdCustodyProbe;
use anyhow::{Context, Result, ensure};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const BARRIER_ENVIRONMENT: &str = "YOLO_TEST_CUSTODY_RELEASE_BARRIER";
const BEFORE_DROP_DIRECTORY: &str = "before-platform-drop";
const AFTER_DROP_DIRECTORY: &str = "after-platform-drop";
const ARM_FILE_NAME: &str = "arm";
const REACHED_FILE_NAME: &str = "reached";
const RELEASE_FILE_NAME: &str = "release";
const PROBE_FILE_NAME: &str = "pty-master-fd-probe.json";
const MARKER_CONTENT: &[u8] = b"ready";
const BARRIER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const ABSOLUTE_ROOT_DETAIL: &str = "custody release test barrier root must be absolute";
const CREATE_DIRECTORY_CONTEXT: &str = "create custody release test barrier directory";
const WRITE_ARM_CONTEXT: &str = "arm custody release test barrier";
const CLAIM_BARRIER_CONTEXT: &str = "claim custody release test barrier";
const READ_RELEASE_CONTEXT: &str = "read custody release test barrier release";
const WRITE_RELEASE_CONTEXT: &str = "release custody release test barrier";
const PROBE_REQUIRED_CONTEXT: &str = "active Host custody has no PTY master-FD probe";
const WRITE_PROBE_CONTEXT: &str = "write custody release PTY master-FD probe";
const READ_PROBE_CONTEXT: &str = "read custody release PTY master-FD probe";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustodyReleaseBoundary {
    BeforePlatformDrop,
    AfterPlatformDrop,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct CustodyReleaseTestBarrier {
    directory: PathBuf,
}

impl CustodyReleaseBoundary {
    fn directory_name(self) -> &'static str {
        match self {
            Self::BeforePlatformDrop => BEFORE_DROP_DIRECTORY,
            Self::AfterPlatformDrop => AFTER_DROP_DIRECTORY,
        }
    }
}

impl CustodyReleaseTestBarrier {
    pub fn environment_variable() -> &'static str {
        BARRIER_ENVIRONMENT
    }

    pub fn new(
        root: impl AsRef<Path>,
        terminal_id: Uuid,
        phase: CustodyReleaseBoundary,
    ) -> Result<Self> {
        let root = root.as_ref();
        ensure!(root.is_absolute(), ABSOLUTE_ROOT_DETAIL);
        Ok(Self {
            directory: root
                .join(terminal_id.to_string())
                .join(phase.directory_name()),
        })
    }

    pub fn arm(&self) -> Result<()> {
        std::fs::create_dir_all(&self.directory).context(CREATE_DIRECTORY_CONTEXT)?;
        remove_if_present(&self.reached_path())?;
        remove_if_present(&self.release_path())?;
        remove_if_present(&self.probe_path())?;
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

    pub fn probe(&self) -> Result<PtyMasterFdCustodyProbe> {
        let bytes = std::fs::read(self.probe_path()).context(READ_PROBE_CONTEXT)?;
        serde_json::from_slice(&bytes).context(READ_PROBE_CONTEXT)
    }

    fn from_environment(terminal_id: Uuid, phase: CustodyReleaseBoundary) -> Result<Option<Self>> {
        std::env::var_os(BARRIER_ENVIRONMENT)
            .map(|root| Self::new(PathBuf::from(root), terminal_id, phase))
            .transpose()
    }

    fn claim(&self, probe: Option<PtyMasterFdCustodyProbe>) -> Result<bool> {
        if !self
            .arm_path()
            .try_exists()
            .context(CLAIM_BARRIER_CONTEXT)?
        {
            return Ok(false);
        }
        let probe = probe.context(PROBE_REQUIRED_CONTEXT)?;
        let bytes = serde_json::to_vec(&probe).context(WRITE_PROBE_CONTEXT)?;
        std::fs::write(self.probe_path(), bytes).context(WRITE_PROBE_CONTEXT)?;
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

    fn arm_path(&self) -> PathBuf {
        self.directory.join(ARM_FILE_NAME)
    }
    fn reached_path(&self) -> PathBuf {
        self.directory.join(REACHED_FILE_NAME)
    }
    fn release_path(&self) -> PathBuf {
        self.directory.join(RELEASE_FILE_NAME)
    }
    fn probe_path(&self) -> PathBuf {
        self.directory.join(PROBE_FILE_NAME)
    }
}

pub(super) async fn pause(
    terminal_id: Uuid,
    phase: CustodyReleaseBoundary,
    probe: Option<PtyMasterFdCustodyProbe>,
) -> Result<()> {
    let Some(barrier) = CustodyReleaseTestBarrier::from_environment(terminal_id, phase)? else {
        return Ok(());
    };
    if !barrier.claim(probe)? {
        return Ok(());
    }
    while !barrier.released()? {
        tokio::time::sleep(BARRIER_POLL_INTERVAL).await;
    }
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context(CLAIM_BARRIER_CONTEXT),
    }
}
