mod constants;
use constants::*;
use crate::constants::*;
use crate::QwenTranscriptParser;
use serde_json::json;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::TranscriptParser;

fn parse(value: serde_json::Value) -> yolo_conversation_transcript::ParsedRecord {
    QwenTranscriptParser.parse(&value)
}

fn record(kind: &str, parts: serde_json::Value) -> serde_json::Value {
    json!({
        (TYPE_FIELD): kind,
        (UUID_FIELD): "record",
        (PARENT_UUID_FIELD): "parent",
        (TIMESTAMP_FIELD): "now",
        (MESSAGE_FIELD): {(ROLE_FIELD): if kind == ASSISTANT_RECORD { MODEL_ROLE } else { USER_ROLE },
                          (PARTS_FIELD): parts}
    })
}

#[test]
fn user_and_assistant_text_keep_native_links_and_roles() {
    let user = parse(record(USER_RECORD, json!([{(TEXT_FIELD): "hello"}])));
    let entry = user.entry.unwrap();
    assert_eq!(entry.native_id.as_deref(), Some("record"));
    assert_eq!(entry.parent_id.as_deref(), Some("parent"));
    assert_eq!(entry.timestamp.as_deref(), Some("now"));
    assert_eq!(entry.role, Role::User);
    assert_eq!(entry.parts, vec![Part::Text { text: "hello".into() }]);

    let assistant = parse(record(
        ASSISTANT_RECORD,
        json!([{(TEXT_FIELD): "answer"}, {(TEXT_FIELD): "reason", (THOUGHT_FIELD): true}]),
    ));
    let entry = assistant.entry.unwrap();
    assert_eq!(entry.role, Role::Assistant);
    assert_eq!(
        entry.parts,
        vec![
            Part::Text { text: "answer".into() },
            Part::Thinking { text: "reason".into() }
        ]
    );
}

#[test]
fn function_calls_and_results_associate_by_call_id() {
    let call = parse(record(
        ASSISTANT_RECORD,
        json!([{(FUNCTION_CALL): {(ID_FIELD): "call", (NAME_FIELD): TOOL_NAME, (ARGS_FIELD): {(TOOL_PATH_KEY): TOOL_PATH}}}]),
    ));
    let entry = call.entry.unwrap();
    assert!(matches!(
        &entry.parts[0],
        Part::ToolCall { call_id, name, arguments }
            if call_id == "call" && name == TOOL_NAME && arguments.contains(TOOL_PATH_KEY)
    ));

    let success = parse(json!({
        (TYPE_FIELD): TOOL_RESULT_RECORD,
        (MESSAGE_FIELD): {(ROLE_FIELD): USER_ROLE, (PARTS_FIELD): [
            {(FUNCTION_RESPONSE): {(ID_FIELD): "call", (RESPONSE_FIELD): {"output": "ok"}}}
        ]},
        (TOOL_CALL_RESULT): {(STATUS_FIELD): "success", (EXECUTION_STATUS_FIELD): "success"}
    }));
    let entry = success.entry.unwrap();
    assert_eq!(entry.role, Role::Tool);
    assert!(matches!(
        &entry.parts[0],
        Part::ToolResult { call_id, is_error: false, .. } if call_id == "call"
    ));

    let failure = parse(json!({
        (TYPE_FIELD): TOOL_RESULT_RECORD,
        (MESSAGE_FIELD): {(ROLE_FIELD): USER_ROLE, (PARTS_FIELD): [
            {(FUNCTION_RESPONSE): {(ID_FIELD): "call", (RESPONSE_FIELD): {"error": "no"}}}
        ]},
        (TOOL_CALL_RESULT): {(STATUS_FIELD): "error"}
    }));
    assert!(matches!(
        &failure.entry.unwrap().parts[0],
        Part::ToolResult { is_error: true, .. }
    ));
}

#[test]
fn system_telemetry_is_ignored_and_unknown_content_is_explicit() {
    assert!(parse(json!({(TYPE_FIELD): SYSTEM_RECORD})).entry.is_none());
    let unknown = parse(json!({(TYPE_FIELD): "future"}));
    assert!(unknown.entry.is_none() && unknown.unsupported);
    let entry = parse(record(
        ASSISTANT_RECORD,
        json!([{"futurePart": 1}, {(INLINE_DATA): {(MEDIA_TYPE_FIELD): "image/png"}}]),
    ))
    .entry
    .unwrap();
    assert!(matches!(
        &entry.parts[0],
        Part::Unsupported { native_kind } if native_kind == "futurePart"
    ));
    assert!(matches!(
        &entry.parts[1],
        Part::ImageUnavailable { media_type: Some(value) } if value == "image/png"
    ));
}

#[test]
fn user_tool_result_messages_are_tool_role() {
    let result = parse(record(
        USER_RECORD,
        json!([{(FUNCTION_RESPONSE): {(ID_FIELD): "call", (RESPONSE_FIELD): {"output": "ok"}}}]),
    ));
    assert_eq!(result.entry.unwrap().role, Role::Tool);
}
