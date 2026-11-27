use super::PrivateDirectory;
use anyhow::{Context, Result};
use std::os::unix::fs::MetadataExt;

const REVISION_CONTEXT: &str = "inspect private directory revision";

/// An enumeration invalidation hint, never proof that an entry is safe to mutate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryRevision {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl PrivateDirectory {
    pub fn revision(&self) -> Result<DirectoryRevision> {
        let metadata = self.file.metadata().context(REVISION_CONTEXT)?;
        Ok(DirectoryRevision {
            device: metadata.dev(),
            inode: metadata.ino(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }
}
