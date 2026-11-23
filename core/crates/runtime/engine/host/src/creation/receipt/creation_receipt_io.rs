use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReceiptFileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

impl ReceiptFileIdentity {
    pub(crate) fn inspect(metadata: &std::fs::Metadata, path: &Path, mode: u32) -> Result<Self> {
        use std::os::unix::fs::MetadataExt;
        anyhow::ensure!(
            metadata.file_type().is_file(),
            "receipt is not a regular file: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "receipt has another owner: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.nlink() == EXPECTED_CREATION_RECEIPT_LINKS,
            "receipt has unexpected hard links: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.mode() & FILE_MODE_MASK == mode,
            "receipt has unsafe permissions: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.len() <= MAX_CREATION_RECEIPT_BYTES,
            "receipt exceeds byte limit: {}",
            path.display()
        );
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    pub(crate) fn verify(
        &self,
        metadata: &std::fs::Metadata,
        path: &Path,
        mode: u32,
    ) -> Result<()> {
        let current = Self::inspect(metadata, path, mode)?;
        anyhow::ensure!(
            *self == current,
            "creation receipt changed while reading: {}",
            path.display()
        );
        Ok(())
    }

    pub(crate) fn same_content(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.length == other.length
            && self.modified_seconds == other.modified_seconds
            && self.modified_nanoseconds == other.modified_nanoseconds
    }
}

pub(crate) fn inspect_receipt_file(
    file: &File,
    path: &Path,
    mode: u32,
) -> Result<ReceiptFileIdentity> {
    crate::private_acl::require_empty(file, path)?;
    ReceiptFileIdentity::inspect(&file.metadata()?, path, mode)
}

pub(crate) fn verify_receipt_file(
    expected: ReceiptFileIdentity,
    file: &File,
    path: &Path,
    mode: u32,
) -> Result<()> {
    crate::private_acl::require_empty(file, path)?;
    expected.verify(&file.metadata()?, path, mode)
}

pub(crate) fn open_receipt_file_at(
    directory: libc::c_int,
    name: &std::ffi::CStr,
    path: &Path,
    label: &str,
    access: libc::c_int,
) -> Result<Option<File>> {
    use std::os::fd::FromRawFd;
    let flags = access | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    let descriptor = unsafe { libc::openat(directory, name.as_ptr(), flags) };
    if descriptor >= 0 {
        return Ok(Some(unsafe { File::from_raw_fd(descriptor) }));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error).with_context(|| format!("open {label} {}", path.display()))
    }
}
