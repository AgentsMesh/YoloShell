use super::*;
use std::os::unix::fs::PermissionsExt;
use std::{fs, os::unix::net::UnixListener};

const HOME: &str = "home";
const UNKNOWN_ENTRY: &str = "user-owned";
const CONTENT: &[u8] = b"preserve";

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Paths) {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(ISOLATED_TEST_SOCKET_PARENT).unwrap();
    let home_path = home.path().join(HOME);
    PrivateDirectory::ensure(&home_path).unwrap();
    let root_path = root.path().canonicalize().unwrap();
    PrivateDirectory::ensure(&root_path).unwrap();
    let paths = Paths::new_with_socket_root(home_path, root_path);
    (home, root, paths)
}

#[test]
fn empty_registered_directory_is_removed_only_when_last_shared_lease_leaves() {
    let (_home, _root, paths) = fixture();
    let first = paths.retain_socket_namespace().unwrap();
    let second = paths.retain_socket_namespace().unwrap();
    let directory = first.directory().unwrap().path().to_path_buf();
    assert_eq!(first.release().unwrap(), NamespaceRelease::Busy);
    assert!(directory.exists());
    second.verify().unwrap();
    assert_eq!(second.release().unwrap(), NamespaceRelease::Removed);
    assert!(!directory.exists());
    let reopened = paths.retain_socket_namespace().unwrap();
    reopened.verify().unwrap();
    drop(reopened);
    assert!(!directory.exists());
}

#[tokio::test]
async fn own_endpoint_withdrawal_allows_bounded_stage_cleanup_and_empty_removal() {
    let (_home, _root, paths) = fixture();
    let namespace = paths.retain_socket_namespace().unwrap();
    let directory = namespace.directory().unwrap();
    let path = namespace.worker_socket(Uuid::new_v4()).unwrap();
    let (listener, publication) = crate::paths::socket_activation::publish(
        directory.try_clone().unwrap(),
        &path,
        SOCKET_STAGE_LEASE_TIMEOUT,
        UNKNOWN_ENTRY,
    )
    .await
    .unwrap();
    drop(listener);
    publication.withdraw().unwrap();
    drop(namespace);
    assert!(!directory.path().exists());
}

#[test]
fn endpoint_and_unknown_entries_are_preserved_even_after_the_last_owner_leaves() {
    for endpoint in [false, true] {
        let (_home, _root, paths) = fixture();
        let namespace = paths.retain_socket_namespace().unwrap();
        let directory = namespace.directory().unwrap();
        let path = if endpoint {
            namespace.worker_socket(Uuid::new_v4()).unwrap()
        } else {
            directory.path().join(UNKNOWN_ENTRY)
        };
        let listener = if endpoint {
            Some(UnixListener::bind(&path).unwrap())
        } else {
            fs::write(&path, CONTENT).unwrap();
            None
        };
        drop(namespace);
        assert!(directory.path().exists());
        assert!(path.exists());
        if !endpoint {
            assert_eq!(fs::read(path).unwrap(), CONTENT);
        }
        drop(listener);
    }
}

#[test]
fn missing_certificate_never_authorizes_removing_even_an_empty_directory() {
    let (_home, _root, paths) = fixture();
    let namespace = paths.retain_socket_namespace().unwrap();
    let directory = namespace.directory().unwrap();
    let name = c"user.yolo.namespace";
    #[cfg(target_os = "macos")]
    let removed = unsafe {
        libc::fremovexattr(
            directory.descriptor(),
            name.as_ptr(),
            libc::c_int::default(),
        )
    };
    #[cfg(target_os = "linux")]
    let removed = unsafe { libc::fremovexattr(directory.descriptor(), name.as_ptr()) };
    assert_eq!(removed, libc::c_int::default());
    drop(namespace);
    assert!(directory.path().exists());
}

#[test]
fn last_registered_owner_removes_only_an_entirely_proven_stale_socket_inventory() {
    let (_home, _root, paths) = fixture();
    let namespace = paths.retain_socket_namespace().unwrap();
    let directory = namespace.directory().unwrap();
    for socket in [
        namespace.host_socket().unwrap(),
        namespace.worker_socket(Uuid::new_v4()).unwrap(),
    ] {
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        drop(listener);
    }
    drop(namespace);
    assert!(!directory.path().exists());
}

#[test]
fn one_live_or_unknown_entry_prevents_deleting_any_stale_endpoint() {
    for live in [true, false] {
        let (_home, _root, paths) = fixture();
        let namespace = paths.retain_socket_namespace().unwrap();
        let directory = namespace.directory().unwrap();
        let stale = namespace.host_socket().unwrap();
        let listener = UnixListener::bind(&stale).unwrap();
        fs::set_permissions(&stale, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        drop(listener);
        let other = if live {
            namespace.worker_socket(Uuid::new_v4()).unwrap()
        } else {
            directory.path().join(UNKNOWN_ENTRY)
        };
        let listener = if live {
            let listener = UnixListener::bind(&other).unwrap();
            fs::set_permissions(&other, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
            Some(listener)
        } else {
            fs::write(&other, CONTENT).unwrap();
            None
        };
        drop(namespace);
        assert!(stale.exists());
        assert!(other.exists());
        assert!(directory.path().exists());
        drop(listener);
    }
}
