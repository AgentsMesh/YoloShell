use crate::{ConversationAddress, ConversationBinding, ConversationDescriptor, ConversationEntry};
use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;

pub const DEFAULT_ENTRY_LIMIT: u32 = 64;
pub const MAX_ENTRY_LIMIT: u32 = 256;
pub const DEFAULT_BYTE_LIMIT: u32 = 256 * 1024;
pub const MAX_BYTE_LIMIT: u32 = 1024 * 1024;
pub const MAX_WATCH_WAIT_MS: u32 = 1000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationCursor {
    pub address: ConversationAddress,
    pub history_generation: Sha256Digest,
    pub before_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLimits {
    pub entries: u32,
    pub bytes: u32,
}

impl Default for ConversationLimits {
    fn default() -> Self {
        Self {
            entries: DEFAULT_ENTRY_LIMIT,
            bytes: DEFAULT_BYTE_LIMIT,
        }
    }
}

impl ConversationLimits {
    pub fn validate(self) -> Result<(), crate::ConversationError> {
        if self.entries == 0
            || self.entries > MAX_ENTRY_LIMIT
            || self.bytes == 0
            || self.bytes > MAX_BYTE_LIMIT
        {
            return Err(crate::ConversationError::InvalidRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationReadRequest {
    pub address: ConversationAddress,
    pub binding: ConversationBinding,
    pub cursor: Option<ConversationCursor>,
    pub limits: ConversationLimits,
    pub since_revision: Option<Sha256Digest>,
    pub since_generation: Option<Sha256Digest>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAvailability {
    Available,
    NotYetWritten,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationPartialReason {
    IncompleteRecord,
    MalformedRecord,
    OversizedRecord,
    UnsupportedRecord,
    PageByteLimit,
    UnresolvedBranch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationPage {
    pub conversation: ConversationDescriptor,
    pub history_generation: Sha256Digest,
    pub revision: Sha256Digest,
    pub availability: ConversationAvailability,
    pub partial: Vec<ConversationPartialReason>,
    pub entries: Vec<ConversationEntry>,
    pub next_cursor: Option<ConversationCursor>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversationReadResult {
    Page {
        page: ConversationPage,
    },
    Unchanged {
        history_generation: Sha256Digest,
        revision: Sha256Digest,
    },
}
