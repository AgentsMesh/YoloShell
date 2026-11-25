use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_timeout};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::*;
use std::path::Path;

#[tokio::test]
async fn ordinary_sftp_init_timeout_disconnects_before_releasing_custody() {
    let fixture = pending_init_fixture().await;
    assert!(matches!(
        fixture
            .connection
            .canonicalize(Path::new(MEMORY_FILE))
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn ordinary_sftp_init_immediate_error_disconnects_transport() {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpLimitsRejected);
    let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    assert!(matches!(
        fixture
            .connection
            .canonicalize(Path::new(MEMORY_FILE))
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn confirmed_open_failure_fail_stops_the_live_transport() {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpConfirmedOpenFailure);
    let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    assert!(matches!(
        fixture
            .connection
            .canonicalize(Path::new(MEMORY_FILE))
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn capability_init_timeout_disconnects_before_releasing_custody() {
    let fixture = pending_init_fixture().await;
    assert!(matches!(
        fixture.connection.require_file_fsync().await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn capability_init_immediate_error_disconnects_transport() {
    let fixture = immediate_init_fixture().await;
    assert!(matches!(
        fixture.connection.require_file_fsync().await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn posix_rename_init_timeout_disconnects_before_releasing_custody() {
    let fixture = pending_init_fixture().await;
    assert!(matches!(
        fixture
            .connection
            .posix_rename(MEMORY_RUNTIME, MEMORY_OUTSIDE)
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn posix_rename_init_immediate_error_disconnects_transport() {
    let fixture = immediate_init_fixture().await;
    assert!(matches!(
        fixture
            .connection
            .posix_rename(MEMORY_RUNTIME, MEMORY_OUTSIDE)
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn cleanup_sftp_init_timeout_disconnects_before_supervisor_abort() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let custody = fixture
        .connection
        .temporary_path_custody(MEMORY_OUTSIDE)
        .unwrap();
    filesystem.fail_once(FailurePoint::SftpInitPending);
    drop(custody);

    assert!(
        fixture
            .connection
            .cleanup
            .settle(cleanup_settlement_timeout(SHORT_OPERATION_TIMEOUT))
            .await
            .is_err()
    );
    assert_disconnected(&fixture).await;
}

#[tokio::test]
async fn cleanup_sftp_init_immediate_error_disconnects_transport() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    let custody = fixture
        .connection
        .temporary_path_custody(MEMORY_OUTSIDE)
        .unwrap();
    filesystem.fail_once(FailurePoint::SftpLimitsRejected);
    drop(custody);

    assert!(
        fixture
            .connection
            .cleanup
            .settle(cleanup_settlement_timeout(SHORT_OPERATION_TIMEOUT))
            .await
            .is_err()
    );
    assert_disconnected(&fixture).await;
}

async fn pending_init_fixture() -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpInitPending);
    connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap()
}

async fn immediate_init_fixture() -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.fail_once(FailurePoint::SftpInit);
    connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap()
}

async fn assert_disconnected(fixture: &MemoryConnection) {
    tokio::time::timeout(TEST_INACTIVITY_TIMEOUT, async {
        while !fixture.connection.session.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::timeout(
        TEST_INACTIVITY_TIMEOUT,
        fixture.filesystem.wait_connection_closed(),
    )
    .await
    .unwrap();
}
