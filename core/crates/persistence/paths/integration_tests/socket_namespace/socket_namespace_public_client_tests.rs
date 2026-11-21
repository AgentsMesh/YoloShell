use super::{test_support::*, *};
use std::{
    ffi::CString,
    fs,
    os::unix::{fs::PermissionsExt, net::UnixListener},
};

const UNMANAGED_NAMES: &[&str] = &[
    "public-client-extra.sock",
    "Public-client.sock",
    "public-client.sock.tmp",
    ".public-client.sock",
];

#[test]
fn public_client_endpoint_name_is_exactly_registered() {
    assert!(socket_path::is_endpoint_name(
        &CString::new(PUBLIC_CLIENT_SOCKET_FILE_NAME).unwrap()
    ));
    for name in UNMANAGED_NAMES {
        assert!(!socket_path::is_endpoint_name(
            &CString::new(*name).unwrap()
        ));
    }
}

#[test]
fn live_public_ingress_is_preserved_and_last_owner_can_reclaim_the_stale_endpoint() {
    let fixture = RegisteredFixture::new();
    let owner = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let socket = directory.path().join(PUBLIC_CLIENT_SOCKET_FILE_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    run_scanner_pass(&owner);
    owner.verify().unwrap();
    assert_eq!(owner.release().unwrap(), NamespaceRelease::Preserved);
    assert!(socket.exists());
    drop(listener);
    let successor = fixture.paths.retain_socket_namespace().unwrap();
    assert_eq!(successor.release().unwrap(), NamespaceRelease::Removed);
    assert!(!directory.path().exists());
}
