use anyhow::{Result, ensure};
use std::fs::File;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const SEALED_INPUT_TYPE_ERROR: &str = "sealed input must be a regular file";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSeal {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    links: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl FileSeal {
    pub fn capture(file: &File) -> Result<Self> {
        let metadata = file.metadata()?;
        ensure!(metadata.is_file(), SEALED_INPUT_TYPE_ERROR);
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            mode: metadata.permissions().mode(),
            links: metadata.nlink(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    pub fn verify(&self, file: &File, error: &str) -> Result<()> {
        ensure!(Self::capture(file)? == *self, error.to_owned());
        Ok(())
    }

    pub fn recapture_after_rename(&self, file: &File, error: &str) -> Result<Self> {
        let current = Self::capture(file)?;
        ensure!(self.stable_across_rename(&current), error.to_owned());
        Ok(current)
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    fn stable_across_rename(&self, current: &Self) -> bool {
        self.device == current.device
            && self.inode == current.inode
            && self.length == current.length
            && self.mode == current.mode
            && self.links == current.links
            && self.modified_seconds == current.modified_seconds
            && self.modified_nanoseconds == current.modified_nanoseconds
    }
}
