use super::RemoteError;
use russh_sftp::protocol::FileAttributes;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SftpFileIdentity {
    size: u64,
    uid: u32,
    gid: u32,
    permissions: u32,
    mtime: u32,
}

impl SftpFileIdentity {
    pub(super) fn regular(
        attributes: &FileAttributes,
        denied_path: &Path,
    ) -> Result<Self, RemoteError> {
        let complete = (
            attributes.size,
            attributes.uid,
            attributes.gid,
            attributes.permissions,
            attributes.mtime,
        );
        let (Some(size), Some(uid), Some(gid), Some(permissions), Some(mtime)) = complete else {
            return Err(RemoteError::PathDenied(denied_path.to_path_buf()));
        };
        if !attributes.file_type().is_file() {
            return Err(RemoteError::PathDenied(denied_path.to_path_buf()));
        }
        Ok(Self {
            size,
            uid,
            gid,
            permissions,
            mtime,
        })
    }

    pub(super) fn verify(
        self,
        attributes: &FileAttributes,
        denied_path: &Path,
    ) -> Result<(), RemoteError> {
        if Self::regular(attributes, denied_path)? != self {
            return Err(RemoteError::PathDenied(denied_path.to_path_buf()));
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/russh_backend/sftp_file_identity_tests.rs"]
mod tests;
