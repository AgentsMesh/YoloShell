use crate::constants::*;
use crate::parts::{content_parts, thought_parts, tool_call, tool_result};
use serde_json::Value;
use yolo_conversation_protocol::ConversationRole as Role;
use yolo_conversation_transcript::{NativeEntry, ParsedRecord, TranscriptParser, bounded_text};

/// Grok records each session update as an ACP notification envelope. Only the
/// conversation-bearing updates project; hook executions, turn journals and
/// background-task snapshots are not conversation content.
#[derive(Clone, Copy, Debug, Default)]
pub struct GrokTranscriptParser;

impl TranscriptParser for GrokTranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        let Some(update) = value
            .get(PARAMS_FIELD)
            .and_then(|params| params.get(UPDATE_FIELD))
        else {
            return ParsedRecord::default();
        };
        let Some(kind) = update.get(KIND_FIELD).and_then(Value::as_str) else {
            return ParsedRecord::default();
        };
        let (role, parts) = match kind {
            USER_CHUNK => (Role::User, content_parts(update)),
            AGENT_CHUNK => (Role::Assistant, content_parts(update)),
            THOUGHT_CHUNK => (Role::Assistant, thought_parts(update)),
            TOOL_CALL => (Role::Assistant, tool_call(update).into_iter().collect()),
            TOOL_CALL_UPDATE => (Role::Tool, tool_result(update).into_iter().collect()),
            _ => return ParsedRecord::default(),
        };
        let unsupported = parts.iter().any(|part| {
            matches!(part, yolo_conversation_protocol::ConversationPart::Unsupported { .. })
        });
        ParsedRecord {
            entry: (!parts.is_empty()).then(|| NativeEntry {
                native_id: None,
                parent_id: None,
                timestamp: timestamp(value),
                role,
                parts,
            }),
            unsupported,
            ..ParsedRecord::default()
        }
    }
}

/// The ACP envelope stamps each update with epoch seconds as a JSON number;
/// keep a string stamp as-is and render a numeric one as its decimal text.
fn timestamp(value: &Value) -> Option<String> {
    match value.get(TIMESTAMP_FIELD)? {
        Value::String(text) => Some(bounded_text(text)),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}
