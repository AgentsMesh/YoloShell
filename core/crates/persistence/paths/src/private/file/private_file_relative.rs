use super::private_file_names::{
    active_name, quarantine_name, retired_name, staging_name, unlink_isolation_name,
};
use super::*;
use crate::paths::constants::{ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX};
use std::ffi::CStr;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;

impl PrivateFileEntry {
    pub const DURABLE_WRITE_RESIDUE_ENTRY_COUNT: usize = 5;

    pub fn from_directory(directory: PrivateDirectory, path: &Path) -> Result<Self> {
        let parent = path.parent().context("private file path has no parent")?;
        anyhow::ensure!(
            super::super::platform::platform_path(directory.path())
                == super::super::platform::platform_path(parent),
            "private file parent does not match retained directory"
        );
        let file_name = path.file_name().context("private file path has no name")?;
        Ok(Self {
            directory,
            name: entry_c_string(file_name)?,
            path: path.to_path_buf(),
        })
    }

    pub fn reopen(&self) -> Result<Self> {
        Self::from_directory(self.directory.try_clone()?, &self.path)
    }

    pub fn parent_descriptor(&self) -> RawFd {
        self.directory.descriptor()
    }

    pub fn file_name(&self) -> &CStr {
        &self.name
    }

    pub fn sibling_with_suffix(&self, suffix: &str) -> Result<Self> {
        let mut name = self
            .path
            .file_name()
            .map(OsString::from)
            .context("private file path has no name")?;
        name.push(suffix);
        let path = self.path.with_file_name(name);
        Self::from_directory(self.directory.try_clone()?, &path)
    }

    pub(in crate::paths) fn sibling_named(&self, name: &CStr) -> Result<Self> {
        let path = self.path.with_file_name(OsStr::from_bytes(name.to_bytes()));
        Self::from_directory(self.directory.try_clone()?, &path)
    }

    pub fn unlink_isolation_entry(&self) -> Result<Self> {
        self.sibling_named(&unlink_isolation_name(&self.name)?)
    }

    pub fn durable_write_residue_entries(
        &self,
    ) -> Result<[Self; Self::DURABLE_WRITE_RESIDUE_ENTRY_COUNT]> {
        let active = active_name(&self.name, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
        Ok([
            self.sibling_named(&active)?,
            self.sibling_named(&retired_name(&self.name)?)?,
            self.sibling_named(&quarantine_name(&self.name, ATOMIC_FILE_PREFIX, &active)?)?,
            self.sibling_named(&staging_name(&self.name, ATOMIC_FILE_PREFIX)?)?,
            self.sibling_named(&unlink_isolation_name(&self.name)?)?,
        ])
    }
}
