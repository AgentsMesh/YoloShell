use crate::constants::*;
use crate::parts::{assistant_parts, content_parts, notice, session_context, tool_result};
use serde_json::Value;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::{
    NativeEntry, ParsedRecord, TranscriptParser, identifier, string,
};

/// Gemini appends records and may later append a revised record with the same
/// `id` (streamed tool calls) or rewrite history with `$patch`/`$rewindTo`.
/// Empty placeholders are skipped; history rewrites are reported as partial
/// because the projection cannot retract already emitted records.
#[derive(Clone, Copy, Debug, Default)]
pub struct GeminiTranscriptParser;

impl TranscriptParser for GeminiTranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        if value.get(MESSAGE_PATCH).is_some() || value.get(REWIND_MARKER).is_some() {
            return ParsedRecord {
                unsupported: true,
                ..ParsedRecord::default()
            };
        }
        let Some(kind) = value.get(TYPE_FIELD).and_then(Value::as_str) else {
            // Header metadata, `$set` patches and unknown non-message records.
            return ParsedRecord::default();
        };
        if kind == USER_RECORD && session_context(value) {
            return ParsedRecord::default();
        }
        let (role, parts) = match kind {
            USER_RECORD => {
                let parts = content_parts(value);
                (
                    if !parts.is_empty()
                        && parts.iter().all(|part| matches!(part, Part::ToolResult { .. }))
                    {
                        Role::Tool
                    } else {
                        Role::User
                    },
                    parts,
                )
            }
            GEMINI_RECORD => (Role::Assistant, assistant_parts(value)),
            TOOL_RESULT_RECORD => (Role::Tool, tool_result(value)),
            INFO_RECORD | WARNING_RECORD | ERROR_RECORD => (Role::Runtime, notice(value)),
            _ => {
                return ParsedRecord {
                    unsupported: true,
                    ..ParsedRecord::default()
                };
            }
        };
        let unsupported = parts
            .iter()
            .any(|part| matches!(part, Part::Unsupported { .. }));
        ParsedRecord {
            entry: (!parts.is_empty()).then(|| NativeEntry {
                native_id: identifier(value, ID_FIELD),
                parent_id: None,
                timestamp: string(value, TIMESTAMP_FIELD),
                role,
                parts,
            }),
            unsupported,
            ..ParsedRecord::default()
        }
    }
}
