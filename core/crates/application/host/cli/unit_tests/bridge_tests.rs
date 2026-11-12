#[cfg(target_os = "macos")]
use super::ancillary_test_support::{assert_reader_eof, pipe, send_right};
use super::*;
#[cfg(target_os = "macos")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "macos")]
use std::time::Duration;
use tokio::net::UnixListener;
use yolo_runtime::model::{HostStatus, PROTOCOL_VERSION};
use yolo_runtime::protocol::{RequestEnvelope, ResponseEnvelope, method};
use yolo_runtime_rpc_testing::Peer;

const TEST_BOOT_ID: &str = "bridge-auth-boot";
const TEST_HOST_PID: u32 = 73;
const TEST_CONTROLLER_GENERATION: u64 = 11;
const FORWARDED_STDIN: &[u8] = b"must-not-cross-home";
const FORWARDED_STDOUT: &[u8] = b"verified-host-output";
const STDIN_PROBE_BYTES: usize = 1;
const RPC_PIPE_BYTES: usize = 4096;
const BRIDGE_RPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const BRIDGE_REQUEST_ID: &str = "bridge-forwarded-request";
const TEST_HOST_SOCKET_FILE: &str = "host.sock";
#[cfg(target_os = "macos")]
const ANCILLARY_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(target_os = "macos")]
const ANCILLARY_SETTLEMENT_TIMEOUT_MESSAGE: &str = "bridge ancillary test did not settle";

#[test]
fn client_deadline_is_a_recoverable_transport_failure() {
    assert!(is_transport_error(&ClientError::Deadline));
}

#[tokio::test]
async fn bridge_authenticates_before_forwarding_stdin() {
    let expected = tempfile::tempdir().unwrap();
    let reported = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let reported_home = reported.path().to_path_buf();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = Peer::new(stream);
        let request = peer.request().await.unwrap();
        assert_eq!(request.method, method::HOST_STATUS);
        let response = ResponseEnvelope::success(&request, &status(reported_home)).unwrap();
        peer.respond(&response).await.unwrap();
        peer.request().await.is_err()
    });
    let client = Client::managed_local(expected.path(), socket).unwrap();
    let error = forward(&client, FORWARDED_STDIN, tokio::io::sink())
        .await
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ClientError>(),
        Some(ClientError::HostHomeMismatch { .. })
    ));
    assert!(server.await.unwrap());
}

#[tokio::test]
async fn bridge_forwards_only_after_same_stream_authentication() {
    let expected = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let expected_home = expected.path().to_path_buf();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = Peer::new(stream);
        let request = peer.request().await.unwrap();
        let response = ResponseEnvelope::success(&request, &status(expected_home)).unwrap();
        peer.respond(&response).await.unwrap();
        let forwarded = peer.request().await.unwrap();
        assert_eq!(forwarded.request_id, BRIDGE_REQUEST_ID);
        peer.respond(&ResponseEnvelope::success(&forwarded, &FORWARDED_STDOUT).unwrap())
            .await
            .unwrap();
        forwarded.method
    });
    let client = Client::managed_local(expected.path(), socket).unwrap();
    let (downstream, bridge) = tokio::io::duplex(RPC_PIPE_BYTES);
    let (input, output) = tokio::io::split(bridge);
    let forwarding = tokio::spawn(async move { forward(&client, input, output).await });
    let connection = yolo_capnp_rpc::Connection::open(
        downstream,
        yolo_capnp_rpc::Protocol::Endpoint,
        tokio::time::Instant::now() + BRIDGE_RPC_TIMEOUT,
    )
    .await
    .unwrap();
    let request = RequestEnvelope::new(
        BRIDGE_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_codec::to_value(&()).unwrap(),
    );
    let result = connection
        .request(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&request).unwrap(),
            tokio::time::Instant::now() + BRIDGE_RPC_TIMEOUT,
        )
        .await
        .unwrap();
    let response: ResponseEnvelope = yolo_capnp_codec::from_value(result.value).unwrap();
    assert_eq!(
        response.decode_payload::<Vec<u8>>().unwrap(),
        FORWARDED_STDOUT
    );
    assert_eq!(server.await.unwrap(), method::HOST_STATUS);
    connection.close();
    let _ = forwarding.await.unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn bridge_rejects_and_closes_a_right_injected_after_authentication() {
    let expected = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let expected_home = expected.path().to_path_buf();
    let transferred = pipe();
    let reader = transferred.reader;
    let writer = transferred.writer;
    let (release_sender, release_receiver) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let descriptor = stream.as_raw_fd();
        let mut peer = Peer::new(stream);
        let request = peer.request().await.unwrap();
        let response = ResponseEnvelope::success(&request, &status(expected_home)).unwrap();
        peer.respond(&response).await.unwrap();
        let _request = peer.request().await.unwrap();
        send_right(descriptor, writer.as_raw_fd());
        drop(writer);
        release_receiver.await.unwrap();
    });
    let client = Client::managed_local(expected.path(), socket).unwrap();
    let (downstream, bridge) = tokio::io::duplex(RPC_PIPE_BYTES);
    let (input, output) = tokio::io::split(bridge);
    let forwarding = tokio::spawn(async move { forward(&client, input, output).await });
    let connection = yolo_capnp_rpc::Connection::open(
        downstream,
        yolo_capnp_rpc::Protocol::Endpoint,
        tokio::time::Instant::now() + BRIDGE_RPC_TIMEOUT,
    )
    .await
    .unwrap();
    let request = RequestEnvelope::new(
        BRIDGE_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_codec::to_value(&()).unwrap(),
    );
    let result = tokio::time::timeout(
        ANCILLARY_SETTLEMENT_TIMEOUT,
        connection.request(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&request).unwrap(),
            tokio::time::Instant::now() + BRIDGE_RPC_TIMEOUT,
        ),
    )
    .await
    .expect(ANCILLARY_SETTLEMENT_TIMEOUT_MESSAGE);
    assert!(result.is_err());

    release_sender.send(()).unwrap();
    server.await.unwrap();
    connection.close();
    let _ = forwarding.await.unwrap();
    assert_reader_eof(&reader);
}

fn status(home: PathBuf) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home,
        capabilities: Vec::new(),
    }
}
