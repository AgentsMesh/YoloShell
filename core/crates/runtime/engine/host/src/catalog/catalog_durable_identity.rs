use super::catalog_durable_file::MAX_DURABLE_JSON_BYTES;
use super::constants::DURABLE_STATE_CHANGED_CONTEXT;
use anyhow::{Context, Result};
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const EXPECTED_DURABLE_LINK_COUNT: u64 = 1;
const PERMISSION_MODE_MASK: u32 = 0o777;
const PRIVATE_DURABLE_FILE_MODE: u32 = 0o600;

pub(crate) struct DurableIdentity {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    owner: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl DurableIdentity {
    pub(crate) fn inspect(metadata: &Metadata, path: &Path) -> Result<Self> {
        anyhow::ensure!(
            metadata.file_type().is_file(),
            "durable state is not a regular file: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.nlink() == EXPECTED_DURABLE_LINK_COUNT,
            "durable state has unexpected hard links: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.len() <= MAX_DURABLE_JSON_BYTES,
            "durable state exceeds byte limit: {}",
            path.display()
        );
        let owner = metadata.uid();
        anyhow::ensure!(
            owner == unsafe { libc::geteuid() },
            "durable state owner mismatch: {}",
            path.display()
        );
        let mode = metadata.mode() & PERMISSION_MODE_MASK;
        anyhow::ensure!(
            mode == PRIVATE_DURABLE_FILE_MODE,
            "durable state permissions are not private: {}",
            path.display()
        );
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            mode,
            owner,
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    pub(crate) fn verify(&self, metadata: &Metadata, bytes_read: u64, path: &Path) -> Result<()> {
        let current = Self::inspect(metadata, path)
            .with_context(|| format!("{DURABLE_STATE_CHANGED_CONTEXT}: {}", path.display()))?;
        anyhow::ensure!(
            self.same_version(&current) && bytes_read == self.length,
            "{DURABLE_STATE_CHANGED_CONTEXT}: {}",
            path.display()
        );
        Ok(())
    }

    pub(crate) fn same_version(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.length == other.length
            && self.mode == other.mode
            && self.owner == other.owner
            && self.modified_seconds == other.modified_seconds
            && self.modified_nanoseconds == other.modified_nanoseconds
            && self.changed_seconds == other.changed_seconds
            && self.changed_nanoseconds == other.changed_nanoseconds
    }
}
