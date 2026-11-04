use super::*;
use crate::paths::constants::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::{Context, Result};
use std::mem::MaybeUninit;
use std::time::Instant;

pub(super) const LIFECYCLE_BUSY_ERROR: &str = "socket publication lifecycle lease is busy";
pub(super) const TARGET_BUSY_ERROR: &str = "socket publication target stage lease is busy";
const LEASE_COLLISION_ERROR: &str = "socket publication stage key collision";
const LEASE_INCOMPLETE_ERROR: &str = "incomplete stage lease still has socket residue";
const LIFECYCLE_WAIT_JOIN_CONTEXT: &str = "join socket publication lifecycle lease wait";
const STAGE_INSPECTION_CONTEXT: &str = "inspect socket publication stage residue";

pub(crate) struct Lease {
    _lock: PrivateFileLock,
}

impl Lease {
    pub(crate) async fn acquire(directory: &PrivateDirectory, target: &Target) -> Result<Self> {
        let deadline = Instant::now() + SOCKET_STAGE_LEASE_TIMEOUT;
        loop {
            let lifecycle = wait_for_lifecycle(directory, target).await?;
            let lease = try_acquire_target(directory, target)?;
            drop(lifecycle);
            if let Some(mut lease) = lease {
                validate_or_initialize(directory, target, &mut lease)?;
                return Ok(Self { _lock: lease });
            }
            anyhow::ensure!(Instant::now() < deadline, TARGET_BUSY_ERROR);
            tokio::time::sleep(std::time::Duration::from_millis(
                SOCKET_STAGE_LEASE_RETRY_MILLIS,
            ))
            .await;
        }
    }

    #[cfg(test)]
    pub(crate) fn acquire_immediate_for_test(
        directory: &PrivateDirectory,
        target: &Target,
    ) -> Result<Self> {
        let lifecycle = acquire_lifecycle(directory, &target.lifecycle_path())?
            .context(LIFECYCLE_BUSY_ERROR)?;
        let mut lease = try_acquire_target(directory, target)?.context(TARGET_BUSY_ERROR)?;
        drop(lifecycle);
        validate_or_initialize(directory, target, &mut lease)?;
        Ok(Self { _lock: lease })
    }
}

async fn wait_for_lifecycle(
    directory: &PrivateDirectory,
    target: &Target,
) -> Result<PrivateFileLock> {
    let directory = directory.try_clone()?;
    let path = target.lifecycle_path();
    tokio::task::spawn_blocking(move || {
        PrivateFileLock::acquire_in_wait(
            directory,
            &path,
            SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
            SOCKET_LIFECYCLE_LOCK_TIMEOUT,
            std::time::Duration::from_millis(SOCKET_STAGE_LEASE_RETRY_MILLIS),
        )
    })
    .await
    .context(LIFECYCLE_WAIT_JOIN_CONTEXT)?
    .context(LIFECYCLE_BUSY_ERROR)
}

fn try_acquire_target(
    directory: &PrivateDirectory,
    target: &Target,
) -> Result<Option<PrivateFileLock>> {
    PrivateFileLock::try_acquire_in(
        directory.try_clone()?,
        &target.lease()?.1,
        SOCKET_STAGE_LEASE_MAX_BYTES,
    )
}

pub(super) fn acquire_lifecycle(
    directory: &PrivateDirectory,
    path: &Path,
) -> Result<Option<PrivateFileLock>> {
    PrivateFileLock::try_acquire_in(
        directory.try_clone()?,
        path,
        SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
    )
}

pub(super) fn acquire_existing_lifecycle(
    directory: &PrivateDirectory,
    path: &Path,
) -> Result<Option<PrivateFileLock>> {
    PrivateFileLock::try_acquire_existing_in(
        directory.try_clone()?,
        path,
        SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
    )
}

fn validate_or_initialize(
    directory: &PrivateDirectory,
    target: &Target,
    lease: &mut PrivateFileLock,
) -> Result<()> {
    let bytes = lease.read_contents()?;
    if bytes.is_empty() {
        for name in target.residue_names()? {
            anyhow::ensure!(!stage_exists(directory, &name)?, LEASE_INCOMPLETE_ERROR);
        }
        lease.replace_contents(&record::encode(target)?)?;
        directory.sync()?;
    } else {
        anyhow::ensure!(
            record::matches_target(&bytes, target)?,
            LEASE_COLLISION_ERROR
        );
    }
    Ok(())
}

pub(super) fn stage_exists(directory: &PrivateDirectory, name: &CStr) -> Result<bool> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.descriptor(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(error).context(STAGE_INSPECTION_CONTEXT)
    }
}
