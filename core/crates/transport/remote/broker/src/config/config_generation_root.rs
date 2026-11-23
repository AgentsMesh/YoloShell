use super::super::constants::IMMUTABLE_DIRECTORY_MODE;
use super::entries;
use super::io::{self, GenerationIdentity};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result, ensure};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub(in crate::remote_broker) struct GenerationRoot {
    directory: PrivateDirectory,
    path: PathBuf,
}

pub(in crate::remote_broker) struct OpenedGeneration {
    directory: File,
    path: PathBuf,
    pub(super) identity: GenerationIdentity,
}

impl GenerationRoot {
    pub(in crate::remote_broker) fn ensure(path: &Path) -> Result<Self> {
        let directory = PrivateDirectory::ensure(path)?;
        Self::finish(directory, path)
    }

    pub(in crate::remote_broker) fn ensure_retained(
        state: PrivateDirectory,
        path: &Path,
    ) -> Result<Self> {
        let directory = state.ensure_descendant(path)?;
        Self::finish(directory, path)
    }

    fn finish(directory: PrivateDirectory, path: &Path) -> Result<Self> {
        directory.verify_path()?;
        Ok(Self {
            directory,
            path: path.to_path_buf(),
        })
    }

    pub(in crate::remote_broker) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn descriptor(&self) -> RawFd {
        self.directory.descriptor()
    }

    pub(in crate::remote_broker) fn sync_and_verify(&self) -> Result<()> {
        self.directory.sync()?;
        self.directory.verify_path()
    }

    pub(super) fn verify_path(&self) -> Result<()> {
        self.directory.verify_path()
    }

    pub(in crate::remote_broker) fn open_generation(
        &self,
        name: &CStr,
    ) -> Result<Option<OpenedGeneration>> {
        self.open_generation_with(name, || Ok(()))
    }

    fn open_generation_with<F>(
        &self,
        name: &CStr,
        after_first_absence: F,
    ) -> Result<Option<OpenedGeneration>>
    where
        F: FnOnce() -> Result<()>,
    {
        self.verify_path()?;
        if io::inspect_at(self.descriptor(), name)?.is_none() {
            after_first_absence()?;
            self.verify_path()?;
            if io::inspect_at(self.descriptor(), name)?.is_none() {
                self.verify_path()?;
                return Ok(None);
            }
        }
        let directory = io::open_directory_at(self.descriptor(), name)?;
        let path = self.path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
        crate::private_acl::require_empty(&directory, &path)?;
        let identity =
            GenerationIdentity::inspect(&directory.metadata()?, IMMUTABLE_DIRECTORY_MODE)?;
        let opened = OpenedGeneration {
            directory,
            path,
            identity,
        };
        opened.verify(self, name)?;
        Ok(Some(opened))
    }

    #[cfg(test)]
    pub(super) fn open_generation_with_absence_hook<F>(
        &self,
        name: &CStr,
        hook: F,
    ) -> Result<Option<OpenedGeneration>>
    where
        F: FnOnce() -> Result<()>,
    {
        self.open_generation_with(name, hook)
    }

    pub(super) fn verify_named(
        &self,
        name: &CStr,
        expected: GenerationIdentity,
        mode: u32,
    ) -> Result<()> {
        let directory = io::open_directory_at(self.descriptor(), name)?;
        let path = self.path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
        crate::private_acl::require_empty(&directory, &path)?;
        let actual = GenerationIdentity::inspect(&directory.metadata()?, mode)?;
        ensure!(
            actual == expected,
            "broker generation path identity changed"
        );
        self.verify_identity(name, expected)
    }

    pub(super) fn verify_identity(&self, name: &CStr, expected: GenerationIdentity) -> Result<()> {
        let stat = io::inspect_at(self.descriptor(), name)?
            .context("broker generation path disappeared")?;
        ensure!(
            expected.matches_stat(&stat),
            "broker generation path identity changed"
        );
        self.verify_path()
    }
}

impl OpenedGeneration {
    pub(in crate::remote_broker) fn verify(
        &self,
        root: &GenerationRoot,
        name: &CStr,
    ) -> Result<()> {
        crate::private_acl::require_empty(&self.directory, &self.path)?;
        let current =
            GenerationIdentity::inspect(&self.directory.metadata()?, IMMUTABLE_DIRECTORY_MODE)?;
        ensure!(
            current == self.identity,
            "broker generation descriptor identity changed"
        );
        entries::ensure_known(&self.directory, &self.path)?;
        root.verify_named(name, self.identity, IMMUTABLE_DIRECTORY_MODE)
    }
}
