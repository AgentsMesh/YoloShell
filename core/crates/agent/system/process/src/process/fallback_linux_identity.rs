use super::super::constants::{MAX_PROCESS_PATH_BYTES, MIN_PROCESS_PID};
use super::fallback_observation::{CwdIdentity, ExecutableIdentity, ProcessIdentity, ProcessRow};
use std::fs::OpenOptions;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

pub(super) fn process_identity(pid: u32) -> Option<ProcessIdentity> {
    if pid < MIN_PROCESS_PID {
        return None;
    }
    let stat_entry = crate::process::procfs::stat_path(pid);
    let process_path = stat_entry.parent()?;
    let process_metadata = std::fs::metadata(process_path).ok()?;
    if !process_metadata.is_dir() {
        return None;
    }
    let stat = super::fallback_linux_stat::read_process_stat(pid)?;
    let cwd = process_cwd_identity(pid)?;
    let executable_file = process_executable_identity(pid)?;
    let stable_process_metadata = std::fs::metadata(process_path).ok()?;
    if process_metadata.dev() != stable_process_metadata.dev()
        || process_metadata.ino() != stable_process_metadata.ino()
    {
        return None;
    }
    Some(ProcessIdentity {
        pid,
        process_device: process_metadata.dev(),
        process_inode: process_metadata.ino(),
        start_time: stat.start_time,
        parent_pid: stat.parent_pid,
        process_group: stat.process_group,
        foreground_group: stat.foreground_group,
        executable: executable_file.path.to_str()?.to_owned(),
        executable_file,
        cwd,
    })
}

fn process_executable_identity(pid: u32) -> Option<ExecutableIdentity> {
    let pinned = pinned_path(crate::process::procfs::executable_path(pid), libc::S_IFREG)?;
    Some(ExecutableIdentity {
        path: pinned.path,
        device: pinned.device,
        inode: pinned.inode,
        file_type: pinned.file_type,
    })
}

fn process_cwd_identity(pid: u32) -> Option<CwdIdentity> {
    let pinned = pinned_path(crate::process::procfs::cwd_path(pid), libc::S_IFDIR)?;
    Some(CwdIdentity {
        path: pinned.path,
        device: pinned.device,
        inode: pinned.inode,
    })
}

struct PinnedPath {
    path: std::path::PathBuf,
    device: u64,
    inode: u64,
    file_type: u32,
}

fn pinned_path(entry: std::path::PathBuf, expected_type: u32) -> Option<PinnedPath> {
    let before = std::fs::read_link(&entry).ok()?;
    if !before.is_absolute() || before.as_os_str().as_bytes().len() > MAX_PROCESS_PATH_BYTES {
        return None;
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
        .open(&entry)
        .ok()?;
    let metadata = file.metadata().ok()?;
    let after = std::fs::read_link(entry).ok()?;
    let file_type = metadata.mode() & libc::S_IFMT;
    (before == after && file_type == expected_type).then_some(PinnedPath {
        path: before,
        device: metadata.dev(),
        inode: metadata.ino(),
        file_type,
    })
}

pub(super) fn process_row_from_identity(identity: ProcessIdentity) -> ProcessRow {
    ProcessRow {
        pid: identity.pid,
        parent_pid: identity.parent_pid,
        process_group: identity.process_group,
        foreground_group: identity.foreground_group,
        executable: identity.executable.clone(),
        inventory_identity: Some(identity),
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_linux_identity_tests.rs"]
mod tests;
