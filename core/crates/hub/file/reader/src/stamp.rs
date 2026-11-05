use crate::{constants::SINGLE_LINK, descriptor::*};
use serde::Serialize;
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::identity::ObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FileStamp {
    object: ObjectIdentity,
    pub length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    mode: u32,
    owner: u32,
    group: u32,
}

impl FileStamp {
    pub fn capture(file: &File) -> Result<Self, PublicError> {
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() || metadata.nlink() != SINGLE_LINK {
            return Err(PublicError::Unauthorized);
        }
        Ok(Self {
            object: identity(file)?,
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode: metadata.mode(),
            owner: metadata.uid(),
            group: metadata.gid(),
        })
    }
}
