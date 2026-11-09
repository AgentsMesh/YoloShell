use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_runtime::paths::{Paths, SOCKET_ROOT_ENVIRONMENT};

const UNIX_SOCKET_PATH_MAX_BYTES: usize = 100;
const LONG_ROOT_COMPONENT: &str = "root-component-too-long-for-a-local-unix-socket-endpoint";
const HOME_DIRECTORY: &str = "home";
const TEST_ROOT_PREFIX: &str = "yn-";
const MAXIMUM_ROOT_TOKEN: &str = "12345678";
const MAXIMUM_ROOT_PARENT: &str = "/tmp";
const RELATIVE_ROOT: &str = "relative";

#[test]
fn explicit_override_routes_host_and_worker_endpoints() {
    let fixture = Fixture::new();
    let paths = Paths::new_with_socket_root(&fixture.home, fixture.socket_root.clone());
    assert_endpoints(&paths, &fixture.socket_root);
}

#[test]
fn environment_override_is_captured_once_per_paths_value() {
    let fixture = Fixture::new();
    unsafe { std::env::set_var(SOCKET_ROOT_ENVIRONMENT, &fixture.socket_root) };
    let paths = Paths::new(&fixture.home);
    unsafe { std::env::remove_var(SOCKET_ROOT_ENVIRONMENT) };
    assert_endpoints(&paths, &fixture.socket_root);
}

#[test]
fn relative_and_empty_environment_overrides_fail_closed() {
    let fixture = Fixture::new();
    for root in [PathBuf::from(RELATIVE_ROOT), PathBuf::new()] {
        let paths = Paths::new_with_socket_root(&fixture.home, root);
        assert!(paths.host_socket().is_err());
    }
}

#[test]
fn override_rejects_endpoints_beyond_the_unix_path_limit() {
    let fixture = Fixture::new();
    let root = fixture
        .socket_root
        .join(LONG_ROOT_COMPONENT)
        .join(LONG_ROOT_COMPONENT);
    assert!(
        Paths::new_with_socket_root(&fixture.home, root)
            .worker_socket(Uuid::new_v4())
            .is_err()
    );
}

#[test]
fn maximum_native_test_root_leaves_room_for_worker_endpoints() {
    let fixture = Fixture::new();
    let root = std::fs::canonicalize(MAXIMUM_ROOT_PARENT)
        .unwrap()
        .join(format!("{TEST_ROOT_PREFIX}{MAXIMUM_ROOT_TOKEN}"));
    let paths = Paths::new_with_socket_root(&fixture.home, root.clone());
    assert_endpoints(&paths, &root);
}

fn assert_endpoints(paths: &Paths, root: &Path) {
    let host = paths.host_socket().unwrap();
    let worker = paths.worker_socket(Uuid::new_v4()).unwrap();
    for endpoint in [host, worker] {
        assert!(endpoint.starts_with(root));
        assert!(endpoint.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES);
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    home: PathBuf,
    socket_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix(TEST_ROOT_PREFIX)
            .tempdir_in(MAXIMUM_ROOT_PARENT)
            .unwrap();
        let home = directory.path().join(HOME_DIRECTORY);
        let socket_root = std::fs::canonicalize(directory.path()).unwrap();
        std::fs::create_dir(&home).unwrap();
        Self {
            _directory: directory,
            home,
            socket_root,
        }
    }
}
