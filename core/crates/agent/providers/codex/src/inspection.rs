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
        let Some(head) = evidence_reader::read_with_hook(data_home, path, after_read)? else {
            return Ok(None);
        };
        let Some(metadata) = metadata::RolloutMetadata::from_head(path, &head, agent_cwd) else {
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
