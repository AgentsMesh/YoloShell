use super::*;

const ACCESS: &std::ffi::CStr = c"system.posix_acl_access";
const DEFAULT: &std::ffi::CStr = c"system.posix_acl_default";
const INVALID_DESCRIPTOR: std::os::fd::RawFd = -1;
const STATE: &str = "acl-fixture";
const ACL_VERSION: u32 = 2;
const ACL_USER_OBJECT: u16 = 1;
const ACL_USER: u16 = 2;
const ACL_GROUP_OBJECT: u16 = 4;
const ACL_MASK: u16 = 16;
const ACL_OTHER: u16 = 32;
const READ: u16 = 4;
const READ_WRITE: u16 = 6;
const NO_PERMISSIONS: u16 = 0;
const NAMED_USER: u32 = 42;
const NO_IDENTIFIER: u32 = u32::MAX;

#[test]
fn linux_empty_acl_is_accepted_and_clear_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(STATE);
    let file = File::create(&path).unwrap();
    require_empty(&file, &path).unwrap();
    clear(&file, &path).unwrap();
    clear(&file, &path).unwrap();
}

#[test]
fn linux_access_and_default_acls_are_rejected_and_removed() {
    for attribute in [ACCESS, DEFAULT] {
        let root = tempfile::tempdir().unwrap();
        let file = File::open(root.path()).unwrap();
        let mut bytes = ACL_VERSION.to_le_bytes().to_vec();
        for (tag, permissions, identifier) in [
            (ACL_USER_OBJECT, READ_WRITE, NO_IDENTIFIER),
            (ACL_USER, READ, NAMED_USER),
            (ACL_GROUP_OBJECT, NO_PERMISSIONS, NO_IDENTIFIER),
            (ACL_MASK, READ, NO_IDENTIFIER),
            (ACL_OTHER, NO_PERMISSIONS, NO_IDENTIFIER),
        ] {
            bytes.extend(tag.to_le_bytes());
            bytes.extend(permissions.to_le_bytes());
            bytes.extend(identifier.to_le_bytes());
        }
        let result = unsafe {
            libc::fsetxattr(
                file.as_raw_fd(),
                attribute.as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
            )
        };
        assert_eq!(result, 0, "{}", std::io::Error::last_os_error());
        assert!(require_empty(&file, root.path()).is_err());
        assert!(require_no_grants(&file, root.path()).is_err());
        clear(&file, root.path()).unwrap();
        require_empty(&file, root.path()).unwrap();
    }
}

#[test]
fn linux_invalid_descriptors_fail_closed() {
    assert!(platform::has_forbidden_entry(INVALID_DESCRIPTOR, true).is_err());
    assert!(platform::clear(INVALID_DESCRIPTOR).is_err());
}
