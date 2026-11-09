use super::*;

impl MemoryFile {
    pub(super) fn new(data: &[u8], permissions: u32) -> Self {
        Self(Arc::new(Mutex::new(Entry {
            data: data.into(),
            permissions,
            uid: MEMORY_UID,
        })))
    }
}

impl MemoryFilesystem {
    pub fn insert_file(&self, path: &str, data: &[u8], permissions: u32) {
        self.0
            .lock()
            .unwrap()
            .files
            .insert(path.into(), MemoryFile::new(data, permissions));
    }

    pub fn replace_file_permissions(&self, file: &MemoryFile, permissions: u32) {
        file.0.lock().unwrap().permissions = permissions;
    }

    pub fn contains_file(&self, path: &str) -> bool {
        self.0.lock().unwrap().files.contains_key(path)
    }

    pub fn permissions(&self, path: &str) -> Option<u32> {
        let file = self.0.lock().unwrap().files.get(path).cloned()?;
        let permissions = file.0.lock().unwrap().permissions;
        Some(permissions)
    }

    pub fn write_file(
        &self,
        file: &MemoryFile,
        offset: u64,
        mut data: Vec<u8>,
    ) -> Result<(), StatusCode> {
        if self.take_failure(FailurePoint::UploadWrite) {
            return Err(StatusCode::PermissionDenied);
        }
        let mut state = self.0.lock().unwrap();
        if state.failure == Some(FailurePoint::CorruptUpload) {
            state.failure = None;
            data.pop();
        }
        state.writes += COUNTER_INCREMENT;
        drop(state);
        let mut entry = file.0.lock().unwrap();
        let start = usize::try_from(offset).map_err(|_| StatusCode::Failure)?;
        if entry.data.len() < start {
            entry.data.resize(start, MEMORY_FILE_GAP_BYTE);
        }
        let end = start.checked_add(data.len()).ok_or(StatusCode::Failure)?;
        if entry.data.len() < end {
            entry.data.resize(end, MEMORY_FILE_GAP_BYTE);
        }
        entry.data[start..end].copy_from_slice(&data);
        Ok(())
    }

    pub fn read_file(
        &self,
        file: &MemoryFile,
        offset: u64,
        length: u32,
    ) -> Result<Vec<u8>, StatusCode> {
        let entry = file.0.lock().unwrap();
        let start = usize::try_from(offset).map_err(|_| StatusCode::Failure)?;
        if start >= entry.data.len() {
            return Err(StatusCode::Eof);
        }
        let end = start.saturating_add(length as usize).min(entry.data.len());
        Ok(entry.data[start..end].to_vec())
    }

    pub fn file_metadata(&self, file: &MemoryFile) -> Result<FileAttributes, StatusCode> {
        if self.take_failure(FailurePoint::Metadata) {
            return Err(StatusCode::PermissionDenied);
        }
        Ok(file_attributes(file))
    }

    pub fn chmod_file(
        &self,
        file: &MemoryFile,
        permissions: Option<u32>,
    ) -> Result<(), StatusCode> {
        let Some(mode) = permissions else {
            return Err(StatusCode::NoSuchFile);
        };
        file.0.lock().unwrap().permissions = mode;
        Ok(())
    }
}

pub(super) fn attributes(size: u64, permissions: u32) -> FileAttributes {
    FileAttributes {
        size: Some(size),
        uid: Some(MEMORY_UID),
        gid: Some(MEMORY_GID),
        permissions: Some(permissions),
        atime: Some(MEMORY_ATIME),
        mtime: Some(MEMORY_MTIME),
        ..Default::default()
    }
}

pub(super) fn file_attributes(file: &MemoryFile) -> FileAttributes {
    let entry = file.0.lock().unwrap();
    let mut attributes = attributes(entry.data.len() as u64, entry.permissions);
    attributes.uid = Some(entry.uid);
    attributes.set_regular(true);
    attributes
}
