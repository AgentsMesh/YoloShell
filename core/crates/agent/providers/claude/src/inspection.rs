use super::{ClaudeAdapter, constants, evidence_reader, metadata};
use crate::agent::AgentEvidence;
use anyhow::Result;
use std::path::Path;

impl ClaudeAdapter {
    pub(super) fn inspect_evidence_with_hook<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        after_read: F,
    ) -> Result<Option<AgentEvidence>>
    where
        F: FnOnce(),
    {
        let Some(data_home) = data_home_for_path(path) else {
            return Ok(None);
        };
        self.inspect_evidence_at(path, agent_cwd, data_home, after_read)
    }

    pub(super) fn inspect_evidence_at<F>(
        &self,
        path: &Path,
        agent_cwd: &Path,
        data_home: &Path,
        after_read: F,
    ) -> Result<Option<AgentEvidence>>
    where
        F: FnOnce(),
    {
        if !metadata::main_transcript_path(path) {
            return Ok(None);
        }
        let Some(window) = evidence_reader::read_with_hook(data_home, path, after_read)? else {
            return Ok(None);
        };
        let Some(metadata) = metadata::TranscriptMetadata::from_read_window(
            path,
            &window.head,
            &window.values,
            agent_cwd,
        ) else {
            return Ok(None);
        };
        Ok(Some(AgentEvidence {
            provider_session_id: metadata.session_id,
            cwd: metadata.cwd,
            path: path.to_path_buf(),
            data_home: data_home.to_path_buf(),
            // Transcript records are an identity/cwd fence, never lifecycle authority.
            state: crate::agent::AgentState::Unknown,
        }))
    }
}

fn data_home_for_path(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|ancestor| {
            ancestor.file_name().and_then(|name| name.to_str()) == Some(constants::EVIDENCE_DIR)
        })
        .and_then(Path::parent)
}
