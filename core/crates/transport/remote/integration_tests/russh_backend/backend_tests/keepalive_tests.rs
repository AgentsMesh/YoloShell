use super::*;
use crate::remote::constants::{
    SSH_INACTIVITY_TIMEOUT, SSH_KEEPALIVE_INTERVAL, SSH_KEEPALIVE_MAX_FAILURES,
};
use russh::keys::ssh_key::LineEnding;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const TEST_IDENTITY_FILE_NAME: &str = "idle-identity";
const TEST_KEEPALIVE_INTERVAL: Duration = Duration::from_millis(25);
const TEST_KEEPALIVE_MAX_FAILURES: usize = 2;
const TEST_IDLE_INACTIVITY_TIMEOUT: Duration = Duration::from_millis(500);
const TEST_IDLE_OBSERVATION: Duration = Duration::from_millis(750);
const TEST_FAILED_PEER_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(1);
const TEST_KEEPALIVE_FAILURE_BOUND: Duration = Duration::from_millis(500);
const PROXY_FORWARD_BUFFER_BYTES: usize = 4_096;
const END_OF_STREAM_BYTES: usize = 0;
const KEEPALIVE_THRESHOLD_NOT_ENFORCED: &str = "keepalive failure threshold was not enforced";

#[test]
fn production_client_config_enables_bounded_keepalives() {
    let source = SshConnectConfig::new(endpoint(), KnownHosts::default());
    let (config, _, _) = super::super::connect::connection_parts(&source);

    assert_eq!(config.keepalive_interval, Some(SSH_KEEPALIVE_INTERVAL));
    assert_eq!(config.keepalive_max, SSH_KEEPALIVE_MAX_FAILURES);
    assert!(std::num::NonZeroUsize::new(config.keepalive_max).is_some());
    assert_eq!(config.inactivity_timeout, Some(SSH_INACTIVITY_TIMEOUT));
    assert!(SSH_KEEPALIVE_INTERVAL < SSH_INACTIVITY_TIMEOUT);
}

#[tokio::test]
async fn responsive_idle_peer_survives_the_inactivity_bound() {
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let connection =
        connect_with_timing(client_stream, server_stream, TEST_IDLE_INACTIVITY_TIMEOUT).await;

    tokio::time::sleep(TEST_IDLE_OBSERVATION).await;
    let policy = CommandPolicy::new([NORMAL_PROGRAM.into()]).unwrap();
    let command = policy
        .authorize(CommandRequest::new(NORMAL_PROGRAM, Vec::new()))
        .unwrap();
    let output = connection.execute(&command).await.unwrap().value;

    assert_eq!(output.stdout, COMMAND_STDOUT);
    connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn silent_peer_is_rejected_before_the_final_inactivity_bound() {
    let (client_stream, server_stream, stall_output) = gated_transport();
    let connection = connect_with_timing(
        client_stream,
        server_stream,
        TEST_FAILED_PEER_INACTIVITY_TIMEOUT,
    )
    .await;
    stall_output.store(true, Ordering::Release);

    let opening = connection.session.channel_open_session();
    let result = tokio::time::timeout(TEST_KEEPALIVE_FAILURE_BOUND, opening)
        .await
        .expect(KEEPALIVE_THRESHOLD_NOT_ENFORCED);

    assert!(result.is_err());
}

async fn connect_with_timing(
    client_stream: DuplexStream,
    server_stream: DuplexStream,
    inactivity_timeout: Duration,
) -> RusshConnection {
    let state = tempfile::tempdir().unwrap();
    let identity_path = state.path().join(TEST_IDENTITY_FILE_NAME);
    let (host_key, user_key) = keys();
    user_key
        .write_openssh_file(&identity_path, LineEnding::LF)
        .unwrap();
    let known = known_for_key(&host_key);
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .use_agent(false)
        .with_identity(IdentityFile::new(identity_path, None))
        .with_timeouts(TEST_CONNECT_TIMEOUT, inactivity_timeout)
        .with_keepalive_for_test(TEST_KEEPALIVE_INTERVAL, TEST_KEEPALIVE_MAX_FAILURES);
    RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap()
}

fn gated_transport() -> (DuplexStream, DuplexStream, Arc<AtomicBool>) {
    let (client_stream, client_proxy) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let (server_proxy, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    let (client_reader, client_writer) = tokio::io::split(client_proxy);
    let (server_reader, server_writer) = tokio::io::split(server_proxy);
    let stall_output = Arc::new(AtomicBool::new(false));
    let output_gate = Arc::clone(&stall_output);

    tokio::spawn(async move {
        let mut reader = client_reader;
        let mut writer = server_writer;
        let _ = tokio::io::copy(&mut reader, &mut writer).await;
    });
    tokio::spawn(async move {
        forward_server_output(server_reader, client_writer, output_gate).await;
    });
    (client_stream, server_stream, stall_output)
}

async fn forward_server_output(
    mut reader: tokio::io::ReadHalf<DuplexStream>,
    mut writer: tokio::io::WriteHalf<DuplexStream>,
    stalled: Arc<AtomicBool>,
) {
    let mut buffer = [0_u8; PROXY_FORWARD_BUFFER_BYTES];
    loop {
        let Ok(bytes) = reader.read(&mut buffer).await else {
            return;
        };
        if bytes == END_OF_STREAM_BYTES {
            return;
        }
        if stalled.load(Ordering::Acquire) {
            std::future::pending::<()>().await;
        }
        if writer.write_all(&buffer[..bytes]).await.is_err() {
            return;
        }
    }
}
