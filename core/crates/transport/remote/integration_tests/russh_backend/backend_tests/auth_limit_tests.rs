use super::constants::*;
use super::fixture::{endpoint, keys, known_for_key};
use super::{IdentityFile, RusshConnection, SshConnectConfig};
use russh::keys::ssh_key::LineEnding;
use russh::keys::{PrivateKey, PublicKey};
use russh::server::Auth;
use std::sync::Arc;
use std::time::Duration;

const NOISY_AGENT_IDENTITIES: usize = 4;
const MAX_REJECTED_AUTH_ATTEMPTS: usize = 1;
const CONFIGURED_IDENTITY_FILE: &str = "configured-identity";

#[tokio::test]
async fn noisy_agent_cannot_exhaust_attempts_before_the_configured_identity() {
    assert!(NOISY_AGENT_IDENTITIES > MAX_REJECTED_AUTH_ATTEMPTS);
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join(CONFIGURED_IDENTITY_FILE);
    let (host_key, accepted_key) = keys();
    accepted_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let noisy_keys = (INITIAL_COUNTER..NOISY_AGENT_IDENTITIES)
        .map(|_| keys().1)
        .collect::<Vec<_>>();
    let agent_socket = super::agent::start_many(state.path(), &noisy_keys).await;
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_limited_server(server_stream, host_key, accepted_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .with_agent_socket(agent_socket)
        .with_identity(IdentityFile::new(identity_path, None));

    let connection = RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap();

    connection.disconnect().await.unwrap();
}

struct AttemptLimitedServer {
    accepted_key: PublicKey,
    remaining_rejections: usize,
}

impl russh::server::Handler for AttemptLimitedServer {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, _: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        if key == &self.accepted_key {
            return Ok(Auth::Accept);
        }
        if self.remaining_rejections == INITIAL_COUNTER {
            return Err(russh::Error::Disconnect);
        }
        self.remaining_rejections -= COUNTER_INCREMENT;
        Ok(Auth::reject())
    }
}

fn spawn_limited_server<R>(stream: R, host_key: PrivateKey, accepted_key: PublicKey)
where
    R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let config = Arc::new(russh::server::Config {
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        max_auth_attempts: MAX_REJECTED_AUTH_ATTEMPTS,
        keys: vec![host_key],
        ..Default::default()
    });
    tokio::spawn(async move {
        let handler = AttemptLimitedServer {
            accepted_key,
            remaining_rejections: MAX_REJECTED_AUTH_ATTEMPTS,
        };
        let running = russh::server::run_stream(config, stream, handler)
            .await
            .unwrap();
        let _ = running.await;
    });
}
