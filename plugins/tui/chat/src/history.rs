use crate::MAX_HISTORY_PAGES;
use std::collections::VecDeque;
use yolo_conversation_protocol::{ConversationCursor, ConversationEntry, ConversationPage};

#[derive(Default)]
pub(crate) struct History {
    pub version: uuid::Uuid,
    pub latest: Option<ConversationPage>,
    older: VecDeque<ConversationPage>,
}

impl History {
    pub fn page_count(&self) -> usize {
        self.older.len() + usize::from(self.latest.is_some())
    }
    pub fn older_cursor(&self) -> Option<&ConversationCursor> {
        if self.page_count() >= MAX_HISTORY_PAGES {
            return None;
        }
        self.older
            .front()
            .or(self.latest.as_ref())?
            .next_cursor
            .as_ref()
    }
    pub fn latest(&mut self, page: ConversationPage) -> bool {
        self.version = uuid::Uuid::new_v4();
        let reset = self.latest.as_ref().is_some_and(|old| {
            old.history_generation != page.history_generation
                || (!old.entries.is_empty()
                    && !page.entries.is_empty()
                    && !old
                        .entries
                        .iter()
                        .any(|entry| page.entries.iter().any(|next| next.id == entry.id)))
        });
        if reset {
            self.older.clear();
        } else if let Some(mut previous) = self.latest.take() {
            previous
                .entries
                .retain(|entry| !page.entries.iter().any(|next| next.id == entry.id));
            if !previous.entries.is_empty() {
                self.older.push_back(previous);
            }
        }
        self.latest = Some(page);
        while self.page_count() > MAX_HISTORY_PAGES {
            self.older.pop_front();
        }
        reset
    }
    pub fn prepend(&mut self, page: ConversationPage) {
        if self.page_count() < MAX_HISTORY_PAGES {
            self.version = uuid::Uuid::new_v4();
            self.older.push_front(page);
        }
    }
    pub fn entries(&self) -> Vec<&ConversationEntry> {
        let mut result: Vec<&ConversationEntry> = Vec::new();
        for entry in self
            .older
            .iter()
            .chain(self.latest.iter())
            .flat_map(|page| &page.entries)
        {
            // Newer pages replace an overlapping entry without moving its chronological slot.
            if let Some(index) = result.iter().position(|old| old.id == entry.id) {
                result[index] = entry;
            } else {
                result.push(entry);
            }
        }
        result
    }
}
