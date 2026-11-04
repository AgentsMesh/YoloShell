use crate::{
    JsonlFramer, MAX_HISTORY_SCAN_BYTES, PageSelection, SCAN_CHUNK_BYTES, TranscriptParser,
};
use std::io::Read;
use yolo_conversation_protocol::{
    ConversationError, ConversationLimits, ConversationPartialReason,
};
use yolo_crypto::Sha256Hasher;

pub struct TranscriptSnapshot {
    pub length: u64,
    pub digest: String,
    pub previous_prefix_digest: String,
    pub selection: PageSelection,
}

/// Memory is bounded by one framing record and one projected page, independently of file size.
/// Existing prefix bytes are hashed as well: rewrite-plus-append cannot masquerade as append.
pub fn scan(
    mut input: impl Read,
    length: u64,
    previous_length: u64,
    before: u64,
    limits: ConversationLimits,
    parser: &dyn TranscriptParser,
) -> Result<TranscriptSnapshot, ConversationError> {
    limits.validate()?;
    if length > MAX_HISTORY_SCAN_BYTES {
        return Err(ConversationError::HistoryLimitExceeded);
    }
    let mut selection = PageSelection::new(before, limits);
    let mut framer = JsonlFramer::default();
    let mut digest = Sha256Hasher::new();
    let mut prefix = Sha256Hasher::new();
    let mut offset = 0;
    let mut chunk = [0; SCAN_CHUNK_BYTES];
    while offset < length {
        let wanted = (length - offset).min(SCAN_CHUNK_BYTES as u64) as usize;
        let read = input
            .read(&mut chunk[..wanted])
            .map_err(|_| ConversationError::EvidenceUnavailable)?;
        if read == 0 {
            return Err(ConversationError::EvidenceChanged);
        }
        let bytes = &chunk[..read];
        digest.update(bytes);
        let previous = previous_length.saturating_sub(offset).min(read as u64) as usize;
        prefix.update(&bytes[..previous]);
        framer.push(bytes, |start, end, value| match value {
            Ok(value) => selection.accept(start, end, parser.parse(&value)),
            Err(reason) => selection.issue(reason),
        });
        offset += read as u64;
    }
    if framer.incomplete() {
        selection.issue(ConversationPartialReason::IncompleteRecord);
    }
    Ok(TranscriptSnapshot {
        length,
        digest: digest.finish_hex(),
        previous_prefix_digest: prefix.finish_hex(),
        selection,
    })
}
