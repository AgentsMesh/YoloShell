use crate::{
    Error,
    model::{HANDLE_CAPACITY, Handle},
};
use std::os::fd::{AsRawFd, BorrowedFd};

#[repr(C)]
struct NativeHandle {
    header: libc::file_handle,
    bytes: [u8; HANDLE_CAPACITY],
}

pub(super) fn capture(descriptor: BorrowedFd<'_>) -> Result<Handle, Error> {
    let mut output = NativeHandle {
        header: libc::file_handle {
            handle_bytes: HANDLE_CAPACITY as libc::c_uint,
            handle_type: libc::c_int::default(),
            f_handle: [],
        },
        bytes: [u8::default(); HANDLE_CAPACITY],
    };
    let mut mount_id = libc::c_int::default();
    // SAFETY: NativeHandle supplies the advertised trailing bytes immediately
    // after libc's flexible-array header. The descriptor remains borrowed.
    let status = unsafe {
        libc::name_to_handle_at(
            descriptor.as_raw_fd(),
            c"".as_ptr(),
            std::ptr::addr_of_mut!(output).cast::<libc::file_handle>(),
            &mut mount_id,
            libc::AT_EMPTY_PATH,
        )
    };
    if status != libc::c_int::default() {
        let source = std::io::Error::last_os_error();
        if matches!(
            source.raw_os_error(),
            Some(libc::EOPNOTSUPP | libc::ENOSYS | libc::EOVERFLOW)
        ) {
            return Err(Error::Unsupported("bounded persistent export handle"));
        }
        return Err(Error::Io {
            operation: "read persistent export handle",
            source,
        });
    }
    parse_handle(&output)
}

fn parse_handle(output: &NativeHandle) -> Result<Handle, Error> {
    let length = output.header.handle_bytes as usize;
    if length > HANDLE_CAPACITY {
        return Err(Error::Invalid("kernel handle length"));
    }
    Handle::new(output.header.handle_type, &output.bytes[..length])
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/linux_handle.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/linux_handle.rs"]
mod unit_tests;
