use crate::constants::*;
use serde_json::Value;
use yolo_conversation_protocol::ConversationPart as Part;
use yolo_conversation_transcript::{identifier, json_text, string};

pub(super) fn response_parts(payload: &Value) -> Vec<Part> {
    match payload.get(TYPE_FIELD).and_then(Value::as_str) {
        Some(MESSAGE) => payload
            .get(CONTENT_FIELD)
            .and_then(Value::as_array)
            .map(|parts| parts.iter().filter_map(content).collect())
            .unwrap_or_default(),
        Some(REASONING) => payload
            .get(SUMMARY_FIELD)
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| string(part, TEXT_FIELD).map(|text| Part::Thinking { text }))
                    .collect()
            })
            .unwrap_or_default(),
        Some(FUNCTION_CALL | CUSTOM_TOOL_CALL) => {
            match (
                identifier(payload, CALL_ID_FIELD),
                string(payload, NAME_FIELD),
            ) {
                (Some(call_id), Some(name)) => vec![Part::ToolCall {
                    call_id,
                    name,
                    arguments: payload
                        .get(ARGUMENTS_FIELD)
                        .or_else(|| payload.get(INPUT_FIELD))
                        .map(json_text)
                        .unwrap_or_default(),
                }],
                _ => Vec::new(),
            }
        }
        Some(FUNCTION_CALL_OUTPUT | CUSTOM_TOOL_CALL_OUTPUT) => identifier(payload, CALL_ID_FIELD)
            .map(|call_id| {
                vec![Part::ToolResult {
                    call_id,
                    output: payload.get(OUTPUT_FIELD).map(json_text).unwrap_or_default(),
                    is_error: payload
                        .get(IS_ERROR_FIELD)
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                }]
            })
            .unwrap_or_default(),
        Some(native_kind) => vec![Part::Unsupported {
            native_kind: native_kind.to_owned(),
        }],
        None => Vec::new(),
    }
}

fn content(value: &Value) -> Option<Part> {
    match value.get(TYPE_FIELD).and_then(Value::as_str)? {
        INPUT_TEXT | OUTPUT_TEXT | TEXT_FIELD => {
            string(value, TEXT_FIELD).map(|text| Part::Text { text })
        }
        INPUT_IMAGE | IMAGE => Some(Part::ImageUnavailable { media_type: None }),
        kind => Some(Part::Unsupported {
            native_kind: kind.to_owned(),
        }),
    }
}
