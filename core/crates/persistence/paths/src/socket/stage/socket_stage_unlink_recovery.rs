use super::super::{LIFECYCLE_FILE_NAME, Target, lease::acquire_lifecycle, record};
use super::Cleanup;
use crate::paths::{
    constants::{PRIVATE_FILE_UNLINK_ISOLATION_PREFIX, SOCKET_STAGE_LEASE_MAX_BYTES},
    directory::PrivateDirectory,
    private_file::{PrivateFileEntry, is_storage_artifact},
    private_file_lock::PrivateFileLock,
};
use anyhow::Result;
use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt,
};

pub(crate) fn is_candidate(name: &CStr) -> bool {
    name.to_bytes()
        .starts_with(PRIVATE_FILE_UNLINK_ISOLATION_PREFIX.as_bytes())
        && is_storage_artifact(OsStr::from_bytes(name.to_bytes()))
}

pub(crate) fn recover(directory: &PrivateDirectory, name: &CStr) -> Result<Cleanup> {
    if !is_candidate(name) {
        return Ok(Cleanup::Unproven);
    }
    let lifecycle = match acquire_lifecycle(directory, &directory.path().join(LIFECYCLE_FILE_NAME))
    {
        Ok(Some(lease)) => lease,
        Ok(None) => return Ok(Cleanup::Busy),
        Err(error) if record::is_unproven(&error) => return Ok(Cleanup::Unproven),
        Err(error) => return Err(error),
    };
    let path = directory.path().join(OsStr::from_bytes(name.to_bytes()));
    let mut isolated = match PrivateFileLock::try_acquire_existing_in(
        directory.try_clone()?,
        &path,
        SOCKET_STAGE_LEASE_MAX_BYTES,
    ) {
        Ok(Some(lock)) => lock,
        Ok(None) => return Ok(Cleanup::Busy),
        Err(error) if record::is_unproven(&error) => return Ok(Cleanup::Unproven),
        Err(error) => return Err(error),
    };
    let record = match record::decode(&isolated.read_contents()?) {
        Ok(record) if record.effective_user_id == unsafe { libc::geteuid() } => record,
        _ => return Ok(Cleanup::Unproven),
    };
    let target = Target::from_existing_digest(
        directory.path(),
        record.effective_user_id,
        record.target_digest,
    )?;
    let source = PrivateFileEntry::from_directory(directory.try_clone()?, &target.lease()?.1)?;
    // The canonical record and deterministic artifact name must identify the same lease.
    if source.unlink_isolation_entry()?.file_name() != name {
        return Ok(Cleanup::Unproven);
    }
    isolated.reclaim_unlink_isolation_of(&source)?;
    drop(lifecycle);
    Ok(Cleanup::Mutated)
}
