use super::{
    SocketTarget,
    io::{SocketIdentity, inspect_at, socket_matches_at, unlink_at},
};
use crate::paths::{
    constants::{SOCKET_ENDPOINT_LINK_COUNT, SOCKET_PROBE_LINK_COUNT},
    directory::PrivateDirectory,
    private_file_lock::{PrivateFileLock, RetainedPrivateFileVerification},
    socket_stage,
};
use anyhow::{Result, ensure};
use std::{ffi::CString, os::unix::ffi::OsStrExt, path::PathBuf};
#[path = "socket_namespace_unbound_connect.rs"]
mod connect;

const CHANGED: &str = "unbound socket probe identity changed";
const LOCK_REQUIRED: &str = "unbound socket probe requires the retained publication lifecycle lock";
const ALIAS_BUSY: &str = "unbound socket probe alias is already occupied";
const LINK_FAILED: &str = "create exact inode unbound socket probe alias";

pub(crate) fn prove_unbound_refused(
    directory: &PrivateDirectory,
    name: &CString,
    stage: &PrivateFileLock,
) -> Result<bool> {
    let path = directory
        .path()
        .join(std::ffi::OsStr::from_bytes(name.as_bytes()));
    let target = SocketTarget::new(directory.try_clone()?, &path)?;
    ensure!(stage.path() == target.stage.lifecycle_path(), LOCK_REQUIRED);
    verify_lock(stage)?;
    let identity = target
        .inspect(true)?
        .ok_or_else(|| anyhow::anyhow!(CHANGED))?;
    ensure!(
        socket_matches_at(directory, name, true, identity, SOCKET_ENDPOINT_LINK_COUNT)?,
        CHANGED
    );
    let (alias_name, alias_path) = target.stage.stage(socket_stage::Kind::Probe)?;
    ensure!(
        inspect_at(directory, &alias_name, false)?.is_none(),
        ALIAS_BUSY
    );
    let mut alias = Alias {
        directory: directory.try_clone()?,
        name: alias_name,
        path: alias_path,
        identity: None,
    };
    let result = unsafe {
        libc::linkat(
            directory.descriptor(),
            name.as_ptr(),
            directory.descriptor(),
            alias.name.as_ptr(),
            0,
        )
    };
    ensure!(
        result == 0,
        "{LINK_FAILED}: {}",
        std::io::Error::last_os_error()
    );
    alias.identity = Some(identity);
    verify_pair(&target, &alias, identity)?;
    let refused = connect::refused(&alias.path)?;
    verify_lock(stage)?;
    verify_pair(&target, &alias, identity)?;
    alias.remove()?;
    ensure!(
        socket_matches_at(directory, name, true, identity, SOCKET_ENDPOINT_LINK_COUNT)?,
        CHANGED
    );
    Ok(refused)
}

fn verify_lock(stage: &PrivateFileLock) -> Result<()> {
    stage.verify()?;
    ensure!(
        matches!(
            stage.verify_retained_path(),
            RetainedPrivateFileVerification::Verified
        ),
        LOCK_REQUIRED
    );
    Ok(())
}

fn verify_pair(target: &SocketTarget, alias: &Alias, identity: SocketIdentity) -> Result<()> {
    target.directory.verify_path()?;
    ensure!(
        socket_matches_at(
            &target.directory,
            &target.name,
            true,
            identity,
            SOCKET_PROBE_LINK_COUNT
        )? && socket_matches_at(
            &alias.directory,
            &alias.name,
            true,
            identity,
            SOCKET_PROBE_LINK_COUNT
        )?,
        CHANGED
    );
    Ok(())
}

struct Alias {
    directory: PrivateDirectory,
    name: CString,
    path: PathBuf,
    identity: Option<SocketIdentity>,
}

impl Alias {
    fn remove(&mut self) -> Result<()> {
        let Some(identity) = self.identity else {
            return Ok(());
        };
        ensure!(
            socket_matches_at(
                &self.directory,
                &self.name,
                true,
                identity,
                SOCKET_PROBE_LINK_COUNT
            )? || socket_matches_at(
                &self.directory,
                &self.name,
                true,
                identity,
                SOCKET_ENDPOINT_LINK_COUNT
            )?,
            CHANGED
        );
        unlink_at(&self.directory, &self.name)?;
        self.identity = None;
        self.directory.sync()?;
        Ok(())
    }
}
impl Drop for Alias {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
