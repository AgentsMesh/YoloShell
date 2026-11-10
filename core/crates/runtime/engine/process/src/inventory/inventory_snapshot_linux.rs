use super::InventoryEntry;
use crate::process::constants::{
    LINUX_PROC_INVENTORY_ENTRY_LIMIT, LINUX_PROC_STAT_BYTES, LINUX_PTY_PROOF_STATUS_BYTES,
    LINUX_START_TIME_FIELD_OFFSET, MAX_PROCESS_INVENTORY_IDS,
};
use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
const FIELD_SEPARATOR: char = ' ';
const PARENT_PID_FIELD_OFFSET: usize = 1;
const PROCESS_GROUP_FIELD_OFFSET: usize = 2;
const SESSION_ID_FIELD_OFFSET: usize = 3;
const EFFECTIVE_UID_FIELD_OFFSET: usize = 1;
const READ_SENTINEL_BYTES: usize = 1;
const STAT_ENTRY: &str = "stat";
const STATUS_UID_FIELD: &str = "Uid:";
const INVENTORY_LIMIT_DETAIL: &str = "Linux process inventory exceeds its bounded limit";
const MALFORMED_STAT_DETAIL: &str = "malformed Linux process inventory stat";
const MALFORMED_STATUS_DETAIL: &str = "malformed Linux process inventory status";

#[derive(Clone, Copy, Eq, PartialEq)]
struct StatFields {
    pid: u32,
    parent_pid: u32,
    process_group: u32,
    session_id: u32,
    start_time: u64,
}

pub(super) fn inventory() -> Result<Vec<InventoryEntry>> {
    let owner = unsafe { libc::geteuid() };
    inventory_at(
        super::super::procfs::root_path(),
        owner,
        LINUX_PROC_INVENTORY_ENTRY_LIMIT,
        MAX_PROCESS_INVENTORY_IDS,
    )
}

fn inventory_at(
    root: &Path,
    owner: u32,
    entry_limit: usize,
    process_limit: usize,
) -> Result<Vec<InventoryEntry>> {
    let mut entries = Vec::new();
    let mut visited = usize::default();
    for directory_entry in root.read_dir()? {
        visited = visited.checked_add(1).context(INVENTORY_LIMIT_DETAIL)?;
        ensure!(visited <= entry_limit, INVENTORY_LIMIT_DETAIL);
        let directory_entry = directory_entry?;
        let Some(pid) = parse_pid(&directory_entry.file_name()) else {
            continue;
        };
        if let Some(entry) = read_entry(root, pid, owner)? {
            ensure!(entries.len() < process_limit, INVENTORY_LIMIT_DETAIL);
            entries.push(entry);
        }
    }
    entries.sort_unstable_by_key(|entry| entry.pid);
    Ok(entries)
}

fn read_entry(root: &Path, pid: u32, owner: u32) -> Result<Option<InventoryEntry>> {
    let directory = root.join(pid.to_string());
    let before = match read_stat(&directory, pid) {
        Ok(stat) => stat,
        Err(error) if disappeared(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let effective_user_id = match read_effective_uid(&directory) {
        Ok(uid) => uid,
        Err(error) if disappeared(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let after = match read_stat(&directory, pid) {
        Ok(stat) => stat,
        Err(error) if disappeared(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(entry_from_observations(
        before,
        effective_user_id,
        after,
        owner,
    ))
}

fn entry_from_observations(
    before: StatFields,
    effective_user_id: u32,
    after: StatFields,
    owner: u32,
) -> Option<InventoryEntry> {
    (before == after && effective_user_id == owner).then_some(InventoryEntry {
        pid: after.pid,
        parent_pid: after.parent_pid,
        process_group: after.process_group,
        session_id: after.session_id,
        start_time: after.start_time,
        effective_user_id,
    })
}

fn read_stat(directory: &Path, expected_pid: u32) -> Result<StatFields> {
    let stat = read_bounded(&directory.join(STAT_ENTRY), LINUX_PROC_STAT_BYTES)?;
    let stat = std::str::from_utf8(&stat).context(MALFORMED_STAT_DETAIL)?;
    parse_stat(stat, expected_pid)
}

fn parse_stat(stat: &str, expected_pid: u32) -> Result<StatFields> {
    let command_end = stat.rfind(')').context(MALFORMED_STAT_DETAIL)?;
    let prefix = &stat[..command_end];
    let pid = prefix
        .split_once(FIELD_SEPARATOR)
        .context(MALFORMED_STAT_DETAIL)?
        .0
        .parse()?;
    ensure!(
        pid == expected_pid && pid != u32::default(),
        MALFORMED_STAT_DETAIL
    );
    let fields = stat[command_end + 1..]
        .strip_prefix(FIELD_SEPARATOR)
        .context(MALFORMED_STAT_DETAIL)?
        .split_whitespace()
        .collect::<Vec<_>>();
    let parse_u32 = |offset: usize| -> Result<u32> {
        Ok(fields.get(offset).context(MALFORMED_STAT_DETAIL)?.parse()?)
    };
    let parse_u64 = |offset: usize| -> Result<u64> {
        Ok(fields.get(offset).context(MALFORMED_STAT_DETAIL)?.parse()?)
    };
    Ok(StatFields {
        pid,
        parent_pid: parse_u32(PARENT_PID_FIELD_OFFSET)?,
        process_group: parse_u32(PROCESS_GROUP_FIELD_OFFSET)?,
        session_id: parse_u32(SESSION_ID_FIELD_OFFSET)?,
        start_time: parse_u64(LINUX_START_TIME_FIELD_OFFSET)?,
    })
}

fn read_effective_uid(directory: &Path) -> Result<u32> {
    let status = read_bounded(
        &directory.join(crate::process::constants::LINUX_PROC_STATUS_ENTRY),
        LINUX_PTY_PROOF_STATUS_BYTES,
    )?;
    let status = std::str::from_utf8(&status).context(MALFORMED_STATUS_DETAIL)?;
    status
        .lines()
        .find_map(|line| line.strip_prefix(STATUS_UID_FIELD))
        .and_then(|values| values.split_whitespace().nth(EFFECTIVE_UID_FIELD_OFFSET))
        .context(MALFORMED_STATUS_DETAIL)?
        .parse()
        .context(MALFORMED_STATUS_DETAIL)
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let capacity = limit
        .checked_add(READ_SENTINEL_BYTES)
        .context(INVENTORY_LIMIT_DETAIL)?;
    let mut bytes = Vec::with_capacity(capacity.min(LINUX_PROC_STAT_BYTES));
    std::fs::File::open(path)?
        .take(u64::try_from(capacity)?)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, INVENTORY_LIMIT_DETAIL);
    Ok(bytes)
}

fn parse_pid(name: &OsStr) -> Option<u32> {
    let name = name.to_str()?;
    (!name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| name.parse().ok())
        .flatten()
        .filter(|pid| *pid != u32::default())
}

fn disappeared(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| {
                error.kind() == std::io::ErrorKind::NotFound
                    || error.raw_os_error() == Some(libc::ESRCH)
            })
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/inventory/inventory_snapshot_linux_tests.rs"]
mod tests;
