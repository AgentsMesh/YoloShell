use crate::{
    ChatUpdate, ConversationAddress, ConversationAvailability, ConversationCursor,
    ConversationPage, ConversationPartialReason, ConversationRole,
};
use serde::Serialize;
use yolo_crypto::{Sha256Digest, Sha256Hasher};

#[derive(Serialize)]
pub struct ChatEntry<'a> {
    pub id: Sha256Digest,
    pub role: ConversationRole,
    pub timestamp: Option<&'a str>,
    pub updates: Vec<ChatUpdate<'a>>,
}

#[derive(Serialize)]
pub struct ChatPage<'a> {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub revision: Sha256Digest,
    pub history_generation: Sha256Digest,
    pub availability: ConversationAvailability,
    pub partial: &'a [ConversationPartialReason],
    pub next_cursor: Option<&'a ConversationCursor>,
    pub entries: Vec<ChatEntry<'a>>,
}

impl ConversationAddress {
    /// Fixed-width origin and conversation IDs distinguish cloned native transcripts.
    pub fn chat_session_id(&self) -> String {
        let mut digest = Sha256Hasher::new();
        digest.update(self.hub_id.as_uuid().as_bytes());
        digest.update(self.conversation_id.as_bytes());
        digest.finish_hex()
    }
}

impl ConversationPage {
    pub fn chat_page(&self) -> ChatPage<'_> {
        ChatPage {
            session_id: self.conversation.address.chat_session_id(),
            revision: self.revision,
            history_generation: self.history_generation,
            availability: self.availability,
            partial: &self.partial,
            next_cursor: self.next_cursor.as_ref(),
            entries: self
                .entries
                .iter()
                .map(|entry| ChatEntry {
                    id: entry.id,
                    role: entry.role,
                    timestamp: entry.timestamp.as_deref(),
                    updates: entry.chat_updates().collect(),
                })
                .collect(),
        }
    }
}
