use crate::{ConversationReadOutcome, EvidenceFile};
use std::sync::Arc;
use yolo_agent_contract::{AgentBinding, AgentKind};
use yolo_conversation_protocol::{ConversationError, ConversationProvider};
use yolo_conversation_transcript::TranscriptParser;

/// A provider's conversation capability. Implementations perform native
/// evidence verification and invoke the service callback with a pinned file.
pub trait ConversationSource: Send + Sync {
    fn kind(&self) -> AgentKind;
    fn conversation_token(&self) -> ConversationProvider;
    fn parser(&self) -> &'static dyn TranscriptParser;
    /// Confirm binding/evidence identity without projecting a page. Resolution
    /// uses this to fail closed before a client receives a conversation address.
    fn verify(&self, binding: &AgentBinding) -> Result<(), ConversationError>;
    fn with_evidence(
        &self,
        binding: &AgentBinding,
        read: &mut dyn FnMut(&EvidenceFile<'_>) -> Result<ConversationReadOutcome, ConversationError>,
    ) -> Result<ConversationReadOutcome, ConversationError>;
}

/// Declarative list of compiled-in conversation sources. Lookup is by agent
/// kind for binding resolution and by conversation token for reads.
#[derive(Clone, Default)]
pub struct ConversationSources {
    entries: Vec<Arc<dyn ConversationSource>>,
}

impl ConversationSources {
    pub fn new(entries: impl IntoIterator<Item = Arc<dyn ConversationSource>>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn source_for_kind(&self, kind: &AgentKind) -> Option<&dyn ConversationSource> {
        self.entries
            .iter()
            .find(|source| source.kind() == *kind)
            .map(Arc::as_ref)
    }

    pub fn source_for_token(&self, token: &ConversationProvider) -> Option<&dyn ConversationSource> {
        self.entries
            .iter()
            .find(|source| source.conversation_token() == *token)
            .map(Arc::as_ref)
    }
}
