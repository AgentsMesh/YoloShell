use crate::host::tests::*;
use super::*;

const UPPERCASE_HOME_NAME: &str = "MissingHome";
const FIRST_HOME_NAME: &str = "home-a";
const SECOND_HOME_NAME: &str = "home-b";
#[cfg(target_os = "macos")]
const VISIBLE_RESERVED_ROOT: &str = "/tmp/yolo-socket-selection-test";
#[cfg(target_os = "macos")]
const VISIBLE_DEFAULT_SOCKET: &str = "/tmp/yolo-socket-selection-test/home/host.sock";
#[cfg(target_os = "macos")]
const PHYSICAL_DEFAULT_SOCKET: &str = "/private/tmp/yolo-socket-selection-test/home/host.sock";
#[cfg(target_os = "macos")]
const PHYSICAL_FOREIGN_SOCKET: &str = "/private/tmp/yolo-socket-selection-test/foreign/host.sock";

#[test]
fn default_socket_comes_from_the_retained_namespace_identity() {
    let root = tempdir().unwrap();
    let home = root.path().join(UPPERCASE_HOME_NAME);
    let paths = Paths::new(&home);
    let before_creation = paths.host_socket().unwrap();

    let host = Host::open(HostConfig::new(&home)).unwrap();
    assert_eq!(host.socket_path(), before_creation);
    assert_eq!(
        host.socket_namespace.host_socket().unwrap(),
        before_creation
    );
    assert_eq!(paths.host_socket().unwrap(), before_creation);

    let namespace = before_creation.parent().unwrap().to_path_buf();
    drop(host);
    assert!(!namespace.exists());
}

#[test]
fn explicit_socket_cannot_enter_another_home_namespace() {
    let root = tempdir().unwrap();
    let home = root.path().join(FIRST_HOME_NAME);
    let foreign_home = root.path().join(SECOND_HOME_NAME);
    let foreign_socket = Paths::new(foreign_home).host_socket().unwrap();
    let mut config = HostConfig::new(home);
    config.socket = Some(foreign_socket.clone());

    let error = Host::open(config).err().unwrap();
    assert!(error.to_string().contains(RESERVED_SOCKET_NAMESPACE_ERROR));
    assert!(!foreign_socket.parent().unwrap().exists());
}

#[test]
fn explicit_default_socket_is_allowed() {
    let root = tempdir().unwrap();
    let home = root.path().join(UPPERCASE_HOME_NAME);
    let default_socket = Paths::new(&home).host_socket().unwrap();
    let mut config = HostConfig::new(home);
    config.socket = Some(default_socket.clone());

    let host = Host::open(config).unwrap();
    assert_eq!(host.socket_path(), default_socket);
}

#[cfg(target_os = "macos")]
#[test]
fn socket_isolation_reserved_root_alias_cannot_bypass_selection() {
    let reserved = Path::new(VISIBLE_RESERVED_ROOT);
    let visible_default = Path::new(VISIBLE_DEFAULT_SOCKET);
    let physical_default = Path::new(PHYSICAL_DEFAULT_SOCKET);
    let foreign = Path::new(PHYSICAL_FOREIGN_SOCKET);

    assert!(validate_socket_namespace_selection(foreign, visible_default, reserved).is_err());
    assert!(
        validate_socket_namespace_selection(physical_default, visible_default, reserved).is_ok()
    );
}
