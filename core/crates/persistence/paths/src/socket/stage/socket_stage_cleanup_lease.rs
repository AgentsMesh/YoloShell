use super::lease::acquire_lifecycle;
use super::*;
use crate::paths::constants::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::Result;

pub(crate) struct CleanupLease {
    directory: PrivateDirectory,
    lease: PrivateFileLock,
    target: Target,
}

pub(crate) enum LeaseState {
    Acquired(Box<CleanupLease>),
    Busy,
    Unproven,
}

impl CleanupLease {
    pub(crate) fn target(&self) -> &Target {
        &self.target
    }

    pub(crate) fn remove(mut self) -> Result<bool> {
        let lifecycle_path = self.directory.path().join(LIFECYCLE_FILE_NAME);
        let Some(lifecycle) = acquire_lifecycle(&self.directory, &lifecycle_path)? else {
            return Ok(false);
        };
        self.lease.unlink_owned()?;
        self.directory.sync()?;
        drop(self.lease);
        drop(lifecycle);
        Ok(true)
    }
}

pub(crate) fn try_cleanup(
    directory: &PrivateDirectory,
    key: &str,
    target_parent: &Path,
) -> Result<LeaseState> {
    let target = record::target_from_key(directory.path(), key)?;
    let Some(lifecycle) = acquire_lifecycle(directory, &target.lifecycle_path())? else {
        return Ok(LeaseState::Busy);
    };
    let opened = PrivateFileLock::try_acquire_existing_in(
        directory.try_clone()?,
        &target.lease()?.1,
        SOCKET_STAGE_LEASE_MAX_BYTES,
    );
    let Some(mut lease) = (match opened {
        Ok(lease) => lease,
        Err(error) if record::is_unproven(&error) => return Ok(LeaseState::Unproven),
        Err(error) => return Err(error),
    }) else {
        return Ok(LeaseState::Busy);
    };
    drop(lifecycle);
    let record = match record::decode(&lease.read_contents()?) {
        Ok(record)
            if record.effective_user_id == unsafe { libc::geteuid() }
                && record.target_digest.starts_with(key) =>
        {
            record
        }
        Ok(_) => return Ok(LeaseState::Unproven),
        Err(error) if record::is_unproven(&error) => return Ok(LeaseState::Unproven),
        Err(error) => return Err(error),
    };
    let target = Target::from_existing_digest(
        target_parent,
        record.effective_user_id,
        record.target_digest,
    )?;
    Ok(LeaseState::Acquired(Box::new(CleanupLease {
        directory: directory.try_clone()?,
        lease,
        target,
    })))
}
