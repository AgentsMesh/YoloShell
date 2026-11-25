use super::constants::MAX_DURABLE_SESSION_DIRECTORY_ENTRIES;
use crate::paths::directory::PrivateDirectory;
use crate::paths::directory_stream::DirectoryStream;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

const DIRECTORY_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
const FILE_FLAGS: libc::c_int =
    libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_NOFOLLOW | libc::O_CLOEXEC;

pub(crate) struct DurableDirectory {
    guard: PrivateDirectory,
}

impl DurableDirectory {
    #[cfg(test)]
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let guard = PrivateDirectory::open(path)
            .with_context(|| format!("open durable directory {}", path.display()))?;
        Ok(Self::from_private(guard))
    }

    pub(crate) fn from_private(guard: PrivateDirectory) -> Self {
        Self { guard }
    }

    pub(crate) fn open_directory(&self, name: &OsStr) -> Result<Option<Self>> {
        let name_c = name_c_string(name)?;
        let path = self.guard.path().join(name);
        self.verify_path()?;
        let Some(opened) = self.open_optional(&name_c, DIRECTORY_FLAGS)? else {
            return Ok(None);
        };
        let child = Self::from_private(PrivateDirectory::from_opened(opened, &path)?);
        self.verify_path()?;
        child.verify_path()?;
        Ok(Some(child))
    }

    pub(crate) fn open_file(&self, name: &OsStr) -> Result<Option<File>> {
        let name_c = name_c_string(name)?;
        self.verify_path()?;
        let opened = self.open_optional(&name_c, FILE_FLAGS)?;
        self.verify_path()?;
        Ok(opened)
    }

    pub(crate) fn entries(&self) -> Result<Vec<OsString>> {
        self.entries_bounded(MAX_DURABLE_SESSION_DIRECTORY_ENTRIES)
    }

    pub(crate) fn entries_bounded(&self, max_entries: usize) -> Result<Vec<OsString>> {
        self.verify_path()?;
        let mut stream = DirectoryStream::open(self.guard.descriptor())?;
        let mut entries = Vec::new();
        while let Some(name) = stream.next_name()? {
            anyhow::ensure!(
                entries.len() < max_entries,
                "durable session directory entry limit exceeded"
            );
            entries.push(name);
        }
        self.verify_path()?;
        Ok(entries)
    }

    pub(crate) fn entry_path(&self, name: &OsStr) -> PathBuf {
        self.guard.path().join(name)
    }

    pub(crate) fn verify_path(&self) -> Result<()> {
        self.guard.verify_descriptor_path()
    }

    pub(crate) fn private(&self) -> &PrivateDirectory {
        &self.guard
    }

    fn open_optional(&self, name: &CStr, flags: libc::c_int) -> Result<Option<File>> {
        match open_at(self.guard.descriptor(), name, flags) {
            Ok(file) => Ok(Some(file)),
            Err(error) if is_missing(&error) => {
                self.verify_path()?;
                let confirmed = open_at(self.guard.descriptor(), name, flags);
                self.verify_path()?;
                match confirmed {
                    Ok(file) => Ok(Some(file)),
                    Err(error) if is_missing(&error) => Ok(None),
                    Err(error) => Err(error.into()),
                }
            }
            Err(error) => Err(error.into()),
        }
    }
}

fn open_at(parent: libc::c_int, name: &CStr, flags: libc::c_int) -> std::io::Result<File> {
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

fn name_c_string(name: &OsStr) -> Result<CString> {
    CString::new(name.as_bytes()).context("durable entry name contains a null byte")
}

fn is_missing(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ENOENT)
}
