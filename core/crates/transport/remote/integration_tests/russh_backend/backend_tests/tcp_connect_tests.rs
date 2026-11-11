use super::constants::{MEMORY_USER, SHORT_OPERATION_TIMEOUT, TEST_CONNECT_TIMEOUT};
use super::fixture::{keys, spawn_server};
use crate::remote::{IdentityFile, KnownHosts, RemoteEndpoint, RusshConnection, SshConnectConfig};
use russh::keys::ssh_key::LineEnding;

#[tokio::test]
async fn production_tcp_connect_verifies_host_and_authenticates() {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join("identity");
    let (host_key, user_key) = keys();
    user_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let public_host_key = host_key.public_key().to_openssh().unwrap();
    let accepted = user_key.public_key().clone();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        spawn_server(stream, host_key, accepted);
    });
    let endpoint = RemoteEndpoint::new(MEMORY_USER, "127.0.0.1", port).unwrap();
    let known = KnownHosts::parse(&format!("[127.0.0.1]:{port} {public_host_key}")).unwrap();
    let config = SshConnectConfig::new(endpoint, known)
        .use_agent(false)
        .with_identity(IdentityFile::new(identity_path, None))
        .with_timeouts(TEST_CONNECT_TIMEOUT, TEST_CONNECT_TIMEOUT)
        .with_operation_timeout(SHORT_OPERATION_TIMEOUT);
    let connection = RusshConnection::connect(config).await.unwrap();
    connection.disconnect().await.unwrap();
}
