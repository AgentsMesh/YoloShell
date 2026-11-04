use super::constants::*;
use super::deployment_support::{artifact, runtime, wait_for_temporary_entries};
use super::fixture::{MemoryConnection, connect_memory_with_fs};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::{RuntimeDeploymentResult, RuntimeInstaller};

#[tokio::test]
async fn cancelled_upload_is_cleaned_before_a_retry_converges() {
    let failure = FailurePoint::SftpOpenResponsePending;
    let fixture = cancellation_fixture(failure).await;
    let install = spawn_install(&fixture);
    wait_for_failure_consumed(&fixture, failure, EXPECTED_TEMPORARY_ENTRIES).await;

    install.abort();
    assert!(install.await.unwrap_err().is_cancelled());
    wait_for_temporary_entries(&fixture.filesystem, NO_TEMPORARY_ENTRIES).await;

    assert_eq!(retry(&fixture).await, RuntimeDeploymentResult::Installed);
    assert_eq!(fixture.filesystem.temporary_entries(), NO_TEMPORARY_ENTRIES);
}

#[tokio::test]
async fn cancelled_activation_link_is_cleaned_before_a_retry_converges() {
    let failure = FailurePoint::SftpReadLinkHang;
    let fixture = cancellation_fixture(failure).await;
    let install = spawn_install(&fixture);
    wait_for_failure_consumed(&fixture, failure, EXPECTED_TEMPORARY_ENTRIES).await;

    install.abort();
    assert!(install.await.unwrap_err().is_cancelled());
    wait_for_temporary_entries(&fixture.filesystem, NO_TEMPORARY_ENTRIES).await;

    assert_eq!(retry(&fixture).await, RuntimeDeploymentResult::Installed);
    assert!(fixture.filesystem.target(MEMORY_RUNTIME).is_some());
    assert_eq!(fixture.filesystem.temporary_entries(), NO_TEMPORARY_ENTRIES);
}

#[tokio::test]
async fn direct_disconnect_waits_for_registered_deployment_cleanup() {
    let failure = FailurePoint::SftpOpenResponsePending;
    let fixture = cancellation_fixture(failure).await;
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let install = spawn_install(&fixture);
    wait_for_failure_consumed(&fixture, failure, EXPECTED_TEMPORARY_ENTRIES).await;

    install.abort();
    assert!(install.await.unwrap_err().is_cancelled());
    let connection = fixture.connection.clone();
    let disconnect = tokio::spawn(async move { connection.disconnect().await });
    barrier.wait_entered().await;
    assert!(!disconnect.is_finished());

    barrier.release();
    disconnect.await.unwrap().unwrap();
    wait_for_temporary_entries(&fixture.filesystem, NO_TEMPORARY_ENTRIES).await;
}

#[tokio::test]
async fn deployment_reserves_cleanup_before_fsync_capability_channel() {
    let failure = FailurePoint::SftpInitPending;
    let fixture = cancellation_fixture(failure).await;
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let install = spawn_install(&fixture);
    wait_for_failure_consumed(&fixture, failure, NO_TEMPORARY_ENTRIES).await;

    install.abort();
    assert!(install.await.unwrap_err().is_cancelled());
    let connection = fixture.connection.clone();
    let disconnect = tokio::spawn(async move { connection.disconnect().await });
    barrier.wait_entered().await;
    assert!(!disconnect.is_finished());

    barrier.release();
    disconnect.await.unwrap().unwrap();
    assert_eq!(fixture.filesystem.temporary_entries(), NO_TEMPORARY_ENTRIES);
}

async fn cancellation_fixture(failure: FailurePoint) -> MemoryConnection {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(failure);
    connect_memory_with_fs(false, filesystem).await.unwrap()
}

async fn wait_for_failure_consumed(
    fixture: &MemoryConnection,
    failure: FailurePoint,
    expected_entries: usize,
) {
    let consumed = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while fixture.filesystem.failure_is_armed(failure) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(consumed.is_ok());
    assert_eq!(fixture.filesystem.temporary_entries(), expected_entries);
}

fn spawn_install(fixture: &MemoryConnection) -> tokio::task::JoinHandle<()> {
    let connection = fixture.connection.clone();
    tokio::spawn(async move {
        let result = RuntimeInstaller
            .ensure(
                connection.as_ref(),
                &runtime(),
                &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
            )
            .await;
        assert!(result.is_ok());
    })
}

async fn retry(fixture: &MemoryConnection) -> RuntimeDeploymentResult {
    RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await
        .unwrap()
}
