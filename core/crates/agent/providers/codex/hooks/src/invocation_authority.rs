use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::ObjectIdentity;
use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodexInvocationAuthority {
    directory: ObjectIdentity,
}

impl CodexInvocationAuthority {
    pub(crate) fn capture(directory: &PrivateDirectory) -> Result<Self> {
        directory.verify_path()?;
        let authority = Self {
            directory: directory.object_identity()?,
        };
        directory.verify_path()?;
        Ok(authority)
    }

    pub(crate) fn require_directory(self, observed: ObjectIdentity) -> Result<()> {
        ensure!(
            self.directory == observed,
            super::constants::errors::SNAPSHOT_DIRECTORY_CHANGED
        );
        Ok(())
    }
}
