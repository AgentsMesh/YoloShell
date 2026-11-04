use super::constants::*;
use super::deployment_support::{artifact, runtime};
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::{RUNTIME_DIRECTORY_MODE, RUNTIME_EXECUTABLE_MODE};
use crate::remote::{RuntimeDeploymentResult, RuntimeInstaller};

#[tokio::test]
async fn stable_regular_file_directory_and_external_link_fail_closed() {
    for kind in [
        StableKind::File,
        StableKind::Directory,
        StableKind::ExternalLink,
    ] {
        let filesystem = MemoryFilesystem::default();
        filesystem.insert_directory(MEMORY_RUNTIME_PARENT, RUNTIME_DIRECTORY_MODE);
        match kind {
            StableKind::File => {
                filesystem.insert_file(MEMORY_RUNTIME, b"keep", RUNTIME_EXECUTABLE_MODE)
            }
            StableKind::Directory => {
                filesystem.insert_directory(MEMORY_RUNTIME, RUNTIME_DIRECTORY_MODE)
            }
            StableKind::ExternalLink => filesystem.insert_link(MEMORY_RUNTIME, MEMORY_OUTSIDE),
        }
        filesystem.set_version(INITIAL_RUNTIME_VERSION);
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
        match kind {
            StableKind::File => assert!(filesystem.contains_file(MEMORY_RUNTIME)),
            StableKind::Directory => assert!(filesystem.target(MEMORY_RUNTIME).is_none()),
            StableKind::ExternalLink => {
                assert_eq!(
                    filesystem.target(MEMORY_RUNTIME).as_deref(),
                    Some(MEMORY_OUTSIDE)
                )
            }
        }
        assert_eq!(filesystem.temporary_entries(), 0);
    }
}

#[tokio::test]
async fn unsafe_parent_and_missing_posix_rename_preserve_existing_state() {
    let filesystem = MemoryFilesystem::default();
    filesystem.insert_directory(MEMORY_RUNTIME_PARENT, UNSAFE_DIRECTORY_MODE);
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    assert!(
        RuntimeInstaller
            .ensure(
                fixture.connection.as_ref(),
                &runtime(),
                &artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL),
            )
            .await
            .is_err()
    );

    let filesystem = installed_filesystem().await;
    let old = filesystem.target(MEMORY_RUNTIME).unwrap();
    filesystem.set_posix_rename_supported(false);
    filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
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
    assert_eq!(filesystem.target(MEMORY_RUNTIME), Some(old));
    assert_eq!(filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn lost_posix_rename_response_is_verified_as_committed() {
    let filesystem = installed_filesystem().await;
    filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    filesystem.fail_once(FailurePoint::ActivationResponseLoss);
    let fixture = super::fixture::connect_memory_with_timeout(
        false,
        filesystem.clone(),
        SHORT_OPERATION_TIMEOUT,
    )
    .await
    .unwrap();
    let result = RuntimeInstaller
        .ensure(
            fixture.connection.as_ref(),
            &runtime(),
            &artifact(UPGRADED_RUNTIME_VERSION, UPGRADED_BINARY_FILL),
        )
        .await
        .unwrap();
    assert_eq!(result, RuntimeDeploymentResult::Upgraded);
    assert!(
        filesystem
            .target(MEMORY_RUNTIME)
            .is_some_and(|target| target.contains(UPGRADED_RUNTIME_VERSION))
    );
    assert_eq!(filesystem.temporary_entries(), 0);
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

#[derive(Clone, Copy)]
enum StableKind {
    File,
    Directory,
    ExternalLink,
}
