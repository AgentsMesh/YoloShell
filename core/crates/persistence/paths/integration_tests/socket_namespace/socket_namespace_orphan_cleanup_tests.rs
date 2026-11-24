use super::{test_support::short_socket_root, *};
use std::{
    fs,
    os::unix::{fs::PermissionsExt, net::UnixListener},
};

const REGISTRATION_FILE: &str = ".socket-namespace.json";
const UNKNOWN_ENTRY: &str = "unknown-data";
const CONTENT: &[u8] = b"preserve";
const PUBLICATION_DETAIL: &str = "orphan bookkeeping fixture endpoint is already live";

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Paths) {
    let home = tempfile::tempdir().unwrap();
    let root = short_socket_root();
    let paths = Paths::new_with_socket_root(home.path(), root.path().to_path_buf());
    (home, root, paths)
}

#[test]
fn abandoned_certified_empty_and_stage_only_directories_are_reclaimed() {
    for with_stage in [false, true] {
        let (_home, _root, paths) = fixture();
        let mut owner = paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let mut scanner = owner.scanner().unwrap();
        if with_stage {
            let name = crate::paths::directory::entry_c_string(std::ffi::OsStr::new(
                HOST_SOCKET_FILE_NAME,
            ))
            .unwrap();
            let target = socket_stage::Target::new(directory.path(), &name).unwrap();
            drop(socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap());
        }
        owner.preserve_on_release();
        drop(owner);
        assert!(directory.path().exists());
        finish_scanner_pass(&mut scanner);
        assert!(!directory.path().exists());
    }
}

#[test]
fn live_directory_lease_fences_scanning_after_registration_or_home_loss() {
    for remove_home in [false, true] {
        let (home, _root, paths) = fixture();
        let owner = paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let mut scanner = owner.scanner().unwrap();
        let retained_home = if remove_home {
            home.close().unwrap();
            None
        } else {
            fs::remove_file(home.path().join(REGISTRATION_FILE)).unwrap();
            Some(home)
        };
        assert!(owner.verify().is_err());
        finish_scanner_pass(&mut scanner);
        directory.verify_path().unwrap();
        drop(owner);
        finish_scanner_pass(&mut scanner);
        assert!(!directory.path().exists());
        drop(retained_home);
    }
}

#[test]
fn scanner_does_not_remove_any_endpoint_or_unknown_data_after_participants_exit() {
    for endpoint in [false, true] {
        let (_home, _root, paths) = fixture();
        let mut owner = paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let mut scanner = owner.scanner().unwrap();
        let path = directory.path().join(if endpoint {
            HOST_SOCKET_FILE_NAME
        } else {
            UNKNOWN_ENTRY
        });
        if endpoint {
            drop(UnixListener::bind(&path).unwrap());
        } else {
            fs::write(&path, CONTENT).unwrap();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        owner.preserve_on_release();
        drop(owner);
        finish_scanner_pass(&mut scanner);
        assert!(path.exists());
        directory.verify_path().unwrap();
        if !endpoint {
            assert_eq!(fs::read(path).unwrap(), CONTENT);
        }
    }
}

#[tokio::test]
async fn admission_reclaims_withdrawn_publication_bookkeeping_only_after_its_lease_releases() {
    for active_lease in [false, true] {
        let (_home, _root, paths) = fixture();
        let mut owner = paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let endpoint = owner.host_socket().unwrap();
        let (listener, publication) = crate::paths::socket_activation::publish(
            directory.try_clone().unwrap(),
            &endpoint,
            SOCKET_STAGE_LEASE_TIMEOUT,
            PUBLICATION_DETAIL,
        )
        .await
        .unwrap();
        drop(listener);
        publication.withdraw().unwrap();
        let name =
            crate::paths::directory::entry_c_string(std::ffi::OsStr::new(HOST_SOCKET_FILE_NAME))
                .unwrap();
        let target = socket_stage::Target::new(directory.path(), &name).unwrap();
        assert!(!endpoint.exists());
        assert!(target.lease().unwrap().1.exists());
        assert!(target.lifecycle_path().exists());
        let stage_owner = active_lease
            .then(|| socket_stage::Lease::acquire_immediate_for_test(&directory, &target).unwrap());
        owner.preserve_on_release();
        drop(owner);
        if active_lease {
            let observer = paths.retain_socket_namespace().unwrap();
            assert!(observer.directory().unwrap().same_identity(&directory));
            assert_eq!(observer.release().unwrap(), NamespaceRelease::Preserved);
            assert!(target.lease().unwrap().1.exists());
        }
        drop(stage_owner);
        let replacement = paths.retain_socket_namespace().unwrap();
        assert!(!replacement.directory().unwrap().same_identity(&directory));
        assert_eq!(replacement.release().unwrap(), NamespaceRelease::Removed);
    }
}
