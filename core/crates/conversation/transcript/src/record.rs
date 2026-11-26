use serde_json::Value;
use yolo_conversation_protocol::{ConversationEntry, ConversationPart, ConversationRole};
use yolo_crypto::Sha256Digest;

pub const MAX_INLINE_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_NATIVE_ID_BYTES: usize = 512;
const TEXT_TRUNCATED: &str = "\n[transcript text truncated]";

#[derive(Clone, Debug, Default)]
pub struct ParsedRecord {
    pub entry: Option<NativeEntry>,
    pub clear: bool,
    pub branch_unresolved: bool,
    /// A malformed or content-bearing record that the reader could not project.
    pub unsupported: bool,
    /// A known provider-internal record that is intentionally not conversation.
    pub ignored: bool,
}

impl ParsedRecord {
    pub fn ignored() -> Self {
        Self {
            ignored: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeEntry {
    pub native_id: Option<String>,
    pub parent_id: Option<String>,
    pub timestamp: Option<String>,
    pub role: ConversationRole,
    pub parts: Vec<ConversationPart>,
}

impl NativeEntry {
    pub fn project(self, generation: Sha256Digest, offset: u64) -> ConversationEntry {
        // All values here have infallible JSON serialization (strings, integers, byte arrays).
        let id = Sha256Digest::capture(&(generation, offset, &self.native_id))
            .expect("conversation identity consists only of JSON scalar values");
        ConversationEntry {
            id,
            native_id: self.native_id,
            parent_id: self.parent_id,
            timestamp: self.timestamp,
            role: self.role,
            parts: self.parts,
        }
    }
}

pub trait TranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord;
}

pub fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(bounded_text)
}

pub fn identifier(value: &Value, key: &str) -> Option<String> {
    let text = value.get(key)?.as_str()?;
    (!text.is_empty() && text.len() <= MAX_NATIVE_ID_BYTES && !text.chars().any(char::is_control))
        .then(|| text.to_owned())
}

pub fn bounded_text(text: &str) -> String {
    if text.len() <= MAX_INLINE_TEXT_BYTES {
        return text.to_owned();
    }
    let mut end = MAX_INLINE_TEXT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TEXT_TRUNCATED}", &text[..end])
}

pub fn json_text(value: &Value) -> String {
    match value.as_str() {
        Some(text) => bounded_text(text),
        None => bounded_text(&value.to_string()),
    }
}
