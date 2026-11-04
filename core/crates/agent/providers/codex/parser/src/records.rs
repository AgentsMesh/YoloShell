use crate::constants::*;
use crate::parts::response_parts;
use serde_json::Value;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::{
    NativeEntry, ParsedRecord, TranscriptParser, identifier, string,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CodexTranscriptParser;

impl TranscriptParser for CodexTranscriptParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        let kind = value.get(TYPE_FIELD).and_then(Value::as_str);
        let payload = value.get(PAYLOAD_FIELD).unwrap_or(&Value::Null);
        match kind {
            Some(SESSION_META | TURN_CONTEXT | WORLD_STATE | TOKEN_USAGE_RECORD) => {
                ParsedRecord::default()
            }
            Some(COMPACTED) => entry(
                value,
                payload,
                Role::Runtime,
                string(payload, MESSAGE)
                    .map(|summary| Part::Compaction { summary })
                    .into_iter()
                    .collect(),
            ),
            Some(RESPONSE_ITEM) => {
                let role = match payload.get(ROLE_FIELD).and_then(Value::as_str) {
                    Some(USER_ROLE) => Role::User,
                    // Developer and system records are model instructions, not
                    // conversation turns. Publishing them floods the reader
                    // with the provider's own prompt on every session.
                    Some(SYSTEM_ROLE | DEVELOPER_ROLE) => return ParsedRecord::ignored(),
                    _ if matches!(
                        payload.get(TYPE_FIELD).and_then(Value::as_str),
                        Some(FUNCTION_CALL_OUTPUT | CUSTOM_TOOL_CALL_OUTPUT)
                    ) =>
                    {
                        Role::Tool
                    }
                    _ => Role::Assistant,
                };
                let parts = response_parts(payload);
                if role == Role::User && injected_context(&parts) {
                    return ParsedRecord::ignored();
                }
                entry(value, payload, role, parts)
            }
            Some(EVENT_MESSAGE) => event(value, payload),
            _ => ParsedRecord {
                unsupported: true,
                ..ParsedRecord::default()
            },
        }
    }
}

fn event(value: &Value, payload: &Value) -> ParsedRecord {
    match payload.get(TYPE_FIELD).and_then(Value::as_str) {
        // These repeat response_item messages. The response stream is the canonical content.
        Some(
            USER_MESSAGE | AGENT_MESSAGE | AGENT_REASONING | TOKEN_COUNT | TASK_STARTED
            | TASK_COMPLETE | TURN_ABORTED | ITEM_COMPLETED | THREAD_SETTINGS_APPLIED,
        ) => ParsedRecord::default(),
        Some(CONTEXT_CLEARED | SESSION_CLEARED) => ParsedRecord {
            clear: true,
            ..ParsedRecord::default()
        },
        Some(THREAD_ROLLED_BACK) => ParsedRecord {
            clear: true,
            branch_unresolved: true,
            ..ParsedRecord::default()
        },
        Some(WARNING | ERROR) => entry(
            value,
            payload,
            Role::Runtime,
            string(payload, MESSAGE)
                .map(|text| Part::Notice { text })
                .into_iter()
                .collect(),
        ),
        _ => ParsedRecord {
            unsupported: true,
            ..ParsedRecord::default()
        },
    }
}

fn injected_context(parts: &[Part]) -> bool {
    !parts.is_empty()
        && parts.iter().all(|part| match part {
            Part::Text { text } => CONTEXT_MESSAGE_PREFIXES
                .iter()
                .any(|prefix| text.trim_start().starts_with(prefix)),
            _ => false,
        })
}

fn entry(value: &Value, payload: &Value, role: Role, parts: Vec<Part>) -> ParsedRecord {
    let unsupported = parts
        .iter()
        .any(|part| matches!(part, Part::Unsupported { .. }));
    ParsedRecord {
        entry: (!parts.is_empty()).then(|| NativeEntry {
            native_id: identifier(payload, ID_FIELD),
            parent_id: None,
            timestamp: string(value, TIMESTAMP_FIELD),
            role,
            parts,
        }),
        unsupported,
        ..ParsedRecord::default()
    }
}
