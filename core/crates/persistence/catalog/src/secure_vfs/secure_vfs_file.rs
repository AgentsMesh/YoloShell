use anyhow::{Context, Result};
use rusqlite::ffi;
use std::ffi::{CStr, OsString};
use std::fs::File;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
const SQLITE_ARTIFACT_MODE: u32 = 0o600;
const PERMISSION_MODE_MASK: u32 = 0o7777;
const SQLITE_JOURNAL_SUFFIX: &str = "-journal";
const SQLITE_SHM_SUFFIX: &str = "-shm";
const SQLITE_WAL_SUFFIX: &str = "-wal";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArtifactKind {
    Main,
    Journal,
    Wal,
    Shm,
}

impl ArtifactKind {
    pub(super) fn from_open_flags(flags: libc::c_int) -> Option<Self> {
        if flags & ffi::SQLITE_OPEN_MAIN_DB != 0 {
            Some(Self::Main)
        } else if flags & ffi::SQLITE_OPEN_MAIN_JOURNAL != 0 {
            Some(Self::Journal)
        } else if flags & ffi::SQLITE_OPEN_WAL != 0 {
            Some(Self::Wal)
        } else {
            None
        }
    }

    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Main => "",
            Self::Journal => SQLITE_JOURNAL_SUFFIX,
            Self::Wal => SQLITE_WAL_SUFFIX,
            Self::Shm => SQLITE_SHM_SUFFIX,
        }
    }
}

pub(super) struct SecureDescriptor {
    file: File,
    identity: FileIdentity,
    path: PathBuf,
    entry: crate::paths::private_file::PrivateFileEntry,
    retained: Option<super::retained_artifact::RetainedArtifact>,
}

impl SecureDescriptor {
    pub(super) fn open(name: &CStr, flags: libc::c_int) -> Result<Self> {
        let path = PathBuf::from(OsString::from_vec(name.to_bytes().to_vec()));
        let mut open_flags = libc::O_CLOEXEC | libc::O_NOFOLLOW;
        open_flags |= if flags & ffi::SQLITE_OPEN_READONLY != 0 {
            libc::O_RDONLY
        } else {
            libc::O_RDWR
        };
        if flags & ffi::SQLITE_OPEN_CREATE != 0 {
            open_flags |= libc::O_CREAT;
        }
        if flags & ffi::SQLITE_OPEN_EXCLUSIVE != 0 {
            open_flags |= libc::O_EXCL;
        }
        let retained = super::retained_namespace::resolve(&path);
        let entry = match &retained {
            Some(retained) => retained.entry().reopen()?,
            None => crate::paths::private_file::PrivateFileEntry::from_directory(
                crate::paths::secure_private_parent(&path)?,
                &path,
            )?,
        };
        let file = super::secure_vfs_retained_file::open(&entry, open_flags)
            .with_context(|| format!("securely open SQLite artifact {}", path.display()))?;
        let identity = FileIdentity::inspect(&file.metadata()?, &path)?;
        if flags & ffi::SQLITE_OPEN_READONLY == 0 {
            file.set_permissions(std::fs::Permissions::from_mode(SQLITE_ARTIFACT_MODE))?;
            crate::private_acl::clear(&file, &path)?;
            let mode = file.metadata()?.mode() & PERMISSION_MODE_MASK;
            anyhow::ensure!(
                mode == SQLITE_ARTIFACT_MODE,
                "SQLite artifact mode is not private"
            );
        } else {
            crate::private_acl::require_empty(&file, &path)?;
        }
        if let Some(retained) = &retained {
            retained.bind(identity)?;
        }
        Ok(Self {
            file,
            identity,
            path,
            entry,
            retained,
        })
    }

    pub(super) fn link_to_directory(&self, directory: &File, name: &std::ffi::OsStr) -> Result<()> {
        use std::os::fd::AsRawFd;
        use std::os::unix::ffi::OsStrExt;

        crate::private_acl::require_empty(&self.file, &self.path)?;
        let name = std::ffi::CString::new(name.as_bytes())?;
        super::secure_vfs_retained_file::link_at(&self.entry, directory.as_raw_fd(), &name)
            .with_context(|| format!("link secure SQLite alias for {}", self.path.display()))
    }

    pub(super) fn verify_alias(&self, alias: &Path) -> Result<()> {
        crate::private_acl::require_empty(&self.file, &self.path)?;
        let alias_metadata = std::fs::symlink_metadata(alias)?;
        self.identity
            .verify_alias(&self.file.metadata()?, &alias_metadata, alias)
    }

    pub(super) fn verify_alias_link(&self, directory: &File, name: &std::ffi::OsStr) -> Result<()> {
        crate::private_acl::require_empty(&self.file, &self.path)?;
        let alias = super::secure_vfs_alias_link::open(directory, name).map_err(|error| {
            super::secure_vfs_identity::unsafe_artifact(format!(
                "SQLite alias link is unsafe for {}: {error:#}",
                self.path.display()
            ))
        })?;
        self.identity
            .verify_alias(&self.file.metadata()?, &alias.metadata()?, self.path())
    }

    pub(super) fn verify_reference(&self, alias: &Path) -> Result<()> {
        crate::private_acl::require_empty(&self.file, &self.path)?;
        let descriptor = self.file.metadata()?;
        let reference = std::fs::metadata(alias)?;
        self.identity.verify_final(&descriptor, &reference, alias)
    }

    pub(super) fn verify_current(&self) -> Result<()> {
        crate::private_acl::require_empty(&self.file, &self.path)?;
        let descriptor_metadata = self.file.metadata()?;
        let path_metadata = match &self.retained {
            Some(_) => super::secure_vfs_retained_file::reopen(&self.entry)?.metadata()?,
            None => std::fs::symlink_metadata(&self.path)
                .with_context(|| format!("reinspect SQLite artifact {}", self.path.display()))?,
        };
        self.identity
            .verify_final(&descriptor_metadata, &path_metadata, &self.path)
    }

    pub(super) fn alias_parent(&self) -> Result<crate::paths::directory::PrivateDirectory> {
        if let Some(retained) = &self.retained {
            return retained.alias_parent();
        }
        crate::paths::secure_private_parent(&self.path)
    }

    pub(super) fn reference_path(&self) -> PathBuf {
        self.path.clone()
    }

    pub(super) fn is_retained(&self) -> bool {
        self.retained.is_some()
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn device(&self) -> u64 {
        self.identity.device()
    }

    pub(super) fn same_object(&self, other: &Self) -> bool {
        self.identity == other.identity
    }

    pub(super) fn identity(&self) -> FileIdentity {
        self.identity
    }

    pub(super) fn delete_current(&self) -> Result<()> {
        if super::retained_delete::remove_descriptor(&self.entry, self.identity)?
            && let Some(retained) = &self.retained
        {
            retained.clear(self.identity)?;
        }
        Ok(())
    }
}
use super::secure_vfs_identity::FileIdentity;
