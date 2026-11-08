use crate::{Launch, constants::*, retry};
use anyhow::{Result, ensure};
use std::path::PathBuf;
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_state::HubState;
use yolo_paths::{coordination::CoordinationLock, directory::PrivateDirectory};

/// Holds startup custody and the complete public identity across every startup wait.
pub(crate) struct StartupFence {
    directory: PrivateDirectory,
    lock: CoordinationLock,
    host_home: PathBuf,
    pub(crate) peer: PeerIdentity,
}

impl StartupFence {
    pub(crate) async fn acquire(
        launch: &Launch,
        expected: Option<PeerIdentity>,
        deadline: Instant,
    ) -> Result<Self> {
        retry::require_budget(deadline)?;
        let path = yolo_paths::platform::platform_path(&launch.state);
        let directory = if expected.is_some() {
            PrivateDirectory::open(&path)?
        } else {
            PrivateDirectory::ensure(&path)?
        };
        Self::acquire_in(launch, expected, directory, deadline).await
    }

    pub(crate) async fn acquire_in(
        launch: &Launch,
        expected: Option<PeerIdentity>,
        directory: PrivateDirectory,
        deadline: Instant,
    ) -> Result<Self> {
        retry::require_budget(deadline)?;
        yolo_paths::validation::validate_absolute(&launch.state)?;
        let path = yolo_paths::platform::platform_path(&launch.state);
        ensure!(directory.path() == path, PIN_MISMATCH);
        directory.verify_path()?;
        let lock_path = directory.path().join(START_LOCK_FILE);
        let lock = loop {
            retry::require_budget(deadline)?;
            directory.verify_path()?;
            check_pin(&path, &launch.host_home, expected)?;
            if let Some(lock) = CoordinationLock::try_acquire(&lock_path, MAX_LOCK_BYTES)? {
                break lock;
            }
            retry::pause(deadline).await?;
        };
        directory.verify_path()?;
        lock.verify()?;
        retry::require_budget(deadline)?;
        let current = HubState::inspect_peer_identity(&path, &launch.host_home)?;
        ensure!(
            expected.is_none_or(|peer| current == Some(peer)),
            PIN_MISMATCH
        );
        let peer = match current {
            Some(peer) => peer,
            None => HubState::open(&path, &launch.host_home)?.peer_identity(),
        };
        let fence = Self {
            directory,
            lock,
            host_home: launch.host_home.clone(),
            peer,
        };
        fence.verify()?;
        Ok(fence)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.directory.verify_path()?;
        self.lock.verify()?;
        check_pin(self.directory.path(), &self.host_home, Some(self.peer))?;
        self.directory.verify_path()?;
        self.lock.verify()
    }
}

fn check_pin(
    path: &std::path::Path,
    home: &std::path::Path,
    expected: Option<PeerIdentity>,
) -> Result<()> {
    if let Some(expected) = expected {
        ensure!(
            HubState::inspect_peer_identity(path, home)? == Some(expected),
            PIN_MISMATCH
        );
    }
    Ok(())
}
