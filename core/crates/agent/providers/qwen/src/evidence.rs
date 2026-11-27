use super::{QwenAdapter, constants, metadata};
use crate::agent::{AgentEvidence, AgentState};
use anyhow::Result;
use std::path::Path;
use yolo_agent_evidence::{AnchoredEvidence, EvidenceLayout, read_anchored};
use yolo_paths::identity::ObjectIdentity;

pub const EVIDENCE_LAYOUT: EvidenceLayout = EvidenceLayout::new(
    constants::EVIDENCE_DIR,
    constants::EVIDENCE_RELATIVE_COMPONENTS,
);

pub struct AuthoritativeQwenEvidence {
    evidence: AgentEvidence,
    anchor: AnchoredEvidence,
    identity: ObjectIdentity,
}

impl AuthoritativeQwenEvidence {
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
        let current = QwenAdapter.inspect_authoritative_evidence_with_hook(
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

impl QwenAdapter {
    pub fn inspect_authoritative_evidence_with_hook<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: F,
    ) -> Result<Option<AuthoritativeQwenEvidence>>
    where
        F: FnOnce(),
    {
        if !metadata::main_transcript_path(path) {
            return Ok(None);
        }
        let Some(mut anchor) = AnchoredEvidence::open(data_home, path, EVIDENCE_LAYOUT)? else {
            return Ok(None);
        };
        let Some(identity) = anchor.durable_object_identity()? else {
            return Ok(None);
        };
        let window = read_anchored(&mut anchor)?;
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
        Ok(Some(AuthoritativeQwenEvidence {
            evidence: AgentEvidence {
                provider_session_id: transcript.session_id,
                cwd: transcript.cwd,
                path: path.to_path_buf(),
                data_home: data_home.to_path_buf(),
                // Transcript records are an identity/cwd fence, never lifecycle authority.
                state: AgentState::Unknown,
            },
            anchor,
            identity,
        }))
    }
}

pub fn named_transcript_identity(data_home: &Path, path: &Path) -> Result<Option<ObjectIdentity>> {
    let Some(anchor) = AnchoredEvidence::open(data_home, path, EVIDENCE_LAYOUT)? else {
        return Ok(None);
    };
    let Some(identity) = anchor.durable_object_identity()? else {
        return Ok(None);
    };
    Ok(anchor.verify()?.then_some(identity))
}
