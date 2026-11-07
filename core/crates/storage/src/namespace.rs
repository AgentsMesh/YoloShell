use crate::constants::{ROOT_DIRECTORY, SESSION_DIRECTORY_NAME, SESSION_MANIFEST_NAME};
use crate::error::StorageError;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNamespace {
    root: PathBuf,
}

impl SessionNamespace {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        validate_absolute(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join(SESSION_DIRECTORY_NAME)
    }

    pub fn session_dir(&self, session_id: Uuid) -> Result<PathBuf, StorageError> {
        if session_id.is_nil() {
            return Err(StorageError::NilSession);
        }
        Ok(self
            .sessions_dir()
            .join(session_id.hyphenated().to_string()))
    }

    pub fn manifest(&self, session_id: Uuid) -> Result<PathBuf, StorageError> {
        Ok(self.session_dir(session_id)?.join(SESSION_MANIFEST_NAME))
    }
}

fn validate_absolute(path: &Path) -> Result<(), StorageError> {
    if !path.is_absolute() {
        return Err(StorageError::InvalidPath);
    }
    let normalized =
        path.components()
            .try_fold(
                PathBuf::from(ROOT_DIRECTORY),
                |mut output, component| match component {
                    Component::RootDir => Ok(output),
                    Component::Normal(value) => {
                        output.push(value);
                        Ok(output)
                    }
                    _ => Err(StorageError::InvalidPath),
                },
            )?;
    (normalized == path)
        .then_some(())
        .ok_or(StorageError::InvalidPath)
}
