use super::constants::*;
use super::deployment_support::{artifact, runtime};
use super::fixture::{connect_memory_with_fs, connect_memory_with_timeout};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::FSYNC_REQUIRED_MESSAGE;
use crate::remote::{RemoteError, RuntimeInstaller};
use std::time::Instant;

#[tokio::test]
async fn deployment_refuses_servers_without_fsync_before_writing() {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.set_fsync_supported(false);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let error = RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message) if message == FSYNC_REQUIRED_MESSAGE
    ));
    assert_eq!(filesystem.writes(), 0);
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn fsync_failure_cleans_upload_and_preserves_stable_runtime() {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await
        .unwrap();
    let stable = filesystem.target(MEMORY_RUNTIME);
    filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    filesystem.fail_once(FailurePoint::FsyncFailure);
    assert!(
        RuntimeInstaller
            .ensure(
                fixture.connection.as_ref(),
                &runtime(),
                &artifact(UPGRADED_RUNTIME_VERSION, UPGRADED_BINARY_FILL),
            )
            .await
            .is_err()
    );
    assert_eq!(filesystem.target(MEMORY_RUNTIME), stable);
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn fsync_capability_setup_failures_are_bounded() {
    for failure in [
        FailurePoint::ChannelOpen,
        FailurePoint::SftpTransport,
        FailurePoint::SftpInit,
    ] {
        let filesystem = MemoryFilesystem::default();
        filesystem.fail_once(failure);
        let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
            .await
            .unwrap();
        let started = Instant::now();
        assert!(fixture.connection.require_file_fsync().await.is_err());
        assert!(started.elapsed().as_millis() < MAX_OPERATION_ELAPSED_MS);
    }
}
