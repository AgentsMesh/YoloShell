use crate::history::History;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use yolo_conversation_protocol::{ConversationAddress, ConversationError};

pub(super) const MAX_RETAINED_CONVERSATIONS: usize = 64;
const REGISTRY_OWNERS: usize = 1;

#[derive(Default)]
pub(super) struct Registry {
    entries: VecDeque<(ConversationAddress, Arc<Mutex<History>>)>,
}

impl Registry {
    /// Leases pin the context through blocking reads and cancellation. A busy
    /// context cannot be evicted and recreated with a competing generation.
    pub fn lease(
        &mut self,
        address: &ConversationAddress,
    ) -> Result<Arc<Mutex<History>>, ConversationError> {
        if let Some(index) = self.entries.iter().position(|(key, _)| key == address) {
            let entry = self
                .entries
                .remove(index)
                .expect("index was located in this deque");
            let state = entry.1.clone();
            self.entries.push_back(entry);
            return Ok(state);
        }
        if self.entries.len() == MAX_RETAINED_CONVERSATIONS {
            let idle = self
                .entries
                .iter()
                .position(|(_, state)| Arc::strong_count(state) == REGISTRY_OWNERS)
                .ok_or(ConversationError::ReaderBusy)?;
            self.entries.remove(idle);
        }
        let state = Arc::new(Mutex::new(History::default()));
        self.entries.push_back((address.clone(), state.clone()));
        Ok(state)
    }
}
