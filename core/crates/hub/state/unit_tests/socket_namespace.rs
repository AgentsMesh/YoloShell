use super::*;
use std::os::unix::ffi::OsStrExt;

const LONG_COMPONENT_BYTES: usize = 180;
const LONG_COMPONENT: &str = "s";

#[test]
fn long_state_paths_use_stable_short_sockets_and_do_not_change_identity() {
    let (_root, path, home) = fixture();
    let path = path.join(LONG_COMPONENT.repeat(LONG_COMPONENT_BYTES));
    let state = HubState::open(&path, &home).unwrap();
    let socket = state.socket_path();
    let id = state.hub_id();
    assert!(path.as_os_str().as_bytes().len() > yolo_paths::constants::UNIX_SOCKET_PATH_MAX_BYTES);
    assert!(
        socket.as_os_str().as_bytes().len() < yolo_paths::constants::UNIX_SOCKET_PATH_MAX_BYTES
    );
    assert_eq!(
        socket.parent(),
        Some(state.socket_directory().unwrap().path())
    );
    assert_eq!(HubState::socket_path_for(&path).unwrap(), socket);
    assert!(!path.join(HUB_SOCKET_FILE).exists());
    drop(state);
    let state = HubState::open(&path, &home).unwrap();
    assert_eq!(state.hub_id(), id);
    assert_eq!(state.socket_path(), socket);
}

#[test]
fn state_directories_do_not_share_public_or_private_host_endpoints() {
    let (root, path, home) = fixture();
    let first = HubState::open(&path, &home).unwrap();
    let other = root.path().canonicalize().unwrap().join(CLONE_DIRECTORY);
    let second = HubState::open(&other, &root.path().join(DIFFERENT_HOST)).unwrap();
    assert_ne!(first.socket_path(), second.socket_path());
    assert_ne!(
        first.socket_path(),
        yolo_paths::Paths::new(&path).host_socket().unwrap()
    );
    assert_ne!(
        first.socket_path(),
        yolo_paths::Paths::new(&home).host_socket().unwrap()
    );
}

#[test]
fn resolving_missing_state_is_read_only() {
    let (_root, path, _home) = fixture();
    let socket = HubState::socket_path_for(&path).unwrap();
    assert!(!path.exists());
    assert!(!socket.parent().unwrap().exists());
    assert!(HubState::socket_path_for(Path::new(STATE_DIRECTORY)).is_err());
}

#[test]
fn replacing_the_socket_directory_fences_the_existing_hub() {
    let (root, path, home) = fixture();
    let state = HubState::open(&path, &home).unwrap();
    let directory = state.socket_directory().unwrap();
    std::fs::rename(directory.path(), root.path().join(CLONE_DIRECTORY)).unwrap();
    PrivateDirectory::ensure(directory.path()).unwrap();
    assert!(state.verify().is_err());
    assert!(state.socket_directory().is_err());
    drop(state);
    assert!(directory.path().is_dir());
    std::fs::remove_dir(directory.path()).unwrap();
}
