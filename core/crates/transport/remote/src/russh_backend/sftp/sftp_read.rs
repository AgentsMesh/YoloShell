use super::deadline::operation;
use super::deployment_operation::settle;
use super::sftp_file_identity::SftpFileIdentity;
use super::{AuthorizedFileRead, FileReadResult, RemoteError};
use russh_sftp::client::SftpSession;
use russh_sftp::client::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub(super) async fn read_file_contents(
    sftp: &SftpSession,
    read: &AuthorizedFileRead,
    limit: Duration,
) -> Result<FileReadResult, RemoteError> {
    let expected = super::sftp::path_text(read.resolved_path())?;
    let baseline = initial_identity(sftp, expected, read.resolved_path(), limit).await?;
    let mut file = operation(limit, sftp.open(expected)).await?;
    let result = read_opened(sftp, &mut file, read, baseline, limit).await;
    let closed = operation(limit, file.shutdown()).await;
    settle(result, closed)
}

async fn initial_identity(
    sftp: &SftpSession,
    expected: &str,
    denied_path: &Path,
    limit: Duration,
) -> Result<SftpFileIdentity, RemoteError> {
    verify_resolved(sftp, expected, denied_path, limit).await?;
    let attributes = operation(limit, sftp.symlink_metadata(expected)).await?;
    SftpFileIdentity::regular(&attributes, denied_path)
}

async fn read_opened(
    sftp: &SftpSession,
    file: &mut File,
    read: &AuthorizedFileRead,
    baseline: SftpFileIdentity,
    limit: Duration,
) -> Result<FileReadResult, RemoteError> {
    let expected = super::sftp::path_text(read.resolved_path())?;
    verify_handle(file, baseline, read.resolved_path(), limit).await?;
    verify_path(sftp, expected, baseline, read.resolved_path(), limit).await?;
    operation(
        limit,
        file.seek(std::io::SeekFrom::Start(read.request().offset())),
    )
    .await?;
    let mut data = Vec::new();
    operation(
        limit,
        (&mut *file)
            .take(read.request().length())
            .read_to_end(&mut data),
    )
    .await?;
    verify_handle(file, baseline, read.resolved_path(), limit).await?;
    verify_path(sftp, expected, baseline, read.resolved_path(), limit).await?;
    Ok(FileReadResult {
        resolved_path: read.resolved_path().to_path_buf(),
        eof: data.len() as u64 != read.request().length(),
        data,
    })
}

async fn verify_handle(
    file: &File,
    baseline: SftpFileIdentity,
    denied_path: &Path,
    limit: Duration,
) -> Result<(), RemoteError> {
    let attributes = operation(limit, file.metadata()).await?;
    baseline.verify(&attributes, denied_path)
}

async fn verify_path(
    sftp: &SftpSession,
    expected: &str,
    baseline: SftpFileIdentity,
    denied_path: &Path,
    limit: Duration,
) -> Result<(), RemoteError> {
    verify_resolved(sftp, expected, denied_path, limit).await?;
    let attributes = operation(limit, sftp.symlink_metadata(expected)).await?;
    baseline.verify(&attributes, denied_path)
}

async fn verify_resolved(
    sftp: &SftpSession,
    expected: &str,
    denied_path: &Path,
    limit: Duration,
) -> Result<(), RemoteError> {
    let resolved = operation(limit, sftp.canonicalize(expected)).await?;
    if Path::new(&resolved) != denied_path {
        return Err(RemoteError::PathDenied(PathBuf::from(resolved)));
    }
    Ok(())
}
