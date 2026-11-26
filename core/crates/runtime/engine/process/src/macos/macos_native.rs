use super::constants::MACOS_PROC_PIDINFO_NO_ARGUMENT;
use super::macos_kernel;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::mem::{MaybeUninit, size_of, size_of_val};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

const PROCESS_METADATA_MISSING: &str = "process does not exist";
const EXECUTABLE_PATH_READ_DETAIL: &str = "read executable path";
const METADATA_TYPE_RANGE_CONTEXT: &str = "process metadata type is too large";
const METADATA_READ_DETAIL: &str = "read process metadata";
const METADATA_PID_MISMATCH_DETAIL: &str = "process metadata identifies a different process";
const METADATA_RESPONSE_SIZE_DETAIL: &str = "process metadata response has the wrong size";
const METADATA_MISSING_ERRNO_DETAIL: &str = "process metadata query failed without errno";
const PROCESS_ID_INVALID_DETAIL: &str = "process id must be positive";
const PROCESS_ID_RANGE_CONTEXT: &str = "process id exceeds pid_t";
const PATH_LENGTH_EXCEEDED_DETAIL: &str = "libproc path length exceeds its buffer";
const UNTERMINATED_PATH_CONTEXT: &str = "libproc returned an unterminated path";
const EMPTY_PATH_DETAIL: &str = "libproc returned an empty path";
const PROCESS_IN_EXIT_FLAG: u32 = 0x0000_0004;
const EMPTY_METADATA_RESPONSE: libc::c_int = 0;
const SUCCESS_ERRNO: libc::c_int = 0;

type ProcPidInfo = unsafe extern "C" fn(
    libc::c_int,
    libc::c_int,
    u64,
    *mut libc::c_void,
    libc::c_int,
) -> libc::c_int;

pub(crate) use super::macos_kernel::KernelProcessMetadata;
#[path = "macos_pid_inventory.rs"]
mod pid_inventory;
pub use pid_inventory::foreground_group_pids;
pub(crate) use pid_inventory::user_pids;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessMetadata {
    pub pid: u32,
    pub parent_pid: u32,
    pub process_group: i32,
    pub foreground_group: i32,
    pub start_time: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessExitState {
    Running,
    Exiting,
    MetadataMissing,
}

impl ProcessExitState {
    pub const fn requires_output_progress(self) -> bool {
        !matches!(self, Self::Running)
    }
}

pub fn process_metadata(pid: u32) -> Result<ProcessMetadata> {
    let raw = kernel_process_metadata(pid)?;
    Ok(ProcessMetadata {
        pid: raw.pid,
        parent_pid: raw.parent_pid,
        process_group: raw.process_group,
        foreground_group: raw.foreground_group,
        start_time: raw.start_time,
    })
}

pub(crate) fn kernel_process_metadata(pid: u32) -> Result<KernelProcessMetadata> {
    try_kernel_process_metadata(pid)?.context(PROCESS_METADATA_MISSING)
}

pub(crate) fn try_kernel_process_metadata(pid: u32) -> Result<Option<KernelProcessMetadata>> {
    macos_kernel::metadata(pid)
}

pub fn executable_path(pid: u32) -> Result<PathBuf> {
    let pid = checked_pid(pid)?;
    let capacity = libc::PROC_PIDPATHINFO_MAXSIZE as usize;
    let mut bytes = vec![u8::default(); capacity];
    let read = unsafe { libc::proc_pidpath(pid, bytes.as_mut_ptr().cast(), capacity as u32) };
    ensure!(
        read.is_positive(),
        "{}: {}",
        EXECUTABLE_PATH_READ_DETAIL,
        std::io::Error::last_os_error()
    );
    parse_reported_path(&bytes, read as usize)
}

pub fn cwd_path(pid: u32) -> Result<PathBuf> {
    let info: libc::proc_vnodepathinfo = pid_info(pid, libc::PROC_PIDVNODEPATHINFO)?;
    let path = &info.pvi_cdir.vip_path;
    let bytes =
        unsafe { std::slice::from_raw_parts(path.as_ptr().cast::<u8>(), size_of_val(path)) };
    parse_c_path(bytes)
}

pub fn process_exit_state(pid: u32) -> Result<ProcessExitState> {
    let Some(info) = try_pid_info::<libc::proc_bsdinfo>(pid, libc::PROC_PIDTBSDINFO)? else {
        return Ok(ProcessExitState::MetadataMissing);
    };
    process_exit_flag(pid, info.pbi_pid, info.pbi_flags)
}

fn process_exit_flag(pid: u32, reported_pid: u32, flags: u32) -> Result<ProcessExitState> {
    ensure!(pid == reported_pid, METADATA_PID_MISMATCH_DETAIL);
    Ok(if flags & PROCESS_IN_EXIT_FLAG == 0 {
        ProcessExitState::Running
    } else {
        ProcessExitState::Exiting
    })
}

fn pid_info<T>(pid: u32, flavor: i32) -> Result<T> {
    try_pid_info(pid, flavor)?.context(PROCESS_METADATA_MISSING)
}

fn try_pid_info<T>(pid: u32, flavor: i32) -> Result<Option<T>> {
    try_pid_info_with(pid, flavor, libc::proc_pidinfo)
}

fn try_pid_info_with<T>(pid: u32, flavor: i32, query: ProcPidInfo) -> Result<Option<T>> {
    let pid = checked_pid(pid)?;
    let mut value = MaybeUninit::<T>::uninit();
    let expected = i32::try_from(size_of::<T>()).context(METADATA_TYPE_RANGE_CONTEXT)?;
    unsafe { *libc::__error() = SUCCESS_ERRNO };
    let read = unsafe {
        query(
            pid,
            flavor,
            MACOS_PROC_PIDINFO_NO_ARGUMENT,
            value.as_mut_ptr().cast(),
            expected,
        )
    };
    if read <= EMPTY_METADATA_RESPONSE {
        let error = std::io::Error::last_os_error();
        ensure!(
            error.raw_os_error() != Some(SUCCESS_ERRNO),
            METADATA_MISSING_ERRNO_DETAIL
        );
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(error).context(METADATA_READ_DETAIL);
    }
    ensure!(read == expected, METADATA_RESPONSE_SIZE_DETAIL);
    Ok(Some(unsafe { value.assume_init() }))
}

fn checked_pid(pid: u32) -> Result<i32> {
    ensure!(pid != u32::default(), PROCESS_ID_INVALID_DETAIL);
    i32::try_from(pid).context(PROCESS_ID_RANGE_CONTEXT)
}

fn parse_reported_path(bytes: &[u8], reported: usize) -> Result<PathBuf> {
    ensure!(reported <= bytes.len(), PATH_LENGTH_EXCEEDED_DETAIL);
    let available = &bytes[..reported];
    let end = available
        .iter()
        .position(|byte| *byte == u8::default())
        .unwrap_or(reported);
    path_from_bytes(&available[..end])
}

fn parse_c_path(bytes: &[u8]) -> Result<PathBuf> {
    let end = bytes
        .iter()
        .position(|byte| *byte == u8::default())
        .context(UNTERMINATED_PATH_CONTEXT)?;
    path_from_bytes(&bytes[..end])
}

fn path_from_bytes(bytes: &[u8]) -> Result<PathBuf> {
    ensure!(!bytes.is_empty(), EMPTY_PATH_DETAIL);
    Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_native_tests.rs"]
mod tests;
