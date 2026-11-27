use super::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

#[test]
fn maximum_uid_namespace_has_room_for_every_production_stage_name() {
    let digest = [SOCKET_DIGEST_SEPARATOR; SOCKET_FULL_HASH_BYTES];
    let location = SocketNamespaceLocation::from_default_digest(u32::MAX, digest);
    let target_name = std::ffi::CString::new(HOST_SOCKET_FILE_NAME).unwrap();
    let stage_target = socket_stage::Target::new(&location.directory, &target_name).unwrap();

    location.validate_generated_paths().unwrap();
    for kind in [socket_stage::Kind::Bind, socket_stage::Kind::Probe] {
        let (_, path) = stage_target.stage(kind).unwrap();
        assert!(path.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES);
        assert_eq!(path.parent(), Some(location.directory.as_path()));
    }
}

#[test]
fn maximum_uid_and_session_endpoints_fit_the_local_socket_limit() {
    let digest = [SOCKET_DIGEST_SEPARATOR; SOCKET_FULL_HASH_BYTES];
    let location = SocketNamespaceLocation::from_default_digest(u32::MAX, digest);
    location.validate_generated_paths().unwrap();
    for endpoint in [
        location.hub_socket().unwrap(),
        location.connector_socket().unwrap(),
        location.directory.join(PUBLIC_CLIENT_SOCKET_FILE_NAME),
        location.worker_socket(Uuid::max()).unwrap(),
        location.directory.join(format!(
            "{CLAUDE_HOOK_SOCKET_PREFIX}{}{SOCKET_FILE_EXTENSION}",
            Uuid::max().simple()
        )),
        location.directory.join(format!(
            "{CODEX_HOOK_SOCKET_PREFIX}{}{SOCKET_FILE_EXTENSION}",
            Uuid::max().simple()
        )),
        location.custody_guardian_socket(Uuid::max()).unwrap(),
    ] {
        assert!(endpoint.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES);
    }
}

#[test]
fn prefixed_endpoint_names_require_the_canonical_simple_uuid() {
    let id = Uuid::new_v4();
    for prefix in [
        CLAUDE_HOOK_SOCKET_PREFIX,
        CODEX_HOOK_SOCKET_PREFIX,
        CUSTODY_GUARDIAN_SOCKET_PREFIX,
    ] {
        let canonical =
            CString::new(format!("{prefix}{}{SOCKET_FILE_EXTENSION}", id.simple())).unwrap();
        let hyphenated = CString::new(format!("{prefix}{id}{SOCKET_FILE_EXTENSION}")).unwrap();

        assert!(socket_path::is_endpoint_name(&canonical));
        assert!(!socket_path::is_endpoint_name(&hyphenated));
    }
}

#[test]
fn hub_endpoints_are_explicit_members_of_the_namespace() {
    let name = CString::new(HUB_SOCKET_FILE_NAME).unwrap();
    assert!(socket_path::is_endpoint_name(&name));
    let digest = [SOCKET_DIGEST_SEPARATOR; SOCKET_FULL_HASH_BYTES];
    let location = SocketNamespaceLocation::from_default_digest(u32::MAX, digest);
    assert_ne!(
        location.hub_socket().unwrap(),
        location.host_socket().unwrap()
    );
}

#[test]
fn connector_endpoint_is_exactly_named_and_does_not_enroll_similar_socket_names() {
    const UNMANAGED_NAMES: &[&str] = &[
        "connector-extra.sock",
        "Connector.sock",
        "connector.sock.tmp",
        ".connector.sock",
    ];
    let connector = CString::new(CONNECTOR_SOCKET_FILE_NAME).unwrap();
    assert!(socket_path::is_endpoint_name(&connector));
    for name in UNMANAGED_NAMES {
        assert!(!socket_path::is_endpoint_name(
            &CString::new(*name).unwrap()
        ));
    }
    let digest = [SOCKET_DIGEST_SEPARATOR; SOCKET_FULL_HASH_BYTES];
    let location = SocketNamespaceLocation::from_default_digest(u32::MAX, digest);
    assert_ne!(
        location.connector_socket().unwrap(),
        location.hub_socket().unwrap()
    );
    assert_ne!(
        location.connector_socket().unwrap(),
        location.host_socket().unwrap()
    );
}
