use super::{fixture::*, *};
use crate::paths::{Paths, socket_namespace::root_visit::StageAuthority};

const REGISTRATION_RECORD: &str = ".socket-namespace.json";
const COPIED_REGISTRATION: &str = "registration-copy";

fn owners() -> (tempfile::TempDir, tempfile::TempDir, Paths) {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir_in(crate::paths::constants::ISOLATED_TEST_SOCKET_PARENT).unwrap();
    PrivateDirectory::ensure(home.path()).unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let paths = Paths::new_with_socket_root(home.path(), root.path().canonicalize().unwrap());
    (home, root, paths)
}

#[test]
fn preserved_custom_namespace_reopens_with_the_same_durable_home_registration() {
    let (_home, _root, paths) = owners();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let certificate = read(&directory).unwrap().unwrap();
    let registration = paths.home().join(REGISTRATION_RECORD);
    let bytes = std::fs::read(&registration).unwrap();
    owner.preserve_on_release();
    drop(owner);

    assert!(paths.host_socket().is_ok());
    assert_eq!(read(&directory).unwrap().unwrap(), certificate);
    let reopened = paths.retain_socket_namespace().unwrap();
    reopened.verify().unwrap();
    assert_eq!(
        read(&reopened.directory().unwrap())
            .unwrap()
            .unwrap()
            .registration,
        certificate.registration
    );
    assert_eq!(std::fs::read(registration).unwrap(), bytes);
}

#[test]
fn retained_namespace_and_stage_authority_reject_changed_current_certificate() {
    let (_home, _root, paths) = owners();
    let owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let certificate = read(&directory).unwrap().unwrap();
    let stage = StageAuthority::capture(&directory, directory.path().file_name().unwrap())
        .unwrap()
        .unwrap();
    let mut changed = certificate.clone();
    changed.registration = registration(&directory);
    assert_ne!(changed.registration, certificate.registration);
    write_attribute(&directory, &serde_json::to_vec(&changed).unwrap());

    assert!(owner.verify().is_err());
    assert!(stage.verify().is_err());
    write_attribute(&directory, &serde_json::to_vec(&certificate).unwrap());
    owner.verify().unwrap();
    stage.verify().unwrap();
}

#[test]
fn copied_registration_file_cannot_refresh_a_live_namespace_certificate() {
    let (_home, _root, paths) = owners();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let certificate = read(&directory).unwrap().unwrap();
    let path = paths.home().join(REGISTRATION_RECORD);
    let copy = paths.home().join(COPIED_REGISTRATION);
    std::fs::copy(&path, &copy).unwrap();
    std::fs::rename(copy, &path).unwrap();

    assert!(owner.verify().is_err());
    assert!(paths.host_socket().is_err());
    assert!(paths.retain_socket_namespace().is_err());
    assert_eq!(read(&directory).unwrap().unwrap(), certificate);
    owner.preserve_on_release();
}
