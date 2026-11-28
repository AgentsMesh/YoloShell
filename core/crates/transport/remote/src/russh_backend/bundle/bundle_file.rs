use super::{
    bundle_directory::{canonical, mode},
    bundle_raw::*,
    deployment_operation::{settle, sftp as call},
    sftp_file_identity::SftpFileIdentity,
};
use crate::remote::{
    RemoteError, RemoteFilesystemOperation as Op, constants::RUNTIME_UPLOAD_CHUNK_BYTES,
};
use russh_sftp::{
    client::{RawSftpSession, error::Error as SftpError},
    protocol::{FileAttributes, OpenFlags, StatusCode},
};
use std::{path::Path, time::Duration};

pub(super) fn identity(
    attributes: &FileAttributes,
    path: &str,
    permissions: u32,
    owner: u32,
) -> Result<SftpFileIdentity, RemoteError> {
    if attributes.permissions.map(|mode| mode & PERMISSION_MASK) != Some(permissions)
        || attributes.uid != Some(owner)
    {
        return Err(invalid());
    }
    SftpFileIdentity::regular(attributes, Path::new(path))
}

pub(super) async fn upload(
    raw: &RawSftpSession,
    path: &str,
    bytes: &[u8],
    permissions: u32,
    owner: u32,
    limit: Duration,
) -> Result<(), RemoteError> {
    let handle = call(
        limit,
        Op::CreateFile,
        path,
        raw.open(
            path,
            OpenFlags::CREATE | OpenFlags::EXCLUDE | OpenFlags::WRITE | OpenFlags::READ,
            mode(permissions),
        ),
    )
    .await?
    .handle;
    let result = async {
        let initial = call(limit, Op::InspectMetadata, path, raw.fstat(&handle))
            .await?
            .attrs;
        identity(&initial, path, permissions, owner)?;
        for (index, chunk) in bytes.chunks(RUNTIME_UPLOAD_CHUNK_BYTES).enumerate() {
            call(
                limit,
                Op::WriteFile,
                path,
                raw.write(
                    &handle,
                    (index * RUNTIME_UPLOAD_CHUNK_BYTES) as u64,
                    chunk.to_vec(),
                ),
            )
            .await?;
        }
        call(limit, Op::SyncFile, path, raw.fsync(&handle)).await?;
        let final_attributes = call(limit, Op::InspectMetadata, path, raw.fstat(&handle))
            .await?
            .attrs;
        identity(&final_attributes, path, permissions, owner)?;
        if final_attributes.size != Some(bytes.len() as u64) {
            return Err(invalid());
        }
        Ok(())
    }
    .await;
    let closed = call(limit, Op::CloseFile, path, raw.close(handle))
        .await
        .map(|_| ());
    settle(result, closed)
}

pub(super) async fn verify(
    raw: &RawSftpSession,
    path: &str,
    bytes: &[u8],
    permissions: u32,
    owner: u32,
    limit: Duration,
) -> Result<(), RemoteError> {
    let handle = call(
        limit,
        Op::OpenFile,
        path,
        raw.open(path, OpenFlags::READ, FileAttributes::empty()),
    )
    .await?
    .handle;
    let result = verify_open(raw, path, &handle, bytes, permissions, owner, limit).await;
    settle(
        result,
        call(limit, Op::CloseFile, path, raw.close(handle))
            .await
            .map(|_| ()),
    )
}

pub(super) async fn verify_open(
    raw: &RawSftpSession,
    path: &str,
    handle: &str,
    bytes: &[u8],
    permissions: u32,
    owner: u32,
    limit: Duration,
) -> Result<(), RemoteError> {
    canonical(raw, path, limit).await?;
    let attributes = call(limit, Op::InspectLink, path, raw.lstat(path))
        .await?
        .attrs;
    let baseline = identity(&attributes, path, permissions, owner)?;
    if attributes.size != Some(bytes.len() as u64) {
        return Err(invalid());
    }
    baseline.verify(
        &call(limit, Op::InspectMetadata, path, raw.fstat(handle))
            .await?
            .attrs,
        Path::new(path),
    )?;
    compare(raw, path, handle, bytes, limit).await?;
    baseline.verify(
        &call(limit, Op::InspectMetadata, path, raw.fstat(handle))
            .await?
            .attrs,
        Path::new(path),
    )?;
    baseline.verify(
        &call(limit, Op::InspectLink, path, raw.lstat(path))
            .await?
            .attrs,
        Path::new(path),
    )?;
    canonical(raw, path, limit).await
}

async fn compare(
    raw: &RawSftpSession,
    path: &str,
    handle: &str,
    expected: &[u8],
    limit: Duration,
) -> Result<(), RemoteError> {
    let mut offset = usize::default();
    loop {
        let count = RUNTIME_UPLOAD_CHUNK_BYTES.min(expected.len() - offset + SENTINEL_BYTES);
        let received = tokio::time::timeout(limit, raw.read(handle, offset as u64, count as u32))
            .await
            .map_err(|_| RemoteError::transport(READBACK_MISMATCH))?;
        let data = match received {
            Err(SftpError::Status(status))
                if status.status_code == StatusCode::Eof && offset == expected.len() =>
            {
                return Ok(());
            }
            received => {
                super::deployment_operation::sftp_result(Op::ReadFile, path, received)?.data
            }
        };
        if data.is_empty()
            || data.len() > count
            || expected.get(offset..offset + data.len()) != Some(data.as_slice())
        {
            return Err(RemoteError::RuntimeIntegrity(READBACK_MISMATCH.into()));
        }
        offset += data.len();
    }
}
