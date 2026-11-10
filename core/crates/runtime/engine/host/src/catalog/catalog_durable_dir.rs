use super::constants::MAX_DURABLE_SESSION_DIRECTORY_ENTRIES;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs::File;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

const CURRENT_DIRECTORY: &[u8] = b".";
const PARENT_DIRECTORY: &[u8] = b"..";

pub(crate) struct DurableDirectory {
    guard: PrivateDirectory,
    path: PathBuf,
}

impl DurableDirectory {
    #[cfg(test)]
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let guard = PrivateDirectory::open(path)
            .with_context(|| format!("open durable directory {}", path.display()))?;
        Ok(Self {
            guard,
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn from_private(guard: PrivateDirectory) -> Self {
        let path = guard.path().to_path_buf();
        Self { guard, path }
    }

    pub(crate) fn open_directory(&self, name: &OsStr) -> Result<Option<Self>> {
        let name_c = name_c_string(name)?;
        let path = self.path.join(name);
        self.verify_path()?;
        let Some(opened) = self.open_optional(&name_c, directory_flags())? else {
            return Ok(None);
        };
        let opened_metadata = opened.metadata()?;
        let child = Self::from_private(PrivateDirectory::from_opened(opened, &path)?);
        let guarded_metadata = child.guard_metadata()?;
        anyhow::ensure!(
            opened_metadata.dev() == guarded_metadata.dev()
                && opened_metadata.ino() == guarded_metadata.ino(),
            "durable directory changed while opening: {}",
            path.display()
        );
        self.verify_path()?;
        child.verify_path()?;
        Ok(Some(child))
    }

    pub(crate) fn open_file(&self, name: &OsStr) -> Result<Option<File>> {
        let name_c = name_c_string(name)?;
        self.verify_path()?;
        let opened = self.open_optional(&name_c, file_flags())?;
        self.verify_path()?;
        Ok(opened)
    }

    pub(crate) fn entries(&self) -> Result<Vec<OsString>> {
        self.entries_bounded(MAX_DURABLE_SESSION_DIRECTORY_ENTRIES)
    }

    pub(crate) fn entries_bounded(&self, max_entries: usize) -> Result<Vec<OsString>> {
        self.verify_path()?;
        let current = CString::new(CURRENT_DIRECTORY)?;
        let reopened = open_at(self.guard.descriptor(), &current, directory_flags())
            .context("reopen durable directory for enumeration")?;
        let stream = DirectoryStream::new(reopened.into_raw_fd())?;
        let mut entries = Vec::new();
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                let error = std::io::Error::last_os_error();
                return if error.raw_os_error() == Some(0) {
                    self.verify_path()?;
                    Ok(entries)
                } else {
                    Err(error).context("enumerate durable directory")
                };
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if bytes != CURRENT_DIRECTORY && bytes != PARENT_DIRECTORY {
                anyhow::ensure!(
                    entries.len() < max_entries,
                    "durable session directory entry limit exceeded"
                );
                entries.push(OsString::from_vec(bytes.to_vec()));
            }
        }
    }

    pub(crate) fn entry_path(&self, name: &OsStr) -> PathBuf {
        self.path.join(name)
    }

    pub(crate) fn verify_path(&self) -> Result<()> {
        self.guard.verify_descriptor_path()
    }

    fn guard_metadata(&self) -> Result<std::fs::Metadata> {
        let current = std::ffi::CString::new(CURRENT_DIRECTORY)?;
        let file = open_at(self.guard.descriptor(), &current, directory_flags())?;
        file.metadata().map_err(Into::into)
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

fn directory_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
}

fn file_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_NOFOLLOW | libc::O_CLOEXEC
}

fn name_c_string(name: &OsStr) -> Result<CString> {
    CString::new(name.as_bytes()).context("durable entry name contains a null byte")
}

fn is_missing(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ENOENT)
}

struct DirectoryStream(*mut libc::DIR);

impl DirectoryStream {
    fn new(fd: libc::c_int) -> Result<Self> {
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            unsafe { libc::close(fd) };
            Err(std::io::Error::last_os_error()).context("open durable directory stream")
        } else {
            Ok(Self(stream))
        }
    }
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

fn clear_errno() {
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = 0;
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        *libc::__errno_location() = 0;
    }
}
