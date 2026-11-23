use super::*;

const HOME: &str = "home";
const SOCKET: &str = "s";
const SOCKET_PARENT: &str = "p";
const RETAINED_DIRECTORY: &str = "retained";

#[test]
fn custom_socket_selection_cannot_bypass_certificate_validation() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join(HOME);
    PrivateDirectory::ensure(&home).unwrap();
    let paths = Paths::new(&home);
    let namespace = paths.retain_socket_namespace().unwrap();
    let socket = root.path().join(SOCKET);
    let absence = Host::observe_absence(&home, Some(&socket))
        .unwrap()
        .unwrap();
    let directory = namespace.directory().unwrap();
    let tamper = certificate_tamper::CorruptCertificate::new(directory.try_clone().unwrap());
    assert!(absence.verify().is_err());
    drop(absence);
    assert!(Host::observe_absence(&home, Some(&socket)).is_err());
    drop(tamper);
    namespace.verify().unwrap();
    drop(namespace);
    assert!(!directory.path().exists());
}

#[test]
fn a_replaced_namespace_with_a_copied_certificate_is_not_the_retained_namespace() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join(HOME);
    PrivateDirectory::ensure(&home).unwrap();
    let paths = Paths::new(&home);
    let namespace = paths.retain_socket_namespace().unwrap();
    let absence = Host::observe_absence(&home, None).unwrap().unwrap();
    let replacement = certificate_tamper::ReplacedDirectory::new(namespace.directory().unwrap());
    assert!(paths.host_socket().is_err());
    assert!(absence.verify().is_err());
    drop(replacement);
    namespace.verify().unwrap();
    drop(absence);
    drop(namespace);
}

#[test]
fn changing_an_empty_custom_socket_parent_invalidates_absence() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join(HOME);
    let parent = root.path().join(SOCKET_PARENT);
    PrivateDirectory::ensure(&parent).unwrap();
    let socket = parent.join(SOCKET);
    let absence = Host::observe_absence(&home, Some(&socket))
        .unwrap()
        .unwrap();
    std::fs::rename(&parent, root.path().join(RETAINED_DIRECTORY)).unwrap();
    PrivateDirectory::ensure(&parent).unwrap();
    assert!(absence.verify().is_err());
}

#[test]
fn reserved_socket_namespace_cannot_be_used_as_a_custom_endpoint() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let home = root.path().join(HOME);
    let paths = Paths::new(&home);
    let directory = paths.socket_dir().unwrap();
    let socket = directory.join(SOCKET);
    assert!(Host::observe_absence(&home, Some(&socket)).is_err());
    assert!(!directory.exists());
    assert!(!home.exists());
}
