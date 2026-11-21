use super::*;
use crate::paths::constants::{
    EXPECTED_FILE_LINK_COUNT, HOST_SOCKET_FILE_NAME, PRIVATE_DIRECTORY_MODE, PRIVATE_FILE_MODE,
    SOCKET_PROBE_LINK_COUNT, SOCKET_ROOT_PREFIX,
};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener as StdListener, UnixStream as StdStream};
use tempfile::{Builder, TempDir};

const BIND_EXTRA_NAME: &str = "bind-extra-link";
const ENDPOINT_EXTRA_NAME: &str = "endpoint-extra-link";
const PROBE_EXTRA_NAME: &str = "probe-extra-link";
const TEST_LIVE_ERROR: &str = "test endpoint is live";
const TEST_TEMP_PREFIX: &str = "ya-";
const TEST_TIMEOUT_MILLIS: u64 = 250;
const TEST_TIMEOUT: Duration = Duration::from_millis(TEST_TIMEOUT_MILLIS);
const PROBE_WITH_EXTRA_LINK_COUNT: u64 = SOCKET_PROBE_LINK_COUNT + EXPECTED_FILE_LINK_COUNT;

struct Fixture {
    _root: TempDir,
    directory: PrivateDirectory,
    endpoint: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let parent = Path::new(SOCKET_ROOT_PREFIX).parent().unwrap();
        let root = Builder::new()
            .prefix(TEST_TEMP_PREFIX)
            .tempdir_in(parent)
            .unwrap();
        fs::set_permissions(
            root.path(),
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        let directory = PrivateDirectory::open(root.path()).unwrap();
        let endpoint = directory.path().join(HOST_SOCKET_FILE_NAME);
        Self {
            _root: root,
            directory,
            endpoint,
        }
    }

    fn listen(&self, path: &Path) -> StdListener {
        let listener = StdListener::bind(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        listener
    }

    fn publish(
        &self,
    ) -> impl std::future::Future<Output = Result<(UnixListener, SocketPublication)>> {
        publish(
            self.directory.try_clone().unwrap(),
            &self.endpoint,
            TEST_TIMEOUT,
            TEST_LIVE_ERROR,
        )
    }

    fn stage_target(&self) -> socket_stage::Target {
        let name = CString::new(HOST_SOCKET_FILE_NAME).unwrap();
        socket_stage::Target::new(self.directory.path(), &name).unwrap()
    }
}

#[tokio::test]
async fn prelinked_endpoint_is_rejected_before_probe() {
    let fixture = Fixture::new();
    let live = fixture.listen(&fixture.endpoint);
    let extra = fixture.directory.path().join(ENDPOINT_EXTRA_NAME);
    fs::hard_link(&fixture.endpoint, &extra).unwrap();

    let error = fixture.publish().await.err().unwrap();

    assert!(format!("{error:#}").contains(PROBE_LINK_COUNT_ERROR));
    assert_same_links(&[&fixture.endpoint, &extra], SOCKET_PROBE_LINK_COUNT);
    assert!(StdStream::connect(&fixture.endpoint).is_ok());
    drop(live);
}

#[tokio::test]
async fn publish_cleanup_preserves_an_extra_linked_probe_residue() {
    let fixture = Fixture::new();
    let live = fixture.listen(&fixture.endpoint);
    let target = fixture.stage_target();
    let lease =
        socket_stage::Lease::acquire_immediate_for_test(&fixture.directory, &target).unwrap();
    let probe = target.stage(socket_stage::Kind::Probe).unwrap().1;
    let extra = fixture.directory.path().join(PROBE_EXTRA_NAME);
    fs::hard_link(&fixture.endpoint, &probe).unwrap();
    fs::hard_link(&fixture.endpoint, &extra).unwrap();
    fixture.directory.sync().unwrap();
    drop(lease);

    let error = fixture.publish().await.err().unwrap();

    assert!(format!("{error:#}").contains(STAGE_RESIDUE_UNPROVEN_ERROR));
    assert_same_links(
        &[&fixture.endpoint, &probe, &extra],
        PROBE_WITH_EXTRA_LINK_COUNT,
    );
    drop(live);
}

#[tokio::test]
async fn publish_cleanup_preserves_an_extra_linked_bind_residue() {
    let fixture = Fixture::new();
    let target = fixture.stage_target();
    let lease =
        socket_stage::Lease::acquire_immediate_for_test(&fixture.directory, &target).unwrap();
    let bind = target.stage(socket_stage::Kind::Bind).unwrap().1;
    let stale = fixture.listen(&bind);
    let extra = fixture.directory.path().join(BIND_EXTRA_NAME);
    fs::hard_link(&bind, &extra).unwrap();
    fixture.directory.sync().unwrap();
    drop((stale, lease));

    let error = fixture.publish().await.err().unwrap();

    assert!(format!("{error:#}").contains(STAGE_RESIDUE_UNPROVEN_ERROR));
    assert_same_links(&[&bind, &extra], SOCKET_PROBE_LINK_COUNT);
}

#[tokio::test]
async fn publication_drop_preserves_an_extra_linked_endpoint() {
    let fixture = Fixture::new();
    let (listener, publication) = fixture.publish().await.unwrap();
    let extra = fixture.directory.path().join(ENDPOINT_EXTRA_NAME);
    fs::hard_link(&fixture.endpoint, &extra).unwrap();

    drop(publication);

    assert_same_links(&[&fixture.endpoint, &extra], SOCKET_PROBE_LINK_COUNT);
    drop(listener);
}

#[tokio::test]
async fn publication_drop_preserves_a_hook_replacement() {
    let fixture = Fixture::new();
    let (listener, mut publication) = fixture.publish().await.unwrap();
    let original_inode = fs::symlink_metadata(&fixture.endpoint).unwrap().ino();
    let endpoint = fixture.endpoint.clone();
    publication.set_before_unlink(move || {
        fs::remove_file(&endpoint).unwrap();
        let replacement = StdListener::bind(&endpoint).unwrap();
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        drop(replacement);
    });

    drop(publication);

    let replacement = fs::symlink_metadata(&fixture.endpoint).unwrap();
    assert_ne!(replacement.ino(), original_inode);
    assert_eq!(replacement.nlink(), EXPECTED_FILE_LINK_COUNT);
    drop(listener);
}

fn assert_same_links(paths: &[&Path], expected_links: u64) {
    let inode = fs::symlink_metadata(paths[0]).unwrap().ino();
    for path in paths {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert_eq!(metadata.ino(), inode);
        assert_eq!(metadata.nlink(), expected_links);
    }
}
