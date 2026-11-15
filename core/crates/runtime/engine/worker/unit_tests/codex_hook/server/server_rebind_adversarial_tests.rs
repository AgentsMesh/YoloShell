use super::super::constants::CODEX_SOCKET_NAMESPACE_ERROR;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::Fixture;
use std::os::unix::fs::PermissionsExt;

const REBIND_AUTHORITY_HOME: &str = "rebind-authority-home";
const DISPLACED_SOCKET_DIRECTORY: &str = "displaced-codex-socket-directory";
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

#[tokio::test]
async fn retained_rebind_rejects_a_substituted_socket_directory() {
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(REBIND_AUTHORITY_HOME);
    let supervisor = CodexHookIngressSupervisor::new();
    let mut server = endpoint.bind_server(&supervisor, &fixture).await;
    let socket = server.socket_path().to_path_buf();
    let parent = socket.parent().unwrap().to_path_buf();
    let displaced = parent.parent().unwrap().join(DISPLACED_SOCKET_DIRECTORY);
    std::fs::rename(&parent, &displaced).unwrap();
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(
        &parent,
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();

    let result = server.restart_retained().await;

    assert!(result.is_err());
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains(CODEX_SOCKET_NAMESPACE_ERROR)
    );
    assert!(!socket.exists());
    std::fs::remove_dir(&parent).unwrap();
    std::fs::rename(&displaced, &parent).unwrap();
    assert!(socket.exists());
    drop(server);
    endpoint.finish();
}
