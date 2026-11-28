use super::super::constants::{FILE_PERMISSION_MASK, RUNTIME_EXECUTABLE_MODE};
use super::deployment_operation::{io as file_operation, settle, sftp as sftp_operation};
use super::*;
use russh_sftp::client::SftpSession;
use tokio::io::AsyncReadExt;
use yolo_crypto::sha256_hex;

use RemoteFilesystemOperation as Operation;

pub(super) async fn verify_file(
    sftp: &SftpSession,
    path: &str,
    expected_bytes: u64,
    digest: &str,
    limit: std::time::Duration,
) -> Result<(), RemoteError> {
    verify_file_mode(
        sftp,
        path,
        expected_bytes,
        digest,
        RUNTIME_EXECUTABLE_MODE,
        limit,
    )
    .await
}

pub(super) async fn verify_file_mode(
    sftp: &SftpSession,
    path: &str,
    expected_bytes: u64,
    digest: &str,
    mode: u32,
    limit: std::time::Duration,
) -> Result<(), RemoteError> {
    let link_metadata = sftp_operation(
        limit,
        Operation::InspectLink,
        path,
        sftp.symlink_metadata(path),
    )
    .await?;
    if !link_metadata.is_regular() {
        return Err(RemoteError::RuntimeIntegrity(
            "remote runtime release is not a regular file".into(),
        ));
    }
    let metadata =
        sftp_operation(limit, Operation::InspectMetadata, path, sftp.metadata(path)).await?;
    let permissions = metadata.permissions.unwrap_or_default() & FILE_PERMISSION_MASK;
    let mut data = Vec::new();
    let mut file = sftp_operation(limit, Operation::OpenFile, path, sftp.open(path)).await?;
    let read = file_operation(
        limit,
        Operation::ReadFile,
        path,
        (&mut file)
            .take(expected_bytes.saturating_add(1))
            .read_to_end(&mut data),
    )
    .await;
    let closed = file_operation(
        limit,
        Operation::CloseFile,
        path,
        tokio::io::AsyncWriteExt::shutdown(&mut file),
    )
    .await;
    settle(read, closed)?;
    if metadata.size != Some(expected_bytes) || permissions != mode || sha256_hex(&data) != digest {
        return Err(RemoteError::RuntimeIntegrity(
            "remote runtime bytes or permissions differ".into(),
        ));
    }
    Ok(())
}
