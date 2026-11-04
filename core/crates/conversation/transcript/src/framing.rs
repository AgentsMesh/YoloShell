use yolo_conversation_protocol::ConversationPartialReason;

pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const SCAN_CHUNK_BYTES: usize = 64 * 1024;
pub const MAX_HISTORY_SCAN_BYTES: u64 = 64 * 1024 * 1024;
const LINE_END: u8 = b'\n';

#[derive(Default)]
pub struct JsonlFramer {
    pending: Vec<u8>,
    start: u64,
    offset: u64,
    oversized: bool,
}

impl JsonlFramer {
    pub fn push(
        &mut self,
        bytes: &[u8],
        mut record: impl FnMut(u64, u64, Result<serde_json::Value, ConversationPartialReason>),
    ) {
        for byte in bytes {
            self.offset += 1;
            if *byte == LINE_END {
                let parsed = if self.oversized {
                    Err(ConversationPartialReason::OversizedRecord)
                } else if self.pending.iter().all(u8::is_ascii_whitespace) {
                    self.pending.clear();
                    self.start = self.offset;
                    continue;
                } else {
                    serde_json::from_slice(&self.pending)
                        .map_err(|_| ConversationPartialReason::MalformedRecord)
                };
                record(self.start, self.offset, parsed);
                self.pending.clear();
                self.oversized = false;
                self.start = self.offset;
            } else if self.pending.len() < MAX_RECORD_BYTES {
                self.pending.push(*byte);
            } else {
                self.oversized = true;
            }
        }
    }

    pub fn incomplete(&self) -> bool {
        self.offset != self.start
    }
}
