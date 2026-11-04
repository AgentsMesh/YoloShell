use super::super::{directory_certificate, directory_lease, empty_remove, lifecycle_lock};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result, ensure};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
#[path = "socket_namespace_orphan_bookkeeping.rs"]
mod bookkeeping;

const CERTIFICATE_CHANGED: &str = "socket namespace stage certificate changed";
const DIRECTORY_SCOPE_CHANGED: &str = "socket namespace cleanup directory is outside its root";

pub(in super::super) struct StageAuthority {
    directory: PrivateDirectory,
    certificate: directory_certificate::Certificate,
    live_identity: crate::paths::identity::ObjectIdentity,
}

impl StageAuthority {
    pub(in super::super) fn capture(
        directory: &PrivateDirectory,
        name: &OsStr,
    ) -> Result<Option<Self>> {
        let certificate = match directory_certificate::read(directory) {
            Ok(Some(certificate)) => certificate,
            Ok(None) | Err(_) => return Ok(None),
        };
        if certificate
            .binding
            .home_digest
            .as_bytes()
            .get(..name.as_bytes().len())
            != Some(name.as_bytes())
        {
            return Ok(None);
        }
        Ok(Some(Self {
            live_identity: directory.object_identity()?,
            directory: directory.try_clone()?,
            certificate,
        }))
    }

    pub(in super::super) fn verify(&self) -> Result<()> {
        ensure!(
            self.directory.object_identity()? == self.live_identity,
            CERTIFICATE_CHANGED
        );
        directory_certificate::verify(&self.directory, &self.certificate)
    }

    /// Only current certificates assert that every participant holds a directory lease.
    /// No durable data or endpoint can be removed by this transport-only recovery.
    pub(in super::super) fn cleanup_empty(
        &self,
        root: &PrivateDirectory,
        lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
    ) -> Result<bool> {
        lifecycle.verify_root(root)?;
        self.verify()?;
        let Some(lease) = directory_lease::DirectoryLease::try_exclusive(&self.directory)? else {
            return Ok(false);
        };
        ensure!(
            self.directory.path().parent() == Some(root.path()),
            DIRECTORY_SCOPE_CHANGED
        );
        if !bookkeeping::settle(&self.directory, || {
            lifecycle.verify_root(root)?;
            lease.verify(&self.directory)?;
            self.verify()
        })? {
            return Ok(false);
        }
        if !only_lifecycle_or_empty(&self.directory)? {
            return Ok(false);
        }
        lease.verify(&self.directory)?;
        self.verify()?;
        match crate::paths::socket_stage::cleanup_lifecycle(&self.directory)? {
            crate::paths::socket_stage::Cleanup::Busy
            | crate::paths::socket_stage::Cleanup::Unproven => return Ok(false),
            crate::paths::socket_stage::Cleanup::Mutated
            | crate::paths::socket_stage::Cleanup::Unchanged => {}
        }
        lease.verify(&self.directory)?;
        self.verify()?;
        let name = crate::paths::directory::entry_c_string(
            self.directory
                .path()
                .file_name()
                .context(DIRECTORY_SCOPE_CHANGED)?,
        )?;
        empty_remove::owned_locked(root, &self.directory, &name, lifecycle)
    }
}

fn only_lifecycle_or_empty(directory: &PrivateDirectory) -> Result<bool> {
    let mut entries =
        crate::paths::directory_stream::DirectoryStream::open(directory.descriptor())?;
    let Some(name) = entries.next_name()? else {
        return Ok(true);
    };
    let name = crate::paths::directory::entry_c_string(&name)?;
    Ok(crate::paths::socket_stage::is_lifecycle(&name) && entries.next_name()?.is_none())
}
