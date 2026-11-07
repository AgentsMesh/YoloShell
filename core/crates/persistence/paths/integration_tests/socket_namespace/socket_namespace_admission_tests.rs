use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};

#[path = "socket_namespace_admission_adversarial_tests.rs"]
pub(super) mod adversarial;
use yolo_maintenance_contract::MaintenanceScanner;

const REGISTRATION_FILE: &str = ".socket-namespace.json";
const UNKNOWN_FILE: &str = "user-data";
const PRESERVED_BYTES: &[u8] = b"keep this data";
const SHORT_ROOT_PREFIX: &str = "ya-";

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Paths) {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::Builder::new()
        .prefix(SHORT_ROOT_PREFIX)
        .tempdir_in(Path::new(SOCKET_ROOT_PREFIX).parent().unwrap())
        .unwrap();
    fs::set_permissions(
        root.path(),
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let paths = Paths::new_with_socket_root(home.path(), root.path().to_owned());
    (home, root, paths)
}

fn stale_socket(path: &Path) {
    let listener = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    drop(listener);
}

#[test]
fn registered_live_owners_need_no_temporary_ownership_files() {
    let (_home, _root, paths) = fixture();
    let owner = paths.retain_socket_namespace().unwrap();
    let endpoint = paths.host_socket().unwrap();
    let listener = UnixListener::bind(&endpoint).unwrap();
    fs::set_permissions(&endpoint, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let inode = fs::metadata(owner.directory.path()).unwrap().ino();
    owner.verify().unwrap();
    let peer = paths.retain_socket_namespace().unwrap();
    assert_eq!(inode, fs::metadata(peer.directory.path()).unwrap().ino());
    assert!(UnixStream::connect(paths.host_socket().unwrap()).is_ok());
    let mut scanner = peer.scanner().unwrap();
    while scanner.scan_chunk().unwrap() != yolo_maintenance_contract::ScanChunk::PassComplete {}
    owner.verify().unwrap();
    drop(listener);
}

#[test]
fn stale_unbound_directory_is_preserved_and_new_namespace_is_certified() {
    let (_home, root, paths) = fixture();
    let location =
        SocketNamespaceLocation::locate(paths.home(), paths.socket_root_override()).unwrap();
    let directory = PrivateDirectory::ensure(&location.directory).unwrap();
    let endpoint = location.host_socket().unwrap();
    stale_socket(&endpoint);
    let old_inode = fs::metadata(&location.directory).unwrap().ino();
    let socket_inode = fs::metadata(&endpoint).unwrap().ino();
    assert!(paths.host_socket().is_err()); // Lookup does not mutate or repair.
    let owner = paths.retain_socket_namespace().unwrap();
    assert_ne!(
        old_inode,
        fs::metadata(owner.directory.path()).unwrap().ino()
    );
    assert!(
        directory_certificate::read(&owner.directory)
            .unwrap()
            .is_some()
    );
    let preserved = fs::read_dir(root.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|path| fs::metadata(path).is_ok_and(|m| m.ino() == old_inode))
        .unwrap();
    assert_eq!(
        socket_inode,
        fs::metadata(preserved.join(HOST_SOCKET_FILE_NAME))
            .unwrap()
            .ino()
    );
    assert!(directory.verify_path().is_err());
}

#[test]
fn live_or_unknown_unbound_state_is_never_moved() {
    for live in [true, false] {
        let (_home, _root, paths) = fixture();
        let location =
            SocketNamespaceLocation::locate(paths.home(), paths.socket_root_override()).unwrap();
        let directory = PrivateDirectory::ensure(&location.directory).unwrap();
        let listener = live.then(|| {
            let path = location.host_socket().unwrap();
            let listener = UnixListener::bind(&path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
            listener
        });
        if !live {
            fs::write(directory.path().join(UNKNOWN_FILE), PRESERVED_BYTES).unwrap();
        }
        assert!(paths.retain_socket_namespace().is_err());
        directory.verify_path().unwrap();
        if live {
            assert!(UnixStream::connect(location.host_socket().unwrap()).is_ok());
        } else {
            assert_eq!(
                fs::read(directory.path().join(UNKNOWN_FILE)).unwrap(),
                PRESERVED_BYTES
            );
        }
        drop(listener);
    }
}

#[test]
fn lost_registration_is_not_recreated_for_a_certified_directory() {
    let (home, _root, paths) = fixture();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory.try_clone().unwrap();
    fs::remove_file(home.path().join(REGISTRATION_FILE)).unwrap();
    assert!(owner.verify().is_err());
    assert!(paths.host_socket().is_err());
    assert!(paths.retain_socket_namespace().is_err());
    assert!(!home.path().join(REGISTRATION_FILE).exists());
    directory.verify_path().unwrap();
    owner.preserve_on_release();
}
