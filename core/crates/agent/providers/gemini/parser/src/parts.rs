use crate::constants::*;
use serde_json::Value;
use yolo_conversation_protocol::ConversationPart as Part;
use yolo_conversation_transcript::{bounded_text, identifier, json_text, string};

pub(super) fn content_parts(value: &Value) -> Vec<Part> {
    match value.get(CONTENT_FIELD) {
        Some(Value::String(text)) if !text.is_empty() => vec![Part::Text {
            text: bounded_text(text),
        }],
        Some(Value::Array(values)) => parts(values),
        _ => Vec::new(),
    }
}

pub(super) fn parts(values: &[Value]) -> Vec<Part> {
    values.iter().filter_map(parse_part).collect()
}

fn parse_part(value: &Value) -> Option<Part> {
    if let Some(text) = value.get(TEXT_FIELD).and_then(Value::as_str) {
        return (!text.is_empty()).then(|| Part::Text {
            text: bounded_text(text),
        });
    }
    if let Some(response) = value.get(FUNCTION_RESPONSE) {
        return function_result(response);
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

pub(super) fn assistant_parts(value: &Value) -> Vec<Part> {
    let mut parts = content_parts(value);
    if let Some(thoughts) = value.get(THOUGHTS_FIELD).and_then(Value::as_array) {
        parts.extend(thoughts.iter().filter_map(thought));
    }
    if let Some(calls) = value.get(TOOL_CALLS_FIELD).and_then(Value::as_array) {
        parts.extend(calls.iter().filter_map(tool_call));
    }
    parts
}

fn thought(value: &Value) -> Option<Part> {
    let description = value.get(THOUGHT_DESCRIPTION_FIELD)?.as_str()?;
    Some(Part::Thinking {
        text: bounded_text(description),
    })
}

fn tool_call(value: &Value) -> Option<Part> {
    Some(Part::ToolCall {
        call_id: identifier(value, ID_FIELD)?,
        name: string(value, NAME_FIELD)?,
        arguments: value.get(ARGS_FIELD).map(json_text).unwrap_or_default(),
    })
}

/// The `functionResponse` part is the same shape in a user content part and in
/// a synthetic `tool_result` record; keep one extraction so error status is
/// interpreted identically on both paths.
fn function_result(response: &Value) -> Option<Part> {
    Some(Part::ToolResult {
        call_id: identifier(response, ID_FIELD)?,
        output: response
            .get(RESPONSE_FIELD)
            .map(json_text)
            .unwrap_or_default(),
        is_error: response
            .get(RESPONSE_FIELD)
            .is_some_and(|response| response.get(ERROR_FIELD).is_some()),
    })
}

pub(super) fn tool_result(value: &Value) -> Vec<Part> {
    if let Some(response) = value.get(FUNCTION_RESPONSE) {
        if let Some(mut part) = function_result(response) {
            if value.get(STATUS_FIELD).and_then(Value::as_str) == Some(ERROR_STATUS) {
                if let Part::ToolResult { is_error, .. } = &mut part {
                    *is_error = true;
                }
            }
            return vec![part];
        }
    }
    let output = value.get(CONTENT_FIELD).map(json_text).unwrap_or_default();
    let is_error = value.get(STATUS_FIELD).and_then(Value::as_str) == Some(ERROR_STATUS);
    match identifier(value, TOOL_ID_FIELD) {
        Some(call_id) => vec![Part::ToolResult {
            call_id,
            output,
            is_error,
        }],
        None => Vec::new(),
    }
}

pub(super) fn notice(value: &Value) -> Vec<Part> {
    value
        .get(CONTENT_FIELD)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(|text| {
            vec![Part::Notice {
                text: bounded_text(text),
            }]
        })
        .unwrap_or_default()
}

pub(super) fn session_context(value: &Value) -> bool {
    value
        .get(CONTENT_FIELD)
        .and_then(|content| content.as_array())
        .and_then(|parts| parts.first())
        .and_then(|part| part.get(TEXT_FIELD))
        .and_then(Value::as_str)
        .is_some_and(|text| text.starts_with(SESSION_CONTEXT_PREFIX))
}
