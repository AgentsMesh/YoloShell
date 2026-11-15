use crate::GrokTranscriptParser;
use crate::constants::*;
use serde_json::json;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::TranscriptParser;

fn update(kind: &str, fields: serde_json::Value) -> serde_json::Value {
    let mut update = json!({ "sessionUpdate": kind });
    update
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    json!({
        "timestamp": 1793255305,
        "method": "_x.ai/session/update",
        "params": { "sessionId": "s", "update": update }
    })
}

#[test]
fn message_chunks_project_to_user_and_assistant_entries() {
    let user = update(
        USER_CHUNK,
        json!({"content": {"type": "text", "text": "hello"}}),
    );
    let entry = GrokTranscriptParser.parse(&user).entry.unwrap();
    assert_eq!(entry.role, Role::User);
    assert_eq!(entry.parts, [Part::Text { text: "hello".into() }]);

    let agent = update(
        AGENT_CHUNK,
        json!({"content": {"type": "text", "text": "hi"}}),
    );
    let entry = GrokTranscriptParser.parse(&agent).entry.unwrap();
    assert_eq!(entry.role, Role::Assistant);
}

#[test]
fn tool_calls_and_updates_project_to_call_and_result() {
    let call = update(
        TOOL_CALL,
        json!({
            "toolCallId": "call-1",
            "title": "Read file",
            "rawInput": {"path": "/tmp/x"}
        }),
    );
    let entry = GrokTranscriptParser.parse(&call).entry.unwrap();
    assert_eq!(entry.role, Role::Assistant);
    assert!(matches!(entry.parts[0], Part::ToolCall { .. }));

    let result = update(
        TOOL_CALL_UPDATE,
        json!({
            "toolCallId": "call-1",
            "status": "completed",
            "content": [{"type": "content", "content": {"type": "text", "text": "done"}}]
        }),
    );
    let entry = GrokTranscriptParser.parse(&result).entry.unwrap();
    assert_eq!(entry.role, Role::Tool);
    assert_eq!(
        entry.parts,
        [Part::ToolResult {
            call_id: "call-1".into(),
            output: "done".into(),
            is_error: false
        }]
    );
}

#[test]
fn reasoning_chunks_stay_separate_from_answer_text() {
    let thought = update(
        THOUGHT_CHUNK,
        json!({"content": {"type": "text", "text": "considering options"}}),
    );
    let entry = GrokTranscriptParser.parse(&thought).entry.unwrap();
    assert_eq!(entry.role, Role::Assistant);
    assert_eq!(
        entry.parts,
        [Part::Thinking {
            text: "considering options".into()
        }]
    );
}

#[test]
fn non_text_content_is_reported_instead_of_dropped() {
    let audio = update(
        AGENT_CHUNK,
        json!({"content": {"type": "audio", "data": "AAAA"}}),
    );
    let parsed = GrokTranscriptParser.parse(&audio);
    assert!(parsed.unsupported);
    assert_eq!(
        parsed.entry.unwrap().parts,
        [Part::Unsupported {
            native_kind: "audio".into()
        }]
    );

    let image = update(
        USER_CHUNK,
        json!({"content": {"type": "image", "mimeType": "image/png"}}),
    );
    assert_eq!(
        GrokTranscriptParser.parse(&image).entry.unwrap().parts,
        [Part::ImageUnavailable {
            media_type: Some("image/png".into())
        }]
    );
}

#[test]
fn numeric_envelope_timestamps_are_preserved() {
    let user = update(
        USER_CHUNK,
        json!({"content": {"type": "text", "text": "hello"}}),
    );
    assert_eq!(
        GrokTranscriptParser.parse(&user).entry.unwrap().timestamp,
        Some("1793255305".into())
    );
}

#[test]
fn status_only_tool_updates_do_not_project_a_result() {
    let pending = update(
        TOOL_CALL_UPDATE,
        json!({"toolCallId": "call-1", "status": "in_progress"}),
    );
    assert!(GrokTranscriptParser.parse(&pending).entry.is_none());

    let failed = update(
        TOOL_CALL_UPDATE,
        json!({"toolCallId": "call-1", "status": "failed"}),
    );
    assert_eq!(
        GrokTranscriptParser.parse(&failed).entry.unwrap().parts,
        [Part::ToolResult {
            call_id: "call-1".into(),
            output: String::new(),
            is_error: true
        }]
    );
}

#[test]
fn non_conversation_updates_are_ignored() {
    for value in [
        update("hook_execution", json!({"event_name": "stop"})),
        update("turn_completed", json!({ (STOP_REASON_FIELD): END_TURN })),
        update("background_tasks", json!({"tasks": []})),
        json!({"timestamp": 1, "method": "other"}),
    ] {
        assert!(GrokTranscriptParser.parse(&value).entry.is_none());
    }
}
