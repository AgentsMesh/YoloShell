use super::super::constants::UNSAFE_DIRECTORY_MODE;
use super::*;

impl MemoryFilesystem {
    pub fn chmod(&self, path: &str, permissions: Option<u32>) -> Result<(), StatusCode> {
        if self.take_failure(FailurePoint::Permission) {
            return Err(StatusCode::PermissionDenied);
        }
        if self.take_failure(FailurePoint::UnsafeDirectory) {
            return Ok(());
        }
        let mut state = self.0.lock().unwrap();
        if let Some(mode) = permissions {
            if let Some(file) = state.files.get(path) {
                file.0.lock().unwrap().permissions = mode;
                return Ok(());
            }
            if let Some(directory) = state.directories.get_mut(path) {
                *directory = mode;
                return Ok(());
            }
        }
        Err(StatusCode::NoSuchFile)
    }

    pub fn mkdir(&self, path: &str, permissions: Option<u32>) -> Result<(), StatusCode> {
        if self.take_failure(FailurePoint::Mkdir) {
            return Err(StatusCode::PermissionDenied);
        }
        if self.take_failure(FailurePoint::MkdirResponseLoss) {
            self.0.lock().unwrap().directories.insert(
                path.into(),
                permissions.unwrap_or(crate::remote::constants::RUNTIME_DIRECTORY_MODE),
            );
            return Err(StatusCode::PermissionDenied);
        }
        let mut state = self.0.lock().unwrap();
        if state.files.contains_key(path)
            || state.directories.contains_key(path)
            || state.links.contains_key(path)
        {
            return Err(StatusCode::Failure);
        }
        let mode = if state.failure == Some(FailurePoint::UnsafeDirectory) {
            UNSAFE_DIRECTORY_MODE
        } else {
            permissions.unwrap_or(crate::remote::constants::RUNTIME_DIRECTORY_MODE)
        };
        state.directories.insert(path.into(), mode);
        Ok(())
    }

    pub fn rmdir(&self, path: &str) -> Result<(), StatusCode> {
        let mut state = self.0.lock().unwrap();
        if !state.directories.contains_key(path) {
            return Err(StatusCode::NoSuchFile);
        }
        let prefix = format!("{path}/");
        if state
            .files
            .keys()
            .chain(state.directories.keys())
            .chain(state.links.keys())
            .any(|entry| entry.starts_with(&prefix))
        {
            return Err(StatusCode::Failure);
        }
        state.directories.remove(path);
        Ok(())
    }
}
