use super::constants::*;
use super::deployment_support::{artifact, artifact_with_size, runtime};
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::{REMOTE_FILESYSTEM_DETAIL_MAX_CHARACTERS, RUNTIME_UPLOAD_PREFIX};
use crate::remote::{RemoteError, RemoteFilesystemOperation as Operation, RuntimeInstaller};
use russh_sftp::client::error::Error as SftpError;
use std::path::Path;

const SETTLEMENT_DETAIL: &str = "session settlement failed";

#[tokio::test]
async fn deployment_status_failures_keep_semantic_operation_and_path() {
    for (failure, operation, path) in [
        (
            FailurePoint::Metadata,
            Operation::InspectMetadata,
            MEMORY_RUNTIME_PARENT,
        ),
        (
            FailurePoint::StableMetadata,
            Operation::InspectLink,
            MEMORY_RUNTIME,
        ),
        (
            FailurePoint::Permission,
            Operation::SetPermissions,
            MEMORY_RUNTIME_PARENT,
        ),
    ] {
        let (error, filesystem) = install_failure(failure, TEST_BINARY_BYTES).await;
        assert_filesystem_error(error, operation, |actual| actual == Path::new(path));
        assert_reclaimed(&filesystem).await;
    }

    let (error, filesystem) =
        install_failure(FailurePoint::UploadWrite, WRITE_ERROR_ARTIFACT_BYTES).await;
    assert_filesystem_error(error, Operation::WriteFile, managed_upload);
    assert_reclaimed(&filesystem).await;
}

#[tokio::test]
async fn oversized_async_write_status_is_bounded_and_semantic() {
    let (error, filesystem) = install_failure(
        FailurePoint::OversizedUploadWrite,
        WRITE_ERROR_ARTIFACT_BYTES,
    )
    .await;
    let RemoteError::RemoteFilesystem {
        operation,
        path,
        detail,
    } = error
    else {
        panic!("oversized server status must remain a filesystem error")
    };
    assert_eq!(operation, Operation::WriteFile);
    assert!(managed_upload(&path));
    assert_eq!(
        detail.chars().count(),
        REMOTE_FILESYSTEM_DETAIL_MAX_CHARACTERS
    );
    assert_reclaimed(&filesystem).await;
}

#[test]
fn settlement_keeps_primary_causality_and_recovery_signal() {
    let semantic = filesystem_error();
    let transport = RemoteError::transport(SETTLEMENT_DETAIL);
    let error = super::super::deployment_operation::settle::<()>(Err(semantic), Err(transport))
        .unwrap_err();
    assert!(!error.is_connection_loss());
    assert!(error.requires_connection_recovery());
    let RemoteError::LifecycleSettlement {
        primary,
        settlement,
    } = error
    else {
        panic!("dual failure must preserve primary and settlement")
    };
    assert!(matches!(
        *primary,
        RemoteError::RemoteFilesystem {
            operation: Operation::WriteFile,
            ..
        }
    ));
    assert!(settlement.is_connection_loss());

    let error = super::super::deployment_operation::settle::<()>(
        Err(RemoteError::transport(SETTLEMENT_DETAIL)),
        Err(filesystem_error()),
    )
    .unwrap_err();
    assert!(error.is_connection_loss());
    assert!(error.requires_connection_recovery());
}

#[test]
fn raw_session_close_failure_uses_its_scope_and_preserves_primary() {
    let scope = crate::remote::constants::POSIX_RENAME_SESSION_SCOPE;
    let close = super::super::deployment_operation::sftp_result::<()>(
        Operation::CloseSession,
        scope,
        Err(SftpError::UnexpectedBehavior(SETTLEMENT_DETAIL.into())),
    );
    let error = close.unwrap_err();
    assert!(error.is_connection_loss());
    assert!(error.to_string().contains(scope));

    let settled =
        super::super::deployment_operation::settle::<()>(Err(filesystem_error()), Err(error))
            .unwrap_err();
    assert!(!settled.is_connection_loss());
    assert!(settled.requires_connection_recovery());
    assert!(matches!(settled, RemoteError::LifecycleSettlement { .. }));
}

async fn install_failure(failure: FailurePoint, bytes: usize) -> (RemoteError, MemoryFilesystem) {
    let filesystem = MemoryFilesystem::default();
    filesystem.set_version(INITIAL_RUNTIME_VERSION);
    filesystem.fail_once(failure);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    let artifact = if bytes == TEST_BINARY_BYTES {
        artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL)
    } else {
        artifact_with_size(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL, bytes)
    };
    let error = RuntimeInstaller
        .ensure(fixture.connection.as_ref(), &runtime(), &artifact)
        .await
        .unwrap_err();
    assert_eq!(filesystem.temporary_entries(), 0);
    (error, filesystem)
}

fn assert_filesystem_error(
    error: RemoteError,
    expected: Operation,
    path_matches: impl FnOnce(&Path) -> bool,
) {
    assert!(!error.is_connection_loss());
    let RemoteError::RemoteFilesystem {
        operation,
        path,
        detail,
    } = error
    else {
        panic!("server status must remain a filesystem error")
    };
    assert_eq!(operation, expected);
    assert!(path_matches(&path));
    assert!(!detail.is_empty());
}

fn managed_upload(path: &Path) -> bool {
    path.to_string_lossy()
        .starts_with(&format!("{MEMORY_RUNTIME_PARENT}/{RUNTIME_UPLOAD_PREFIX}"))
}

fn filesystem_error() -> RemoteError {
    RemoteError::RemoteFilesystem {
        operation: Operation::WriteFile,
        path: MEMORY_RUNTIME.into(),
        detail: SETTLEMENT_DETAIL.into(),
    }
}

async fn assert_reclaimed(filesystem: &MemoryFilesystem) {
    let settled = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        loop {
            if filesystem.live_handles() == NO_LIVE_HANDLES
                && filesystem.sftp_sessions_started() == filesystem.sftp_sessions_dropped()
            {
                return;
            }
            tokio::time::sleep(SFTP_LIFECYCLE_POLL_INTERVAL).await;
        }
    })
    .await;
    assert!(settled.is_ok());
}
