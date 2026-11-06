use super::super::*;
use super::support::{CONFIG_REVISION, HOST_ID};
use crate::remote::{CommandOutput, FileReadResult};
use std::fs;

#[path = "protocol_client_server.rs"]
mod client_server;
use client_server::{CONNECTION_GENERATION, ReplyKind, Server};
#[path = "protocol_bootstrap_tests.rs"]
mod bootstrap;

const CLIENT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);
const ADOPTED_FIELD: &str = "adopted";
const CAPABILITIES_FIELD: &str = "capabilities";

#[tokio::test]
async fn typed_client_methods_reject_crossed_results_and_shutdown_revision() {
    let server = Server::start(ReplyKind::Status).await;
    assert!(BrokerClient::new(&server.path).bootstrap().await.is_err());
    server.task.await.unwrap();

    let server = Server::start(ReplyKind::Status).await;
    assert!(
        BrokerClient::new(&server.path)
            .working_directory()
            .await
            .is_err()
    );
    server.task.await.unwrap();

    let server = Server::start(ReplyKind::Status).await;
    assert!(
        BrokerClient::new(&server.path)
            .run_command(BrokerCommandRequest::standard("/bin/echo", Vec::new()))
            .await
            .is_err()
    );
    server.task.await.unwrap();

    let server = Server::start(ReplyKind::Status).await;
    assert!(
        BrokerClient::new(&server.path)
            .read_file(BrokerFileRequest {
                path: "/srv/file".into(),
                offset: 0,
                length: 1,
            })
            .await
            .is_err()
    );
    server.task.await.unwrap();

    for kind in [
        ReplyKind::Status,
        ReplyKind::WrongShutdownRevision,
        ReplyKind::WrongShutdownInstance,
    ] {
        let server = Server::start(kind).await;
        assert!(
            BrokerClient::new(&server.path)
                .shutdown(CONFIG_REVISION.into())
                .await
                .is_err()
        );
        server.task.await.unwrap();
    }
}

#[tokio::test]
async fn client_decodes_status_and_validates_response_correlation() {
    let server = Server::start(ReplyKind::Status).await;
    let client = BrokerClient::new(&server.path);
    assert_eq!(client.socket_path(), server.path);
    let status = client.status().await.unwrap();
    assert_eq!(status.host_id, HOST_ID);
    assert_eq!(status.connection_generation, CONNECTION_GENERATION);
    server.task.await.unwrap();

    for (kind, expected) in [
        (ReplyKind::WrongVersion, "incompatible"),
        (ReplyKind::WrongId, "request id"),
        (ReplyKind::Failure, "denied"),
        (ReplyKind::WrongResult, "unexpected"),
    ] {
        let server = Server::start(kind).await;
        let error = BrokerClient::new(&server.path)
            .status()
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains(expected));
        server.task.await.unwrap();
    }
}

#[tokio::test]
async fn legacy_status_without_required_capability_is_retired() {
    let server = Server::start(ReplyKind::LegacyStatus).await;
    let identity = super::super::socket_identity::socket_identity(&server.path).unwrap();
    let transition = BrokerClient::new(&server.path)
        .transition_at(
            CONFIG_REVISION,
            identity,
            tokio::time::Instant::now() + CLIENT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
    assert_eq!(transition, BrokerTransition::Shutdown);
    server.task.await.unwrap();
}

#[tokio::test]
async fn matching_status_remains_pending_until_adoption_commits() {
    let server = Server::start(ReplyKind::PendingStatus).await;
    let identity = super::super::socket_identity::socket_identity(&server.path).unwrap();
    let transition = BrokerClient::new(&server.path)
        .transition_at(
            CONFIG_REVISION,
            identity,
            tokio::time::Instant::now() + CLIENT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
    let BrokerTransition::Pending(status) = transition else {
        panic!("matching pre-adoption broker was reported current");
    };
    assert!(!status.adopted);
    server.task.await.unwrap();
}

#[tokio::test]
async fn client_rejects_unsafe_socket_before_connecting() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("management.sock");
    fs::write(&path, b"not a socket").unwrap();
    assert!(matches!(
        BrokerClient::new(path).status().await,
        Err(BrokerClientError::UnsafeSocket(_))
    ));
}

#[test]
fn protocol_models_preserve_binary_outputs_and_defaults() {
    let command = BrokerCommandRequest::standard("/bin/echo", vec!["ok".into()]);
    assert_eq!(command.timeout_millis, DEFAULT_COMMAND_TIMEOUT_MILLIS);
    assert_eq!(command.output_limit, DEFAULT_COMMAND_OUTPUT_BYTES);
    let envelope = BrokerRequestEnvelope::new(BrokerRequest::Command(command));
    assert_eq!(
        envelope.protocol_version,
        super::super::constants::BROKER_PROTOCOL_VERSION
    );
    assert_eq!(
        serde_json::to_value(BrokerRequest::Bootstrap).unwrap(),
        serde_json::json!(crate::cli::constants::BOOTSTRAP_COMMAND)
    );

    let output: super::super::protocol::BrokerCommandOutput = CommandOutput {
        stdout: vec![0, 1],
        stderr: vec![2],
        exit_status: Some(7),
        timed_out: true,
        truncated: true,
    }
    .into();
    assert_eq!(output.stdout, [0, 1]);
    let file: super::super::protocol::BrokerFileResult = FileReadResult {
        resolved_path: "/srv/file".into(),
        data: vec![3, 4],
        eof: false,
    }
    .into();
    assert_eq!(file.data, [3, 4]);
    let status =
        BrokerStatus::disconnected(uuid::Uuid::nil(), HOST_ID.into(), CONFIG_REVISION.into(), 0);
    assert!(status.supports(CONNECTION_BOOTSTRAP_CAPABILITY));
    assert!(status.supports(WORKING_DIRECTORY_CAPABILITY));
    for field in [ADOPTED_FIELD, CAPABILITIES_FIELD] {
        let mut incomplete = serde_json::to_value(&status).unwrap();
        incomplete.as_object_mut().unwrap().remove(field).unwrap();
        assert!(serde_json::from_value::<BrokerStatus>(incomplete.clone()).is_err());
        let encoded = crate::protocol::to_value(&incomplete).unwrap();
        assert!(crate::protocol::from_value::<BrokerStatus>(encoded).is_err());
    }
}
