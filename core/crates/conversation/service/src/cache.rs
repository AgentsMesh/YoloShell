use yolo_conversation_protocol::{ConversationPage, ConversationReadResult};
use yolo_crypto::Sha256Digest;

pub(super) struct CachedPage {
    pub stamp: Sha256Digest,
    pub key: Sha256Digest,
    pub page: ConversationPage,
}

impl CachedPage {
    pub fn read(
        &self,
        since: Option<Sha256Digest>,
        generation: Option<Sha256Digest>,
    ) -> ConversationReadResult {
        if since == Some(self.page.revision) && generation == Some(self.page.history_generation) {
            ConversationReadResult::Unchanged {
                history_generation: self.page.history_generation,
                revision: self.page.revision,
            }
        } else {
            ConversationReadResult::Page {
                page: self.page.clone(),
            }
        }
    }
}
