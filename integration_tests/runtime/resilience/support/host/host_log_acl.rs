use std::io;
use std::os::fd::RawFd;

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    type AclPointer = *mut libc::c_void;
    type AclEntryPointer = *mut libc::c_void;
    const ACL_TYPE_EXTENDED: libc::c_int = 0x0000_0100;
    const ACL_FIRST_ENTRY: libc::c_int = 0;

    unsafe extern "C" {
        fn acl_init(count: libc::c_int) -> AclPointer;
        fn acl_free(object: *mut libc::c_void) -> libc::c_int;
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> AclPointer;
        fn acl_get_entry(
            acl: AclPointer,
            entry_id: libc::c_int,
            entry: *mut AclEntryPointer,
        ) -> libc::c_int;
        fn acl_set_fd_np(fd: libc::c_int, acl: AclPointer, kind: libc::c_int) -> libc::c_int;
    }

    struct OwnedAcl(AclPointer);

    impl OwnedAcl {
        fn empty() -> io::Result<Self> {
            let pointer = unsafe { acl_init(0) };
            (!pointer.is_null())
                .then_some(Self(pointer))
                .ok_or_else(io::Error::last_os_error)
        }

        fn load(fd: RawFd) -> io::Result<Option<Self>> {
            let pointer = unsafe { acl_get_fd_np(fd, ACL_TYPE_EXTENDED) };
            if !pointer.is_null() {
                return Ok(Some(Self(pointer)));
            }
            let error = io::Error::last_os_error();
            (error.raw_os_error() == Some(libc::ENOENT))
                .then_some(None)
                .ok_or(error)
        }

        fn has_entry(&self) -> io::Result<bool> {
            let mut entry = std::ptr::null_mut();
            let result = unsafe { acl_get_entry(self.0, ACL_FIRST_ENTRY, &mut entry) };
            if result == 0 {
                return Ok(true);
            }
            let error = io::Error::last_os_error();
            (error.raw_os_error() == Some(libc::EINVAL))
                .then_some(false)
                .ok_or(error)
        }
    }

    impl Drop for OwnedAcl {
        fn drop(&mut self) {
            let _ = unsafe { acl_free(self.0) };
        }
    }

    pub(super) fn clear_and_verify(fd: RawFd) -> io::Result<()> {
        let empty = OwnedAcl::empty()?;
        let result = unsafe { acl_set_fd_np(fd, empty.0, ACL_TYPE_EXTENDED) };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        match OwnedAcl::load(fd)? {
            Some(acl) if acl.has_entry()? => Err(io::Error::other("stderr ACL is not empty")),
            _ => Ok(()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub(super) fn clear_and_verify(_fd: RawFd) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn clear_and_verify(fd: RawFd) -> io::Result<()> {
    platform::clear_and_verify(fd)
}
