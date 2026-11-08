use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_timeout};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::*;
use std::future::Future;
use std::sync::Arc;

#[tokio::test]
async fn command_open_timeout_disconnects_an_unconfirming_peer() {
    let fixture = hostile_fixture().await;
    let command = authorized_command(NORMAL_PROGRAM);
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move { connection.execute(&command).await.map(|_| ()) });
    assert_unconfirmed_transport(&fixture, operation).await;
}

#[tokio::test]
async fn runtime_open_timeout_disconnects_an_unconfirming_peer() {
    let fixture = hostile_fixture().await;
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    let request = RuntimeChannelRequest::control(runtime);
    let connection = Arc::clone(&fixture.connection);
    let operation =
        tokio::spawn(async move { connection.open_runtime(&request).await.map(|_| ()) });
    assert_unconfirmed_transport(&fixture, operation).await;
}

#[tokio::test]
async fn posix_rename_open_timeout_disconnects_an_unconfirming_peer() {
    let fixture = hostile_fixture().await;
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move {
        connection
            .posix_rename(MEMORY_RUNTIME, MEMORY_OUTSIDE)
            .await
    });
    assert_unconfirmed_transport(&fixture, operation).await;
}

#[tokio::test]
async fn capability_open_timeout_disconnects_an_unconfirming_peer() {
    let fixture = hostile_fixture().await;
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move { connection.require_file_fsync().await });
    assert_unconfirmed_transport(&fixture, operation).await;
}

#[tokio::test]
async fn sftp_open_timeout_disconnects_an_unconfirming_peer() {
    let fixture = hostile_fixture().await;
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move {
        connection
            .canonicalize(std::path::Path::new(MEMORY_FILE))
            .await
            .map(|_| ())
    });
    assert_unconfirmed_transport(&fixture, operation).await;
}

#[tokio::test]
async fn cleanup_sftp_open_timeout_disconnects_before_task_abort() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let custody = fixture
        .connection
        .temporary_path_custody(MEMORY_OUTSIDE)
        .unwrap();
    filesystem.fail_once(FailurePoint::ChannelOpen);
    drop(custody);
    filesystem.wait_channel_open_pending().await;

    let result = fixture
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(SHORT_OPERATION_TIMEOUT))
        .await;
    assert!(result.is_err());
    finish_hostile_disconnect(&fixture).await;
}

#[tokio::test]
async fn cancelled_delayed_open_closes_exactly_without_disconnect() {
    let fixture = hostile_fixture().await;
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move { connection.require_file_fsync().await });
    fixture.filesystem.wait_channel_open_pending().await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    fixture.filesystem.release_channel_open();
    fixture.filesystem.wait_channel_open_accepted().await;
    barrier.release();
    wait_for_close(&fixture).await;

    assert!(!fixture.connection.session.is_closed());
    let output = fixture
        .connection
        .execute(&authorized_command(NORMAL_PROGRAM))
        .await
        .unwrap();
    assert_eq!(output.value.stdout, COMMAND_STDOUT);
}

#[tokio::test]
async fn confirmed_command_execution_timeout_remains_a_command_result() {
    let fixture =
        connect_memory_with_timeout(false, MemoryFilesystem::default(), SHORT_OPERATION_TIMEOUT)
            .await
            .unwrap();
    let output = fixture
        .connection
        .execute(&authorized_command(HANG_PROGRAM))
        .await
        .unwrap();
    assert!(output.value.timed_out);
    assert!(!fixture.connection.session.is_closed());
}

async fn hostile_fixture() -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::ChannelOpen);
    connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap()
}

fn authorized_command(program: &str) -> AuthorizedCommand {
    CommandPolicy::new([program.into()])
        .unwrap()
        .authorize(
            CommandRequest::new(program, vec![])
                .with_limits(SHORT_OPERATION_TIMEOUT, TEST_COMMAND_OUTPUT_LIMIT),
        )
        .unwrap()
}

async fn assert_unconfirmed_transport(
    fixture: &MemoryConnection,
    operation: impl Future<Output = Result<Result<(), RemoteError>, tokio::task::JoinError>>,
) {
    fixture.filesystem.wait_channel_open_pending().await;
    assert!(matches!(
        operation.await.unwrap(),
        Err(RemoteError::Transport(_))
    ));
    finish_hostile_disconnect(fixture).await;
}

async fn wait_for_close(fixture: &MemoryConnection) {
    tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while fixture.filesystem.command_closes() != EXPECTED_COMMAND_CLEANUPS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn finish_hostile_disconnect(fixture: &MemoryConnection) {
    tokio::time::timeout(TEST_INACTIVITY_TIMEOUT, async {
        while !fixture.connection.session.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    fixture.filesystem.release_channel_open();
    tokio::time::timeout(
        TEST_INACTIVITY_TIMEOUT,
        fixture.filesystem.wait_connection_closed(),
    )
    .await
    .unwrap();
}
