use super::constants::*;
use super::deployment_support::{artifact, artifact_with_size, runtime};
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::{
    RUNTIME_DIRECTORY_MODE, RUNTIME_LINK_PREFIX, RUNTIME_UPLOAD_MODE, RUNTIME_UPLOAD_PREFIX,
};
use crate::remote::{ChannelPurpose, RuntimeDeploymentResult, RuntimeInstaller};

const CLEANUP_SUFFIX: &str = "coverage-cleanup";
const ROOT_PATH: &str = "/";
const UPLOAD_BYTES: &[u8] = b"upload";

#[tokio::test]
async fn lost_mkdir_response_converges_on_the_created_private_parent() {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(FailurePoint::MkdirResponseLoss);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();

    let result = RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await
        .unwrap();

    assert_eq!(result, RuntimeDeploymentResult::Installed);
    assert!(filesystem.target(MEMORY_RUNTIME).is_some());
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn pipelined_upload_reports_a_write_rejection_before_flush() {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(FailurePoint::UploadWrite);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let artifact = artifact_with_size(
        INITIAL_RUNTIME_VERSION,
        INITIAL_BINARY_FILL,
        WRITE_ERROR_ARTIFACT_BYTES,
    );

    let result = RuntimeInstaller
        .ensure(fixture.connection.as_ref(), &runtime(), &artifact)
        .await;

    assert!(result.is_err());
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn an_existing_release_directory_is_never_accepted_as_an_executable() {
    let filesystem = installed_filesystem().await;
    let release = filesystem.target(MEMORY_RUNTIME).unwrap();
    filesystem.remove(&release);
    filesystem.insert_directory(&release, RUNTIME_DIRECTORY_MODE);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();

    let result = RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(filesystem.target(MEMORY_RUNTIME), Some(release));
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn activation_rejects_a_successful_rename_that_reselects_the_previous_release() {
    let filesystem = installed_filesystem().await;
    let previous = filesystem.target(MEMORY_RUNTIME).unwrap();
    filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    filesystem.fail_once(FailurePoint::ActivationPreviousTarget);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();

    let result = RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(UPGRADED_RUNTIME_VERSION, UPGRADED_BINARY_FILL),
        )
        .await;

    assert!(result.is_err());
    assert_eq!(filesystem.target(MEMORY_RUNTIME), Some(previous));
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn temporary_cleanup_is_strictly_scoped_to_managed_names() {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let (_, sftp) = fixture
        .connection
        .open_sftp(ChannelPurpose::RuntimeDeployment)
        .await
        .unwrap();
    let upload = managed_temporary(RUNTIME_UPLOAD_PREFIX);
    let link = managed_temporary(RUNTIME_LINK_PREFIX);
    filesystem.insert_file(&upload, UPLOAD_BYTES, RUNTIME_UPLOAD_MODE);
    filesystem.insert_link(&link, MEMORY_OUTSIDE);

    for retained in [ROOT_PATH, MEMORY_OUTSIDE] {
        super::super::deployment_fs::cleanup_temporary(&sftp, retained, NORMAL_OPERATION_TIMEOUT)
            .await
            .unwrap();
    }
    assert!(filesystem.contains_file(MEMORY_OUTSIDE));
    super::super::deployment_fs::cleanup_temporary(&sftp, &upload, NORMAL_OPERATION_TIMEOUT)
        .await
        .unwrap();
    super::super::deployment_fs::cleanup_temporary(&sftp, &link, NORMAL_OPERATION_TIMEOUT)
        .await
        .unwrap();

    assert!(!filesystem.contains_file(&upload));
    assert!(filesystem.target(&link).is_none());
    sftp.close().await.unwrap();
}

async fn installed_filesystem() -> MemoryFilesystem {
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
    filesystem
}

fn managed_temporary(prefix: &str) -> String {
    format!("{MEMORY_RUNTIME_PARENT}/{prefix}{CLEANUP_SUFFIX}")
}
