use super::constants::*;
use super::rpc::request;
use std::path::Path;
use uuid::Uuid;
use yolo_remote::{CommandPolicy, FileReadPolicy};
use yolo_remote_broker::{
    BrokerCommandRequest, BrokerConnectionPool, BrokerFileRequest, BrokerResult,
};
use yolo_runtime::lease::LeaseRequest;
use yolo_runtime::model::{WriterCapability, WriterLeaseGrant};
use yolo_runtime::protocol::method;
use yolo_runtime::worker::{InputRequest, InputResult, RuntimeRef};

pub(super) async fn assert_multiplexed(
    pool: &BrokerConnectionPool,
    command_policy: &CommandPolicy,
    file_policy: &FileReadPolicy,
    path: &Path,
) {
    let command = pool.run_command(
        command_policy,
        BrokerCommandRequest::standard(PRINTF_PROGRAM, vec![COMMAND_MARKER.into()]),
    );
    let file = pool.read_file(
        file_policy,
        BrokerFileRequest {
            path: path.to_path_buf(),
            offset: 0,
            length: FILE_FIXTURE_CONTENT.len() as u64,
        },
    );
    let (command, file) = tokio::join!(command, file);
    let expected_connection = pool.status().await.connection_id.unwrap();
    match command.unwrap() {
        BrokerResult::Command {
            connection_id,
            output,
        } => {
            assert_eq!(connection_id, expected_connection);
            assert_eq!(output.stdout, COMMAND_MARKER.as_bytes());
            assert_eq!(output.exit_status, Some(0));
        }
        other => panic!("unexpected command result: {other:?}"),
    }
    match file.unwrap() {
        BrokerResult::FileRead {
            connection_id,
            result,
        } => {
            assert_eq!(connection_id, expected_connection);
            assert_eq!(result.data, FILE_FIXTURE_CONTENT);
        }
        other => panic!("unexpected file result: {other:?}"),
    }
}

pub(super) async fn write_terminal_marker(control: &mut yolo_runtime_rpc::RuntimeConnection, target: RuntimeRef) {
    let capability = WriterCapability::generate();
    let lease: WriterLeaseGrant = request(
        control,
        method::ACQUIRE_WRITER,
        &LeaseRequest {
            terminal_id: target.terminal_id,
            runtime_epoch: target.runtime_epoch,
            holder_id: TEST_WRITER_HOLDER.into(),
            ttl_ms: None,
            generation: None,
            controller_instance_id: None,
            writer_capability: capability,
        },
    )
    .await;
    let input: InputResult = request(
        control,
        method::TERMINAL_INPUT,
        &InputRequest {
            terminal_id: target.terminal_id,
            runtime_epoch: target.runtime_epoch,
            generation: lease.generation,
            writer_capability: lease.writer_capability,
            input_id: Uuid::new_v4(),
            data: PTY_COMMAND.to_vec(),
        },
    )
    .await;
    assert_eq!(input.delivery, yolo_runtime::protocol::delivery::DELIVERED);
    assert!(!input.duplicate);
}
