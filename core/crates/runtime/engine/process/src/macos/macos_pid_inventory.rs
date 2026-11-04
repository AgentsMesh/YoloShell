use super::super::constants::{MACOS_PROC_PGRP_ONLY, MACOS_PROC_UID_ONLY, PID_LIST_SENTINEL_COUNT};
use anyhow::{Context, Result, ensure};
use std::mem::size_of;

const FOREGROUND_GROUP_INVALID_DETAIL: &str = "foreground process group must be positive";
const PID_LIMIT_OVERFLOW_CONTEXT: &str = "process id limit overflows";
const PID_BUFFER_OVERFLOW_CONTEXT: &str = "process id buffer overflows";
const PID_BUFFER_RANGE_CONTEXT: &str = "process id buffer exceeds libproc range";
const ENUMERATION_FAILURE_CONTEXT: &str = "enumerate processes with libproc";
const MISALIGNED_RESPONSE_DETAIL: &str = "misaligned process id response";
const BUFFER_EXCEEDED_DETAIL: &str = "libproc exceeded its process id buffer";
const INVENTORY_BUDGET_DETAIL: &str = "process id inventory exceeds its budget";
const NEGATIVE_PROCESS_ID_CONTEXT: &str = "libproc returned a negative process id";
const EMPTY_QUERY_RESULT: i32 = 0;
const SUCCESS_ERRNO: i32 = 0;

type ListPidsQuery = unsafe extern "C" fn(u32, u32, *mut libc::c_void, i32) -> i32;

pub fn foreground_group_pids(group: i32, limit: usize) -> Result<Vec<u32>> {
    ensure!(group.is_positive(), FOREGROUND_GROUP_INVALID_DETAIL);
    list_pids_with(
        MACOS_PROC_PGRP_ONLY,
        group as u32,
        limit,
        libc::proc_listpids,
    )
}

pub(crate) fn user_pids(user: libc::uid_t, limit: usize) -> Result<Vec<u32>> {
    list_pids_with(MACOS_PROC_UID_ONLY, user, limit, libc::proc_listpids)
}

fn list_pids_with(
    kind: u32,
    selector: u32,
    limit: usize,
    query: ListPidsQuery,
) -> Result<Vec<u32>> {
    let (mut buffer, bytes) = pid_buffer(limit)?;
    reset_errno();
    let returned = unsafe { query(kind, selector, buffer.as_mut_ptr().cast(), bytes) };
    validate_query_result(returned)?;
    parse_byte_counted_pids(&buffer, returned, limit)
}

fn validate_query_result(returned: i32) -> Result<()> {
    if returned.is_positive() {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if returned == EMPTY_QUERY_RESULT && error.raw_os_error() == Some(SUCCESS_ERRNO) {
        return Ok(());
    }
    Err(error).context(ENUMERATION_FAILURE_CONTEXT)
}

fn reset_errno() {
    unsafe { *libc::__error() = SUCCESS_ERRNO };
}

fn pid_buffer(limit: usize) -> Result<(Vec<libc::pid_t>, i32)> {
    let capacity = limit
        .checked_add(PID_LIST_SENTINEL_COUNT)
        .context(PID_LIMIT_OVERFLOW_CONTEXT)?;
    let bytes = capacity
        .checked_mul(size_of::<libc::pid_t>())
        .context(PID_BUFFER_OVERFLOW_CONTEXT)?;
    let bytes = i32::try_from(bytes).context(PID_BUFFER_RANGE_CONTEXT)?;
    Ok((vec![libc::pid_t::default(); capacity], bytes))
}

fn parse_byte_counted_pids(buffer: &[libc::pid_t], bytes: i32, limit: usize) -> Result<Vec<u32>> {
    ensure!(!bytes.is_negative(), ENUMERATION_FAILURE_CONTEXT);
    let bytes = bytes as usize;
    ensure!(
        bytes.is_multiple_of(size_of::<libc::pid_t>()),
        MISALIGNED_RESPONSE_DETAIL
    );
    parse_pid_values(buffer, bytes / size_of::<libc::pid_t>(), limit)
}

fn parse_pid_values(buffer: &[libc::pid_t], count: usize, limit: usize) -> Result<Vec<u32>> {
    ensure!(count <= buffer.len(), BUFFER_EXCEEDED_DETAIL);
    ensure!(count <= limit, INVENTORY_BUDGET_DETAIL);
    buffer[..count]
        .iter()
        .copied()
        .filter(|pid| *pid != libc::pid_t::default())
        .map(|pid| u32::try_from(pid).context(NEGATIVE_PROCESS_ID_CONTEXT))
        .collect()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_pid_inventory_tests.rs"]
mod tests;
