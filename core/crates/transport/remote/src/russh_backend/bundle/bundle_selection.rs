use super::{
    RusshConnection,
    bundle_directory::{self, canonical, text},
    bundle_file::identity,
    bundle_raw::*,
    deployment_operation::{optional, settle, sftp as call},
};
use crate::remote::{
    ChannelOutcome, ChannelPurpose, RemoteError, RemoteFilesystemOperation as Op,
    constants::RUNTIME_UPLOAD_CHUNK_BYTES,
};
use russh_sftp::{
    client::{RawSftpSession, error::Error as SftpError},
    protocol::{FileAttributes, OpenFlags, StatusCode},
};
use std::{path::Path, time::Duration};
use yolo_distribution_publication_protocol::{
    CURRENT_FILE_NAME, MANIFEST_MODE, MAX_PUBLICATION_BYTES,
};

impl RusshConnection {
    pub(super) async fn read_bundle_selection_sftp(
        &self,
        root: &Path,
    ) -> Result<ChannelOutcome<Option<Vec<u8>>>, RemoteError> {
        super::super::bundle::validate_install_root(root)?;
        let uid = self.bundle_owner().await?;
        let raw = self.open_bundle_sftp(false).await?;
        let identity = self.identity(ChannelPurpose::FileRead);
        let result = read(&raw, root, uid, self.operation_timeout).await;
        let value = settle(result, raw.close(SCOPE).await)?;
        Ok(ChannelOutcome { identity, value })
    }
}

async fn read(
    raw: &RawSftpSession,
    root: &Path,
    uid: u32,
    limit: Duration,
) -> Result<Option<Vec<u8>>, RemoteError> {
    let Some(owner) = bundle_directory::root(raw, root, false, limit).await? else {
        return Ok(None);
    };
    if owner.uid() != uid {
        return Err(invalid());
    }
    let path = root.join(CURRENT_FILE_NAME);
    let name = text(&path)?;
    let Some(attributes) = optional(limit, Op::InspectLink, name, raw.lstat(name)).await? else {
        return Ok(None);
    };
    let baseline = identity(&attributes.attrs, name, MANIFEST_MODE, owner.uid())?;
    let size =
        usize::try_from(attributes.attrs.size.ok_or_else(invalid)?).map_err(|_| invalid())?;
    if size > MAX_PUBLICATION_BYTES {
        return Err(RemoteError::LimitExceeded);
    }
    canonical(raw, name, limit).await?;
    let handle = call(
        limit,
        Op::OpenFile,
        name,
        raw.open(name, OpenFlags::READ, FileAttributes::empty()),
    )
    .await?
    .handle;
    let result = async {
        baseline.verify(
            &call(limit, Op::InspectMetadata, name, raw.fstat(&handle))
                .await?
                .attrs,
            &path,
        )?;
        let bytes = read_bounded(raw, name, &handle, size, limit).await?;
        baseline.verify(
            &call(limit, Op::InspectMetadata, name, raw.fstat(&handle))
                .await?
                .attrs,
            &path,
        )?;
        baseline.verify(
            &call(limit, Op::InspectLink, name, raw.lstat(name))
                .await?
                .attrs,
            &path,
        )?;
        owner.verify(raw, text(root)?, limit).await?;
        canonical(raw, name, limit).await?;
        Ok(Some(bytes))
    }
    .await;
    settle(
        result,
        call(limit, Op::CloseFile, name, raw.close(handle))
            .await
            .map(|_| ()),
    )
}

async fn read_bounded(
    raw: &RawSftpSession,
    path: &str,
    handle: &str,
    size: usize,
    limit: Duration,
) -> Result<Vec<u8>, RemoteError> {
    let mut bytes = Vec::with_capacity(size);
    loop {
        let count = RUNTIME_UPLOAD_CHUNK_BYTES.min(size - bytes.len() + SENTINEL_BYTES);
        let result =
            tokio::time::timeout(limit, raw.read(handle, bytes.len() as u64, count as u32))
                .await
                .map_err(|_| RemoteError::transport(READBACK_MISMATCH))?;
        let data = match result {
            Err(SftpError::Status(status))
                if status.status_code == StatusCode::Eof && bytes.len() == size =>
            {
                return Ok(bytes);
            }
            result => super::deployment_operation::sftp_result(Op::ReadFile, path, result)?.data,
        };
        if data.is_empty() || data.len() > count || data.len() > size - bytes.len() {
            return Err(RemoteError::LimitExceeded);
        }
        bytes.extend_from_slice(&data);
    }
}
