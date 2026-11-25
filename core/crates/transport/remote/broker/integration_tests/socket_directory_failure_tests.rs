use super::*;
use crate::paths::directory::PrivateDirectory;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const SOCKET_NAME: &str = "immutable.sock";
const UNSAFE_SOCKET_MODE: u32 = 0o700;
const CLEAR_FLAGS: libc::c_uint = 0;
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;

#[test]
fn immutable_socket_rejects_permission_repair_without_replacement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(STATE_DIRECTORY);
    let directory = SocketDirectory::new(PrivateDirectory::ensure(&path).unwrap());
    let name = CString::new(SOCKET_NAME).unwrap();
    let socket_path = path.join(SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let identity = directory.identity(&name).unwrap().unwrap();
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(UNSAFE_SOCKET_MODE)).unwrap();
    let immutable = UserImmutable::new(&socket_path);

    assert!(directory.set_permissions(&name, identity).is_err());
    assert_eq!(
        bound_socket_identity_at(directory.descriptor(), &name).unwrap(),
        Some(identity)
    );
    drop(immutable);
    drop(listener);
}

struct UserImmutable(PathBuf);

impl UserImmutable {
    fn new(path: &Path) -> Self {
        set_flags(path, libc::UF_IMMUTABLE);
        Self(path.to_owned())
    }
}

impl Drop for UserImmutable {
    fn drop(&mut self) {
        set_flags(&self.0, CLEAR_FLAGS);
    }
}

fn set_flags(path: &Path, flags: libc::c_uint) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::chflags(path.as_ptr(), flags) },
        SYSTEM_CALL_SUCCESS
    );
}
