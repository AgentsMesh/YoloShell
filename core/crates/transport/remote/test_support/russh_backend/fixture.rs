use super::super::*;
use super::constants::*;
use super::memory_fs::MemoryFilesystem;
use super::server::MemorySshServer;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey, PublicKey};
use std::sync::Arc;
use std::time::Duration;

impl RusshConnection {
    pub(super) async fn connect_stream<R>(
        config: SshConnectConfig,
        stream: R,
    ) -> Result<Self, RemoteError>
    where
        R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (client_config, handler, presented) = super::super::connect::connection_parts(&config);
        let connection = russh::client::connect_stream(client_config, stream, handler);
        let session = tokio::time::timeout(config.connect_timeout, connection)
            .await
            .map_err(|_| RemoteError::transport("SSH connection timed out"))?
            .map_err(super::super::connect::handler_error)?;
        Self::finish_connect(session, config, presented).await
    }
}

pub struct MemoryConnection {
    pub connection: Arc<RusshConnection>,
    pub filesystem: MemoryFilesystem,
    pub _state: tempfile::TempDir,
}

pub async fn connect_memory(use_agent: bool) -> Result<MemoryConnection, RemoteError> {
    connect_memory_with_fs(use_agent, MemoryFilesystem::default()).await
}

pub async fn connect_memory_with_fs(
    use_agent: bool,
    filesystem: MemoryFilesystem,
) -> Result<MemoryConnection, RemoteError> {
    connect_memory_with_timeout(use_agent, filesystem, NORMAL_OPERATION_TIMEOUT).await
}

pub async fn connect_memory_with_timeout(
    use_agent: bool,
    filesystem: MemoryFilesystem,
    operation_timeout: Duration,
) -> Result<MemoryConnection, RemoteError> {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join("identity");
    let (host_key, user_key) = keys();
    user_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server_with_fs(
        server_stream,
        host_key,
        user_key.public_key().clone(),
        filesystem.clone(),
    );
    let mut config = SshConnectConfig::new(endpoint(), known)
        .use_agent(use_agent)
        .with_identity(IdentityFile::new(identity_path, None))
        .with_timeouts(TEST_CONNECT_TIMEOUT, TEST_INACTIVITY_TIMEOUT)
        .with_operation_timeout(operation_timeout);
    config.keepalive_interval = MEMORY_KEEPALIVE_INTERVAL;
    let connection = RusshConnection::connect_stream(config, client_stream).await?;
    Ok(MemoryConnection {
        connection: Arc::new(connection),
        filesystem,
        _state: state,
    })
}

pub fn spawn_server<R>(stream: R, host_key: PrivateKey, accepted_key: PublicKey)
where
    R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    spawn_server_with_fs(stream, host_key, accepted_key, MemoryFilesystem::default());
}

pub fn spawn_server_with_fs<R>(
    stream: R,
    host_key: PrivateKey,
    accepted_key: PublicKey,
    filesystem: MemoryFilesystem,
) where
    R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let config = Arc::new(russh::server::Config {
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        keys: vec![host_key],
        ..Default::default()
    });
    tokio::spawn(async move {
        let lifecycle = filesystem.clone();
        let handler = MemorySshServer::with_filesystem(accepted_key, filesystem);
        let running = russh::server::run_stream(config, stream, handler)
            .await
            .unwrap();
        lifecycle.record_connection_closed(running.await);
    });
}

pub fn keys() -> (PrivateKey, PrivateKey) {
    let mut random = russh::keys::key::safe_rng();
    (
        PrivateKey::random(&mut random, Algorithm::Ed25519).unwrap(),
        PrivateKey::random(&mut random, Algorithm::Ed25519).unwrap(),
    )
}

pub fn endpoint() -> RemoteEndpoint {
    RemoteEndpoint::standard(MEMORY_USER, MEMORY_HOST).unwrap()
}

pub fn known_for_key(key: &PrivateKey) -> KnownHosts {
    KnownHosts::parse(&format!(
        "{MEMORY_HOST} {}",
        key.public_key().to_openssh().unwrap()
    ))
    .unwrap()
}

pub fn trusted_hosts(info: &ConnectionInfo) -> KnownHosts {
    KnownHosts::parse(&format!(
        "{MEMORY_HOST} {} {}",
        info.host_key.algorithm(),
        info.host_key.encoded()
    ))
    .unwrap()
}
