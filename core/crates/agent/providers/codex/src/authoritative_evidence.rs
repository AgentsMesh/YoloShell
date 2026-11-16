use super::evidence_path::AnchoredEvidence;
use super::{CodexAdapter, evidence_reader, metadata};
use crate::agent::{AgentEvidence, AgentState};
use crate::paths::identity::ObjectIdentity;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct AuthoritativeCodexEvidence {
    evidence: AgentEvidence,
    anchor: AnchoredEvidence,
    objects: Vec<ObjectIdentity>,
    agent_cwd: PathBuf,
}

impl AuthoritativeCodexEvidence {
    pub fn evidence(&self) -> &AgentEvidence {
        &self.evidence
    }

    pub fn same_evidence(&self, other: &Self) -> bool {
        self.objects == other.objects && self.evidence == other.evidence
    }

    /// Read a pinned prefix under object and native identity guards. The caller
    /// must verify prefix content integrity if it publishes bytes while appends occur.
    pub fn with_transcript_snapshot<T>(
        &mut self,
        read: impl FnOnce(&std::fs::File, u64) -> Result<T>,
    ) -> Result<Option<T>> {
        if !self.anchor.verify_object()? || !self.verify()? {
            return Ok(None);
        }
        let length = self.anchor.file_mut().metadata()?.len();
        let result = read(self.anchor.file_mut(), length)?;
        Ok((self.anchor.file_mut().metadata()?.len() >= length
            && self.anchor.verify_object()?
            && self.verify()?)
        .then_some(result))
    }

    pub fn verify(&self) -> Result<bool> {
        if self.anchor.object_identities()?.as_ref() != Some(&self.objects) {
            return Ok(false);
        }
        let current = CodexAdapter.inspect_authoritative_evidence_with_hook(
            &self.evidence.path,
            &self.agent_cwd,
            &self.evidence.data_home,
            || {},
        )?;
        Ok(current.is_some_and(|current| self.same_evidence(&current)))
    }
}

impl CodexAdapter {
    pub fn inspect_authoritative_evidence_with_hook<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: F,
    ) -> Result<Option<AuthoritativeCodexEvidence>>
    where
        F: FnOnce(),
    {
        if !metadata::main_rollout_path(path) {
            return Ok(None);
        }
        let Some(mut anchor) = AnchoredEvidence::open(data_home, path)? else {
            return Ok(None);
        };
        let Some(objects) = anchor.object_identities()? else {
            return Ok(None);
        };
        let head = evidence_reader::read_complete_head(anchor.file_mut())?;
        after_read();
        if !anchor.verify()? || anchor.object_identities()?.as_ref() != Some(&objects) {
            return Ok(None);
        }
        let Some(metadata) = metadata::RolloutMetadata::from_head(path, &head, agent_cwd) else {
            return Ok(None);
        };
        Ok(Some(AuthoritativeCodexEvidence {
            evidence: AgentEvidence {
                provider_session_id: metadata.session_id,
                cwd: metadata.cwd,
                path: path.to_path_buf(),
                data_home: data_home.to_path_buf(),
                state: AgentState::Unknown,
            },
            anchor,
            objects,
            agent_cwd: agent_cwd.to_path_buf(),
        }))
    }
}
