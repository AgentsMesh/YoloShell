use super::*;

use crate::paths::identity::FileIdentity;

pub(crate) const RESUME_BINDING_EVIDENCE_MAX_BYTES: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResumeBindingEvidence {
    file_identity: FileIdentity,
}

impl Host {
    pub(crate) fn capture_resume_binding_evidence(
        &self,
        path: &Path,
    ) -> Result<Option<ResumeBindingEvidence>> {
        let identity = self
            .namespace
            .file(path)?
            .inspect(RESUME_BINDING_EVIDENCE_MAX_BYTES)?;
        self.namespace.verify()?;
        Ok(identity.map(|file_identity| ResumeBindingEvidence { file_identity }))
    }

    pub(crate) fn clear_resume_binding_evidence(
        &self,
        path: &Path,
        evidence: &ResumeBindingEvidence,
    ) -> Result<()> {
        let entry = self.namespace.file(path)?;
        self.namespace.verify()?;
        entry.remove_if_unchanged(evidence.file_identity)?;
        self.namespace.verify()
    }
}
