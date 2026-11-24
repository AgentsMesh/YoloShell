use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use tempfile::{Builder, TempDir};

const TEST_TEMP_PREFIX: &str = "yp-";

#[test]
fn crash_left_probe_hardlink_is_reclaimed_from_its_committed_target() {
    let fixture = ProbeFixture::new();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Mutated));
    fixture.assert_probe_reclaimed();
}

#[test]
fn crash_left_probe_quarantine_is_reclaimed_on_the_next_pass() {
    let fixture = ProbeFixture::new();
    fs::rename(&fixture.probe, &fixture.quarantine).unwrap();
    fixture.directory.sync().unwrap();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Mutated));
    fixture.assert_probe_reclaimed();
}

#[test]
fn single_link_probe_stage_converges_after_the_target_disappears() {
    let fixture = ProbeFixture::new();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Mutated));
    assert!(!fixture.probe.exists());
    assert!(!fixture.quarantine.exists());
    assert!(!fixture.lease.exists());
}

#[test]
fn single_link_quarantine_converges_after_the_target_disappears() {
    let fixture = ProbeFixture::new();
    fs::rename(&fixture.probe, &fixture.quarantine).unwrap();
    fixture.directory.sync().unwrap();
    fs::remove_file(&fixture.endpoint).unwrap();
    fixture.directory.sync().unwrap();

    let cleanup = socket_stage::cleanup_key(&fixture.directory, fixture.target.key()).unwrap();

    assert!(matches!(cleanup, socket_stage::Cleanup::Mutated));
    assert!(!fixture.quarantine.exists());
    assert!(!fixture.lease.exists());
    assert!(matches!(
        socket_stage::cleanup_lifecycle(&fixture.directory).unwrap(),
        socket_stage::Cleanup::Mutated
    ));
}

pub(super) struct ProbeFixture {
    _temporary: TempDir,
    pub(super) directory: PrivateDirectory,
    pub(super) target: socket_stage::Target,
    pub(super) endpoint: PathBuf,
    pub(super) probe: PathBuf,
    pub(super) quarantine: PathBuf,
    pub(super) lease: PathBuf,
    pub(super) lifecycle: PathBuf,
}

impl ProbeFixture {
    pub(super) fn new() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let temporary = Builder::new()
            .prefix(TEST_TEMP_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        let directory = PrivateDirectory::ensure(temporary.path()).unwrap();
        let endpoint = directory.path().join(HOST_SOCKET_FILE_NAME);
        let target_name = std::ffi::CString::new(HOST_SOCKET_FILE_NAME).unwrap();
        let target = socket_stage::Target::new(directory.path(), &target_name).unwrap();
        let lease_guard =
            socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap();
        let probe = target.stage(socket_stage::Kind::Probe).unwrap().1;
        let quarantine = target.probe_quarantine().unwrap().1;
        let lease = target.lease().unwrap().1;
        let lifecycle = target.lifecycle_path();
        create_private_socket(&endpoint);
        fs::hard_link(&endpoint, &probe).unwrap();
        drop(lease_guard);
        Self {
            _temporary: temporary,
            directory,
            target,
            endpoint,
            probe,
            quarantine,
            lease,
            lifecycle,
        }
    }

    pub(super) fn assert_probe_reclaimed(&self) {
        assert!(!self.probe.exists());
        assert!(!self.quarantine.exists());
        assert!(!self.lease.exists());
        assert_eq!(
            fs::symlink_metadata(&self.endpoint).unwrap().nlink(),
            SOCKET_ENDPOINT_LINK_COUNT
        );
        assert!(matches!(
            socket_stage::cleanup_lifecycle(&self.directory).unwrap(),
            socket_stage::Cleanup::Mutated
        ));
        assert!(!self.lifecycle.exists());
    }
}

pub(super) fn create_private_socket(path: &Path) -> u64 {
    let listener = UnixListener::bind(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let inode = fs::symlink_metadata(path).unwrap().ino();
    drop(listener);
    inode
}
