use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

const UNSAFE_DIRECTORY_MODE: u32 = 0o755;
const LONG_ROOT_COMPONENT: &str = "socket-root-component-that-cannot-fit-a-unix-endpoint";
const LONG_ROOT_COMPONENT_COUNT: usize = 2;
const RELATIVE_ROOT: &str = "relative";
const HOME_DIRECTORY: &str = "home";
const SOCKET_ROOT_DIRECTORY: &str = "socket-root";
const DISPLACED_ROOT_DIRECTORY: &str = "displaced-root";
const REPLACEMENT_MARKER_FILE: &str = "replacement-marker";
const REPLACEMENT_MARKER_BYTES: &[u8] = b"replacement must remain untouched";
const SHORT_FIXTURE_PREFIX: &str = "yo-";

#[test]
fn override_routes_every_endpoint_below_the_selected_root() {
    let fixture = OverrideFixture::new_short();
    let paths = fixture.paths();
    let host = paths.host_socket().unwrap();
    let worker = paths.worker_socket(Uuid::new_v4()).unwrap();

    assert!(host.starts_with(&fixture.socket_root));
    assert!(worker.starts_with(&fixture.socket_root));
    let namespace = paths.retain_socket_namespace().unwrap();
    assert_eq!(namespace.root_path(), fixture.socket_root);
    run_scanner_pass(&namespace);
}

#[test]
fn missing_override_fails_closed_without_creating_it() {
    let fixture = OverrideFixture::without_root();
    let paths = fixture.paths();
    assert!(paths.retain_socket_namespace().is_err());
    assert!(!fixture.socket_root.exists());
}

#[test]
fn unsafe_override_is_rejected_without_normalizing_permissions() {
    let fixture = OverrideFixture::new();
    fs::set_permissions(
        &fixture.socket_root,
        fs::Permissions::from_mode(UNSAFE_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(fixture.paths().retain_socket_namespace().is_err());
    assert_eq!(
        fs::metadata(&fixture.socket_root)
            .unwrap()
            .permissions()
            .mode()
            & u32::from(PERMISSION_MODE_MASK),
        UNSAFE_DIRECTORY_MODE
    );
}

#[test]
fn symlink_override_never_reaches_its_target() {
    let fixture = OverrideFixture::without_root();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), &fixture.socket_root).unwrap();
    assert!(fixture.paths().retain_socket_namespace().is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

#[test]
fn relative_and_overlong_overrides_fail_before_use() {
    let home = tempfile::tempdir().unwrap();
    assert!(
        Paths::new_with_socket_root(home.path(), PathBuf::from(RELATIVE_ROOT))
            .host_socket()
            .is_err()
    );
    let mut root = home.path().to_path_buf();
    for _ in 0..LONG_ROOT_COMPONENT_COUNT {
        root.push(LONG_ROOT_COMPONENT);
    }
    assert!(
        Paths::new_with_socket_root(home.path(), root)
            .host_socket()
            .is_err()
    );
}

#[test]
fn socket_isolation_scanner_reset_rejects_root_replacement() {
    let fixture = OverrideFixture::new();
    let root = PrivateDirectory::open(&fixture.socket_root).unwrap();
    let mut scanner = SocketNamespaceScanner::from_root(&root, true).unwrap();
    scanner.scan_chunk().unwrap();
    scanner.reset();

    let displaced = fixture._directory.path().join(DISPLACED_ROOT_DIRECTORY);
    fs::rename(&fixture.socket_root, &displaced).unwrap();
    fs::create_dir(&fixture.socket_root).unwrap();
    fs::set_permissions(
        &fixture.socket_root,
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let marker = fixture.socket_root.join(REPLACEMENT_MARKER_FILE);
    fs::write(&marker, REPLACEMENT_MARKER_BYTES).unwrap();

    assert!(scanner.scan_chunk().is_err());
    scanner.reset();
    assert!(scanner.scan_chunk().is_err());
    assert_eq!(fs::read(marker).unwrap(), REPLACEMENT_MARKER_BYTES);
    assert!(displaced.is_dir());
}

struct OverrideFixture {
    _directory: tempfile::TempDir,
    home: PathBuf,
    socket_root: PathBuf,
}

impl OverrideFixture {
    fn new_short() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let directory = tempfile::Builder::new()
            .prefix(SHORT_FIXTURE_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        fs::set_permissions(
            directory.path(),
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        let home = directory.path().join(HOME_DIRECTORY);
        let socket_root = directory.path().to_path_buf();
        fs::create_dir(&home).unwrap();
        Self {
            _directory: directory,
            home,
            socket_root,
        }
    }

    fn new() -> Self {
        Self::create(tempfile::tempdir().unwrap())
    }

    fn create(directory: tempfile::TempDir) -> Self {
        let home = directory.path().join(HOME_DIRECTORY);
        let socket_root = directory.path().join(SOCKET_ROOT_DIRECTORY);
        fs::create_dir(&home).unwrap();
        let fixture = Self {
            _directory: directory,
            home,
            socket_root,
        };
        fs::create_dir(&fixture.socket_root).unwrap();
        fs::set_permissions(
            &fixture.socket_root,
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        fixture
    }

    fn without_root() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join(HOME_DIRECTORY);
        let socket_root = directory.path().join(SOCKET_ROOT_DIRECTORY);
        fs::create_dir(&home).unwrap();
        Self {
            _directory: directory,
            home,
            socket_root,
        }
    }

    fn paths(&self) -> Paths {
        Paths::new_with_socket_root(&self.home, self.socket_root.clone())
    }
}
