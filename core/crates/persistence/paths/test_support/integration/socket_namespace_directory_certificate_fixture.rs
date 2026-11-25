use super::{NamespaceBinding, constants::*};
use crate::paths::directory::PrivateDirectory;

pub(super) const ORIGINAL_DIRECTORY: &str = "certified";
pub(super) const OTHER_DIRECTORY: &str = "other";
pub(super) const SAVED_DIRECTORY: &str = "saved";
pub(super) const MARKER: &str = "temporary-regular-file";
pub(super) const MARKER_BYTES: &[u8] = b"temporary data";
pub(super) const INVALID_BYTES: &[u8] = b"not a certificate";
pub(super) const OVERSIZE_BYTES: usize = ATTRIBUTE_MAX_BYTES + 1;
const REPLACE_ATTRIBUTE: libc::c_int = 0;
const REGISTRATION_NAME: &str = "registration.json";
const REGISTRATION_VALUE: &str = "fixture registration";

pub(super) fn registration(
    directory: &PrivateDirectory,
) -> crate::paths::identity::DurableObjectIdentity {
    let path = directory.path().parent().unwrap().join(REGISTRATION_NAME);
    PrivateDirectory::ensure(path.parent().unwrap()).unwrap();
    let entry = crate::paths::private_file::PrivateFileEntry::open(&path).unwrap();
    if entry.is_missing().unwrap() {
        crate::paths::atomic_write_json(&path, &REGISTRATION_VALUE).unwrap();
    }
    entry
        .open_read(crate::paths::MAX_DURABLE_JSON_BYTES)
        .unwrap()
        .durable_version(&path, crate::paths::MAX_DURABLE_JSON_BYTES)
        .unwrap()
        .object_identity()
}

pub(super) fn binding() -> NamespaceBinding {
    NamespaceBinding {
        schema_version: super::super::SOCKET_BINDING_SCHEMA_VERSION,
        effective_user_id: unsafe { libc::geteuid() },
        home_digest: "a".repeat(super::super::SOCKET_FULL_HASH_HEX_LENGTH),
    }
}

pub(super) fn write_attribute(directory: &PrivateDirectory, bytes: &[u8]) {
    // Test-only corruption injection into this fixture's retained directory.
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            REPLACE_ATTRIBUTE,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            REPLACE_ATTRIBUTE,
        )
    };
    assert_eq!(
        result,
        libc::c_int::default(),
        "{}",
        std::io::Error::last_os_error()
    );
}
