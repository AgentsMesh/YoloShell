use super::secure_vfs_alias_cleanup::cleanup_best_effort;
use super::secure_vfs_alias_directory::{create, verify_entry, verify_path};
use super::secure_vfs_alias_io::{
    ALIAS_INITIALIZING_PREFIX, ALIAS_PREFIX, create_dummy_main, create_lock_file,
    set_directory_mode,
};
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const ALIAS_CREATE_ATTEMPTS: usize = 16;

pub(super) struct PublishedAlias {
    parent: PrivateDirectory,
    name: OsString,
    path: PathBuf,
    directory: File,
    _lock: super::secure_vfs_alias_io::AliasOwnerLock,
}

impl PublishedAlias {
    pub(super) fn create(descriptor: &SecureDescriptor, kind: ArtifactKind) -> Result<Self> {
        let parent = descriptor.alias_parent()?;
        let mut initialization = AliasInitialization::create(parent)?;
        anyhow::ensure!(
            initialization.directory()?.metadata()?.dev() == descriptor.device(),
            "SQLite alias must share the artifact filesystem"
        );
        if kind != ArtifactKind::Main {
            create_dummy_main(initialization.directory()?, &initialization.path)?;
        }
        initialization.publish()
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn directory(&self) -> &File {
        &self.directory
    }

    pub(super) fn set_mode(&self, mode: u32) -> Result<()> {
        set_directory_mode(&self.directory, &self.path, mode)
    }
}

impl Drop for PublishedAlias {
    fn drop(&mut self) {
        cleanup_best_effort(&self.parent, &self.name, &self.path, &self.directory);
    }
}

struct AliasInitialization {
    parent: Option<PrivateDirectory>,
    name: OsString,
    path: PathBuf,
    directory: Option<File>,
    lock: Option<super::secure_vfs_alias_io::AliasOwnerLock>,
    armed: bool,
}

impl AliasInitialization {
    fn create(parent: PrivateDirectory) -> Result<Self> {
        for _ in 0..ALIAS_CREATE_ATTEMPTS {
            let name = OsString::from(format!(
                "{ALIAS_INITIALIZING_PREFIX}{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
            let path = parent.path().join(&name);
            match create(&parent, &name, &path) {
                Ok(directory) => {
                    let lock = create_lock_file(&directory, &path)?;
                    return Ok(Self {
                        parent: Some(parent),
                        name,
                        path,
                        directory: Some(directory),
                        lock: Some(lock),
                        armed: true,
                    });
                }
                Err(error) if already_exists(&error) => continue,
                Err(error) => return Err(error).context("create SQLite alias initialization"),
            }
        }
        anyhow::bail!("exhausted secure SQLite alias initialization attempts")
    }

    fn parent(&self) -> Result<&PrivateDirectory> {
        self.parent.as_ref().context("alias parent unavailable")
    }

    fn directory(&self) -> Result<&File> {
        self.directory
            .as_ref()
            .context("alias directory unavailable")
    }

    fn publish(&mut self) -> Result<PublishedAlias> {
        let published_name = OsString::from(format!("{ALIAS_PREFIX}{}", Uuid::new_v4()));
        let published = self.parent()?.path().join(&published_name);
        run_publish_test_hook(&self.path, &published)?;
        verify_entry(self.parent()?, &self.name, &self.path, self.directory()?)?;
        rename_exclusive(self.parent()?.descriptor(), &self.name, &published_name)?;
        self.name = published_name;
        self.path = published;
        verify_entry(self.parent()?, &self.name, &self.path, self.directory()?)?;
        verify_path(self.directory()?, &self.path)?;
        let alias = PublishedAlias {
            parent: self.parent.take().context("alias parent unavailable")?,
            name: self.name.clone(),
            path: self.path.clone(),
            directory: self
                .directory
                .take()
                .context("alias directory unavailable")?,
            _lock: self.lock.take().context("alias lock unavailable")?,
        };
        self.armed = false;
        Ok(alias)
    }
}

impl Drop for AliasInitialization {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let (Some(parent), Some(directory)) = (&self.parent, &self.directory) {
            cleanup_best_effort(parent, &self.name, &self.path, directory);
        }
    }
}

fn rename_exclusive(parent: libc::c_int, source: &OsStr, target: &OsStr) -> Result<()> {
    let source = CString::new(source.as_bytes())?;
    let target = CString::new(target.as_bytes())?;
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("publish secure SQLite alias")
    }
}

fn already_exists(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::AlreadyExists)
    })
}

fn run_publish_test_hook(source: &Path, target: &Path) -> Result<()> {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    return super::secure_vfs_alias_identity_tests::run_publish_hook(source, target);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = (source, target);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    Ok(())
}
