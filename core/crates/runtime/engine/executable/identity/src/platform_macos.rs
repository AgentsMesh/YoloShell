use crate::{constants::*, fingerprint::Fingerprint};
use anyhow::{Result, ensure};
use std::{
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::PathBuf,
};

pub(crate) struct Opened {
    pub file: File,
    pub binding: Binding,
}
pub(crate) struct Binding {
    process: u32,
    reported: PathBuf,
    path: PathBuf,
}

pub(crate) fn open_current() -> Result<Opened> {
    open_named(std::env::current_exe()?)
}

pub(crate) fn open_named(reported: PathBuf) -> Result<Opened> {
    ensure!(reported.is_absolute(), INVALID_PATH);
    // macOS may report the launch symlink (including Bazel runfiles). Resolving
    // that spelling selects a candidate; the mapped-vnode check below proves it.
    let path = std::fs::canonicalize(&reported)?;
    // Nonblocking excludes a substituted FIFO before regular-file inspection.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&path)?;
    Ok(Opened {
        file,
        binding: Binding {
            process: std::process::id(),
            reported,
            path,
        },
    })
}

impl Binding {
    pub fn verify(&self, file: &File) -> Result<()> {
        ensure!(self.process == std::process::id(), WRONG_PROCESS);
        let retained = Fingerprint::inspect(&file.metadata()?)?;
        yolo_process::macos_mapped_executable::verify_mapped_executable(
            self.process,
            file.as_raw_fd(),
        )?;
        ensure!(
            std::fs::canonicalize(&self.reported)? == self.path,
            IMAGE_CHANGED
        );
        let named = Fingerprint::inspect(&std::fs::symlink_metadata(&self.path)?)?;
        ensure!(named == retained, IMAGE_CHANGED);
        ensure!(
            Fingerprint::inspect(&std::fs::symlink_metadata(&self.path)?)? == retained,
            IMAGE_CHANGED
        );
        ensure!(
            std::fs::canonicalize(&self.reported)? == self.path,
            IMAGE_CHANGED
        );
        Ok(())
    }
}
