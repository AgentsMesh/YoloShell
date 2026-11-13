use super::*;
use russh::keys::ssh_key::LineEnding;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;

const INVALID_IDENTITY_FILE: &str = "invalid-identity";
const INVALID_IDENTITY_TEXT: &[u8] = b"not an OpenSSH private key";
const PRIVATE_IDENTITY_MODE: u32 = 0o600;

#[tokio::test]
async fn key_mismatch_and_missing_authentication_fail_closed() {
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let (host_key, user_key) = keys();
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let wrong = KnownHosts::parse(&format!("{MEMORY_HOST} ssh-ed25519 AAAAC3NzaWrongKey")).unwrap();
    let config = SshConnectConfig::new(endpoint(), wrong).use_agent(false);
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::ChangedHostKey { .. })
    ));

    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let (host_key, user_key) = keys();
    let known = known_for_key(&host_key);
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known).use_agent(false);
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::AuthenticationFailed(_))
    ));
}

#[tokio::test]
async fn invalid_and_rejected_identity_files_fail_closed() {
    let state = tempfile::tempdir().unwrap();
    let (host_key, accepted_key) = keys();
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .use_agent(false)
        .with_identity(IdentityFile::new(state.path().join("missing"), None));
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::AuthenticationFailed(message))
            if message.contains(crate::remote::constants::SSH_IDENTITY_FAILURE_CONTEXT)
    ));

    let identity_path = state.path().join("rejected");
    let (host_key, configured_key) = keys();
    configured_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let known = known_for_key(&host_key);
    let (_, accepted_key) = keys();
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .use_agent(false)
        .with_identity(IdentityFile::new(identity_path, None));
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::AuthenticationFailed(_))
    ));
}

#[tokio::test]
async fn readable_but_undecodable_identity_preserves_failure_context() {
    let state = tempfile::tempdir().unwrap();
    let identity = state.path().join(INVALID_IDENTITY_FILE);
    std::fs::write(&identity, INVALID_IDENTITY_TEXT).unwrap();
    let permissions = std::fs::Permissions::from_mode(PRIVATE_IDENTITY_MODE);
    std::fs::set_permissions(&identity, permissions).unwrap();
    let (host_key, accepted_key) = keys();
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .use_agent(false)
        .with_identity(IdentityFile::new(identity, None));

    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::AuthenticationFailed(context))
            if context.contains(crate::remote::constants::SSH_IDENTITY_FAILURE_CONTEXT)
    ));
}

#[tokio::test]
async fn invalid_identity_is_skipped_before_a_valid_configured_fallback() {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join("accepted");
    let (host_key, accepted_key) = keys();
    accepted_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .use_agent(false)
        .with_identity(IdentityFile::new(state.path().join("missing"), None))
        .with_identity(IdentityFile::new(identity_path, None));
    let connection = RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap();
    connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn configured_identity_precedes_a_rejected_agent_key() {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join("accepted");
    let (host_key, accepted_key) = keys();
    let (_, rejected_agent_key) = keys();
    accepted_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let agent_socket = super::agent::start(state.path(), &rejected_agent_key).await;
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .with_agent_socket(agent_socket)
        .with_identity(IdentityFile::new(identity_path, None));
    let connection = RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap();
    connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn stalled_and_broken_handshakes_are_transport_errors() {
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let config = SshConnectConfig::new(endpoint(), KnownHosts::default())
        .use_agent(false)
        .with_timeouts(STALLED_CONNECT_TIMEOUT, STALLED_INACTIVITY_TIMEOUT);
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::Transport(_))
    ));
    drop(server_stream);

    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    drop(server_stream);
    let config = SshConnectConfig::new(endpoint(), KnownHosts::default()).use_agent(false);
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::Transport(_))
    ));
}

#[tokio::test]
async fn non_utf8_sftp_path_is_rejected_without_an_unbounded_read() {
    let fixture = connect_memory(false).await.unwrap();
    let path = PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));
    assert!(matches!(
        fixture.connection.canonicalize(&path).await,
        Err(RemoteError::PathDenied(actual)) if actual == path
    ));
}

#[tokio::test]
async fn configured_identity_avoids_broken_agent_and_agent_failure_keeps_context() {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join("fallback-identity");
    let (host_key, user_key) = keys();
    user_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let known = known_for_key(&host_key);
    let agent_socket = agent::start_broken(state.path()).await;
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .with_agent_socket(agent_socket)
        .with_identity(IdentityFile::new(identity_path, None));
    RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap()
        .disconnect()
        .await
        .unwrap();

    let state = tempfile::tempdir().unwrap();
    let (host_key, user_key) = keys();
    let known = known_for_key(&host_key);
    let agent_socket = agent::start_broken(state.path()).await;
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known).with_agent_socket(agent_socket);
    assert!(matches!(
        RusshConnection::connect_stream(config, client_stream).await,
        Err(RemoteError::AuthenticationFailed(context))
            if context.contains(crate::remote::constants::SSH_AGENT_FAILURE_CONTEXT)
    ));
}
