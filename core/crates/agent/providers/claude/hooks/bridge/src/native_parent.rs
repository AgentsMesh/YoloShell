use crate::claude_hooks::{ClaudeProcessIdentity, NATIVE_CALLBACK_SHELLS};
use anyhow::{Result, ensure};
use std::collections::HashSet;
use std::path::PathBuf;

const MISSING: &str = "Claude native hook parent process is unavailable";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Parent {
    pub(crate) pid: u32,
    pub(crate) parent_pid: u32,
    pub(crate) start_time: u64,
    pub(crate) executable: PathBuf,
}

pub(super) fn candidate() -> Result<ClaudeProcessIdentity> {
    candidate_from(std::process::id(), observe)
}

fn candidate_from(
    pid: u32,
    mut observe: impl FnMut(u32) -> Result<Parent>,
) -> Result<ClaudeProcessIdentity> {
    let mut current = observe(pid)?;
    ensure!(
        current.pid == pid && current.start_time != u64::default(),
        MISSING
    );
    let mut seen = HashSet::from([pid]);
    for _ in 0..crate::claude_hooks::CLAUDE_NATIVE_MAX_ANCESTORS {
        ensure!(
            current.parent_pid != u32::default() && seen.insert(current.parent_pid),
            MISSING
        );
        let parent = observe(current.parent_pid)?;
        ensure!(
            parent.pid == current.parent_pid
                && parent.start_time != u64::default()
                && observe(current.pid)? == current,
            MISSING
        );
        if !NATIVE_CALLBACK_SHELLS
            .iter()
            .any(|shell| parent.executable == std::path::Path::new(shell))
        {
            return Ok(ClaudeProcessIdentity {
                pid: parent.pid,
                start_time: parent.start_time,
            });
        }
        current = parent;
    }
    anyhow::bail!(MISSING)
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
        MISSING
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
    anyhow::bail!(MISSING)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/native_parent_tests.rs"]
mod tests;
