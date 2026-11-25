use super::constants::MEMORY_OUTSIDE;
use super::memory_fs::{FailurePoint, MemoryFile, MemoryFilesystem};
use super::sftp_response::ok;
use super::sftp_wire::{decode_path, decode_path_pair};
use crate::remote::constants::{FSYNC_EXTENSION, POSIX_RENAME_EXTENSION};
use russh_sftp::protocol::{ExtendedReply, Packet, StatusCode};
use std::collections::HashMap;

pub async fn respond(
    filesystem: &MemoryFilesystem,
    handles: &HashMap<String, MemoryFile>,
    reject_limits: bool,
    id: u32,
    request: &str,
    data: &[u8],
) -> Result<Packet, StatusCode> {
    if reject_limits && request == russh_sftp::extensions::LIMITS {
        return Err(StatusCode::Failure);
    }
    match request {
        FSYNC_EXTENSION if filesystem.supports_fsync() => fsync(filesystem, handles, id, data),
        POSIX_RENAME_EXTENSION if filesystem.supports_posix_rename() => {
            posix_rename(filesystem, id, data).await
        }
        _ => Err(StatusCode::OpUnsupported),
    }
}

fn fsync(
    filesystem: &MemoryFilesystem,
    handles: &HashMap<String, MemoryFile>,
    id: u32,
    data: &[u8],
) -> Result<Packet, StatusCode> {
    let handle = decode_path(data)?;
    handles.get(&handle).ok_or(StatusCode::NoSuchFile)?;
    if filesystem.take_failure(FailurePoint::FsyncFailure) {
        return Err(StatusCode::Failure);
    }
    Ok(Packet::Status(ok(id)))
}

async fn posix_rename(
    filesystem: &MemoryFilesystem,
    id: u32,
    data: &[u8],
) -> Result<Packet, StatusCode> {
    let (old, new) = decode_path_pair(data)?;
    if filesystem.take_failure(FailurePoint::PosixUnexpectedPacket) {
        return Ok(Packet::ExtendedReply(ExtendedReply {
            id,
            data: Vec::new(),
        }));
    }
    let previous = filesystem.target(&new);
    filesystem.posix_rename(&old, &new)?;
    if filesystem.take_failure(FailurePoint::ActivationWrongTarget) {
        filesystem.insert_link(&new, MEMORY_OUTSIDE);
    }
    if filesystem.take_failure(FailurePoint::ActivationPreviousTarget) {
        filesystem.insert_link(&new, previous.as_deref().unwrap_or(MEMORY_OUTSIDE));
    }
    if filesystem.take_activation_response_loss() {
        std::future::pending::<()>().await;
    }
    Ok(Packet::Status(ok(id)))
}
