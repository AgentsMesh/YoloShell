use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_fs};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::MultiplexedSshConnection;
use std::path::Path;

#[tokio::test]
async fn ordinary_sftp_cancellation_registers_exact_close_and_preserves_transport() {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpCanonicalize);
    let fixture = connect_memory_with_fs(false, filesystem).await.unwrap();
    let first_close = next_count(fixture.filesystem.command_closes());
    let second_close = next_count(first_close);
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let operation = spawn_canonicalize(&fixture);
    wait_until(&fixture, |fixture| {
        !fixture
            .filesystem
            .failure_is_armed(FailurePoint::SftpCanonicalize)
    })
    .await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    assert!(!fixture.connection.session.is_closed());

    barrier.release();
    wait_for_channel_closes(&fixture, first_close).await;
    assert!(!fixture.connection.session.is_closed());
    fixture
        .connection
        .canonicalize(Path::new(MEMORY_FILE))
        .await
        .unwrap();
    wait_for_channel_closes(&fixture, second_close).await;
    fixture.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn posix_rename_cancellation_registers_exact_close_and_preserves_transport() {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::ActivationResponseLoss);
    let fixture = connect_memory_with_fs(false, filesystem).await.unwrap();
    let first_close = next_count(fixture.filesystem.command_closes());
    let second_close = next_count(first_close);
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let connection = fixture.connection.clone();
    let operation =
        tokio::spawn(async move { connection.posix_rename(MEMORY_LINK, MEMORY_RUNTIME).await });
    wait_until(&fixture, |fixture| {
        !fixture
            .filesystem
            .failure_is_armed(FailurePoint::ActivationResponseLoss)
    })
    .await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    barrier.wait_entered().await;
    assert!(!fixture.connection.session.is_closed());

    barrier.release();
    wait_for_channel_closes(&fixture, first_close).await;
    assert!(!fixture.connection.session.is_closed());
    fixture.connection.require_file_fsync().await.unwrap();
    wait_for_channel_closes(&fixture, second_close).await;
    fixture.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn direct_disconnect_waits_for_cancelled_sftp_exact_close() {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpCanonicalize);
    let fixture = connect_memory_with_fs(false, filesystem).await.unwrap();
    let first_close = next_count(fixture.filesystem.command_closes());
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let operation = spawn_canonicalize(&fixture);
    wait_until(&fixture, |fixture| {
        !fixture
            .filesystem
            .failure_is_armed(FailurePoint::SftpCanonicalize)
    })
    .await;

    operation.abort();
    assert!(operation.await.unwrap_err().is_cancelled());
    let connection = fixture.connection.clone();
    let disconnect = tokio::spawn(async move { connection.disconnect().await });
    barrier.wait_entered().await;
    assert!(!disconnect.is_finished());

    barrier.release();
    disconnect.await.unwrap().unwrap();
    wait_for_channel_closes(&fixture, first_close).await;
}

fn spawn_canonicalize(fixture: &MemoryConnection) -> tokio::task::JoinHandle<()> {
    let connection = fixture.connection.clone();
    tokio::spawn(async move {
        let _ = connection.canonicalize(Path::new(MEMORY_FILE)).await;
    })
}

async fn wait_for_channel_closes(fixture: &MemoryConnection, expected: usize) {
    let settled = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while fixture.filesystem.command_closes() < expected {
            tokio::task::yield_now().await;
        }
    })
    .await;
    settled.unwrap();
    assert_eq!(fixture.filesystem.command_closes(), expected);
}

fn next_count(current: usize) -> usize {
    current.checked_add(COUNTER_INCREMENT).unwrap()
}

async fn wait_until(fixture: &MemoryConnection, condition: impl Fn(&MemoryConnection) -> bool) {
    tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while !condition(fixture) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
