use crate::process::{LiveProcessIdentity, ProcessIdentity};
use std::path::{Path, PathBuf};

pub(super) fn capture_pid_file(path: &Path) -> Option<(ProcessIdentity, LiveProcessIdentity)> {
    let pid = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    let durable = ProcessIdentity::capture(pid).ok()?;
    let live = LiveProcessIdentity::capture(durable).ok()??;
    Some((durable, live))
}

pub(super) fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
