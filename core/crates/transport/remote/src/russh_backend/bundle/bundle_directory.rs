use super::{
    bundle_raw::*,
    deployment_operation::{optional, sftp as call},
};
use crate::remote::{RemoteError, RemoteFilesystemOperation as Op};
use russh_sftp::{client::RawSftpSession, protocol::FileAttributes};
use std::{path::Path, time::Duration};
use yolo_distribution_publication_protocol::DIRECTORY_MODE;

#[derive(Clone, Copy)]
pub(super) struct DirectorySeal {
    uid: u32,
    gid: u32,
}

impl DirectorySeal {
    pub fn uid(self) -> u32 {
        self.uid
    }
    pub fn verify_metadata(self, attributes: &FileAttributes) -> Result<(), RemoteError> {
        let current = Self::new(attributes)?;
        if current.uid != self.uid || current.gid != self.gid {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn new(attributes: &FileAttributes) -> Result<Self, RemoteError> {
        if !attributes.is_dir()
            || attributes.permissions.map(|mode| mode & PERMISSION_MASK) != Some(DIRECTORY_MODE)
        {
            return Err(invalid());
        }
        Ok(Self {
            uid: attributes.uid.ok_or_else(invalid)?,
            gid: attributes.gid.ok_or_else(invalid)?,
        })
    }
    pub async fn verify(
        self,
        raw: &RawSftpSession,
        path: &str,
        limit: Duration,
    ) -> Result<(), RemoteError> {
        let attributes = call(limit, Op::InspectLink, path, raw.lstat(path))
            .await?
            .attrs;
        self.verify_metadata(&attributes)?;
        canonical(raw, path, limit).await
    }
}

pub(super) async fn canonical(
    raw: &RawSftpSession,
    path: &str,
    limit: Duration,
) -> Result<(), RemoteError> {
    let paths = call(limit, Op::InspectMetadata, path, raw.realpath(path)).await?;
    if paths.files.len() != SENTINEL_BYTES
        || paths.files.first().is_none_or(|name| name.filename != path)
    {
        return Err(invalid());
    }
    Ok(())
}

/// Missing is allowed only for the final installation-root component.
pub(super) async fn root(
    raw: &RawSftpSession,
    path: &Path,
    create: bool,
    limit: Duration,
) -> Result<Option<DirectorySeal>, RemoteError> {
    super::super::bundle::validate_install_root(path)?;
    let mut ancestors = path
        .ancestors()
        .filter(|path| path.parent().is_some())
        .collect::<Vec<_>>();
    ancestors.reverse();
    for ancestor in ancestors {
        let text = ancestor.to_str().ok_or_else(invalid)?;
        let found = optional(limit, Op::InspectLink, text, raw.lstat(text)).await?;
        let attributes = match found {
            Some(attributes) => attributes.attrs,
            None if ancestor == path && create => {
                call(
                    limit,
                    Op::CreateDirectory,
                    text,
                    raw.mkdir(text, mode(DIRECTORY_MODE)),
                )
                .await?;
                call(limit, Op::InspectLink, text, raw.lstat(text))
                    .await?
                    .attrs
            }
            None if ancestor == path => return Ok(None),
            None => return Err(invalid()),
        };
        if ancestor == path {
            let seal = DirectorySeal::new(&attributes)?;
            canonical(raw, text, limit).await?;
            return Ok(Some(seal));
        }
        let permissions = attributes.permissions.ok_or_else(invalid)?;
        let writable = permissions & WRITE_ACCESS_MASK != 0;
        let protected_shared = permissions & STICKY_MODE != 0 && attributes.uid == Some(ROOT_OWNER);
        if !attributes.is_dir() || (writable && !protected_shared) {
            return Err(invalid());
        }
        canonical(raw, text, limit).await?;
    }
    Err(invalid())
}

pub(super) async fn create(
    raw: &RawSftpSession,
    path: &Path,
    owner: DirectorySeal,
    limit: Duration,
) -> Result<DirectorySeal, RemoteError> {
    let text = path.to_str().ok_or_else(invalid)?;
    // MKDIR refuses an existing name. Never adopt a pre-existing stage directory.
    call(
        limit,
        Op::CreateDirectory,
        text,
        raw.mkdir(text, mode(DIRECTORY_MODE)),
    )
    .await?;
    let attributes = call(limit, Op::InspectLink, text, raw.lstat(text))
        .await?
        .attrs;
    let created = DirectorySeal::new(&attributes)?;
    if created.uid != owner.uid || created.gid != owner.gid {
        return Err(invalid());
    }
    canonical(raw, text, limit).await?;
    Ok(created)
}

pub(super) fn mode(permissions: u32) -> FileAttributes {
    FileAttributes {
        permissions: Some(permissions),
        ..FileAttributes::empty()
    }
}
pub(super) fn text(path: &Path) -> Result<&str, RemoteError> {
    path.to_str().ok_or_else(invalid)
}
