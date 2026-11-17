use super::*;
use crate::host::{Host, HostConfig};
use std::os::unix::{fs::PermissionsExt, net::UnixListener};
use std::panic::{AssertUnwindSafe, catch_unwind};

const UNWIND_DETAIL: &str = "injected Host fixture unwind";

#[test]
fn host_fixture_directory_releases_stale_transport_before_its_home() {
    let (_owner, directory, host) = fixture();
    let home = host.paths.home().to_path_buf();
    let namespace = host.paths.socket_dir().unwrap();
    let socket = host
        .paths
        .custody_guardian_socket(uuid::Uuid::new_v4())
        .unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(
        &socket,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    drop(listener);
    drop(host);

    drop(directory);

    assert!(!socket.exists());
    assert!(!namespace.exists());
    assert!(!home.exists());
}

#[test]
fn host_fixture_directory_preserves_evidence_during_unwind() {
    let (_owner, directory, host) = fixture();
    let paths = host.paths.clone();
    let namespace = paths.socket_dir().unwrap();
    drop(host);

    let failed = catch_unwind(AssertUnwindSafe(move || {
        let _directory = directory;
        panic!("{UNWIND_DETAIL}");
    }));

    assert!(failed.is_err());
    assert!(paths.home().is_dir());
    assert!(namespace.is_dir());
    assert_eq!(
        paths.retain_socket_namespace().unwrap().release().unwrap(),
        NamespaceRelease::Removed
    );
}

#[test]
fn host_fixture_directory_preserves_home_when_another_owner_prevents_release() {
    let (_owner, directory, host) = fixture();
    let home = host.paths.home().to_path_buf();
    let namespace = host.paths.socket_dir().unwrap();
    let retained = host.paths.retain_socket_namespace().unwrap();
    drop(host);

    let failed = catch_unwind(AssertUnwindSafe(move || drop(directory)));

    assert!(failed.is_err());
    assert!(home.is_dir());
    assert!(namespace.is_dir());
    assert_eq!(retained.release().unwrap(), NamespaceRelease::Removed);
}

fn fixture() -> (tempfile::TempDir, HostTestDirectory, Host) {
    let owner = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir_in(owner.path()).unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let directory = HostTestDirectory::capture(directory, &host.paths);
    (owner, directory, host)
}
