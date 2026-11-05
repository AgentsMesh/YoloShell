use super::constants::*;
use super::deployment_support::{artifact, runtime};
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::{RUNTIME_DIRECTORY_MODE, RUNTIME_EXECUTABLE_MODE};
use crate::remote::{MultiplexedSshConnection, RuntimeInstaller};

#[tokio::test]
async fn existing_release_must_remain_a_verified_regular_executable() {
    for corruption in [
        Corruption::Symlink,
        Corruption::Mode,
        Corruption::Digest,
        Corruption::Close,
    ] {
        let (filesystem, release) = installed().await;
        let artifact = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
        filesystem.remove(MEMORY_RUNTIME);
        match corruption {
            Corruption::Symlink => {
                filesystem.remove(&release);
                filesystem.insert_link(&release, MEMORY_OUTSIDE);
            }
            Corruption::Mode => {
                filesystem.insert_file(&release, artifact.bytes(), UNSAFE_DIRECTORY_MODE)
            }
            Corruption::Digest => {
                let mut bytes = artifact.bytes().to_vec();
                let last = bytes.last_mut().unwrap();
                *last = last.wrapping_add(1);
                filesystem.insert_file(&release, &bytes, RUNTIME_EXECUTABLE_MODE);
            }
            Corruption::Close => filesystem.fail_once(FailurePoint::FileClose),
        }
        let fixture = connect_memory_with_fs(false, filesystem.clone())
            .await
            .unwrap();
        assert!(
            RuntimeInstaller
                .ensure(fixture.connection.as_ref(), &runtime(), &artifact)
                .await
                .is_err()
        );
        assert_eq!(filesystem.temporary_entries(), 0);
    }
}

#[tokio::test]
async fn parent_creation_and_upload_fail_closed_at_each_owned_resource() {
    for failure in [
        FailurePoint::Metadata,
        FailurePoint::StableMetadata,
        FailurePoint::Mkdir,
        FailurePoint::UnsafeDirectory,
        FailurePoint::UploadWrite,
        FailurePoint::FileClose,
    ] {
        let filesystem = MemoryFilesystem::default();
        filesystem.set_version(INITIAL_RUNTIME_VERSION);
        filesystem.fail_once(failure);
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
        assert_eq!(filesystem.temporary_entries(), 0);
    }
    let filesystem = MemoryFilesystem::default();
    filesystem.insert_file(
        MEMORY_RUNTIME_PARENT,
        b"not-a-directory",
        RUNTIME_DIRECTORY_MODE,
    );
    assert_install_fails(filesystem).await;
}

#[tokio::test]
async fn every_managed_link_target_is_verified_before_and_after_activation() {
    for target in [MEMORY_RELATIVE_RELEASE, MEMORY_WRONG_PREFIX_RELEASE] {
        let filesystem = MemoryFilesystem::default();
        filesystem.insert_directory(MEMORY_RUNTIME_PARENT, RUNTIME_DIRECTORY_MODE);
        filesystem.insert_link(MEMORY_RUNTIME, target);
        assert_install_fails(filesystem).await;
    }
    assert_failure_on_fresh_install(FailurePoint::TemporaryLinkTarget).await;
    for failure in [
        FailurePoint::ActivationWrongTarget,
        FailurePoint::PosixUnexpectedPacket,
    ] {
        let (filesystem, old_release) = installed().await;
        filesystem.set_version(UPGRADED_RUNTIME_VERSION);
        filesystem.fail_once(failure);
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
        if failure == FailurePoint::PosixUnexpectedPacket {
            assert_eq!(filesystem.target(MEMORY_RUNTIME), Some(old_release));
        }
        assert_eq!(filesystem.temporary_entries(), 0);
    }
}

async fn assert_failure_on_fresh_install(failure: FailurePoint) {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(failure);
    assert_install_fails(filesystem).await;
}

async fn assert_install_fails(filesystem: MemoryFilesystem) {
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
    assert_eq!(filesystem.temporary_entries(), 0);
}

async fn installed() -> (MemoryFilesystem, String) {
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
    let release = filesystem.target(MEMORY_RUNTIME).unwrap();
    (filesystem, release)
}

#[derive(Clone, Copy)]
enum Corruption {
    Symlink,
    Mode,
    Digest,
    Close,
}
