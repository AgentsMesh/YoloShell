use super::deadline::wait;
use super::{RemoteError, RemoteFilesystemOperation};
use crate::remote::constants::REMOTE_FILESYSTEM_DETAIL_MAX_CHARACTERS;
use russh_sftp::client::RawSftpSession;
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::protocol::{Status, StatusCode};
use std::future::Future;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) async fn sftp<T>(
    limit: Duration,
    operation: RemoteFilesystemOperation,
    path: &str,
    future: impl Future<Output = Result<T, SftpError>>,
) -> Result<T, RemoteError> {
    match wait(limit, future).await {
        Ok(result) => sftp_result(operation, path, result),
        Err(error) => Err(transport(operation, path, error.to_string())),
    }
}

pub(super) fn close_raw_session(raw: &RawSftpSession, scope: &str) -> Result<(), RemoteError> {
    sftp_result(
        RemoteFilesystemOperation::CloseSession,
        scope,
        raw.close_session(),
    )
}

pub(super) fn sftp_result<T>(
    operation: RemoteFilesystemOperation,
    path: &str,
    result: Result<T, SftpError>,
) -> Result<T, RemoteError> {
    result.map_err(|error| classify_sftp(operation, path, error))
}

pub(super) async fn io<T>(
    limit: Duration,
    operation: RemoteFilesystemOperation,
    path: &str,
    future: impl Future<Output = Result<T, IoError>>,
) -> Result<T, RemoteError> {
    match wait(limit, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(classify_io(operation, path, error)),
        Err(error) => Err(transport(operation, path, error.to_string())),
    }
}

pub(super) async fn optional<T>(
    limit: Duration,
    operation: RemoteFilesystemOperation,
    path: &str,
    future: impl Future<Output = Result<T, SftpError>>,
) -> Result<Option<T>, RemoteError> {
    match wait(limit, future).await {
        Ok(Ok(value)) => Ok(Some(value)),
        Ok(Err(SftpError::Status(status))) if status.status_code == StatusCode::NoSuchFile => {
            Ok(None)
        }
        Ok(Err(error)) => Err(classify_sftp(operation, path, error)),
        Err(error) => Err(transport(operation, path, error.to_string())),
    }
}

pub(super) fn semantic(
    operation: RemoteFilesystemOperation,
    path: &str,
    detail: impl Into<String>,
) -> RemoteError {
    RemoteError::RemoteFilesystem {
        operation,
        path: PathBuf::from(path),
        detail: bounded(detail.into()),
    }
}

pub(super) fn status(
    operation: RemoteFilesystemOperation,
    path: &str,
    status: Status,
) -> RemoteError {
    semantic(operation, path, status_detail(status))
}

pub(super) fn protocol(
    operation: RemoteFilesystemOperation,
    path: &str,
    detail: impl Into<String>,
) -> RemoteError {
    transport(operation, path, detail.into())
}

pub(super) fn settle<T>(
    primary: Result<T, RemoteError>,
    close: Result<(), RemoteError>,
) -> Result<T, RemoteError> {
    match (primary, close) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(close)) => Err(close),
        (Err(primary), Err(settlement)) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}

fn classify_sftp(
    operation: RemoteFilesystemOperation,
    path: &str,
    error: SftpError,
) -> RemoteError {
    match error {
        SftpError::Status(reply) => status(operation, path, reply),
        SftpError::Limited(detail) => semantic(operation, path, detail),
        other => transport(operation, path, other.to_string()),
    }
}

fn status_detail(status: Status) -> String {
    let detail = format!(
        "{}: {} [{}]",
        status.status_code, status.error_message, status.language_tag
    );
    bounded(detail)
}

fn bounded(detail: String) -> String {
    detail
        .chars()
        .take(REMOTE_FILESYSTEM_DETAIL_MAX_CHARACTERS)
        .collect()
}

fn classify_io(operation: RemoteFilesystemOperation, path: &str, error: IoError) -> RemoteError {
    if connection_error(error.kind()) {
        transport(operation, path, error.to_string())
    } else {
        semantic(operation, path, error.to_string())
    }
}

fn connection_error(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::BrokenPipe
            | ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionReset
            | ErrorKind::NotConnected
            | ErrorKind::TimedOut
            | ErrorKind::UnexpectedEof
    )
}

fn transport(operation: RemoteFilesystemOperation, path: &str, detail: String) -> RemoteError {
    let detail = bounded(detail);
    RemoteError::transport(format!(
        "remote filesystem {operation} failed for {}: {detail}",
        Path::new(path).display()
    ))
}
