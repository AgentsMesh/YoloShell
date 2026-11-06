use super::*;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WorkerSocketIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
pub(super) fn worker_bound_socket_identity(path: &Path) -> io::Result<WorkerSocketIdentity> {
    worker_socket_metadata(path, unsafe { libc::geteuid() })
        .map(|metadata| worker_identity_from_metadata(&metadata))
}

#[cfg(unix)]
pub(super) fn worker_socket_identity(path: &Path) -> io::Result<WorkerSocketIdentity> {
    worker_socket_identity_for_owner(path, unsafe { libc::geteuid() })
}

#[cfg(unix)]
pub(super) fn worker_socket_identity_for_owner(
    path: &Path,
    expected_owner: libc::uid_t,
) -> io::Result<WorkerSocketIdentity> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = worker_socket_metadata(path, expected_owner)?;
    if metadata.permissions().mode() & WORKER_SOCKET_PERMISSION_MASK
        != WORKER_SOCKET_PERMISSION_MODE
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            WORKER_SOCKET_PERMISSION_ERROR,
        ));
    }
    Ok(worker_identity_from_metadata(&metadata))
}

#[cfg(unix)]
pub(super) fn worker_socket_metadata(
    path: &Path,
    expected_owner: libc::uid_t,
) -> io::Result<std::fs::Metadata> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            WORKER_SOCKET_TYPE_ERROR,
        ));
    }
    if metadata.uid() != expected_owner {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            WORKER_SOCKET_OWNER_ERROR,
        ));
    }
    Ok(metadata)
}

#[cfg(unix)]
pub(super) fn worker_identity_from_metadata(metadata: &std::fs::Metadata) -> WorkerSocketIdentity {
    use std::os::unix::fs::MetadataExt;
    WorkerSocketIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

pub(super) fn set_worker_socket_permissions(path: &Path, expected: WorkerSocketIdentity) -> Result<()> {
    set_worker_socket_permissions_inner(path, expected, || Ok(()))
}

#[cfg(test)]
pub(super) fn set_current_worker_socket_permissions(path: &Path) -> Result<()> {
    let identity = worker_bound_socket_identity(path)?;
    set_worker_socket_permissions(path, identity)
}

#[cfg(test)]
pub(super) fn set_worker_socket_permissions_with_hook<F>(
    path: &Path,
    expected: WorkerSocketIdentity,
    before_chmod: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    set_worker_socket_permissions_inner(path, expected, before_chmod)
}

pub(super) fn set_worker_socket_permissions_inner<F>(
    path: &Path,
    expected: WorkerSocketIdentity,
    before_chmod: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    anyhow::ensure!(
        worker_bound_socket_identity(path).ok() == Some(expected),
        WORKER_SOCKET_CHANGED_CONTEXT
    );
    before_chmod()?;
    anyhow::ensure!(
        worker_bound_socket_identity(path).ok() == Some(expected),
        WORKER_SOCKET_CHANGED_CONTEXT
    );
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(WORKER_SOCKET_PERMISSION_MODE);
    std::fs::set_permissions(path, permissions).context(WORKER_SOCKET_SECURE_CONTEXT)?;
    anyhow::ensure!(
        worker_socket_identity(path).ok() == Some(expected),
        WORKER_SOCKET_CHANGED_CONTEXT
    );
    Ok(())
}
