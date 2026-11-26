use super::super::constants::{
    MACOS_PROC_PIDINFO_NO_ARGUMENT, MACOS_VNODE_PATH_INFO_FLAVOR, PID_LIST_SENTINEL_COUNT,
};
use super::macos_contract::{CapabilityProofError, CapabilityProofResult};
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::{Context, Result, anyhow};
use std::mem::{MaybeUninit, size_of};

const FD_BUFFER_OVERFLOW_CONTEXT: &str = "PTY proof descriptor buffer overflows";
const FD_BUFFER_RANGE_CONTEXT: &str = "PTY proof descriptor buffer exceeds libproc range";
const FD_INVENTORY_CONTEXT: &str = "list PTY proof process descriptors";
const FD_INVENTORY_MISALIGNED_DETAIL: &str = "misaligned PTY proof descriptor inventory";
const FD_INVENTORY_LIMIT_DETAIL: &str = "PTY proof descriptor inventory exceeds its budget";
const VNODE_INFO_CONTEXT: &str = "read PTY proof vnode descriptor";
const VNODE_INFO_SHORT_DETAIL: &str = "short PTY proof vnode descriptor response";
const PID_RANGE_CONTEXT: &str = "PTY proof process id exceeds platform range";
const EMPTY_QUERY_RESULT: i32 = 0;
const SUCCESS_ERRNO: i32 = 0;

type ProcessInfoQuery = unsafe extern "C" fn(i32, i32, u64, *mut libc::c_void, i32) -> i32;
type ProcessFdInfoQuery = unsafe extern "C" fn(i32, i32, i32, *mut libc::c_void, i32) -> i32;

#[repr(C)]
struct ProcessFileInfo {
    open_flags: u32,
    status: u32,
    offset: libc::off_t,
    file_type: i32,
    guard_flags: u32,
}

#[repr(C)]
struct VnodeFdInfo {
    file: ProcessFileInfo,
    vnode_path: libc::vnode_info_path,
}

pub(super) fn list_descriptors(
    pid: u32,
    limit: usize,
) -> CapabilityProofResult<Vec<libc::proc_fdinfo>> {
    list_descriptors_with(pid, limit, libc::proc_pidinfo)
}

fn list_descriptors_with(
    pid: u32,
    limit: usize,
    query: ProcessInfoQuery,
) -> CapabilityProofResult<Vec<libc::proc_fdinfo>> {
    let capacity = limit
        .checked_add(PID_LIST_SENTINEL_COUNT)
        .context(FD_BUFFER_OVERFLOW_CONTEXT)
        .map_err(CapabilityProofError::invariant)?;
    let bytes = capacity
        .checked_mul(size_of::<libc::proc_fdinfo>())
        .context(FD_BUFFER_OVERFLOW_CONTEXT)
        .map_err(CapabilityProofError::invariant)?;
    let bytes = i32::try_from(bytes)
        .context(FD_BUFFER_RANGE_CONTEXT)
        .map_err(CapabilityProofError::invariant)?;
    let mut values = Vec::<libc::proc_fdinfo>::with_capacity(capacity);
    reset_errno();
    let read = unsafe {
        query(
            checked_pid(pid).map_err(CapabilityProofError::invariant)?,
            libc::PROC_PIDLISTFDS,
            MACOS_PROC_PIDINFO_NO_ARGUMENT,
            values.as_mut_ptr().cast(),
            bytes,
        )
    };
    validate_query_result(read).map_err(CapabilityProofError::candidate)?;
    let read = read as usize;
    if !read.is_multiple_of(size_of::<libc::proc_fdinfo>()) {
        return Err(CapabilityProofError::invariant(anyhow!(
            FD_INVENTORY_MISALIGNED_DETAIL
        )));
    }
    let count = read / size_of::<libc::proc_fdinfo>();
    if count > limit {
        return Err(CapabilityProofError::budget(anyhow!(
            FD_INVENTORY_LIMIT_DETAIL
        )));
    }
    // libproc initialized count complete entries; the sentinel check keeps them within capacity.
    unsafe { values.set_len(count) };
    Ok(values)
}

fn validate_query_result(read: i32) -> Result<()> {
    if read.is_positive() {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if read == EMPTY_QUERY_RESULT && error.raw_os_error() == Some(SUCCESS_ERRNO) {
        return Ok(());
    }
    Err(error).context(FD_INVENTORY_CONTEXT)
}

fn reset_errno() {
    unsafe { *libc::__error() = SUCCESS_ERRNO };
}

pub(super) fn vnode_identity(
    pid: u32,
    descriptor: i32,
) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>> {
    vnode_identity_with(pid, descriptor, libc::proc_pidfdinfo)
}

fn vnode_identity_with(
    pid: u32,
    descriptor: i32,
    query: ProcessFdInfoQuery,
) -> CapabilityProofResult<Option<MacosPtyObjectIdentity>> {
    let mut value = MaybeUninit::<VnodeFdInfo>::uninit();
    let expected =
        i32::try_from(size_of::<VnodeFdInfo>()).map_err(CapabilityProofError::invariant)?;
    reset_errno();
    let read = unsafe {
        query(
            checked_pid(pid).map_err(CapabilityProofError::invariant)?,
            descriptor,
            MACOS_VNODE_PATH_INFO_FLAVOR,
            value.as_mut_ptr().cast(),
            expected,
        )
    };
    if read <= EMPTY_QUERY_RESULT {
        let error = std::io::Error::last_os_error();
        return Err(CapabilityProofError::candidate(
            anyhow::Error::new(error).context(VNODE_INFO_CONTEXT),
        ));
    }
    if read != expected {
        return Err(CapabilityProofError::invariant(anyhow!(
            VNODE_INFO_SHORT_DETAIL
        )));
    }
    let value = unsafe { value.assume_init() };
    Ok(Some(MacosPtyObjectIdentity::from_vnode_stat(
        &value.vnode_path.vip_vi.vi_stat,
    )))
}

fn checked_pid(pid: u32) -> Result<i32> {
    i32::try_from(pid).context(PID_RANGE_CONTEXT)
}

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_fds_macos_tests.rs"]
mod integration_tests;
#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../../unit_tests/pty_session_proof_fds_macos_tests.rs"]
mod unit_tests;
