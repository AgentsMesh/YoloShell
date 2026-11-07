use crate::constants::*;
use anyhow::{Result, ensure};
use std::{fs::Metadata, os::unix::fs::MetadataExt};
use yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Fingerprint {
    device: u64,
    inode: u64,
    pub length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    mode: u32,
    owner: (u32, u32),
    links: u64,
    #[cfg(target_os = "macos")]
    birth: (i64, i64),
}

impl Fingerprint {
    pub fn inspect(metadata: &Metadata) -> Result<Self> {
        ensure!(
            metadata.is_file()
                && metadata.len() > u64::default()
                && metadata.len() <= MAX_EXECUTABLE_IMAGE_BYTES
                && metadata.mode() & EXECUTABLE_MODE != u32::default(),
            INVALID_IMAGE
        );
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt as _;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode: metadata.mode(),
            owner: (metadata.uid(), metadata.gid()),
            links: metadata.nlink(),
            #[cfg(target_os = "macos")]
            birth: (metadata.st_birthtime(), metadata.st_birthtime_nsec()),
        })
    }
}
