use super::*;
use russh::keys::ssh_key::certificate::{Builder, CertType};
use std::time::Duration;

const CERTIFICATE_SERIAL: u64 = 17;
const CERTIFICATE_VALID_AFTER: Duration = Duration::ZERO;
const CERTIFICATE_VALID_BEFORE: Duration = Duration::MAX;
const CERTIFICATE_KEY_ID: &str = "yolo-test-agent-certificate";

#[tokio::test]
async fn certificate_agent_identity_authenticates_through_the_real_signing_flow() {
    let state = tempfile::tempdir().unwrap();
    let (host_key, identity) = keys();
    let (_, certificate_authority) = keys();
    let mut random = russh::keys::key::safe_rng();
    let mut builder = Builder::new_with_random_nonce(
        &mut random,
        identity.public_key(),
        CERTIFICATE_VALID_AFTER.as_secs(),
        CERTIFICATE_VALID_BEFORE.as_secs(),
    )
    .unwrap();
    builder.serial(CERTIFICATE_SERIAL).unwrap();
    builder.cert_type(CertType::User).unwrap();
    builder.key_id(CERTIFICATE_KEY_ID).unwrap();
    builder.valid_principal(MEMORY_USER).unwrap();
    let certificate = builder.sign(&certificate_authority).unwrap();
    let agent_socket = agent::start_certificate(state.path(), &identity, &certificate).await;
    let known = known_for_key(&host_key);
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, identity.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .with_agent_socket(agent_socket)
        .with_timeouts(TEST_CONNECT_TIMEOUT, TEST_INACTIVITY_TIMEOUT);

    let connection = RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap();

    connection.disconnect().await.unwrap();
}
