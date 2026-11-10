use crate::constants::*;
use serde_json::Value;
use yolo_conversation_protocol::ConversationPart as Part;
use yolo_conversation_transcript::{bounded_text, identifier, json_text, string};

pub(super) fn content(value: &Value) -> Vec<Part> {
    if let Some(text) = value.as_str() {
        return vec![Part::Text {
            text: bounded_text(text),
        }];
    }
    value
        .as_array()
        .map(|parts| parts.iter().filter_map(part).collect())
        .unwrap_or_default()
}

fn part(value: &Value) -> Option<Part> {
    match value.get(TYPE_FIELD).and_then(Value::as_str)? {
        TEXT_FIELD => string(value, TEXT_FIELD).map(|text| Part::Text { text }),
        THINKING => string(value, THINKING).map(|text| Part::Thinking { text }),
        TOOL_USE => Some(Part::ToolCall {
            call_id: identifier(value, ID_FIELD)?,
            name: string(value, NAME_FIELD)?,
            arguments: value.get(INPUT_FIELD).map(json_text).unwrap_or_default(),
        }),
        TOOL_RESULT => Some(Part::ToolResult {
            call_id: identifier(value, TOOL_USE_ID_FIELD)?,
            output: value.get(CONTENT_FIELD).map(json_text).unwrap_or_default(),
            is_error: value
                .get(IS_ERROR_FIELD)
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        IMAGE => Some(Part::ImageUnavailable {
            media_type: value
                .get(SOURCE_FIELD)
                .and_then(|source| string(source, MEDIA_TYPE_FIELD)),
        }),
        kind => Some(Part::Unsupported {
            native_kind: bounded_text(kind),
        }),
    }
}
