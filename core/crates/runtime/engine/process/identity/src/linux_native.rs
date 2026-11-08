use crate::{process::linux::linux_process_stat, procfs};
use anyhow::{Result, ensure};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::PathBuf;

const CHANGED: &str = "Linux native process identity changed during observation";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessMetadata {
    pub pid: u32,
    pub parent_pid: u32,
    pub start_time: u64,
}

pub fn process_metadata(pid: u32) -> Result<ProcessMetadata> {
    let stat = linux_process_stat(pid)?;
    ensure!(
        stat.is_active() && stat.start_time != u64::default(),
        CHANGED
    );
    Ok(ProcessMetadata {
        pid: stat.pid,
        parent_pid: stat.parent_pid,
        start_time: stat.start_time,
    })
}

pub fn executable_path(pid: u32) -> Result<PathBuf> {
    let before = process_metadata(pid)?;
    let path = std::fs::read_link(procfs::executable_path(pid))?;
    ensure!(
        path.is_absolute()
            && process_metadata(pid)? == before
            && std::fs::read_link(procfs::executable_path(pid))? == path,
        CHANGED
    );
    Ok(path)
}

pub fn open_executable(pid: u32) -> Result<File> {
    let before = process_metadata(pid)?;
    // /proc/<pid>/exe is an intentional kernel-owned magic link to the mapped executable.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(procfs::executable_path(pid))?;
    let metadata = file.metadata()?;
    let current = std::fs::metadata(procfs::executable_path(pid))?;
    ensure!(
        metadata.is_file()
            && current.is_file()
            && (
                metadata.dev(),
                metadata.ino(),
                metadata.len(),
                metadata.mtime(),
                metadata.mtime_nsec(),
                metadata.ctime(),
                metadata.ctime_nsec()
            ) == (
                current.dev(),
                current.ino(),
                current.len(),
                current.mtime(),
                current.mtime_nsec(),
                current.ctime(),
                current.ctime_nsec()
            )
            && process_metadata(pid)? == before,
        CHANGED
    );
    Ok(file)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/linux_native_tests.rs"]
mod tests;
