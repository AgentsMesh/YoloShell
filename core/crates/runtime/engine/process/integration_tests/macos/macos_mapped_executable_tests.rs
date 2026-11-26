use super::*;
use std::os::fd::AsRawFd;

const ORIGINAL_EXECUTABLE: &str = "/usr/bin/true";
const SHORT_RESPONSE_BYTES: libc::c_int = 1;

#[test]
fn query_failures_and_invalid_inputs_fail_closed() {
    let executable = std::fs::File::open(ORIGINAL_EXECUTABLE).unwrap();
    let descriptor = executable.as_raw_fd();
    let failure =
        verify_mapped_executable_with(std::process::id(), descriptor, failed_query).unwrap_err();
    assert!(failure.to_string().contains(QUERY_CONTEXT));

    let short =
        verify_mapped_executable_with(std::process::id(), descriptor, short_query).unwrap_err();
    assert!(short.to_string().contains(RESPONSE_SIZE_DETAIL));

    let invalid_pid =
        verify_mapped_executable_with(u32::MAX, descriptor, failed_query).unwrap_err();
    assert!(invalid_pid.to_string().contains(INVALID_PROCESS_CONTEXT));

    let invalid_descriptor =
        verify_mapped_executable(std::process::id(), SYSTEM_CALL_FAILURE).unwrap_err();
    assert!(
        invalid_descriptor
            .to_string()
            .contains(CAPTURE_DESCRIPTOR_CONTEXT)
    );
}

#[test]
fn mismatched_or_ineligible_regions_fail_closed() {
    let executable = std::fs::File::open(ORIGINAL_EXECUTABLE).unwrap();
    let retained = retained_identity(executable.as_raw_fd()).unwrap();
    let mismatch = region(retained.device, retained.inode.wrapping_add(1));
    assert_rejected(retained, &mismatch, IDENTITY_MISMATCH_DETAIL);

    let mut non_executable = region(retained.device, retained.inode);
    non_executable.region.protection = u32::default();
    assert_rejected(retained, &non_executable, REGION_PROTECTION_DETAIL);

    let mut non_file = region(retained.device, retained.inode);
    non_file.vnode.vip_vi.vi_stat.vst_mode = libc::S_IFDIR;
    assert_rejected(retained, &non_file, FILE_TYPE_DETAIL);
}

fn assert_rejected(
    retained: ExecutableVnodeIdentity,
    mapped: &ProcRegionWithPathInfo,
    detail: &str,
) {
    let error = validate_region(retained, mapped).unwrap_err();
    assert!(error.to_string().contains(detail));
}

unsafe extern "C" fn failed_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _arg: u64,
    _buffer: *mut libc::c_void,
    _size: libc::c_int,
) -> libc::c_int {
    unsafe { *libc::__error() = libc::EACCES };
    QUERY_FAILURE_BOUNDARY
}

unsafe extern "C" fn short_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _arg: u64,
    _buffer: *mut libc::c_void,
    _size: libc::c_int,
) -> libc::c_int {
    SHORT_RESPONSE_BYTES
}

fn region(device: u32, inode: u64) -> ProcRegionWithPathInfo {
    let mut region = unsafe { MaybeUninit::<ProcRegionWithPathInfo>::zeroed().assume_init() };
    region.region.protection = libc::VM_PROT_EXECUTE.cast_unsigned();
    region.vnode.vip_vi.vi_stat.vst_mode = libc::S_IFREG;
    region.vnode.vip_vi.vi_stat.vst_dev = device;
    region.vnode.vip_vi.vi_stat.vst_ino = inode;
    region
}
