use super::super::constants::{
    DRIFTED_FILE_MODE, FIRST_SFTP_HANDLE_SEQUENCE, MEMORY_FILE, MEMORY_OUTSIDE,
    REPLACEMENT_FILE_CONTENT, SFTP_HANDLE_PREFIX, SFTP_HANDLE_SEQUENCE_INCREMENT,
    SFTP_SERVER_STALL,
};
use super::super::memory_fs::{FailurePoint, MemoryFile, MemoryFilesystem};
use super::MemorySftp;
use crate::remote::constants::RUNTIME_UPLOAD_MODE;
use russh_sftp::protocol::{FileAttributes, OpenFlags, StatusCode};
use std::collections::{HashMap, HashSet};

impl MemorySftp {
    pub fn new(filesystem: MemoryFilesystem) -> Self {
        filesystem.record_sftp_session_open();
        Self {
            filesystem,
            handles: HashMap::new(),
            blocked_closes: HashSet::new(),
            reject_limits: false,
            next_handle: FIRST_SFTP_HANDLE_SEQUENCE,
        }
    }

    pub(super) fn open_handle(
        &mut self,
        filename: &str,
        flags: OpenFlags,
        attrs: FileAttributes,
    ) -> Result<String, StatusCode> {
        if self
            .filesystem
            .take_failure(FailurePoint::SftpSwapBeforeOpen)
        {
            self.swap_read_path();
        }
        let file = self.filesystem.open(filename, flags, attrs)?;
        let sequence = self.next_handle;
        self.next_handle = sequence
            .checked_add(SFTP_HANDLE_SEQUENCE_INCREMENT)
            .ok_or(StatusCode::Failure)?;
        let handle = format!("{SFTP_HANDLE_PREFIX}{sequence}");
        self.handles.insert(handle.clone(), file.clone());
        self.filesystem.record_handle_open();
        if self
            .filesystem
            .take_failure(FailurePoint::SftpSwapAfterOpen)
        {
            self.swap_read_path();
        }
        if self
            .filesystem
            .take_failure(FailurePoint::SftpReplaceAfterOpen)
        {
            self.filesystem
                .insert_file(MEMORY_FILE, REPLACEMENT_FILE_CONTENT, RUNTIME_UPLOAD_MODE);
        }
        if self
            .filesystem
            .take_failure(FailurePoint::SftpMetadataDriftAfterOpen)
        {
            self.filesystem
                .replace_file_permissions(&file, DRIFTED_FILE_MODE);
        }
        Ok(handle)
    }

    pub(super) async fn close_handle(&mut self, handle: &str) -> Result<(), StatusCode> {
        self.filesystem.record_close_attempt();
        if !self.handles.contains_key(handle) {
            return Err(StatusCode::NoSuchFile);
        }
        if self.blocked_closes.contains(handle) {
            return Err(StatusCode::PermissionDenied);
        }
        if self.filesystem.take_failure(FailurePoint::FileClose) {
            self.blocked_closes.insert(handle.into());
            return Err(StatusCode::PermissionDenied);
        }
        if self.filesystem.take_failure(FailurePoint::SftpCloseHang) {
            self.blocked_closes.insert(handle.into());
            tokio::time::sleep(SFTP_SERVER_STALL).await;
            return Err(StatusCode::PermissionDenied);
        }
        self.handles.remove(handle).ok_or(StatusCode::NoSuchFile)?;
        self.filesystem.record_handle_close();
        Ok(())
    }

    pub(super) async fn read_handle(
        &self,
        handle: &str,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, StatusCode> {
        if self
            .filesystem
            .take_failure(FailurePoint::BundleReadPending)
        {
            std::future::pending::<()>().await;
        }
        if self.filesystem.take_read_failure() {
            tokio::time::sleep(SFTP_SERVER_STALL).await;
        }
        let file = self.file_for_handle(handle)?;
        let data = self.filesystem.read_file(&file, offset, length)?;
        if self
            .filesystem
            .take_failure(FailurePoint::SftpSwapAfterRead)
        {
            self.swap_read_path();
        }
        Ok(data)
    }

    pub(super) fn write_handle(
        &self,
        handle: &str,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<(), StatusCode> {
        let file = self.file_for_handle(handle)?;
        self.filesystem.write_file(&file, offset, data)
    }

    pub(super) fn metadata_handle(&self, handle: &str) -> Result<FileAttributes, StatusCode> {
        let file = self.file_for_handle(handle)?;
        self.filesystem.file_metadata(&file)
    }

    pub(super) fn chmod_handle(
        &self,
        handle: &str,
        permissions: Option<u32>,
    ) -> Result<(), StatusCode> {
        let file = self.file_for_handle(handle)?;
        self.filesystem.chmod_file(&file, permissions)
    }

    pub(super) fn file_for_handle(&self, handle: &str) -> Result<MemoryFile, StatusCode> {
        self.handles
            .get(handle)
            .cloned()
            .ok_or(StatusCode::NoSuchFile)
    }

    fn swap_read_path(&self) {
        self.filesystem.insert_link(MEMORY_FILE, MEMORY_OUTSIDE);
    }
}

impl Drop for MemorySftp {
    fn drop(&mut self) {
        self.filesystem.record_sftp_session_drop(self.handles.len());
    }
}
