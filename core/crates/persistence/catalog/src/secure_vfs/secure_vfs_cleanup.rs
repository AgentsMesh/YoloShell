use super::constants::{MAX_SECURE_VFS_PARENT_ENTRIES, MIN_ALIAS_OWNER_PROCESS_ID};
use super::secure_vfs_alias_cleanup::cleanup_strict;
use super::secure_vfs_alias_directory::open_verified_at;
use super::secure_vfs_alias_io::{ALIAS_INITIALIZING_PREFIX, ALIAS_PREFIX, open_lock_file};
use super::secure_vfs_cleanup_lock::retain_after_open;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AliasIdentity {
    device: u64,
    inode: u64,
}

pub(super) fn cleanup_stale_aliases(artifact: &Path) -> Result<()> {
    cleanup_stale_aliases_with(artifact, MAX_SECURE_VFS_PARENT_ENTRIES, |_| Ok(()))
}

pub(super) fn cleanup_stale_aliases_in(parent: &PrivateDirectory) -> Result<()> {
    cleanup_parent_with(parent, MAX_SECURE_VFS_PARENT_ENTRIES, |_| Ok(()))
}

fn cleanup_stale_aliases_with<F>(artifact: &Path, max_entries: usize, hook: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    let parent = crate::paths::secure_private_parent(artifact)?;
    cleanup_parent_with(&parent, max_entries, hook)
}

fn cleanup_parent_with<F>(parent: &PrivateDirectory, max_entries: usize, mut hook: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    for (index, entry) in std::fs::read_dir(parent.path())
        .context("scan secure SQLite aliases")?
        .enumerate()
    {
        anyhow::ensure!(
            index < max_entries,
            "secure SQLite alias parent entry limit exceeded"
        );
        let entry = entry?;
        let name = entry.file_name();
        let Some(text) = name.to_str() else {
            continue;
        };
        let published = canonical_alias_name(text);
        let initialization_owner = initializing_owner(text);
        if !published && initialization_owner.is_none() {
            continue;
        }
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let identity = identity(&entry.metadata()?);
        let path = parent.path().join(&name);
        hook(&path)?;
        if published {
            let _ = cleanup_published(parent, &name, &path, identity)?;
        } else if let Some(owner) = initialization_owner {
            let _ = cleanup_initialization(parent, &name, &path, identity, owner)?;
        }
    }
    Ok(())
}

fn cleanup_published(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    expected: AliasIdentity,
) -> Result<bool> {
    let directory = match open_verified_at(parent, name, path) {
        Ok(directory) => directory,
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(false),
        Err(error) => return Err(error),
    };
    if identity(&directory.metadata()?) != expected {
        return Ok(false);
    }
    let lock = match open_lock_file(&directory, path) {
        Ok(lock) => lock,
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(false),
        Err(error) => return Err(error),
    };
    let Some(lock) = lock else {
        return Ok(false);
    };
    let Some(_retained_lock) = retain_after_open(parent, name, path, &directory, lock)? else {
        return Ok(false);
    };
    cleanup_strict(parent, name, path, &directory)
}

fn cleanup_initialization(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    expected: AliasIdentity,
    owner: libc::pid_t,
) -> Result<bool> {
    let directory = match open_verified_at(parent, name, path) {
        Ok(directory) => directory,
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(false),
        Err(error) => return Err(error),
    };
    if identity(&directory.metadata()?) != expected {
        return Ok(false);
    }
    let lock = match open_lock_file(&directory, path) {
        Ok(lock) => lock,
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(false),
        Err(error) => return Err(error),
    };
    if let Some(lock) = lock {
        let Some(_retained_lock) = retain_after_open(parent, name, path, &directory, lock)? else {
            return Ok(false);
        };
        return cleanup_strict(parent, name, path, &directory);
    } else if super::secure_vfs_cleanup_error::process_alive(owner) {
        return Ok(false);
    }
    cleanup_strict(parent, name, path, &directory)
}

pub(super) fn cleanup_named_candidate(parent: &PrivateDirectory, name: &OsStr) -> Result<bool> {
    let Some(text) = name.to_str() else {
        return Ok(false);
    };
    let published = canonical_alias_name(text);
    let owner = initializing_owner(text);
    if !published && owner.is_none() {
        return Ok(false);
    }
    let path = parent.path().join(name);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_dir() => metadata,
        Ok(_) => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let expected = identity(&metadata);
    if published {
        cleanup_published(parent, name, &path, expected)
    } else {
        cleanup_initialization(parent, name, &path, expected, owner.expect("owner checked"))
    }
}

pub(super) fn is_alias_candidate(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|text| canonical_alias_name(text) || initializing_owner(text).is_some())
}

fn identity(metadata: &std::fs::Metadata) -> AliasIdentity {
    AliasIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(test)]
pub(super) fn cleanup_with_hook<F>(artifact: &Path, hook: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    cleanup_stale_aliases_with(artifact, MAX_SECURE_VFS_PARENT_ENTRIES, hook)
}

#[cfg(test)]
pub(super) fn cleanup_with_limit(artifact: &Path, max_entries: usize) -> Result<()> {
    cleanup_stale_aliases_with(artifact, max_entries, |_| Ok(()))
}

pub(super) fn canonical_alias_name(name: &str) -> bool {
    name.strip_prefix(ALIAS_PREFIX)
        .and_then(|suffix| Uuid::parse_str(suffix).ok())
        .is_some_and(|id| name == format!("{ALIAS_PREFIX}{id}"))
}

fn initializing_owner(name: &str) -> Option<libc::pid_t> {
    let suffix = name.strip_prefix(ALIAS_INITIALIZING_PREFIX)?;
    let (pid, id) = suffix.split_once('-')?;
    Uuid::parse_str(id).ok()?;
    let pid = pid.parse().ok()?;
    (pid >= MIN_ALIAS_OWNER_PROCESS_ID).then_some(pid)
}
