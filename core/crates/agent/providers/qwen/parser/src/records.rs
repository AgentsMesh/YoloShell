use crate::constants::*;
use crate::parts::parts;
use serde_json::Value;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::{
    NativeEntry, ParsedRecord, TranscriptParser, identifier, string,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct QwenTranscriptParser;

impl TranscriptParser for QwenTranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        let kind = value.get(TYPE_FIELD).and_then(Value::as_str);
        if kind == Some(SYSTEM_RECORD) {
            return ParsedRecord::default();
        }
        let message = value.get(MESSAGE_FIELD).unwrap_or(&Value::Null);
        let parsed = parts(
            message.get(PARTS_FIELD).unwrap_or(&Value::Null),
            value.get(TOOL_CALL_RESULT),
        );
        let role = role(kind, message, &parsed);
        let Some(role) = role else {
            return ParsedRecord {
                unsupported: true,
                ..ParsedRecord::default()
            };
        };
        let unsupported = parsed
            .iter()
            .any(|part| matches!(part, Part::Unsupported { .. }));
        ParsedRecord {
            entry: (!parsed.is_empty()).then(|| NativeEntry {
                native_id: identifier(value, UUID_FIELD),
                parent_id: identifier(value, PARENT_UUID_FIELD),
                timestamp: string(value, TIMESTAMP_FIELD),
                role,
                parts: parsed,
            }),
            unsupported,
            ..ParsedRecord::default()
        }
    }
}

fn role(kind: Option<&str>, message: &Value, parts: &[Part]) -> Option<Role> {
    match kind {
        Some(USER_RECORD) if parts.iter().all(|part| matches!(part, Part::ToolResult { .. })) => {
            Some(Role::Tool)
        }
        Some(USER_RECORD) => Some(Role::User),
        Some(ASSISTANT_RECORD) => Some(Role::Assistant),
        Some(TOOL_RESULT_RECORD) => Some(Role::Tool),
        _ if message.get(ROLE_FIELD).and_then(Value::as_str) == Some(USER_ROLE) => Some(Role::User),
        _ if message.get(ROLE_FIELD).and_then(Value::as_str) == Some(MODEL_ROLE) => {
            Some(Role::Assistant)
        }
        _ => None,
    }
}
