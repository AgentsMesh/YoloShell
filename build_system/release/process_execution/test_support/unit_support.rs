use crate::ProcessCommand;
use std::ffi::CStr;
use std::os::unix::fs::PermissionsExt;
use yolo_release_filesystem::DirectoryHandle;

pub const EXECUTABLE: &str = "/usr/bin/true";
pub const DESCRIPTION: &str = "process execution unit fixture";
pub const INVALID_DESCRIPTOR: libc::c_int = -1;
pub const FIRST_ARGUMENT: &str = "first argument";
pub const SECOND_ARGUMENT: &str = "second argument";
pub const NUL_ARGUMENT: &str = "invalid\0argument";
pub const FAILURE_DETAIL: &str = "injected operation failure";
pub const FIRST_CONTEXT: &str = "first cleanup operation";
pub const SECOND_CONTEXT: &str = "second cleanup operation";
pub const EMPTY_WAIT_STATUS: libc::c_int = 0;
const PRIVATE_MODE: u32 = 0o700;

pub struct Fixture {
    _directory: tempfile::TempDir,
    directory: DirectoryHandle,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(PRIVATE_MODE))
            .unwrap();
        Self {
            directory: DirectoryHandle::open(root.path()).unwrap(),
            _directory: root,
        }
    }

    pub fn command(&self) -> ProcessCommand {
        ProcessCommand::new(
            EXECUTABLE,
            Vec::new(),
            &self.directory,
            &self.directory,
            &self.directory,
            DESCRIPTION,
        )
    }
}

pub fn decode_vector(vector: &crate::posix_spawn_strings::CStringVector) -> Vec<String> {
    assert!(vector.pointers.last().unwrap().is_null());
    vector
        .pointers
        .iter()
        .take_while(|pointer| !pointer.is_null())
        .map(|pointer| {
            unsafe { CStr::from_ptr(*pointer) }
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}
