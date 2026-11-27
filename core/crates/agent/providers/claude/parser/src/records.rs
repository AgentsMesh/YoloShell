use crate::constants::*;
use crate::parts::content;
use serde_json::Value;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::{
    NativeEntry, ParsedRecord, TranscriptParser, identifier, string,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeTranscriptParser;

impl TranscriptParser for ClaudeTranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        let kind = value.get(TYPE_FIELD).and_then(Value::as_str);
        if kind == Some(SYSTEM_RECORD)
            && value.get(SUBTYPE_FIELD).and_then(Value::as_str) == Some(CLEAR)
        {
            return ParsedRecord {
                clear: true,
                ..ParsedRecord::default()
            };
        }
        if kind.is_some_and(internal_record) {
            return ParsedRecord::ignored();
        }
        if value.get(SIDECHAIN_FIELD).and_then(Value::as_bool) == Some(true) {
            return ParsedRecord {
                branch_unresolved: true,
                ..ParsedRecord::default()
            };
        }
        let message = value.get(MESSAGE_FIELD).unwrap_or(&Value::Null);
        let (role, parts) = match kind {
            Some(USER_RECORD | ASSISTANT_RECORD) => {
                let parts = content(message.get(CONTENT_FIELD).unwrap_or(&Value::Null));
                let role = if kind == Some(ASSISTANT_RECORD) {
                    Role::Assistant
                } else if parts
                    .iter()
                    .all(|part| matches!(part, Part::ToolResult { .. }))
                {
                    Role::Tool
                } else {
                    Role::User
                };
                (role, parts)
            }
            Some(SYSTEM_RECORD) => (
                Role::Runtime,
                string(value, CONTENT_FIELD)
                    .map(|text| Part::Notice { text })
                    .into_iter()
                    .collect(),
            ),
            Some(SUMMARY) => (
                Role::Runtime,
                string(value, SUMMARY)
                    .map(|summary| Part::Compaction { summary })
                    .into_iter()
                    .collect(),
            ),
            Some(FILE_HISTORY_SNAPSHOT | PROGRESS | QUEUE_OPERATION) => {
                return ParsedRecord::default();
            }
            _ => {
                return ParsedRecord {
                    unsupported: true,
                    ..ParsedRecord::default()
                };
            }
        };
        let parts = if value.get(COMPACT_SUMMARY_FIELD).and_then(Value::as_bool) == Some(true) {
            parts
                .into_iter()
                .map(|part| match part {
                    Part::Text { text } => Part::Compaction { summary: text },
                    other => other,
                })
                .collect()
        } else {
            parts
        };
        let unsupported = parts
            .iter()
            .any(|part| matches!(part, Part::Unsupported { .. }));
        ParsedRecord {
            entry: (!parts.is_empty()).then(|| NativeEntry {
                native_id: identifier(value, UUID_FIELD),
                parent_id: identifier(value, PARENT_UUID_FIELD),
                timestamp: string(value, TIMESTAMP_FIELD),
                role,
                parts,
            }),
            unsupported,
            ..ParsedRecord::default()
        }
    }
}
