use yolo_conversation_protocol::ConversationReadResult;
use yolo_conversation_transcript::TranscriptSnapshot;
use yolo_crypto::Sha256Digest;

/// Result of one verified transcript snapshot read. Cached pages reuse the
/// service's revision accounting; scanned pages carry the native object and
/// snapshot stamps needed for later cache validation.
pub enum ConversationReadOutcome {
    Cached(ConversationReadResult),
    Scanned {
        object: Sha256Digest,
        stamp: Option<Sha256Digest>,
        snapshot: TranscriptSnapshot,
    },
}
