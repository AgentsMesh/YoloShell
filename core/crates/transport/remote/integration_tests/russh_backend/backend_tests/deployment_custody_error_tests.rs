use super::constants::*;
use super::deployment_support::{artifact, runtime, wait_for_temporary_entries};
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::{RuntimeDeploymentResult, RuntimeInstaller};

#[tokio::test]
async fn lost_upload_creation_response_is_cleaned_before_retry() {
    assert_lost_creation_converges(FailurePoint::SftpOpenResponsePending).await;
}

#[tokio::test]
async fn lost_link_creation_response_is_cleaned_before_retry() {
    assert_lost_creation_converges(FailurePoint::SftpSymlinkResponsePending).await;
}

async fn assert_lost_creation_converges(failure: FailurePoint) {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(failure);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();

    let first = install(&fixture.connection).await;

    assert!(first.is_err());
    wait_for_temporary_entries(&filesystem, NO_TEMPORARY_ENTRIES).await;
    assert_eq!(
        install(&fixture.connection).await.unwrap(),
        RuntimeDeploymentResult::Installed
    );
    assert!(filesystem.target(MEMORY_RUNTIME).is_some());
    assert_eq!(filesystem.temporary_entries(), NO_TEMPORARY_ENTRIES);
}

async fn install(
    connection: &std::sync::Arc<super::super::RusshConnection>,
) -> Result<RuntimeDeploymentResult, crate::remote::RemoteError> {
    RuntimeInstaller
        .ensure(
            connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await
}
