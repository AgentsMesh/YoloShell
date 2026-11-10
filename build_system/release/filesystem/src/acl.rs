use crate::constants::{ACL_ABSENT_ERROR, ACL_FIRST_ENTRY, ACL_SUCCESS, ACL_TYPE_EXTENDED};
use anyhow::{Context, Result, ensure};
use std::ffi::c_void;
use std::os::fd::RawFd;
use std::ptr;

pub fn ensure_no_extended_acl(descriptor: RawFd, description: &str) -> Result<()> {
    let value = unsafe { acl_get_fd_np(descriptor, ACL_TYPE_EXTENDED) };
    if value.is_null() {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ACL_ABSENT_ERROR) {
            return Ok(());
        }
        return Err(error).with_context(|| format!("inspect {description} ACL"));
    }
    let acl = Acl(value);
    let mut entry = ptr::null_mut();
    let result = unsafe { acl_get_entry(acl.0, ACL_FIRST_ENTRY, &mut entry) };
    if result != ACL_SUCCESS {
        let error = std::io::Error::last_os_error();
        ensure!(
            error.raw_os_error() == Some(libc::EINVAL),
            "inspect {description} ACL entries: {error}"
        );
    }
    ensure!(result != ACL_SUCCESS, "{description} must not have an ACL");
    Ok(())
}

struct Acl(*mut c_void);

impl Drop for Acl {
    fn drop(&mut self) {
        unsafe {
            acl_free(self.0);
        }
    }
}

unsafe extern "C" {
    fn acl_get_fd_np(descriptor: libc::c_int, acl_type: libc::c_int) -> *mut c_void;
    fn acl_get_entry(
        acl: *mut c_void,
        entry_id: libc::c_int,
        entry: *mut *mut c_void,
    ) -> libc::c_int;
    fn acl_free(value: *mut c_void) -> libc::c_int;
}
