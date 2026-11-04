use super::RemoteError;
use super::constants::{
    CURRENT_PATH_COMPONENT, MAX_FILE_READ_BYTES, PARENT_PATH_COMPONENT, PATH_SEPARATOR, ROOT_PATH,
};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadRequest {
    path: PathBuf,
    offset: u64,
    length: u64,
}

impl FileReadRequest {
    pub fn new(path: impl Into<PathBuf>, offset: u64, length: u64) -> Self {
        Self {
            path: path.into(),
            offset,
            length,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn length(&self) -> u64 {
        self.length
    }
}

#[derive(Debug, Clone)]
pub struct FileReadPolicy {
    roots: Vec<PathBuf>,
    max_bytes: u64,
}

impl FileReadPolicy {
    pub fn standard(roots: Vec<PathBuf>) -> Result<Self, RemoteError> {
        Self::new(roots, MAX_FILE_READ_BYTES)
    }

    pub fn new(roots: Vec<PathBuf>, max_bytes: u64) -> Result<Self, RemoteError> {
        if roots.is_empty()
            || max_bytes == 0
            || max_bytes > MAX_FILE_READ_BYTES
            || roots.iter().any(|path| !scoped_absolute(path))
        {
            return Err(RemoteError::PathDenied(PathBuf::new()));
        }
        Ok(Self { roots, max_bytes })
    }

    pub fn authorize(
        &self,
        request: FileReadRequest,
        resolved: impl Into<PathBuf>,
    ) -> Result<AuthorizedFileRead, RemoteError> {
        self.preflight(&request)?;
        let resolved = resolved.into();
        let resolved_inside = self.roots.iter().any(|root| resolved.starts_with(root));
        if !strict_absolute(&resolved) || !resolved_inside {
            return Err(RemoteError::PathDenied(request.path));
        }
        Ok(AuthorizedFileRead { request, resolved })
    }

    pub fn preflight(&self, request: &FileReadRequest) -> Result<(), RemoteError> {
        let requested_inside = self
            .roots
            .iter()
            .any(|root| request.path().starts_with(root));
        if !strict_absolute(request.path())
            || !requested_inside
            || request.length() == 0
            || request.length() > self.max_bytes
            || request.offset().checked_add(request.length()).is_none()
        {
            return Err(RemoteError::PathDenied(request.path.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedFileRead {
    request: FileReadRequest,
    resolved: PathBuf,
}

impl AuthorizedFileRead {
    pub fn request(&self) -> &FileReadRequest {
        &self.request
    }

    pub fn resolved_path(&self) -> &Path {
        &self.resolved
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadResult {
    pub resolved_path: PathBuf,
    pub data: Vec<u8>,
    pub eof: bool,
}

impl FileReadResult {
    pub fn validate(&self, read: &AuthorizedFileRead) -> Result<(), RemoteError> {
        if self.resolved_path != read.resolved || self.data.len() as u64 > read.request.length() {
            return Err(RemoteError::LimitExceeded);
        }
        Ok(())
    }
}

pub fn strict_absolute(path: &Path) -> bool {
    let Some(value) = path.to_str() else {
        return false;
    };
    if value == ROOT_PATH {
        return true;
    }
    value.strip_prefix(PATH_SEPARATOR).is_some_and(|relative| {
        relative.split(PATH_SEPARATOR).all(|item| {
            !item.is_empty() && item != CURRENT_PATH_COMPONENT && item != PARENT_PATH_COMPONENT
        })
    })
}

pub(crate) fn scoped_absolute(path: &Path) -> bool {
    strict_absolute(path)
        && path
            .components()
            .any(|item| matches!(item, Component::Normal(_)))
}
