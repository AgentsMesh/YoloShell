use super::*;
use std::os::unix::fs::PermissionsExt;

pub(super) struct Fixture {
    pub(super) namespace: SocketNamespace,
    pub(super) directory: PrivateDirectory,
    pub(super) isolation: PrivateFileEntry,
    _root: tempfile::TempDir,
    _home: tempfile::TempDir,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let root = tempfile::tempdir_in(parent).unwrap();
        fs::set_permissions(
            root.path(),
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        let home = tempfile::tempdir().unwrap();
        let paths = Paths::new_with_socket_root(home.path(), root.path().to_path_buf());
        let namespace = paths.retain_socket_namespace().unwrap();
        let directory = namespace.directory().unwrap();
        let target = socket_stage::Target::new(
            directory.path(),
            &CString::new(HOST_SOCKET_FILE_NAME).unwrap(),
        )
        .unwrap();
        let lease = socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap();
        let source = PrivateFileEntry::from_directory(
            directory.try_clone().unwrap(),
            &target.lease().unwrap().1,
        )
        .unwrap();
        let isolation = source.unlink_isolation_entry().unwrap();
        fs::rename(source.path(), isolation.path()).unwrap();
        drop(lease);
        Self {
            namespace,
            directory,
            isolation,
            _root: root,
            _home: home,
        }
    }
}
