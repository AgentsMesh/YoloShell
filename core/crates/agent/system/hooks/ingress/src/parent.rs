use super::contract::HookProcessIdentity;
use super::profile::CALLBACK_SHELLS;
use anyhow::{Result, ensure};
use std::collections::HashSet;
use std::path::PathBuf;

const INVALID: &str = "provider hook process ancestry is invalid";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Parent {
    pub(super) pid: u32,
    pub(super) parent_pid: u32,
    pub(super) start_time: u64,
    pub(super) executable: PathBuf,
}

pub(super) fn candidate(max_ancestors: usize) -> Result<HookProcessIdentity> {
    candidate_from(std::process::id(), max_ancestors, observe)
}

pub(super) fn candidate_from(
    pid: u32,
    max_ancestors: usize,
    mut observe: impl FnMut(u32) -> Result<Parent>,
) -> Result<HookProcessIdentity> {
    let mut current = observe(pid)?;
    ensure!(
        current.pid == pid && current.start_time != u64::default(),
        INVALID
    );
    let mut seen = HashSet::from([pid]);
    for _ in 0..max_ancestors {
        ensure!(
            current.parent_pid != u32::default() && seen.insert(current.parent_pid),
            INVALID
        );
        let parent = observe(current.parent_pid)?;
        ensure!(
            parent.pid == current.parent_pid
                && parent.start_time != u64::default()
                && observe(current.pid)? == current,
            INVALID
        );
        if !CALLBACK_SHELLS
            .iter()
            .any(|shell| parent.executable == std::path::Path::new(shell))
        {
            return Ok(HookProcessIdentity {
                pid: parent.pid,
                start_time: parent.start_time,
            });
        }
        current = parent;
    }
    anyhow::bail!(INVALID)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn observe(pid: u32) -> Result<Parent> {
    #[cfg(target_os = "linux")]
    use yolo_process::linux_native::{executable_path, process_metadata};
    #[cfg(target_os = "macos")]
    use yolo_process::macos_native::{executable_path, process_metadata};
    let before = process_metadata(pid)?;
    let executable = executable_path(pid)?;
    let after = process_metadata(pid)?;
    ensure!(
        before.pid == after.pid
            && before.start_time == after.start_time
            && before.parent_pid == after.parent_pid,
        INVALID
    );
    Ok(Parent {
        pid,
        parent_pid: after.parent_pid,
        start_time: after.start_time,
        executable,
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn observe(_: u32) -> Result<Parent> {
    anyhow::bail!(INVALID)
}
