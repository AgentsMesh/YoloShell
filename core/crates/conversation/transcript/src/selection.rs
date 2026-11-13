use crate::{NativeEntry, ParsedRecord};
use std::collections::VecDeque;
use yolo_conversation_protocol::{ConversationLimits, ConversationPartialReason};

/// Retains only a bounded newest page while the input is scanned without retaining the file.
pub struct PageSelection {
    pub entries: VecDeque<(u64, NativeEntry)>,
    pub partial: Vec<ConversationPartialReason>,
    pub floor: u64,
    pub has_older: bool,
    pub has_content: bool,
    before: u64,
    limits: ConversationLimits,
    bytes: usize,
}

impl PageSelection {
    pub fn new(before: u64, limits: ConversationLimits) -> Self {
        Self {
            entries: VecDeque::new(),
            partial: Vec::new(),
            floor: 0,
            has_older: false,
            has_content: false,
            before,
            limits,
            bytes: 0,
        }
    }

    pub fn issue(&mut self, reason: ConversationPartialReason) {
        if !self.partial.contains(&reason) {
            self.partial.push(reason);
        }
    }

    pub fn accept(&mut self, start: u64, end: u64, parsed: ParsedRecord) {
        if parsed.clear {
            self.floor = end;
            self.entries.clear();
            self.bytes = 0;
            self.has_older = false;
            self.has_content = false;
            self.partial.clear();
        }
        if parsed.ignored {
            return;
        }
        if parsed.branch_unresolved {
            self.issue(ConversationPartialReason::UnresolvedBranch);
        }
        if parsed.unsupported {
            self.issue(ConversationPartialReason::UnsupportedRecord);
        }
        let Some(entry) = parsed.entry else { return };
        self.has_content = true;
        if start >= self.before {
            return;
        }
        let bytes = entry_size(&entry);
        if bytes > self.limits.bytes as usize {
            self.issue(ConversationPartialReason::PageByteLimit);
            return;
        }
        self.bytes += bytes;
        self.entries.push_back((start, entry));
        while self.entries.len() > self.limits.entries as usize
            || self.bytes > self.limits.bytes as usize
        {
            if let Some((_, removed)) = self.entries.pop_front() {
                self.bytes -= entry_size(&removed);
                self.has_older = true;
            }
        }
    }
}

fn entry_size(entry: &NativeEntry) -> usize {
    // JSON digests are byte arrays. The largest byte gives a conservative wire
    // size before the history generation and resulting entry digest are known.
    let mut projected = entry
        .clone()
        .project(yolo_crypto::Sha256Digest::repeated(0), 0);
    projected.id = yolo_crypto::Sha256Digest::repeated(u8::MAX);
    serde_json::to_vec(&projected).map_or(usize::MAX, |bytes| bytes.len())
}
