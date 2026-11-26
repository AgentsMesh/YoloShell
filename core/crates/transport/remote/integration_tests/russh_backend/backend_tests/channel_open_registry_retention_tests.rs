use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_timeout};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn registry_timeout_preserves_delayed_open_cleanup() {
    let fixture = hostile_fixture().await;
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move { connection.require_file_fsync().await });
    fixture.filesystem.wait_channel_open_pending().await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    assert!(
        fixture
            .connection
            .cleanup
            .settle(Duration::ZERO)
            .await
            .is_err()
    );
    fixture.filesystem.release_channel_open();
    fixture.filesystem.wait_channel_open_accepted().await;
    barrier.release();
    wait_for_exact_close(&fixture).await;

    assert!(!fixture.connection.session.is_closed());
}

#[tokio::test]
async fn registry_timeout_preserves_unconfirmed_open_escalation() {
    let fixture = hostile_fixture().await;
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let connection = Arc::clone(&fixture.connection);
    let operation = tokio::spawn(async move { connection.require_file_fsync().await });
    fixture.filesystem.wait_channel_open_pending().await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    assert!(
        fixture
            .connection
            .cleanup
            .settle(Duration::ZERO)
            .await
            .is_err()
    );
    barrier.release();

    finish_hostile_disconnect(&fixture).await;
}

#[tokio::test]
async fn registry_timeout_preserves_cleanup_sftp_supervisor() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let custody = fixture
        .connection
        .temporary_path_custody(MEMORY_OUTSIDE)
        .unwrap();
    filesystem.fail_once(FailurePoint::ChannelOpen);
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    drop(custody);
    barrier.wait_entered().await;
    assert!(
        fixture
            .connection
            .cleanup
            .settle(Duration::ZERO)
            .await
            .is_err()
    );
    barrier.release();
    filesystem.wait_channel_open_pending().await;

    finish_hostile_disconnect(&fixture).await;
}

async fn hostile_fixture() -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::ChannelOpen);
    connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap()
}

async fn wait_for_exact_close(fixture: &MemoryConnection) {
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
