use std::ffi::CStr;
use std::io;
use std::os::fd::RawFd;

const ACL_ATTRIBUTES: &[&CStr] = &[c"system.posix_acl_access", c"system.posix_acl_default"];

pub(super) fn has_forbidden_entry(fd: RawFd, _: bool) -> io::Result<bool> {
    for name in ACL_ATTRIBUTES {
        let size = unsafe { libc::fgetxattr(fd, name.as_ptr(), std::ptr::null_mut(), 0) };
        if size >= 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        if !absent(&error) {
            return Err(error);
        }
    }
    Ok(false)
}

pub(super) fn clear(fd: RawFd) -> io::Result<()> {
    for name in ACL_ATTRIBUTES {
        if unsafe { libc::fremovexattr(fd, name.as_ptr()) } == 0 {
            continue;
        }
        let error = io::Error::last_os_error();
        if !absent(&error) {
            return Err(error);
        }
    }
    Ok(())
}

fn absent(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENODATA | libc::ENOTSUP))
}
