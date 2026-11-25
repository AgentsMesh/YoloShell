use super::constants::{LINUX_PROC_ENVIRONMENT_ENTRY, LINUX_PROC_STATUS_ENTRY};
use std::path::{Path, PathBuf};

const PROCFS_ROOT: &str = "/proc";
const BOOT_ID_ENTRY: &str = "sys/kernel/random/boot_id";
const CMDLINE_ENTRY: &str = "cmdline";
const CWD_ENTRY: &str = "cwd";
const EXECUTABLE_ENTRY: &str = "exe";
const FILE_DESCRIPTORS_ENTRY: &str = "fd";
const STAT_ENTRY: &str = "stat";

pub fn root_path() -> &'static Path {
    Path::new(PROCFS_ROOT)
}

pub fn boot_id_path() -> PathBuf {
    Path::new(PROCFS_ROOT).join(BOOT_ID_ENTRY)
}

pub fn cmdline_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, CMDLINE_ENTRY)
}

pub fn cwd_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, CWD_ENTRY)
}

pub fn environment_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, LINUX_PROC_ENVIRONMENT_ENTRY)
}

pub fn executable_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, EXECUTABLE_ENTRY)
}

pub fn file_descriptors_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, FILE_DESCRIPTORS_ENTRY)
}

pub fn stat_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, STAT_ENTRY)
}

pub fn status_path(pid: u32) -> PathBuf {
    process_entry_at(root_path(), pid, LINUX_PROC_STATUS_ENTRY)
}

fn process_entry_at(root: &Path, pid: u32, entry: &str) -> PathBuf {
    root.join(pid.to_string()).join(entry)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/procfs_inline_tests.rs"]
mod tests;
