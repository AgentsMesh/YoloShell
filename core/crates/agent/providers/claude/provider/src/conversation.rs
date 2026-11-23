use crate::{CONVERSATION_TOKEN, identity};
use std::path::Path;
use yolo_agent_contract::{AgentBinding, AgentKind};
use yolo_conversation_claude::ClaudeTranscriptParser;
use yolo_conversation_protocol::{ConversationError, ConversationProvider};
use yolo_conversation_source::{
    ConversationReadOutcome, ConversationSource, EvidenceFile, conversation_error, require_binding,
};
use yolo_conversation_transcript::TranscriptParser;

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeConversationSource;

impl ClaudeConversationSource {
    fn inspect(
        binding: &AgentBinding,
    ) -> Result<yolo_claude::AuthoritativeClaudeEvidence, ConversationError> {
        yolo_claude::ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                Path::new(&binding.evidence_path),
                Path::new(&binding.cwd),
                Path::new(&binding.data_home),
                || {},
            )
            .map_err(|_| ConversationError::EvidenceUnavailable)?
            .ok_or(ConversationError::EvidenceUnavailable)
    }
}

impl ConversationSource for ClaudeConversationSource {
    fn kind(&self) -> AgentKind {
        identity::kind()
    }

    fn conversation_token(&self) -> ConversationProvider {
        ConversationProvider::new(CONVERSATION_TOKEN)
            .expect("claude conversation token is a valid provider token")
    }

    fn parser(&self) -> &'static dyn TranscriptParser {
        &ClaudeTranscriptParser
    }

    fn verify(&self, binding: &AgentBinding) -> Result<(), ConversationError> {
        let mut evidence = Self::inspect(binding)?;
        require_binding(binding, evidence.evidence())?;
        evidence
            .with_transcript_snapshot(|_, _| Ok(()))
            .map_err(conversation_error)?
            .ok_or(ConversationError::EvidenceChanged)?;
        Ok(())
    }

    fn with_evidence(
        &self,
        binding: &AgentBinding,
        read: &mut dyn FnMut(
            &EvidenceFile<'_>,
        ) -> Result<ConversationReadOutcome, ConversationError>,
    ) -> Result<ConversationReadOutcome, ConversationError> {
        let mut evidence = Self::inspect(binding)?;
        require_binding(binding, evidence.evidence())?;
        evidence
            .with_transcript_snapshot(|file, length| {
                read(&EvidenceFile::new(file, length)).map_err(anyhow::Error::from)
            })
            .map_err(conversation_error)?
            .ok_or(ConversationError::EvidenceChanged)
    }
}
