use super::super::constants::{MAX_PROCESS_STAT_BYTES, PROCESS_STAT_READ_SENTINEL_BYTES};
use std::io::Read;

const CLOSE_MARKER: &[u8] = b") ";
const EXECUTABLE_OPEN: u8 = b'(';
const FOREGROUND_GROUP_INDEX: usize = 5;
const PARENT_PID_INDEX: usize = 1;
const PROCESS_GROUP_INDEX: usize = 2;
const START_TIME_INDEX: usize = 19;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinuxProcessStat {
    pub(super) pid: u32,
    pub(super) parent_pid: u32,
    pub(super) process_group: i32,
    pub(super) foreground_group: i32,
    pub(super) executable: String,
    pub(super) start_time: u64,
}

#[cfg(target_os = "linux")]
pub(super) fn read_process_stat(pid: u32) -> Option<LinuxProcessStat> {
    let file = std::fs::File::open(crate::process::procfs::stat_path(pid)).ok()?;
    let bytes = read_bounded(file)?;
    parse_process_stat(&bytes).filter(|stat| stat.pid == pid)
}

fn read_bounded(reader: impl Read) -> Option<Vec<u8>> {
    let byte_limit = MAX_PROCESS_STAT_BYTES.checked_add(PROCESS_STAT_READ_SENTINEL_BYTES)?;
    let read_limit = u64::try_from(byte_limit).ok()?;
    let mut bytes = Vec::with_capacity(byte_limit);
    reader.take(read_limit).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= MAX_PROCESS_STAT_BYTES).then_some(bytes)
}

pub(super) fn parse_process_stat(bytes: &[u8]) -> Option<LinuxProcessStat> {
    if bytes.len() > MAX_PROCESS_STAT_BYTES {
        return None;
    }
    let executable_open = bytes.iter().position(|byte| *byte == EXECUTABLE_OPEN)?;
    let executable_close = bytes
        .windows(CLOSE_MARKER.len())
        .rposition(|window| window == CLOSE_MARKER)?;
    if executable_close <= executable_open {
        return None;
    }
    let pid = parse::<u32>(&bytes[..executable_open])?;
    let executable = std::str::from_utf8(&bytes[executable_open + 1..executable_close]).ok()?;
    if pid == u32::default() || executable.is_empty() {
        return None;
    }
    let fields: Vec<_> = std::str::from_utf8(&bytes[executable_close + CLOSE_MARKER.len()..])
        .ok()?
        .split_whitespace()
        .collect();
    let start_time: u64 = fields.get(START_TIME_INDEX)?.parse().ok()?;
    if start_time == u64::default() {
        return None;
    }
    Some(LinuxProcessStat {
        pid,
        parent_pid: fields.get(PARENT_PID_INDEX)?.parse().ok()?,
        process_group: fields.get(PROCESS_GROUP_INDEX)?.parse().ok()?,
        foreground_group: fields.get(FOREGROUND_GROUP_INDEX)?.parse().ok()?,
        executable: executable.to_owned(),
        start_time,
    })
}

fn parse<T: std::str::FromStr>(bytes: &[u8]) -> Option<T> {
    std::str::from_utf8(bytes).ok()?.trim().parse().ok()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_linux_stat_tests.rs"]
mod tests;
