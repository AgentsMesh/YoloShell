use super::HostNamespace;
use std::os::unix::fs::symlink;
use yolo_runtime::paths::{Paths, directory::PrivateDirectory};

const HOST_HOME: &str = "host";
const FOREIGN_HOME: &str = "foreign";
const DISPLACED_HOME: &str = "displaced";

#[test]
fn startup_admits_a_new_host_namespace_before_validated_endpoint_lookup() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path().join(HOST_HOME));
    let namespace = HostNamespace::retain(&paths).unwrap();
    assert_eq!(namespace.socket().unwrap(), paths.host_socket().unwrap());
    namespace.verify().unwrap();
}

#[test]
fn startup_rejects_symlink_home_without_writing_to_its_target() {
    let root = tempfile::tempdir().unwrap();
    let foreign = root.path().join(FOREIGN_HOME);
    PrivateDirectory::open_or_create_strict(&foreign).unwrap();
    let home = root.path().join(HOST_HOME);
    symlink(&foreign, &home).unwrap();
    assert!(HostNamespace::retain(&Paths::new(home)).is_err());
    assert_eq!(std::fs::read_dir(foreign).unwrap().count(), 0);
}

#[test]
fn replaced_home_invalidates_admitted_endpoint_without_touching_replacement() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(HOST_HOME);
    let namespace = HostNamespace::retain(&Paths::new(&home)).unwrap();
    std::fs::rename(&home, root.path().join(DISPLACED_HOME)).unwrap();
    PrivateDirectory::open_or_create_strict(&home).unwrap();
    assert!(namespace.socket().is_err());
    assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    std::fs::remove_dir(&home).unwrap();
    std::fs::rename(root.path().join(DISPLACED_HOME), &home).unwrap();
    namespace.verify().unwrap();
    drop(namespace);
}
