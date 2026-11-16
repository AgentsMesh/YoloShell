use super::constants::{MACOS_PROC_PIDINFO_NO_ARGUMENT, MICROSECONDS_PER_SECOND};
use anyhow::{Context, Result, ensure};
use std::mem::{MaybeUninit, offset_of, size_of};

#[path = "macos_kernel_inactive.rs"]
mod inactive;
#[path = "macos_kernel_retry.rs"]
mod retry;
#[cfg(test)]
use inactive::INACTIVE_CONFIRMATION_ATTEMPTS;
use inactive::confirmed_inactive;

const EXECUTABLE_UUID_BYTES: usize = 16;
const PROC_PIDT_BSDINFOWITHUNIQID: i32 = 18;
const PROC_BSD_INFO_SIZE: usize = 136;
const PROC_UNIQUE_INFO_SIZE: usize = 56;
const PROC_BSD_WITH_UNIQUE_INFO_SIZE: usize = 192;
const PROC_BSD_STATUS_OFFSET: usize = 4;
const PROC_BSD_PID_OFFSET: usize = 12;
const PROC_BSD_PARENT_PID_OFFSET: usize = 16;
const PROC_BSD_PROCESS_GROUP_OFFSET: usize = 100;
const PROC_BSD_FOREGROUND_GROUP_OFFSET: usize = 112;
const PROC_BSD_START_SECONDS_OFFSET: usize = 120;
const PROC_BSD_START_MICROSECONDS_OFFSET: usize = 128;
const PROC_EXECUTABLE_UUID_OFFSET: usize = 0;
const PROC_UNIQUE_ID_OFFSET: usize = 16;
const PROC_PARENT_UNIQUE_ID_OFFSET: usize = 24;
const PROC_PID_VERSION_OFFSET: usize = 32;
const PROC_ORIGINAL_PARENT_PID_VERSION_OFFSET: usize = 36;
const PROC_RESERVED_2_OFFSET: usize = 40;
const PROC_RESERVED_3_OFFSET: usize = 48;
const PROC_BSD_INFO_OFFSET: usize = 0;
const PROC_UNIQUE_INFO_OFFSET: usize = 136;
const CAPTURE_CONTEXT: &str = "capture macOS kernel process metadata";
const CANDIDATE_CAPTURE_CONTEXT: &str = "capture macOS kernel process metadata candidate";
const RESPONSE_SIZE_CONTEXT: &str = "macOS kernel process metadata response has the wrong size";
const UNIQUE_ID_CONTEXT: &str = "macOS kernel process unique id must be positive";
const MISSING_QUERY_ERRNO_DETAIL: &str = "macOS kernel process metadata query failed without errno";
const MIN_VALID_PROCESS_ID: i32 = 1;
const MAX_EMPTY_RESPONSE_SIZE: i32 = 0;
const MISSING_UNIQUE_ID: u64 = 0;

type ProcPidInfo = unsafe extern "C" fn(
    libc::c_int,
    libc::c_int,
    u64,
    *mut libc::c_void,
    libc::c_int,
) -> libc::c_int;

#[repr(C)]
pub(super) struct ProcUniqueIdentifierInfo {
    executable_uuid: [u8; EXECUTABLE_UUID_BYTES],
    pub(super) unique_id: u64,
    pub(super) parent_unique_id: u64,
    pid_version: i32,
    original_parent_pid_version: i32,
    reserved_2: u64,
    reserved_3: u64,
}

#[repr(C)]
struct ProcBsdInfoWithUniqueId {
    bsd: libc::proc_bsdinfo,
    unique: ProcUniqueIdentifierInfo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelProcessMetadata {
    pub pid: u32,
    pub status: u32,
    pub parent_pid: u32,
    pub process_group: i32,
    pub foreground_group: i32,
    pub start_time: u64,
    pub unique_id: u64,
    pub parent_unique_id: u64,
    pub pid_version: u32,
}

const _: () = {
    assert!(size_of::<libc::proc_bsdinfo>() == PROC_BSD_INFO_SIZE);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_status) == PROC_BSD_STATUS_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_pid) == PROC_BSD_PID_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_ppid) == PROC_BSD_PARENT_PID_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_pgid) == PROC_BSD_PROCESS_GROUP_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, e_tpgid) == PROC_BSD_FOREGROUND_GROUP_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_start_tvsec) == PROC_BSD_START_SECONDS_OFFSET);
    assert!(offset_of!(libc::proc_bsdinfo, pbi_start_tvusec) == PROC_BSD_START_MICROSECONDS_OFFSET);
    assert!(size_of::<ProcUniqueIdentifierInfo>() == PROC_UNIQUE_INFO_SIZE);
    assert!(offset_of!(ProcUniqueIdentifierInfo, executable_uuid) == PROC_EXECUTABLE_UUID_OFFSET);
    assert!(offset_of!(ProcUniqueIdentifierInfo, unique_id) == PROC_UNIQUE_ID_OFFSET);
    assert!(offset_of!(ProcUniqueIdentifierInfo, parent_unique_id) == PROC_PARENT_UNIQUE_ID_OFFSET);
    assert!(offset_of!(ProcUniqueIdentifierInfo, pid_version) == PROC_PID_VERSION_OFFSET);
    assert!(
        offset_of!(ProcUniqueIdentifierInfo, original_parent_pid_version)
            == PROC_ORIGINAL_PARENT_PID_VERSION_OFFSET
    );
    assert!(offset_of!(ProcUniqueIdentifierInfo, reserved_2) == PROC_RESERVED_2_OFFSET);
    assert!(offset_of!(ProcUniqueIdentifierInfo, reserved_3) == PROC_RESERVED_3_OFFSET);
    assert!(size_of::<ProcBsdInfoWithUniqueId>() == PROC_BSD_WITH_UNIQUE_INFO_SIZE);
    assert!(offset_of!(ProcBsdInfoWithUniqueId, bsd) == PROC_BSD_INFO_OFFSET);
    assert!(offset_of!(ProcBsdInfoWithUniqueId, unique) == PROC_UNIQUE_INFO_OFFSET);
};

pub fn metadata(pid: u32) -> Result<Option<KernelProcessMetadata>> {
    metadata_with_context(pid, libc::proc_pidinfo)
}

fn metadata_with_context(pid: u32, query: ProcPidInfo) -> Result<Option<KernelProcessMetadata>> {
    retry::metadata(pid, query).map_err(|error| {
        let context = format!("{CANDIDATE_CAPTURE_CONTEXT}: pid={pid}: {error:#}");
        error.context(context)
    })
}

fn metadata_with(pid: u32, query: ProcPidInfo) -> Result<Option<KernelProcessMetadata>> {
    let checked_pid = i32::try_from(pid).context(CAPTURE_CONTEXT)?;
    ensure!(checked_pid >= MIN_VALID_PROCESS_ID, CAPTURE_CONTEXT);
    let mut raw = MaybeUninit::<ProcBsdInfoWithUniqueId>::uninit();
    let expected = i32::try_from(size_of::<ProcBsdInfoWithUniqueId>()).context(CAPTURE_CONTEXT)?;
    reset_errno();
    let read = unsafe {
        query(
            checked_pid,
            PROC_PIDT_BSDINFOWITHUNIQID,
            MACOS_PROC_PIDINFO_NO_ARGUMENT,
            raw.as_mut_ptr().cast(),
            expected,
        )
    };
    if read <= MAX_EMPTY_RESPONSE_SIZE {
        let error = std::io::Error::last_os_error();
        ensure!(
            error.raw_os_error() != Some(libc::c_int::default()),
            MISSING_QUERY_ERRNO_DETAIL
        );
        return match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(None),
            Some(libc::EPERM | libc::EINVAL | libc::ENOENT)
                if confirmed_inactive(checked_pid, query) =>
            {
                Ok(None)
            }
            _ => Err(error).context(CAPTURE_CONTEXT),
        };
    }
    ensure!(read == expected, RESPONSE_SIZE_CONTEXT);
    let raw = unsafe { raw.assume_init() };
    if raw.bsd.pbi_status == libc::SZOMB {
        return Ok(None);
    }
    convert(pid, raw).map(Some)
}

fn reset_errno() {
    unsafe { *libc::__error() = libc::c_int::default() };
}

fn convert(pid: u32, raw: ProcBsdInfoWithUniqueId) -> Result<KernelProcessMetadata> {
    ensure!(raw.bsd.pbi_pid == pid, CAPTURE_CONTEXT);
    ensure!(raw.unique.unique_id != MISSING_UNIQUE_ID, UNIQUE_ID_CONTEXT);
    Ok(KernelProcessMetadata {
        pid,
        status: raw.bsd.pbi_status,
        parent_pid: raw.bsd.pbi_ppid,
        process_group: i32::try_from(raw.bsd.pbi_pgid).context(CAPTURE_CONTEXT)?,
        foreground_group: i32::from_ne_bytes(raw.bsd.e_tpgid.to_ne_bytes()),
        start_time: raw
            .bsd
            .pbi_start_tvsec
            .saturating_mul(MICROSECONDS_PER_SECOND)
            .saturating_add(raw.bsd.pbi_start_tvusec),
        unique_id: raw.unique.unique_id,
        parent_unique_id: raw.unique.parent_unique_id,
        pid_version: u32::from_ne_bytes(raw.unique.pid_version.to_ne_bytes()),
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/macos_kernel_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/macos_kernel_exit_tests.rs"]
mod exit_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/macos_kernel_absence_tests.rs"]
mod absence_tests;
