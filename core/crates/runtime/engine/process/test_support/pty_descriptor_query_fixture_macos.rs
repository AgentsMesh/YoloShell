use super::*;

pub(super) unsafe extern "C" fn exactly_at_limit_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    buffer: *mut libc::c_void,
    bytes: i32,
) -> i32 {
    let entry_bytes = i32::try_from(size_of::<libc::proc_fdinfo>()).unwrap();
    let count = usize::try_from(bytes / entry_bytes).unwrap() - PID_LIST_SENTINEL_COUNT;
    for index in 0..count {
        unsafe {
            buffer
                .cast::<libc::proc_fdinfo>()
                .add(index)
                .write(libc::proc_fdinfo {
                    proc_fd: TEST_DESCRIPTOR,
                    proc_fdtype: libc::PROX_FDTYPE_VNODE as u32,
                });
        }
    }
    i32::try_from(count).unwrap() * entry_bytes
}

pub(super) unsafe extern "C" fn sentinel_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    bytes: i32,
) -> i32 {
    bytes
}

pub(super) unsafe extern "C" fn oversized_inventory_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    bytes: i32,
) -> i32 {
    bytes + i32::try_from(size_of::<libc::proc_fdinfo>()).unwrap()
}

pub(super) unsafe extern "C" fn empty_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    EMPTY_QUERY_RESULT
}
pub(super) unsafe extern "C" fn failed_inventory_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe { *libc::__error() = INJECTED_QUERY_ERROR };
    EMPTY_QUERY_RESULT
}

pub(super) unsafe extern "C" fn one_descriptor_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe {
        buffer.cast::<libc::proc_fdinfo>().write(libc::proc_fdinfo {
            proc_fd: TEST_DESCRIPTOR,
            proc_fdtype: libc::PROX_FDTYPE_VNODE as u32,
        });
    }
    i32::try_from(size_of::<libc::proc_fdinfo>()).unwrap()
}

pub(super) unsafe extern "C" fn misaligned_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    MISALIGNED_DESCRIPTOR_BYTES
}

pub(super) unsafe extern "C" fn closed_descriptor_query(
    _pid: i32,
    _descriptor: i32,
    _flavor: i32,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe { *libc::__error() = libc::EBADF };
    EMPTY_QUERY_RESULT
}

pub(super) unsafe extern "C" fn invalid_vnode_query(
    _pid: i32,
    _descriptor: i32,
    _flavor: i32,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe { *libc::__error() = libc::EINVAL };
    EMPTY_QUERY_RESULT
}

pub(super) unsafe extern "C" fn exited_process_query(
    _pid: i32,
    _descriptor: i32,
    _flavor: i32,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe { *libc::__error() = libc::ESRCH };
    EMPTY_QUERY_RESULT
}

pub(super) unsafe extern "C" fn successful_vnode_query(
    _pid: i32,
    _descriptor: i32,
    flavor: i32,
    buffer: *mut libc::c_void,
    bytes: i32,
) -> i32 {
    if flavor != MACOS_VNODE_PATH_INFO_FLAVOR {
        unsafe { *libc::__error() = libc::EINVAL };
        return EMPTY_QUERY_RESULT;
    }
    let mut value = unsafe { std::mem::zeroed::<VnodeFdInfo>() };
    value.vnode_path.vip_vi.vi_stat.vst_dev = TEST_FILESYSTEM_DEVICE;
    value.vnode_path.vip_vi.vi_stat.vst_ino = TEST_INODE;
    value.vnode_path.vip_vi.vi_stat.vst_rdev = TEST_CHARACTER_DEVICE;
    unsafe { buffer.cast::<VnodeFdInfo>().write(value) };
    bytes
}

pub(super) unsafe extern "C" fn short_vnode_query(
    _pid: i32,
    _descriptor: i32,
    _flavor: i32,
    _buffer: *mut libc::c_void,
    bytes: i32,
) -> i32 {
    bytes - SHORT_RESPONSE_DECREMENT
}
