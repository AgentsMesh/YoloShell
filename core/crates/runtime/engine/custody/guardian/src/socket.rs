use anyhow::{Context, Result, ensure};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;

const BIND_CONTEXT: &str = "bind detached PTY custody guardian socket";
const IDENTITY_DETAIL: &str = "detached PTY custody guardian socket identity changed";
const PRIVATE_SOCKET_MODE: u32 = 0o600;

pub(super) struct GuardianSocket {
    listener: UnixListener,
    path: PathBuf,
    identity: crate::paths::identity::ObjectIdentity,
}

impl GuardianSocket {
    pub(super) fn bind(path: PathBuf) -> Result<Self> {
        ensure!(!path.exists(), BIND_CONTEXT);
        let listener = UnixListener::bind(&path).context(BIND_CONTEXT)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_SOCKET_MODE))?;
        let metadata = std::fs::symlink_metadata(&path)?;
        ensure!(metadata.file_type().is_socket(), IDENTITY_DETAIL);
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            IDENTITY_DETAIL
        );
        ensure!(
            metadata.mode() & crate::paths::constants::PERMISSION_MODE_MASK == PRIVATE_SOCKET_MODE,
            IDENTITY_DETAIL
        );
        let identity = crate::paths::identity::ObjectIdentity::from_metadata(&metadata);
        Ok(Self {
            listener,
            path,
            identity,
        })
    }

    pub(super) async fn accept(&self) -> Result<tokio::net::UnixStream> {
        self.listener
            .accept()
            .await
            .map(|(stream, _)| stream)
            .map_err(Into::into)
    }

    fn still_owned(&self) -> bool {
        std::fs::symlink_metadata(&self.path)
            .ok()
            .filter(|metadata| metadata.file_type().is_socket())
            .is_some_and(|metadata| {
                crate::paths::identity::ObjectIdentity::from_metadata(&metadata) == self.identity
            })
    }
}

impl Drop for GuardianSocket {
    fn drop(&mut self) {
        if self.still_owned() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(super) fn path_for(paths: &crate::paths::Paths, guardian_id: uuid::Uuid) -> Result<PathBuf> {
    paths.custody_guardian_socket(guardian_id)
}

pub(super) fn verify_peer(
    stream: &crate::pty_custody::CustodyUnixStream,
    holder: crate::pty_custody::CustodyHolderIdentity,
) -> Result<()> {
    ensure!(stream.peer_pid()? == holder.pid, IDENTITY_DETAIL);
    ensure!(holder.matches(), IDENTITY_DETAIL);
    Ok(())
}

pub(super) fn verify_named(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(metadata.file_type().is_socket(), IDENTITY_DETAIL);
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        IDENTITY_DETAIL
    );
    ensure!(
        metadata.mode() & crate::paths::constants::PERMISSION_MODE_MASK == PRIVATE_SOCKET_MODE,
        IDENTITY_DETAIL
    );
    Ok(())
}
