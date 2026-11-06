use super::constants::*;
use super::deployment_support::{artifact, artifact_for, runtime};
use super::fixture::{connect_memory_with_fs, trusted_hosts};
use super::memory_fs::MemoryFilesystem;
use crate::remote::{
    MultiplexedSshConnection, RemoteArchitecture, RemoteHost, RuntimeDeploymentResult,
    RuntimeInstaller,
};

#[tokio::test]
async fn version_architecture_and_reconnect_are_checked_without_stopping_runtime() {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    let first = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let known = trusted_hosts(first.connection.info());
    let initial_artifact = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
    let (mut host, result) = RemoteHost::connect_with_artifact(
        first.connection.clone(),
        &known,
        runtime(),
        &initial_artifact,
    )
    .await
    .unwrap();
    assert_eq!(result, RuntimeDeploymentResult::Installed);
    let stable = filesystem.target(MEMORY_RUNTIME);
    host.mark_transport_lost();
    first.connection.disconnect().await.unwrap();

    let replacement = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let replacement_known = trusted_hosts(replacement.connection.info());
    let report = host
        .reconnect(replacement.connection.clone(), &replacement_known)
        .await
        .unwrap();
    assert!(report.same_boot);
    assert_eq!(
        RuntimeInstaller
            .ensure(
                replacement.connection.as_ref(),
                &runtime(),
                &initial_artifact,
            )
            .await
            .unwrap(),
        RuntimeDeploymentResult::Current
    );
    assert_eq!(filesystem.target(MEMORY_RUNTIME), stable);

    filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    let wrong_version = artifact(INITIAL_RUNTIME_VERSION, UPGRADED_BINARY_FILL);
    assert!(
        RuntimeInstaller
            .ensure(replacement.connection.as_ref(), &runtime(), &wrong_version)
            .await
            .is_err()
    );
    let arm = artifact_for(
        RemoteArchitecture::Aarch64,
        INITIAL_RUNTIME_VERSION,
        INITIAL_BINARY_FILL,
    );
    assert!(
        RuntimeInstaller
            .ensure(replacement.connection.as_ref(), &runtime(), &arm)
            .await
            .is_err()
    );
    assert_eq!(filesystem.target(MEMORY_RUNTIME), stable);
}
