use super::*;

pub(crate) const EXPECTED_CREATION_LEDGER_LINKS: u64 = 1;
pub(crate) const CREATION_LEDGER_FILE_MODE: u32 = 0o600;
pub(crate) const FILE_MODE_MASK: u32 = 0o777;
pub(crate) const SIZE_LIMIT_SENTINEL_BYTES: u64 = 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PrivateFileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

impl PrivateFileIdentity {
    pub(crate) fn inspect(
        metadata: &std::fs::Metadata,
        path: &Path,
        label: &str,
        max: u64,
    ) -> Result<Self> {
        use std::os::unix::fs::MetadataExt;
        anyhow::ensure!(
            metadata.file_type().is_file(),
            "{label} is not a regular file: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "{label} is owned by another user: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.nlink() == EXPECTED_CREATION_LEDGER_LINKS,
            "{label} has unexpected hard links: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.mode() & FILE_MODE_MASK == CREATION_LEDGER_FILE_MODE,
            "{label} has unsafe permissions: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.len() <= max,
            "{label} exceeds byte limit: {}",
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
        label: &str,
    ) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        anyhow::ensure!(
            metadata.file_type().is_file()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & FILE_MODE_MASK == CREATION_LEDGER_FILE_MODE
                && metadata.dev() == self.device
                && metadata.ino() == self.inode
                && metadata.len() == self.length
                && metadata.mtime() == self.modified_seconds
                && metadata.mtime_nsec() == self.modified_nanoseconds
                && metadata.ctime() == self.changed_seconds
                && metadata.ctime_nsec() == self.changed_nanoseconds
                && metadata.nlink() == EXPECTED_CREATION_LEDGER_LINKS,
            "{label} changed while reading: {}",
            path.display()
        );
        Ok(())
    }
}
