use super::*;
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use tempfile::{Builder, TempDir};

const TEST_SHORT_SOCKET_ROOT_PREFIX: &str = "ys-";
const SOCKET_ROOT_PARENT_REQUIRED: &str = "socket root prefix must have a parent";

pub(super) struct RegisteredFixture {
    _home: TempDir,
    _root: TempDir,
    pub(super) paths: Paths,
}

impl RegisteredFixture {
    pub(super) fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = short_socket_root();
        let paths = Paths::new_with_socket_root(home.path(), root.path().to_path_buf());
        Self {
            _home: home,
            _root: root,
            paths,
        }
    }
}

pub(super) fn short_socket_root() -> TempDir {
    let parent = Path::new(SOCKET_ROOT_PREFIX)
        .parent()
        .expect(SOCKET_ROOT_PARENT_REQUIRED);
    let directory = Builder::new()
        .prefix(TEST_SHORT_SOCKET_ROOT_PREFIX)
        .tempdir_in(parent)
        .unwrap();
    fs::set_permissions(
        directory.path(),
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    directory
}

pub(super) fn inode(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().ino()
}

pub(super) fn create_crash_stage(directory: &Path, kind: socket_stage::Kind, mode: u32) -> PathBuf {
    let parent = PrivateDirectory::open(directory).unwrap();
    let target_name = std::ffi::CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let target = socket_stage::Target::new(directory, &target_name).unwrap();
    let lease = socket_stage::Lease::acquire_immediate_for_test(&parent, &target).unwrap();
    let path = target.stage(kind).unwrap().1;
    let listener = match kind {
        socket_stage::Kind::Bind => UnixListener::bind(&path).unwrap(),
        socket_stage::Kind::Probe => {
            let endpoint = directory.join(HOST_SOCKET_FILE_NAME);
            let listener = UnixListener::bind(&endpoint).unwrap();
            fs::set_permissions(&endpoint, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
            fs::hard_link(endpoint, &path).unwrap();
            listener
        }
    };
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    assert!(fs::symlink_metadata(&path).unwrap().file_type().is_socket());
    drop(listener);
    drop(lease);
    path
}
