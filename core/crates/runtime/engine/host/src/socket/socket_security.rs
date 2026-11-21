pub(crate) fn validate_socket_path(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if path.as_os_str().as_bytes().len() >= UNIX_SOCKET_PATH_MAX_BYTES {
            bail!("host socket path is too long: {}", path.display());
        }
    }
    Ok(())
}

pub(crate) fn validate_socket_namespace_selection(
    socket: &Path,
    default: &Path,
    reserved_root: &Path,
) -> Result<()> {
    let socket = crate::paths::platform::platform_path(socket);
    let default = crate::paths::platform::platform_path(default);
    let reserved_root = crate::paths::platform::platform_path(reserved_root);
    anyhow::ensure!(
        !socket.starts_with(reserved_root) || socket == default,
        RESERVED_SOCKET_NAMESPACE_ERROR
    );
    Ok(())
}

pub(crate) fn ensure_socket_is_not_lock(socket: &Path, lock: &Path) -> Result<()> {
    anyhow::ensure!(
        socket != lock,
        "host socket cannot replace the active instance lock: {}",
        socket.display()
    );
    let socket = match socket.canonicalize() {
        Ok(socket) => socket,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("resolve host socket path {}", socket.display()));
        }
    };
    let lock = lock
        .canonicalize()
        .with_context(|| format!("resolve host lock path {}", lock.display()))?;
    anyhow::ensure!(
        socket != lock,
        "host socket cannot replace the active instance lock: {}",
        socket.display()
    );
    Ok(())
}

pub(crate) fn socket_lock_path(socket: &Path) -> Result<PathBuf> {
    crate::coordination::lock_path(
        crate::coordination::CoordinationScope::HostEndpoint,
        socket,
        None,
    )
}

pub(crate) fn verify_peer_uid(stream: &UnixStream) -> Result<()> {
    let peer = stream
        .peer_cred()
        .context("read local host client credentials")?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    validate_peer_uid(peer.uid(), unsafe { libc::geteuid() })
}

pub(crate) fn validate_peer_uid(peer_uid: libc::uid_t, host_uid: libc::uid_t) -> Result<()> {
    anyhow::ensure!(
        peer_uid == host_uid,
        "rejecting local host client owned by uid {peer_uid}; expected uid {host_uid}"
    );
    Ok(())
}

#[cfg(unix)]
#[cfg(test)]
pub(crate) fn bound_socket_identity(path: &Path) -> std::io::Result<SocketIdentity> {
    socket_metadata(path).map(|metadata| identity_from_metadata(&metadata))
}

#[cfg(unix)]
#[cfg(test)]
pub(crate) fn socket_identity(path: &Path) -> std::io::Result<SocketIdentity> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = socket_metadata(path)?;
    let mode = metadata.permissions().mode() & HOST_SOCKET_PERMISSION_MASK;
    if mode != crate::paths::constants::PRIVATE_FILE_MODE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            HOST_SOCKET_PERMISSION_ERROR,
        ));
    }
    Ok(identity_from_metadata(&metadata))
}

#[cfg(unix)]
#[cfg(test)]
pub(crate) fn socket_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_socket() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            HOST_SOCKET_TYPE_ERROR,
        ));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            HOST_SOCKET_OWNER_ERROR,
        ));
    }
    Ok(metadata)
}

#[cfg(unix)]
#[cfg(test)]
pub(crate) fn identity_from_metadata(metadata: &std::fs::Metadata) -> SocketIdentity {
    use std::os::unix::fs::MetadataExt;
    SocketIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(test)]
pub(crate) fn set_socket_permissions(path: &Path, expected: SocketIdentity) -> Result<()> {
    set_socket_permissions_inner(path, expected, || Ok(()))
}

#[cfg(test)]
pub(crate) fn set_current_socket_permissions(path: &Path) -> Result<()> {
    let identity = bound_socket_identity(path)?;
    set_socket_permissions(path, identity)
}

#[cfg(test)]
pub(crate) fn set_socket_permissions_with_hook<F>(
    path: &Path,
    expected: SocketIdentity,
    before_chmod: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    set_socket_permissions_inner(path, expected, before_chmod)
}

#[cfg(test)]
pub(crate) fn set_socket_permissions_inner<F>(
    path: &Path,
    expected: SocketIdentity,
    before_chmod: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    anyhow::ensure!(
        bound_socket_identity(path).ok() == Some(expected),
        HOST_SOCKET_CHANGED_CONTEXT
    );
    before_chmod()?;
    anyhow::ensure!(
        bound_socket_identity(path).ok() == Some(expected),
        HOST_SOCKET_CHANGED_CONTEXT
    );
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE);
    std::fs::set_permissions(path, permissions).context("secure bound host socket")?;
    anyhow::ensure!(
        socket_identity(path).ok() == Some(expected),
        HOST_SOCKET_CHANGED_CONTEXT
    );
    Ok(())
}
