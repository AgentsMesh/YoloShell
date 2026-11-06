use crate::constants::*;
use serde_json::Value;
use yolo_conversation_protocol::ConversationPart as Part;
use yolo_conversation_transcript::{bounded_text, identifier, json_text, string};

pub(super) fn content_parts(update: &Value) -> Vec<Part> {
    match update.get(CONTENT_FIELD) {
        Some(Value::String(text)) if !text.is_empty() => vec![Part::Text {
            text: bounded_text(text),
        }],
        Some(value @ Value::Object(_)) => content_part(value).into_iter().collect(),
        Some(Value::Array(values)) => values.iter().filter_map(content_part).collect(),
        _ => Vec::new(),
    }
}

/// A thought chunk is assistant reasoning, not answer text. Keep the ordering
/// and non-text kinds but publish reasoning as `Thinking` so clients can
/// separate it from the reply.
pub(super) fn thought_parts(update: &Value) -> Vec<Part> {
    content_parts(update)
        .into_iter()
        .map(|part| match part {
            Part::Text { text } => Part::Thinking { text },
            other => other,
        })
        .collect()
}

fn content_part(content: &Value) -> Option<Part> {
    match content.get(TYPE_FIELD).and_then(Value::as_str) {
        Some("text") => text_part(content),
        Some(kind) if kind.starts_with("image") => Some(Part::ImageUnavailable {
            media_type: string(content, MEDIA_TYPE_FIELD)
                .or_else(|| string(content, MEDIA_TYPE_SNAKE_FIELD)),
        }),
        Some(kind) => Some(Part::Unsupported {
            native_kind: bounded_text(kind),
        }),
        None => text_part(content),
    }
}

fn text_part(content: &Value) -> Option<Part> {
    let text = content.get(TEXT_FIELD)?.as_str()?;
    (!text.is_empty()).then(|| Part::Text {
        text: bounded_text(text),
    })
}

pub(super) fn tool_call(update: &Value) -> Option<Part> {
    Some(Part::ToolCall {
        call_id: identifier(update, TOOL_CALL_ID_FIELD)?,
        name: string(update, TITLE_FIELD).unwrap_or_else(|| "tool".into()),
        arguments: update.get(RAW_INPUT_FIELD).map(json_text).unwrap_or_default(),
    })
}

/// A tool call update carries the completion content and status for the same
/// call id; its text content is the tool's output. Status-only deltas (a call
/// moving through pending or in progress) carry no result, so they do not
/// project an empty entry.
pub(super) fn tool_result(update: &Value) -> Option<Part> {
    let call_id = identifier(update, TOOL_CALL_ID_FIELD)?;
    let output = match update.get(CONTENT_FIELD) {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|item| content_part(item.get(CONTENT_FIELD).unwrap_or(item)))
            .map(|part| match part {
                Part::Text { text } => text,
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(content) => content_part(content)
            .map(|part| match part {
                Part::Text { text } => text,
                _ => String::new(),
            })
            .unwrap_or_default(),
        None => String::new(),
    };
    let is_error = update.get(STATUS_FIELD).and_then(Value::as_str) == Some(ERROR_STATUS);
    if output.is_empty() && !is_error {
        return None;
    }
    Some(Part::ToolResult {
        call_id,
        output,
        is_error,
    })
}
