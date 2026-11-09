use anyhow::{Context, Result, ensure};
use std::mem::{MaybeUninit, size_of};
use std::os::fd::RawFd;

const FIRST_MAPPED_ADDRESS: u64 = 0;
// Darwin <sys/proc_info.h>; the current libc crate omits this flavor.
const PROC_PIDREGIONPATHINFO: libc::c_int = 8;
const PROC_REGION_DETAIL_FIELDS: usize = 14;
const PROC_REGION_INFO_ABI_SIZE: usize = 96;
const QUERY_FAILURE_BOUNDARY: libc::c_int = 0;
const SYSTEM_CALL_FAILURE: libc::c_int = -1;
const CAPTURE_DESCRIPTOR_CONTEXT: &str = "capture retained executable identity";
const INVALID_PROCESS_CONTEXT: &str = "mapped executable process id exceeds pid_t";
const QUERY_CONTEXT: &str = "query lowest mapped executable vnode";
const RESPONSE_SIZE_DETAIL: &str = "mapped executable query returned the wrong size";
const FILE_TYPE_DETAIL: &str = "lowest mapped executable vnode is not a regular file";
const REGION_PROTECTION_DETAIL: &str = "lowest file mapping is not executable";
const IDENTITY_MISMATCH_DETAIL: &str = "mapped executable vnode does not match retained descriptor";

type ProcPidInfo = unsafe extern "C" fn(
    libc::c_int,
    libc::c_int,
    u64,
    *mut libc::c_void,
    libc::c_int,
) -> libc::c_int;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutableVnodeIdentity {
    pub(crate) device: u32,
    pub(crate) inode: u64,
}

#[repr(C)]
struct ProcRegionInfo {
    protection: u32,
    _max_protection: u32,
    _inheritance: u32,
    _flags: u32,
    _offset: u64,
    _details: [u32; PROC_REGION_DETAIL_FIELDS],
    _address: u64,
    _size: u64,
}

#[repr(C)]
struct ProcRegionWithPathInfo {
    region: ProcRegionInfo,
    vnode: libc::vnode_info_path,
}

const _: () = assert!(size_of::<ProcRegionInfo>() == PROC_REGION_INFO_ABI_SIZE);

pub fn verify_mapped_executable(pid: u32, descriptor: RawFd) -> Result<ExecutableVnodeIdentity> {
    verify_mapped_executable_with(pid, descriptor, libc::proc_pidinfo)
}

fn verify_mapped_executable_with(
    pid: u32,
    descriptor: RawFd,
    query: ProcPidInfo,
) -> Result<ExecutableVnodeIdentity> {
    let retained = retained_identity(descriptor)?;
    let mapped = mapped_region(pid, query)?;
    validate_region(retained, &mapped)
}

fn retained_identity(descriptor: RawFd) -> Result<ExecutableVnodeIdentity> {
    let mut status = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstat(descriptor, status.as_mut_ptr()) };
    ensure!(
        result != SYSTEM_CALL_FAILURE,
        "{}: {}",
        CAPTURE_DESCRIPTOR_CONTEXT,
        std::io::Error::last_os_error()
    );
    let status = unsafe { status.assume_init() };
    Ok(ExecutableVnodeIdentity {
        device: status.st_dev.cast_unsigned(),
        inode: status.st_ino,
    })
}

fn mapped_region(pid: u32, query: ProcPidInfo) -> Result<ProcRegionWithPathInfo> {
    let pid = libc::pid_t::try_from(pid).context(INVALID_PROCESS_CONTEXT)?;
    let mut value = MaybeUninit::<ProcRegionWithPathInfo>::zeroed();
    let expected =
        libc::c_int::try_from(size_of::<ProcRegionWithPathInfo>()).context(RESPONSE_SIZE_DETAIL)?;
    unsafe { *libc::__error() = libc::c_int::default() };
    let read = unsafe {
        query(
            pid,
            PROC_PIDREGIONPATHINFO,
            FIRST_MAPPED_ADDRESS,
            value.as_mut_ptr().cast(),
            expected,
        )
    };
    if read <= QUERY_FAILURE_BOUNDARY {
        return Err(std::io::Error::last_os_error()).context(QUERY_CONTEXT);
    }
    ensure!(read == expected, RESPONSE_SIZE_DETAIL);
    Ok(unsafe { value.assume_init() })
}

fn validate_region(
    retained: ExecutableVnodeIdentity,
    mapped: &ProcRegionWithPathInfo,
) -> Result<ExecutableVnodeIdentity> {
    let status = &mapped.vnode.vip_vi.vi_stat;
    ensure!(
        status.vst_mode & libc::S_IFMT == libc::S_IFREG,
        FILE_TYPE_DETAIL
    );
    ensure!(
        mapped.region.protection & libc::VM_PROT_EXECUTE.cast_unsigned() != 0,
        REGION_PROTECTION_DETAIL
    );
    let actual = ExecutableVnodeIdentity {
        device: status.vst_dev,
        inode: status.vst_ino,
    };
    ensure!(actual == retained, IDENTITY_MISMATCH_DETAIL);
    Ok(actual)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_mapped_executable_process_tests.rs"]
mod process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_mapped_executable_tests.rs"]
mod tests;
