use super::*;
use crate::paths::identity::ObjectIdentity;
use crate::paths::process_file_lock::{Mode, ProcessFileLock, ProcessLockTurn};
use anyhow::{Context, Result};
use std::fs::File;
use std::os::fd::BorrowedFd;
use std::time::{Duration, Instant};

const LIFECYCLE_LOCK_CONTEXT: &str = "lock socket namespace lifecycle";
const LIFECYCLE_TURN_MISSING: &str = "lifecycle admission turn was not retained";
const ROOT_LOCK_CHANGED: &str = "socket namespace lifecycle authority belongs to another root";

pub(crate) struct SocketNamespaceLifecycleLock {
    root_lock: ProcessFileLock,
    root_identity: ObjectIdentity,
    _turn: ProcessLockTurn,
}

impl SocketNamespaceLifecycleLock {
    pub(super) fn acquire(root: &PrivateDirectory) -> Result<Self> {
        let identity = root.object_identity()?;
        let Some(turn) = ProcessLockTurn::acquire(identity, SOCKET_LIFECYCLE_LOCK_TIMEOUT)? else {
            anyhow::bail!(LIFECYCLE_LOCK_CONTEXT);
        };
        let mut turn = Some(turn);
        let deadline = Instant::now() + SOCKET_LIFECYCLE_LOCK_TIMEOUT;
        loop {
            if let Some(lock) = Self::try_acquire_inner(root, &mut turn)? {
                return Ok(lock);
            }
            if !retry_until(deadline) {
                anyhow::bail!(LIFECYCLE_LOCK_CONTEXT);
            }
        }
    }

    pub(crate) fn try_acquire(root: &PrivateDirectory) -> Result<Option<Self>> {
        let Some(turn) = ProcessLockTurn::try_acquire(root.object_identity()?)? else {
            return Ok(None);
        };
        Self::try_acquire_inner(root, &mut Some(turn))
    }

    fn try_acquire_inner(
        root: &PrivateDirectory,
        turn: &mut Option<ProcessLockTurn>,
    ) -> Result<Option<Self>> {
        let root_identity = root.object_identity()?;
        let descriptor =
            unsafe { BorrowedFd::borrow_raw(root.descriptor()) }.try_clone_to_owned()?;
        let root_file = File::from(descriptor);
        let Some(root_lock) = ProcessFileLock::try_lock(&root_file, root_identity, Mode::Exclusive)
            .context(LIFECYCLE_LOCK_CONTEXT)?
        else {
            return Ok(None);
        };
        root.verify_path()?;
        Ok(Some(Self {
            root_lock,
            root_identity,
            _turn: turn.take().expect(LIFECYCLE_TURN_MISSING),
        }))
    }

    pub(super) fn verify_root(&self, root: &PrivateDirectory) -> Result<()> {
        root.verify_path()?;
        anyhow::ensure!(
            root.object_identity()? == self.root_identity
                && self.root_lock.holds_exclusive(self.root_identity),
            ROOT_LOCK_CHANGED
        );
        Ok(())
    }
}

fn retry_until(deadline: Instant) -> bool {
    let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
        return false;
    };
    std::thread::sleep(remaining.min(Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS)));
    Instant::now() < deadline
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_root_lock_tests.rs"]
mod root_tests;
