use anyhow::Error;
use std::fs::File;
use std::path::Path;
use yolo_agent_contract::{AgentBinding, AgentEvidence};
use yolo_conversation_protocol::ConversationError;

/// One provider-verified transcript handle. The file is borrowed for the
/// duration of the callback, so a provider cannot hand out a handle that
/// outlives its native identity checks.
pub struct EvidenceFile<'a> {
    file: &'a File,
    length: u64,
}

impl<'a> EvidenceFile<'a> {
    pub fn new(file: &'a File, length: u64) -> Self {
        Self { file, length }
    }

    pub fn file(&self) -> &File {
        self.file
    }

    pub fn length(&self) -> u64 {
        self.length
    }
}

pub fn require_binding(
    binding: &AgentBinding,
    evidence: &AgentEvidence,
) -> Result<(), ConversationError> {
    if evidence.provider_session_id != binding.provider_session_id
        || evidence.cwd != Path::new(&binding.cwd)
        || evidence.data_home != Path::new(&binding.data_home)
        || evidence.path != Path::new(&binding.evidence_path)
    {
        return Err(ConversationError::BindingChanged);
    }
    Ok(())
}

pub fn conversation_error(error: Error) -> ConversationError {
    error
        .downcast_ref::<ConversationError>()
        .copied()
        .unwrap_or(ConversationError::EvidenceUnavailable)
}
