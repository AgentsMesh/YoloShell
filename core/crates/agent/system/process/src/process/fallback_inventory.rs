#[cfg(target_os = "linux")]
use super::super::constants::MAX_PROCESS_INVENTORY_PIDS;
use super::super::constants::MIN_PROCESS_PID;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

#[cfg(target_os = "linux")]
pub(super) fn inventory_pids() -> Result<Vec<u32>> {
    inventory_at(
        crate::process::procfs::root_path(),
        MAX_PROCESS_INVENTORY_PIDS,
    )
}

fn inventory_at(root: &Path, pid_limit: usize) -> Result<Vec<u32>> {
    use super::super::constants::process_inventory_error;
    let entries = fs::read_dir(root).context(process_inventory_error::OPEN)?;
    let mut seen = HashSet::new();
    let mut pids = Vec::new();
    for entry in entries {
        let entry = entry.context(process_inventory_error::READ)?;
        let Some(pid) = parse_pid_name(&entry.file_name())? else {
            continue;
        };
        ensure!(pids.len() < pid_limit, process_inventory_error::PID_LIMIT);
        ensure!(seen.insert(pid), process_inventory_error::DUPLICATE_PID);
        pids.push(pid);
    }
    pids.sort_unstable();
    Ok(pids)
}

fn parse_pid_name(name: &OsStr) -> Result<Option<u32>> {
    use super::super::constants::process_inventory_error;
    let Some(name) = name.to_str() else {
        return Ok(None);
    };
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    let pid = name
        .parse::<u32>()
        .context(process_inventory_error::MALFORMED_PID)?;
    ensure!(
        pid >= MIN_PROCESS_PID,
        process_inventory_error::MALFORMED_PID
    );
    Ok(Some(pid))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_inventory_tests.rs"]
mod tests;
