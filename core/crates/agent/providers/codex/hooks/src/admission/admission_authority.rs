use super::{
    CodexAdmissionArtifact, CodexAdmissionPublication, CodexAdmissionRecord, CodexAdmissionStorage,
    CodexInvocationAuthority,
};
use crate::paths::identity::FileIdentity;
use anyhow::Result;
use std::path::Path;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../test_support/admission_authority_api.rs"]
mod test_api;

#[derive(Clone, Debug)]
pub struct CodexAdmissionAuthority {
    storage: CodexAdmissionStorage,
    artifact: CodexAdmissionArtifact,
}

impl CodexAdmissionAuthority {
    pub fn from_publication(
        storage: CodexAdmissionStorage,
        publication: CodexAdmissionPublication,
    ) -> Result<Self> {
        Self::from_artifact(storage, publication.into_artifact())
    }

    pub fn recover(storage: CodexAdmissionStorage) -> Result<Option<Self>> {
        let Some(artifact) = storage.read()? else {
            return Ok(None);
        };
        Self::from_artifact(storage, artifact).map(Some)
    }

    pub fn verify(&self) -> Result<()> {
        self.artifact
            .record()
            .validate_for(&self.artifact.record().request().identity)?;
        self.storage.verify(&self.artifact)
    }

    pub fn record(&self) -> &CodexAdmissionRecord {
        self.artifact.record()
    }

    pub fn path(&self) -> &Path {
        self.storage.path()
    }

    pub(crate) fn file_identity(&self) -> FileIdentity {
        self.artifact.file_identity()
    }

    pub(crate) fn invocation_authority(&self) -> Result<CodexInvocationAuthority> {
        self.storage.invocation_authority()
    }

    pub fn acquire_retirement_writer_fence(&self) -> Result<super::CodexRetirementWriterFence> {
        self.verify()?;
        self.storage.acquire_retirement_writer_fence()
    }

    pub fn exactly_matches(&self, other: &Self) -> bool {
        self.storage.path() == other.storage.path() && self.artifact == other.artifact
    }

    pub(super) fn from_artifact(
        storage: CodexAdmissionStorage,
        artifact: CodexAdmissionArtifact,
    ) -> Result<Self> {
        let authority = Self { storage, artifact };
        authority.verify()?;
        Ok(authority)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_authority_tests.rs"]
mod tests;
