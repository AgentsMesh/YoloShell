use super::super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

pub(super) fn clean(root: &PrivateDirectory, name: &OsStr) -> Result<bool> {
    let path = root.path().join(name);
    let Some(_lease) = CoordinationLease::try_acquire(&path)? else {
        return Ok(false);
    };
    let lock = PrivateFileLock::try_acquire_existing_in(
        root.try_clone()?,
        &path,
        MAX_COORDINATION_LOCK_BYTES,
    );
    let Some(mut lock) = (match lock {
        Ok(lock) => lock,
        Err(error) if changed_or_unproven(&error) => return Ok(false),
        Err(error) => return Err(error).context(SWEEP_OPEN_CONTEXT),
    }) else {
        return Ok(false);
    };
    retire::reclaim_locked(&mut lock, root)
}

pub(super) fn is_lock_name(name: &OsStr) -> bool {
    let bytes = name.as_bytes();
    let Some(stem) = bytes.strip_suffix(LOCK_FILE_EXTENSION.as_bytes()) else {
        return false;
    };
    let labels = [
        HOST_HOME_SCOPE,
        HOST_ENDPOINT_SCOPE,
        WORKER_SCOPE,
        BROKER_INSTANCE_SCOPE,
        BROKER_LAUNCH_PUBLICATION_SCOPE,
        BROKER_RECONCILE_SCOPE,
        SOCKET_MAINTENANCE_SCOPE,
    ];
    labels.iter().any(|label| {
        let mut prefix = label.as_bytes().to_vec();
        prefix.push(b'-');
        stem.strip_prefix(prefix.as_slice())
            .is_some_and(valid_digest)
    })
}

fn valid_digest(value: &[u8]) -> bool {
    value.len() == LOCK_DIGEST_HEX_BYTES
        && value
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

pub(in crate::coordination) fn changed_or_unproven(error: &anyhow::Error) -> bool {
    let mut saw_io = false;
    for cause in error.chain() {
        let Some(io) = cause.downcast_ref::<std::io::Error>() else {
            continue;
        };
        saw_io = true;
        if matches!(
            io.kind(),
            std::io::ErrorKind::NotFound
                | std::io::ErrorKind::PermissionDenied
                | std::io::ErrorKind::IsADirectory
                | std::io::ErrorKind::NotADirectory
                | std::io::ErrorKind::InvalidInput
                | std::io::ErrorKind::InvalidData
        ) || matches!(
            io.raw_os_error(),
            Some(libc::ELOOP) | Some(libc::EISDIR) | Some(libc::ENOTDIR)
        ) {
            return true;
        }
    }
    !saw_io
}
