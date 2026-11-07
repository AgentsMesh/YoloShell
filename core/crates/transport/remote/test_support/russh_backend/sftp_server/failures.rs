use super::super::constants::{OVERSIZED_SFTP_ERROR_CHARACTERS, SFTP_ERROR_FILL};
use super::super::memory_fs::{FailurePoint, MemoryFilesystem};
use russh_sftp::protocol::StatusCode;
use russh_sftp::server::StatusReply;

pub(super) async fn initialize(filesystem: &MemoryFilesystem) -> Result<(), StatusReply> {
    if filesystem.take_failure(FailurePoint::SftpInitPending) {
        std::future::pending::<()>().await;
    }
    if filesystem.take_failure(FailurePoint::SftpInit) {
        return Err(StatusCode::Failure.into());
    }
    Ok(())
}

pub(super) async fn stall_fstat(filesystem: &MemoryFilesystem) {
    if filesystem.take_failure(FailurePoint::SftpFstatHang) {
        tokio::time::sleep(super::super::constants::SFTP_SERVER_STALL).await;
    }
}

pub(super) async fn stall_readlink(filesystem: &MemoryFilesystem) {
    if filesystem.take_failure(FailurePoint::SftpReadLinkHang) {
        std::future::pending::<()>().await;
    }
}

pub(super) async fn stall_response(filesystem: &MemoryFilesystem, failure: FailurePoint) {
    if filesystem.take_failure(failure) {
        std::future::pending::<()>().await;
    }
}

pub(super) fn reject_oversized_write(filesystem: &MemoryFilesystem) -> Result<(), StatusReply> {
    if !filesystem.take_failure(FailurePoint::OversizedUploadWrite) {
        return Ok(());
    }
    let message: String =
        std::iter::repeat_n(SFTP_ERROR_FILL, OVERSIZED_SFTP_ERROR_CHARACTERS).collect();
    Err(StatusCode::PermissionDenied.with_message(message))
}
