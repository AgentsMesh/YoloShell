use super::guard::{connect_peer_until, read_status_until, retryable};
use super::validate_opened_channel;
use std::time::Duration;
use tokio::net::UnixListener;
use yolo_client::ClientError;
use yolo_remote::{ChannelIdentity, ChannelPurpose};
use yolo_runtime::model::HostStatus;
use yolo_runtime::protocol::ProtocolError;
use yolo_runtime::protocol::ResponseEnvelope;

const TEST_IO_DETAIL: &str = "setup probe test I/O failure";
const EXPECTED_CONNECTION_ID_VALUE: u128 = 1;
const OTHER_CONNECTION_ID_VALUE: u128 = EXPECTED_CONNECTION_ID_VALUE + 1;
const TEST_CHANNEL_ID: u64 = 7;
const TRUNCATED_HEADER_BYTES: usize = 1;
const DELAYED_BIND_INTERVAL: Duration = Duration::from_millis(75);
const PROBE_TEST_TIMEOUT: Duration = Duration::from_secs(2);
const DELAYED_SOCKET_NAME: &str = "delayed-host.sock";
const STATUS_SOCKET_NAME: &str = "status-host.sock";
const TEST_BOOT_ID: &str = "setup-probe-test-boot";
const TEST_CONTROLLER_GENERATION: u64 = 7;

#[test]
fn detached_host_probe_retries_only_transient_connection_errors() {
    for kind in [
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::ConnectionRefused,
        std::io::ErrorKind::Interrupted,
        std::io::ErrorKind::UnexpectedEof,
        std::io::ErrorKind::ConnectionReset,
        std::io::ErrorKind::ConnectionAborted,
        std::io::ErrorKind::BrokenPipe,
    ] {
        assert!(retryable(&client_io_error(kind)));
    }
    assert!(!retryable(&client_io_error(
        std::io::ErrorKind::PermissionDenied
    )));
    assert!(!retryable(&ClientError::Protocol(
        ProtocolError::ConnectionPoisoned
    )));
    assert!(retryable(&ClientError::Protocol(
        ProtocolError::EndOfStream
    )));
    assert!(retryable(&ClientError::Protocol(
        ProtocolError::OutcomeUnknown
    )));
    assert!(retryable(&ClientError::Protocol(
        ProtocolError::TruncatedHeader {
            received: TRUNCATED_HEADER_BYTES
        }
    )));
}

#[test]
fn opened_runtime_validation_rejects_connection_and_purpose_mismatches() {
    let expected = uuid::Uuid::from_u128(EXPECTED_CONNECTION_ID_VALUE);
    let valid = ChannelIdentity {
        connection_id: expected,
        channel_id: TEST_CHANNEL_ID,
        purpose: ChannelPurpose::RuntimeControl,
    };
    assert!(validate_opened_channel(expected, valid).is_ok());
    assert!(
        validate_opened_channel(
            expected,
            ChannelIdentity {
                connection_id: uuid::Uuid::from_u128(OTHER_CONNECTION_ID_VALUE),
                ..valid
            }
        )
        .is_err()
    );
    assert!(
        validate_opened_channel(
            expected,
            ChannelIdentity {
                purpose: ChannelPurpose::TerminalAttachment,
                ..valid
            }
        )
        .is_err()
    );
}

#[tokio::test]
async fn peer_probe_waits_for_delayed_host_socket_publication() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(DELAYED_SOCKET_NAME);
    let server_socket = socket.clone();
    let server = tokio::spawn(async move {
        tokio::time::sleep(DELAYED_BIND_INTERVAL).await;
        let listener = UnixListener::bind(server_socket).unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        drop(stream);
    });
    let stream = connect_peer_until(&socket, tokio::time::Instant::now() + PROBE_TEST_TIMEOUT)
        .await
        .unwrap();
    drop(stream);
    server.await.unwrap();
}

#[tokio::test]
async fn status_probe_retries_clean_eof_during_host_publication() {
    let home = tempfile::tempdir().unwrap();
    let socket = home.path().join(STATUS_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let expected_home = home.path().canonicalize().unwrap();
    let reported_home = expected_home.clone();
    let server = tokio::spawn(async move {
        let (first, _) = listener.accept().await.unwrap();
        drop(first);
        let (second, _) = listener.accept().await.unwrap();
        let mut second = yolo_runtime_rpc_testing::Peer::new(second);
        let request = second.request().await.unwrap();
        let response = ResponseEnvelope::success(
            &request,
            &HostStatus {
                protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
                boot_id: TEST_BOOT_ID.into(),
                host_pid: std::process::id(),
                controller_generation: TEST_CONTROLLER_GENERATION,
                home: reported_home,
                capabilities: Vec::new(),
            },
        )
        .unwrap();
        second.respond(&response).await.unwrap();
        assert!(matches!(
            second.request().await,
            Err(ProtocolError::EndOfStream)
        ));
    });
    let client = yolo_client::Client::managed_local(&expected_home, &socket).unwrap();
    let status = read_status_until(&client, tokio::time::Instant::now() + PROBE_TEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(status.boot_id, TEST_BOOT_ID);
    server.await.unwrap();
}

fn client_io_error(kind: std::io::ErrorKind) -> ClientError {
    ClientError::Protocol(ProtocolError::Io(std::io::Error::new(kind, TEST_IO_DETAIL)))
}
