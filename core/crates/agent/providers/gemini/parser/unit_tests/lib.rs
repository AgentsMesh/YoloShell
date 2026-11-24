mod constants;
use crate::GeminiTranscriptParser;
use constants::*;
use serde_json::json;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::TranscriptParser;

fn parse(value: serde_json::Value) -> yolo_conversation_transcript::ParsedRecord {
    GeminiTranscriptParser.parse(&value)
}

fn user(content: serde_json::Value) -> serde_json::Value {
    json!({(ID): "u1", (TIMESTAMP): "now", (TYPE): USER, (CONTENT): content})
}

fn gemini(content: &str) -> serde_json::Value {
    json!({(ID): "g1", (TIMESTAMP): "now", (TYPE): GEMINI, (CONTENT): content})
}

#[test]
fn user_and_assistant_text_keep_native_links_and_roles() {
    let entry = parse(user(json!([{(TEXT): "hello"}]))).entry.unwrap();
    assert_eq!(entry.native_id.as_deref(), Some("u1"));
    assert_eq!(entry.timestamp.as_deref(), Some("now"));
    assert_eq!(entry.role, Role::User);
    assert_eq!(entry.parts, vec![Part::Text { text: "hello".into() }]);

    let entry = parse(gemini("answer")).entry.unwrap();
    assert_eq!(entry.role, Role::Assistant);
    assert_eq!(entry.parts, vec![Part::Text { text: "answer".into() }]);
}

#[test]
fn tool_calls_and_function_responses_associate_by_call_id() {
    let call = parse(json!({
        (ID): "g1", (TYPE): GEMINI, (CONTENT): "",
        (TOOL_CALLS): [{(ID): "call-1", (NAME): READ_FILE_TOOL, (ARGS): {(FILE_PATH_ARGUMENT): "a"}}]
    }))
    .entry
    .unwrap();
    assert_eq!(call.role, Role::Assistant);
    assert!(matches!(
        &call.parts[0],
        Part::ToolCall { call_id, name, arguments }
            if call_id == "call-1" && name == READ_FILE_TOOL && arguments.contains(FILE_PATH_ARGUMENT)
    ));

    let result = parse(user(json!([
        {(FUNCTION_RESPONSE): {(ID): "call-1", (RESPONSE): {"output": "ok"}}}
    ])))
    .entry
    .unwrap();
    assert_eq!(result.role, Role::Tool);
    assert!(matches!(
        &result.parts[0],
        Part::ToolResult { call_id, is_error: false, .. } if call_id == "call-1"
    ));

    let failed = parse(user(json!([
        {(FUNCTION_RESPONSE): {(ID): "call-1", (RESPONSE): {(ERROR): "denied"}}}
    ])))
    .entry
    .unwrap();
    assert!(matches!(
        &failed.parts[0],
        Part::ToolResult { is_error: true, .. }
    ));
}

#[test]
fn thoughts_and_duplicate_placeholders_are_projected_once() {
    let placeholder = parse(gemini(""));
    assert!(placeholder.entry.is_none());

    let entry = parse(json!({
        (ID): "g1", (TYPE): GEMINI, (CONTENT): "answer",
        (THOUGHTS): [{(DESCRIPTION): "reason"}]
    }))
    .entry
    .unwrap();
    assert_eq!(
        entry.parts,
        vec![
            Part::Text { text: "answer".into() },
            Part::Thinking { text: "reason".into() }
        ]
    );
}

#[test]
fn session_context_metadata_and_history_rewrites_are_not_messages() {
    assert!(parse(user(json!([{(TEXT): SESSION_CONTEXT}]))).entry.is_none());
    assert!(parse(json!({"$set": {"lastUpdated": "now"}})).entry.is_none());
    assert!(parse(json!({"sessionId": "s", "projectHash": "p"})).entry.is_none());
    let patch = parse(json!({"$patch": {"removeIds": ["a"]}}));
    assert!(patch.entry.is_none() && patch.unsupported);
    let rewind = parse(json!({"$rewindTo": "a"}));
    assert!(rewind.entry.is_none() && rewind.unsupported);
}

#[test]
fn runtime_notices_and_unknown_records_are_explicit() {
    let info = parse(json!({(ID): "i1", (TYPE): INFO, (CONTENT): "compressed"}))
        .entry
        .unwrap();
    assert_eq!(info.role, Role::Runtime);
    assert_eq!(
        info.parts,
        vec![Part::Notice { text: "compressed".into() }]
    );
    let unknown = parse(json!({(TYPE): "future", (CONTENT): "x"}));
    assert!(unknown.entry.is_none() && unknown.unsupported);
    let entry = parse(user(json!([{"futurePart": 1}, {"inlineData": {"mimeType": "image/png"}}])))
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
