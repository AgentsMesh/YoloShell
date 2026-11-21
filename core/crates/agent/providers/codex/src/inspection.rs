use super::evidence_path::AnchoredEvidence;
use super::{CodexAdapter, evidence_reader, metadata};
use crate::agent::AgentEvidence;
use anyhow::Result;
use std::path::Path;

impl CodexAdapter {
    pub(super) fn inspect_evidence_with_hook<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: F,
    ) -> Result<Option<AgentEvidence>>
    where
        F: FnOnce(),
    {
        if !metadata::main_rollout_path(path) {
            return Ok(None);
        }
        let Some(mut anchor) = AnchoredEvidence::open(data_home, path)? else {
            return Ok(None);
        };
        Self::read_evidence(&mut anchor, path, agent_cwd, data_home, after_read)
    }

    pub(super) fn read_evidence(
        anchor: &mut AnchoredEvidence,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: impl FnOnce(),
    ) -> Result<Option<AgentEvidence>> {
        let head = evidence_reader::read_complete_head(anchor.file_mut())?;
        after_read();
        if head != evidence_reader::read_complete_head(anchor.file_mut())? || !anchor.verify()? {
            return Ok(None);
        }
        let records = evidence_reader::parse_bytes(&head);
        let Some(metadata) = metadata::RolloutMetadata::from_head(path, &records, agent_cwd) else {
            return Ok(None);
        };
        Ok(Some(AgentEvidence {
            provider_session_id: metadata.session_id,
            cwd: metadata.cwd,
            path: path.to_path_buf(),
            data_home: data_home.to_path_buf(),
            // Codex rollout JSONL is intentionally opaque for lifecycle
            // state. A hook receipt is required before recovery is offered.
            state: crate::agent::AgentState::Unknown,
        }))
    }
}
