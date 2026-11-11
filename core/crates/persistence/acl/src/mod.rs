use anyhow::{Context, Result};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::Path;

pub fn require_empty(file: &File, path: &Path) -> Result<()> {
    require(file, path, true)
}

pub fn require_no_grants(file: &File, path: &Path) -> Result<()> {
    require(file, path, false)
}

pub fn clear(file: &File, path: &Path) -> Result<()> {
    platform::clear(file.as_raw_fd())
        .with_context(|| format!("clear ACL for {}", path.display()))?;
    require_empty(file, path)
}

fn require(file: &File, path: &Path, reject_denials: bool) -> Result<()> {
    let forbidden = platform::has_forbidden_entry(file.as_raw_fd(), reject_denials)
        .with_context(|| format!("inspect ACL for {}", path.display()))?;
    anyhow::ensure!(
        !forbidden,
        "path has an unsafe extended ACL: {}",
        path.display()
    );
    Ok(())
}

#[cfg(target_os = "macos")]
mod platform {
    use std::io;
    use std::os::fd::RawFd;
    use std::ptr;

    type AclPointer = *mut libc::c_void;
    type AclEntryPointer = *mut libc::c_void;
    const ACL_TYPE_EXTENDED: libc::c_int = 0x0000_0100;
    const ACL_FIRST_ENTRY: libc::c_int = 0;
    const ACL_NEXT_ENTRY: libc::c_int = -1;
    const ACL_EXTENDED_DENY: libc::c_int = 2;

    unsafe extern "C" {
        fn acl_init(count: libc::c_int) -> AclPointer;
        fn acl_free(object: *mut libc::c_void) -> libc::c_int;
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> AclPointer;
        fn acl_get_entry(
            acl: AclPointer,
            entry_id: libc::c_int,
            entry: *mut AclEntryPointer,
        ) -> libc::c_int;
        fn acl_get_tag_type(entry: AclEntryPointer, tag: *mut libc::c_int) -> libc::c_int;
        fn acl_set_fd_np(fd: libc::c_int, acl: AclPointer, kind: libc::c_int) -> libc::c_int;
    }

    struct OwnedAcl(AclPointer);

    impl OwnedAcl {
        fn load(fd: RawFd) -> io::Result<Option<Self>> {
            let acl = unsafe { acl_get_fd_np(fd, ACL_TYPE_EXTENDED) };
            if !acl.is_null() {
                return Ok(Some(Self(acl)));
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ENOENT) {
                Ok(None)
            } else {
                Err(error)
            }
        }

        fn empty() -> io::Result<Self> {
            let acl = unsafe { acl_init(0) };
            (!acl.is_null())
                .then_some(Self(acl))
                .ok_or_else(io::Error::last_os_error)
        }

        fn has_forbidden_entry(&self, reject_denials: bool) -> io::Result<bool> {
            let mut entry_id = ACL_FIRST_ENTRY;
            loop {
                let mut entry = ptr::null_mut();
                if unsafe { acl_get_entry(self.0, entry_id, &mut entry) } == 0 {
                    if reject_denials || entry_tag(entry)? != ACL_EXTENDED_DENY {
                        return Ok(true);
                    }
                    entry_id = ACL_NEXT_ENTRY;
                    continue;
                }
                let error = io::Error::last_os_error();
                return if error.raw_os_error() == Some(libc::EINVAL) {
                    Ok(false)
                } else {
                    Err(error)
                };
            }
        }
    }

    impl Drop for OwnedAcl {
        fn drop(&mut self) {
            let _ = unsafe { acl_free(self.0) };
        }
    }

    fn entry_tag(entry: AclEntryPointer) -> io::Result<libc::c_int> {
        let mut tag = 0;
        if unsafe { acl_get_tag_type(entry, &mut tag) } == 0 {
            Ok(tag)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(super) fn has_forbidden_entry(fd: RawFd, reject_denials: bool) -> io::Result<bool> {
        match OwnedAcl::load(fd)? {
            Some(acl) => acl.has_forbidden_entry(reject_denials),
            None => Ok(false),
        }
    }

    pub(super) fn clear(fd: RawFd) -> io::Result<()> {
        let acl = OwnedAcl::empty()?;
        if unsafe { acl_set_fd_np(fd, acl.0, ACL_TYPE_EXTENDED) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod platform {
    use std::io;
    use std::os::fd::RawFd;

    pub(super) fn has_forbidden_entry(_: RawFd, _: bool) -> io::Result<bool> {
        Err(io::ErrorKind::Unsupported.into())
    }

    pub(super) fn clear(_: RawFd) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/private_acl_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "linux", feature = "yolo-unit-tests"))]
#[path = "../unit_tests/linux_acl_tests.rs"]
mod linux_tests;
