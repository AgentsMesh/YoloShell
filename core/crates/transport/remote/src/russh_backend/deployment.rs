use super::super::constants::{
    DIRECTORY_DURABILITY_WARNING, RUNTIME_EXECUTABLE_MODE, RUNTIME_UPLOAD_CHUNK_BYTES,
    RUNTIME_UPLOAD_MODE,
};
use super::deployment_activation::activate;
use super::deployment_fs::*;
use super::deployment_operation::{io as file_operation, settle, sftp as sftp_operation};
use super::deployment_verify::{verify_file, verify_file_mode};
use super::*;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use tokio::io::AsyncWriteExt;

use RemoteFilesystemOperation as Operation;

impl RusshConnection {
    pub(super) async fn stage_runtime_sftp(
        &self,
        request: &RuntimeStageRequest,
    ) -> Result<ChannelOutcome<RuntimeStageResult>, RemoteError> {
        let custody = self.temporary_path_custody(&request.temporary_path)?;
        self.require_file_fsync().await?;
        tracing::warn!(message = DIRECTORY_DURABILITY_WARNING);
        let (identity, sftp) = self.open_sftp(ChannelPurpose::RuntimeDeployment).await?;
        let result = stage(&sftp, request, self.operation_timeout).await;
        let closed = sftp.close().await;
        let value = settle(result, closed)?;
        custody.release();
        Ok(ChannelOutcome { identity, value })
    }

    pub(super) async fn activate_runtime_sftp(
        &self,
        request: &RuntimeActivationRequest,
    ) -> Result<ChannelOutcome<()>, RemoteError> {
        let custody = self.temporary_path_custody(&request.temporary_link)?;
        let (identity, sftp) = self.open_sftp(ChannelPurpose::RuntimeDeployment).await?;
        let result = activate(&sftp, self, request, self.operation_timeout).await;
        let closed = sftp.close().await;
        settle(result, closed)?;
        custody.release();
        Ok(ChannelOutcome {
            identity,
            value: (),
        })
    }
}

async fn stage(
    sftp: &SftpSession,
    request: &RuntimeStageRequest,
    limit: std::time::Duration,
) -> Result<RuntimeStageResult, RemoteError> {
    ensure_install_directory(sftp, &request.directory_path, limit).await?;
    let previous = stable_target(sftp, &request.stable_path, limit).await?;
    let had_previous_runtime = previous.is_some();
    let release = &request.release_path;
    if exists(sftp, release, limit).await? {
        verify_file(
            sftp,
            release,
            request.bytes.len() as u64,
            &request.sha256,
            limit,
        )
        .await?;
    } else {
        upload(sftp, request, limit).await?;
        match sftp_operation(
            limit,
            Operation::Rename,
            release,
            sftp.rename(&request.temporary_path, release),
        )
        .await
        {
            Ok(()) => {}
            Err(error) => {
                if verify_file(
                    sftp,
                    release,
                    request.bytes.len() as u64,
                    &request.sha256,
                    limit,
                )
                .await
                .is_err()
                {
                    let _ = cleanup_temporary(sftp, &request.temporary_path, limit).await;
                    return Err(error);
                }
                let _ = cleanup_temporary(sftp, &request.temporary_path, limit).await;
            }
        }
        verify_file(
            sftp,
            release,
            request.bytes.len() as u64,
            &request.sha256,
            limit,
        )
        .await?;
    }
    let already_active = previous.as_deref() == Some(release);
    Ok(RuntimeStageResult {
        already_active,
        had_previous_runtime,
    })
}

async fn upload(
    sftp: &SftpSession,
    request: &RuntimeStageRequest,
    limit: std::time::Duration,
) -> Result<(), RemoteError> {
    let temporary = &request.temporary_path;
    let attributes = FileAttributes {
        permissions: Some(RUNTIME_UPLOAD_MODE),
        ..FileAttributes::empty()
    };
    let flags = OpenFlags::CREATE | OpenFlags::EXCLUDE | OpenFlags::WRITE;
    let mut file = sftp_operation(
        limit,
        Operation::CreateFile,
        temporary,
        sftp.open_with_flags_and_attributes(temporary, flags, attributes),
    )
    .await?;
    let mut result = Ok(());
    for chunk in request.bytes.chunks(RUNTIME_UPLOAD_CHUNK_BYTES) {
        if let Err(error) = file_operation(
            limit,
            Operation::WriteFile,
            temporary,
            file.write_all(chunk),
        )
        .await
        {
            result = Err(error);
            break;
        }
    }
    if result.is_ok() {
        result = file_operation(limit, Operation::FlushFile, temporary, file.flush()).await;
    }
    if result.is_ok() {
        result = sftp_operation(limit, Operation::SyncFile, temporary, file.sync_all()).await;
    }
    let closed = file_operation(limit, Operation::CloseFile, temporary, file.shutdown()).await;
    result = settle(result, closed);
    if result.is_ok() {
        result = verify_file_mode(
            sftp,
            temporary,
            request.bytes.len() as u64,
            &request.sha256,
            RUNTIME_UPLOAD_MODE,
            limit,
        )
        .await;
    }
    if result.is_ok() {
        result = sftp_operation(
            limit,
            Operation::SetPermissions,
            temporary,
            sftp.set_metadata(
                temporary,
                FileAttributes {
                    permissions: Some(RUNTIME_EXECUTABLE_MODE),
                    ..FileAttributes::empty()
                },
            ),
        )
        .await;
    }
    if result.is_err() {
        let _ = cleanup_temporary(sftp, temporary, limit).await;
    }
    result
}
