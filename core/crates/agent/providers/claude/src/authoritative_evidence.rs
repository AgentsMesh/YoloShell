use super::evidence_path::AnchoredEvidence;
use super::{ClaudeAdapter, evidence_reader, metadata};
use crate::agent::AgentEvidence;
use crate::paths::identity::ObjectIdentity;
use anyhow::Result;
use std::path::Path;

pub struct AuthoritativeClaudeEvidence {
    evidence: AgentEvidence,
    anchor: AnchoredEvidence,
    identity: ObjectIdentity,
}

impl AuthoritativeClaudeEvidence {
    pub fn evidence(&self) -> &AgentEvidence {
        &self.evidence
    }

    pub fn identity(&self) -> ObjectIdentity {
        self.identity
    }

    pub fn verify(&self) -> Result<bool> {
        Ok(self.anchor.verify()? && self.anchor.durable_object_identity()? == Some(self.identity))
    }

    /// Reads a pinned prefix under native identity and path guards. The caller
    /// verifies prefix integrity when metadata changes through an allowed append.
    pub fn with_transcript_snapshot<T>(
        &mut self,
        read: impl FnOnce(&std::fs::File, u64) -> Result<T>,
    ) -> Result<Option<T>> {
        if !self.verify_transcript_identity()? {
            return Ok(None);
        }
        let length = self.anchor.file_mut().metadata()?.len();
        let result = read(self.anchor.file_mut(), length)?;
        Ok((self.anchor.file_mut().metadata()?.len() >= length
            && self.verify_transcript_identity()?)
        .then_some(result))
    }

    fn verify_transcript_identity(&self) -> Result<bool> {
        if !self.anchor.verify_object()?
            || self.anchor.durable_object_identity()? != Some(self.identity)
        {
            return Ok(false);
        }
        let current = ClaudeAdapter.inspect_authoritative_evidence_with_hook(
            &self.evidence.path,
            &self.evidence.cwd,
            &self.evidence.data_home,
            || {},
        )?;
        Ok(current.is_some_and(|current| {
            current.identity == self.identity && current.evidence == self.evidence
        }))
    }
}

impl ClaudeAdapter {
    pub fn inspect_authoritative_evidence_with_hook<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: F,
    ) -> Result<Option<AuthoritativeClaudeEvidence>>
    where
        F: FnOnce(),
    {
        if !metadata::main_transcript_path(path) {
            return Ok(None);
        }
        let Some(mut anchor) = AnchoredEvidence::open(data_home, path)? else {
            return Ok(None);
        };
        let Some(identity) = anchor.durable_object_identity()? else {
            return Ok(None);
        };
        let window = evidence_reader::read_anchored(&mut anchor)?;
        after_read();
        if !anchor.verify()? || anchor.durable_object_identity()? != Some(identity) {
            return Ok(None);
        }
        let Some(transcript) = metadata::TranscriptMetadata::from_read_window(
            path,
            &window.head,
            &window.values,
            agent_cwd,
        ) else {
            return Ok(None);
        };
        Ok(Some(AuthoritativeClaudeEvidence {
            evidence: AgentEvidence {
                provider_session_id: transcript.session_id,
                cwd: transcript.cwd,
                path: path.to_path_buf(),
                data_home: data_home.to_path_buf(),
                // Transcript records are an identity/cwd fence, never lifecycle authority.
                state: crate::agent::AgentState::Unknown,
            },
            anchor,
            identity,
        }))
    }
}

pub fn named_transcript_identity(data_home: &Path, path: &Path) -> Result<Option<ObjectIdentity>> {
    let Some(anchor) = AnchoredEvidence::open(data_home, path)? else {
        return Ok(None);
    };
    let Some(identity) = anchor.durable_object_identity()? else {
        return Ok(None);
    };
    Ok(anchor.verify()?.then_some(identity))
}
