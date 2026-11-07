use super::constants::*;
use anyhow::Result;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

impl DirectoryIdentity {
    pub(super) fn inspect_private(metadata: &Metadata, path: &Path) -> Result<Self> {
        verify_directory_type(metadata, path)?;
        anyhow::ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "private directory owner mismatch: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.mode() & PERMISSION_MODE_MASK == PRIVATE_DIRECTORY_MODE,
            "private directory permissions are unsafe: {}",
            path.display()
        );
        Ok(Self::new(metadata))
    }

    pub(super) fn inspect_ancestor(metadata: &Metadata, path: &Path) -> Result<Self> {
        verify_directory_type(metadata, path)?;
        let owner = metadata.uid();
        let mode = metadata.mode();
        let trusted_owner = owner == unsafe { libc::geteuid() } || owner == ROOT_USER_ID;
        let special = mode & SPECIAL_PERMISSION_MODE;
        let sticky_root = owner == ROOT_USER_ID && special == STICKY_MODE;
        let unsafe_write = mode & GROUP_OR_OTHER_WRITE_MODE != 0 && !sticky_root;
        let unsafe_special = special != 0 && !sticky_root;
        anyhow::ensure!(
            trusted_owner,
            "directory owner mismatch: {}",
            path.display()
        );
        anyhow::ensure!(
            !unsafe_write && !unsafe_special,
            "directory permissions are unsafe: {}",
            path.display()
        );
        Ok(Self::new(metadata))
    }

    pub(super) fn matches_metadata(self, metadata: &Metadata) -> bool {
        self == Self::new(metadata)
    }

    fn new(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

fn verify_directory_type(metadata: &Metadata, path: &Path) -> Result<()> {
    anyhow::ensure!(
        metadata.is_dir(),
        "path is not a directory: {}",
        path.display()
    );
    Ok(())
}
