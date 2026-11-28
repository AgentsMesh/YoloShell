use super::*;

use crate::paths::directory::PrivateDirectory;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

pub(crate) struct ReceiptDirectory {
    pub(crate) guard: PrivateDirectory,
    pub(crate) file: File,
    pub(crate) path: PathBuf,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) exact_mode: bool,
}

impl ReceiptDirectory {
    #[cfg(test)]
    pub(crate) fn open(path: &Path, label: &str) -> Result<Self> {
        let guard = PrivateDirectory::open(path)
            .with_context(|| format!("open {label} {}", path.display()))?;
        Self::from_private(guard, label, false)
    }

    pub(crate) fn from_private(
        guard: PrivateDirectory,
        label: &str,
        exact_mode: bool,
    ) -> Result<Self> {
        let path = guard.path().to_path_buf();
        let file = duplicate_receipt_directory(guard.descriptor())?;
        Self::from_file(guard, file, &path, label, exact_mode)
    }

    pub(crate) fn ensure_child(&self, name: &str, label: &str) -> Result<Self> {
        self.verify_path(label)?;
        let path = self.path.join(name);
        let guard = self.guard.ensure_descendant(&path)?;
        let child = Self::from_private(guard, label, true)?;
        self.verify_path(label)?;
        child.verify_path(label)?;
        Ok(child)
    }

    pub(crate) fn verify_path(&self, label: &str) -> Result<()> {
        self.guard
            .verify_path()
            .with_context(|| format!("verify {label} {}", self.path.display()))?;
        let metadata = self.file.metadata()?;
        crate::private_acl::require_empty(&self.file, &self.path)?;
        verify_private_directory(&metadata, &self.path, label)?;
        if self.exact_mode {
            anyhow::ensure!(
                metadata.mode() & FILE_MODE_MASK == CREATION_RECEIPT_DIRECTORY_MODE,
                "{label} has unsafe permissions: {}",
                self.path.display()
            );
        }
        anyhow::ensure!(
            metadata.dev() == self.device && metadata.ino() == self.inode,
            "{label} path identity changed: {}",
            self.path.display()
        );
        Ok(())
    }

    pub(crate) fn from_file(
        guard: PrivateDirectory,
        file: File,
        path: &Path,
        label: &str,
        exact_mode: bool,
    ) -> Result<Self> {
        let metadata = file.metadata()?;
        crate::private_acl::require_empty(&file, path)?;
        verify_private_directory(&metadata, path, label)?;
        if exact_mode {
            anyhow::ensure!(
                metadata.mode() & FILE_MODE_MASK == CREATION_RECEIPT_DIRECTORY_MODE,
                "{label} has unsafe permissions: {}",
                path.display()
            );
        }
        Ok(Self {
            guard,
            file,
            path: path.to_path_buf(),
            device: metadata.dev(),
            inode: metadata.ino(),
            exact_mode,
        })
    }
}

pub(crate) fn duplicate_receipt_directory(descriptor: libc::c_int) -> std::io::Result<File> {
    let duplicate = unsafe { libc::fcntl(descriptor, libc::F_DUPFD_CLOEXEC, libc::STDIN_FILENO) };
    if duplicate < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(duplicate) })
    }
}
