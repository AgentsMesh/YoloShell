use super::identity::FileIdentity;
use super::private_file::PrivateFileEntry;
use super::{Paths, RESUME_BINDING_CLEAR_FILE_NAME, RESUME_TRANSACTION_FILE_NAME};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub trait ResumeMarkerNamespace {
    fn resume_marker_file(&self, path: &Path) -> Result<PrivateFileEntry>;
    fn verify_resume_marker_namespace(&self) -> Result<()>;
}

impl Paths {
    pub fn resume_transaction(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(RESUME_TRANSACTION_FILE_NAME)
    }

    pub fn resume_binding_clear(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join(RESUME_BINDING_CLEAR_FILE_NAME)
    }

    pub fn replace_resume_binding_marker<T, N>(
        &self,
        namespace: &N,
        terminal_id: Uuid,
        expected: FileIdentity,
        replacement: &T,
    ) -> Result<()>
    where
        T: Serialize,
        N: ResumeMarkerNamespace,
    {
        let entry = namespace.resume_marker_file(&self.resume_binding_clear(terminal_id))?;
        super::durable_json_guarded::replace_entry_guarded(entry, expected, replacement, || {
            namespace.verify_resume_marker_namespace()
        })?;
        namespace.verify_resume_marker_namespace()
    }
}
