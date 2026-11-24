use serde_json::json;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::net::UnixListener;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::protocol::{RequestEnvelope, failure_kind, field};

#[path = "catalog_stream_process_support.rs"]
mod support;
use support::{ChildGuard, read_catalog_frame, respond_empty, write_catalog_frame};

const HOST_BINARY_ENVIRONMENT: &str = "YOLO_TEST_HOST_BINARY";
const REFRESH_COUNT: usize = 8;
const RECOVERY_CONNECTION_COUNT: usize = 2;
const HUNG_REQUEST_DEADLINE_MILLISECONDS: u64 = 20;
const HUNG_SERVER_RELEASE_DELAY: Duration = Duration::from_millis(100);
const OUTER_TYPED_TIMEOUT_BOUND: Duration = Duration::from_secs(2);

#[tokio::test(flavor = "multi_thread")]
async fn one_catalog_helper_serves_repeated_refreshes() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("catalog.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        for _ in 0..REFRESH_COUNT {
            respond_empty(&mut stream).await;
        }
    });
    let executable = std::env::var(HOST_BINARY_ENVIRONMENT).unwrap();
    let mut helper = ChildGuard::spawn(
        Command::new(executable)
            .args(yolo_launch_contract::ProcessRole::Host.arguments())
            .args([
                cli::CATALOG_STREAM_COMMAND,
                cli::EXISTING_SOCKET_ARGUMENT,
                cli::SOCKET_ARGUMENT,
            ])
            .arg(&socket),
    );
    let helper_pid = helper.child.id();
    let mut input = helper.child.stdin.take().unwrap();
    let mut output = helper.child.stdout.take().unwrap();
    for _ in 0..REFRESH_COUNT {
        let identifier = uuid::Uuid::new_v4();
        write_catalog_frame(
            &mut input,
            &json!({
                "id": identifier,
                "command": cli::LIST_COMMAND,
                (field::DEADLINE_MS): cli::CATALOG_DEFAULT_DEADLINE_MILLISECONDS,
            }),
        );
        let response = read_catalog_frame(&mut output);
        assert_eq!(response[field::ID], identifier.to_string());
        assert_eq!(response[field::OK], true);
        assert_eq!(response[field::SESSIONS], json!([]));
        assert_eq!(helper.child.id(), helper_pid);
        assert!(helper.child.try_wait().unwrap().is_none());
    }
    drop(input);
    assert!(helper.wait().success());
    server.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn downstream_timeout_keeps_sidecar_and_recovers_connection() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("timeout-recovery.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (hung, _) = listener.accept().await.unwrap();
        let mut hung = yolo_runtime_rpc_testing::Peer::new(hung);
        respond_empty(&mut hung).await;
        let _: RequestEnvelope = hung.request().await.unwrap();
        tokio::time::sleep(HUNG_SERVER_RELEASE_DELAY).await;
        drop(hung);
        let (recovered, _) = listener.accept().await.unwrap();
        let mut recovered = yolo_runtime_rpc_testing::Peer::new(recovered);
        respond_empty(&mut recovered).await;
        RECOVERY_CONNECTION_COUNT
    });
    let executable = std::env::var(HOST_BINARY_ENVIRONMENT).unwrap();
    let mut helper = ChildGuard::spawn(
        Command::new(executable)
            .args(yolo_launch_contract::ProcessRole::Host.arguments())
            .args([
                cli::CATALOG_STREAM_COMMAND,
                cli::EXISTING_SOCKET_ARGUMENT,
                cli::SOCKET_ARGUMENT,
            ])
            .arg(&socket),
    );
    let helper_pid = helper.child.id();
    let mut input = helper.child.stdin.take().unwrap();
    let mut output = helper.child.stdout.take().unwrap();

    let warmup_identifier = uuid::Uuid::new_v4();
    write_catalog_request(
        &mut input,
        warmup_identifier,
        cli::CATALOG_DEFAULT_DEADLINE_MILLISECONDS,
    );
    let warmup = read_catalog_frame(&mut output);
    assert_eq!(warmup[field::ID], warmup_identifier.to_string());
    assert_eq!(warmup[field::OK], true);

    let timed_out_identifier = uuid::Uuid::new_v4();
    let started = Instant::now();
    write_catalog_request(
        &mut input,
        timed_out_identifier,
        HUNG_REQUEST_DEADLINE_MILLISECONDS,
    );
    let timed_out = read_catalog_frame(&mut output);
    assert!(started.elapsed() < OUTER_TYPED_TIMEOUT_BOUND);
    assert_eq!(timed_out[field::ID], timed_out_identifier.to_string());
    assert_eq!(timed_out[field::OK], false);
    assert_eq!(
        timed_out[field::ERROR][field::KIND],
        failure_kind::TRANSPORT
    );
    assert_eq!(helper.child.id(), helper_pid);
    assert!(helper.child.try_wait().unwrap().is_none());

    let recovered_identifier = uuid::Uuid::new_v4();
    write_catalog_request(
        &mut input,
        recovered_identifier,
        cli::CATALOG_DEFAULT_DEADLINE_MILLISECONDS,
    );
    let recovered = read_catalog_frame(&mut output);
    assert_eq!(recovered[field::ID], recovered_identifier.to_string());
    assert_eq!(recovered[field::OK], true);
    assert_eq!(recovered[field::SESSIONS], json!([]));
    assert_eq!(helper.child.id(), helper_pid);

    drop(input);
    assert!(helper.wait().success());
    assert_eq!(server.await.unwrap(), RECOVERY_CONNECTION_COUNT);
}

fn write_catalog_request(
    input: &mut impl std::io::Write,
    identifier: uuid::Uuid,
    deadline_milliseconds: u64,
) {
    write_catalog_frame(
        input,
        &json!({
            "id": identifier,
            "command": cli::LIST_COMMAND,
            (field::DEADLINE_MS): deadline_milliseconds,
        }),
    );
}
