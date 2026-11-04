use crate::constants::*;
use serde_json::Value;
use yolo_conversation_protocol::ConversationPart as Part;
use yolo_conversation_transcript::{bounded_text, identifier, json_text, string};

pub(super) fn parts(value: &Value, tool_result: Option<&Value>) -> Vec<Part> {
    value
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| parse_part(part, tool_result))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_part(value: &Value, tool_result: Option<&Value>) -> Option<Part> {
    if let Some(text) = value.get(TEXT_FIELD).and_then(Value::as_str) {
        let text = bounded_text(text);
        return Some(if value.get(THOUGHT_FIELD).and_then(Value::as_bool) == Some(true) {
            Part::Thinking { text }
        } else {
            Part::Text { text }
        });
    }
    if let Some(call) = value.get(FUNCTION_CALL) {
        return Some(Part::ToolCall {
            call_id: identifier(call, ID_FIELD)?,
            name: string(call, NAME_FIELD)?,
            arguments: call.get(ARGS_FIELD).map(json_text).unwrap_or_default(),
        });
    }
    if let Some(response) = value.get(FUNCTION_RESPONSE) {
        let is_error = tool_result.is_some_and(error_status);
        return Some(Part::ToolResult {
            call_id: identifier(response, ID_FIELD)?,
            output: response
                .get(RESPONSE_FIELD)
                .map(json_text)
                .unwrap_or_default(),
            is_error,
        });
    }
    if let Some(data) = value.get(INLINE_DATA) {
        return Some(Part::ImageUnavailable {
            media_type: string(data, MEDIA_TYPE_FIELD),
        });
    }
    let native_kind = value
        .as_object()
        .and_then(|object| object.keys().next())
        .map(|key| bounded_text(key))
        .unwrap_or_else(|| "empty".into());
    Some(Part::Unsupported { native_kind })
}

fn error_status(value: &Value) -> bool {
    [STATUS_FIELD, EXECUTION_STATUS_FIELD].iter().any(|field| {
        value.get(*field).and_then(Value::as_str) == Some(ERROR_STATUS)
    })
}
