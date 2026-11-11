use super::*;
use crate::{constants::LINUX_START_TIME_FIELD_OFFSET, procfs};
use std::io::Read;

const MAX_STAT_BYTES: u64 = 64 * 1024;
const READ_SENTINEL: u64 = 1;
const PARENT_PID_OFFSET: usize = 1;
const COMMAND_OPEN: &str = " (";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LinuxProcessStat {
    pub(crate) pid: u32,
    pub(crate) parent_pid: u32,
    pub(crate) state: char,
    pub(crate) start_time: u64,
}

impl LinuxProcessStat {
    pub(crate) fn is_active(&self) -> bool {
        !LINUX_INACTIVE_PROCESS_STATES.contains(&self.state)
    }
}

pub(crate) fn linux_process_stat(pid: u32) -> Result<LinuxProcessStat> {
    let mut bytes = Vec::new();
    std::fs::File::open(procfs::stat_path(pid))?
        .take(MAX_STAT_BYTES + READ_SENTINEL)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_STAT_BYTES,
        LINUX_MALFORMED_STAT_DETAIL
    );
    let stat = parse_linux_process_stat(std::str::from_utf8(&bytes)?)?;
    anyhow::ensure!(stat.pid == pid, LINUX_MALFORMED_STAT_DETAIL);
    Ok(stat)
}

pub(crate) fn parse_linux_process_stat(stat: &str) -> Result<LinuxProcessStat> {
    let end = stat.rfind(')').context(LINUX_MALFORMED_STAT_DETAIL)?;
    let fields = stat[end + 1..]
        .strip_prefix(LINUX_STAT_FIELD_SEPARATOR)
        .context(LINUX_PROCESS_STATE_MISSING_DETAIL)?;
    let state_field = fields
        .split(LINUX_STAT_FIELD_SEPARATOR)
        .nth(LINUX_PROCESS_STATE_FIELD_OFFSET)
        .context(LINUX_PROCESS_STATE_MISSING_DETAIL)?;
    let mut state_chars = state_field.chars();
    let state = state_chars
        .next()
        .context(LINUX_PROCESS_STATE_EMPTY_DETAIL)?;
    anyhow::ensure!(
        state_chars.next().is_none(),
        LINUX_PROCESS_STATE_INVALID_DETAIL
    );
    let start_time = fields
        .split_whitespace()
        .nth(LINUX_START_TIME_FIELD_OFFSET)
        .context(LINUX_START_TIME_MISSING_DETAIL)?
        .parse()
        .context(LINUX_START_TIME_INVALID_DETAIL)?;
    let (pid, command) = stat[..end]
        .split_once(COMMAND_OPEN)
        .context(LINUX_MALFORMED_STAT_DETAIL)?;
    let pid = pid.parse().context(LINUX_MALFORMED_STAT_DETAIL)?;
    anyhow::ensure!(
        pid != u32::default() && !command.is_empty(),
        LINUX_MALFORMED_STAT_DETAIL
    );
    let parent_pid = fields
        .split_whitespace()
        .nth(PARENT_PID_OFFSET)
        .context(LINUX_MALFORMED_STAT_DETAIL)?
        .parse()
        .context(LINUX_MALFORMED_STAT_DETAIL)?;
    Ok(LinuxProcessStat {
        pid,
        parent_pid,
        state,
        start_time,
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/linux_process_tests.rs"]
mod tests;
