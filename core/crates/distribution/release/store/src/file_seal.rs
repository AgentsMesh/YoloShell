use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::{fs::File, os::unix::fs::MetadataExt, path::Path};
use yolo_paths::identity::ObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileSeal {
    object: ObjectIdentity,
    pub(crate) length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileSeal {
    pub(crate) fn capture(file: &File, path: &Path, mode: u32, maximum: u64) -> Result<Self> {
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.nlink() == SINGLE_FILE_LINK
                && metadata.mode() & PERMISSION_MASK == mode
                && metadata.len() <= maximum,
            INVALID_FILE
        );
        yolo_paths::private_acl::require_empty(file, path)?;
        let object = ObjectIdentity::from_file(file)?.context(INVALID_FILE)?;
        Ok(Self {
            object,
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}
