use super::queue::ObservationQueue;
use crate::paths::identity::ObjectIdentity;
use anyhow::{Context, Result, ensure};
use std::fs::{File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

const OPEN_CONTEXT: &str = "retain Guardian idle observation vnode";
const OBJECT_DETAIL: &str = "Guardian idle observation requires a regular file or directory";

pub(super) struct ObservedVnode {
    file: File,
    path: PathBuf,
    directory: bool,
}

impl ObservedVnode {
    pub(super) fn open(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_EVTONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(&path)
            .context(OPEN_CONTEXT)?;
        let metadata = file.metadata()?;
        ensure!(metadata.is_dir() || metadata.is_file(), OBJECT_DETAIL);
        Ok(Self {
            file,
            path,
            directory: metadata.is_dir(),
        })
    }

    pub(super) fn register(&self, queue: &ObservationQueue) -> Result<()> {
        queue.watch_vnode(self.file.as_raw_fd(), self.directory)
    }

    pub(super) fn matches_current_path(&self) -> Result<bool> {
        let current = std::fs::symlink_metadata(&self.path)?;
        Ok(ObjectIdentity::from_metadata(&self.file.metadata()?)
            == ObjectIdentity::from_metadata(&current))
    }
}
