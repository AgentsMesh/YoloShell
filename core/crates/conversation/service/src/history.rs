use crate::cache::CachedPage;
use std::collections::VecDeque;
use uuid::Uuid;
use yolo_conversation_protocol::ConversationAddress;
use yolo_conversation_transcript::TranscriptSnapshot;
use yolo_crypto::Sha256Digest;

const MAX_CHECKPOINTS: usize = 64;
const MAX_CACHED_PAGES: usize = 16;
const HISTORY_DOMAIN: &str = "yolo.conversation.history.v1";

pub(super) struct Checkpoint {
    pub address: ConversationAddress,
    pub object: Sha256Digest,
    pub length: u64,
    pub digest: String,
    pub floor: u64,
    pub generation: Sha256Digest,
    pub cache: Option<CachedPage>,
}

#[derive(Default)]
pub(super) struct History {
    checkpoints: VecDeque<Checkpoint>,
}

impl History {
    pub fn get(&self, address: &ConversationAddress) -> Option<&Checkpoint> {
        self.checkpoints
            .iter()
            .find(|entry| &entry.address == address)
    }

    pub fn record(
        &mut self,
        address: &ConversationAddress,
        object: Sha256Digest,
        scan: &TranscriptSnapshot,
    ) -> Sha256Digest {
        let previous = self.get(address);
        let unchanged_prefix = previous.is_some_and(|entry| {
            entry.object == object
                && entry.length <= scan.length
                && entry.digest == scan.previous_prefix_digest
                && entry.floor == scan.selection.floor
        });
        let generation = previous
            .filter(|_| unchanged_prefix)
            .map(|entry| entry.generation)
            .unwrap_or_else(|| {
                Sha256Digest::capture(&(HISTORY_DOMAIN, Uuid::new_v4().to_string()))
                    .expect("history generation is a pair of JSON strings")
            });
        self.checkpoints.retain(|entry| &entry.address != address);
        self.checkpoints.push_back(Checkpoint {
            address: address.clone(),
            object,
            length: scan.length,
            digest: scan.digest.clone(),
            floor: scan.selection.floor,
            generation,
            cache: None,
        });
        while self.checkpoints.len() > MAX_CHECKPOINTS {
            self.checkpoints.pop_front();
        }
        generation
    }

    pub fn cache_page(&mut self, address: &ConversationAddress, cache: CachedPage) {
        if let Some(checkpoint) = self
            .checkpoints
            .iter_mut()
            .find(|entry| &entry.address == address)
        {
            checkpoint.cache = Some(cache);
        }
        let mut extra = self
            .checkpoints
            .iter()
            .filter(|entry| entry.cache.is_some())
            .count()
            .saturating_sub(MAX_CACHED_PAGES);
        for checkpoint in &mut self.checkpoints {
            if extra == 0 {
                break;
            }
            if checkpoint.cache.take().is_some() {
                extra -= 1;
            }
        }
    }
}
