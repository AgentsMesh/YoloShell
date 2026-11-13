use super::*;
use crate::constants::*;
use serde_json::json;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::TranscriptParser;

fn parse(payload: serde_json::Value) -> yolo_conversation_transcript::ParsedRecord {
    CodexTranscriptParser.parse(
        &json!({(TYPE_FIELD):RESPONSE_ITEM, (TIMESTAMP_FIELD):"now", (PAYLOAD_FIELD):payload}),
    )
}

#[test]
fn message_projection_preserves_role_text_native_identity_and_time() {
    let parsed = parse(
        json!({(TYPE_FIELD):MESSAGE,(ROLE_FIELD):USER_ROLE,(ID_FIELD):"msg-1",
        (CONTENT_FIELD):[{(TYPE_FIELD):INPUT_TEXT,(TEXT_FIELD):"hello"}]}),
    )
    .entry
    .unwrap();
    assert_eq!(parsed.native_id.as_deref(), Some("msg-1"));
    assert_eq!(parsed.timestamp.as_deref(), Some("now"));
    assert_eq!(parsed.role, Role::User);
    assert_eq!(
        parsed.parts,
        vec![Part::Text {
            text: "hello".into()
        }]
    );
    assert_eq!(
        parse(json!({(TYPE_FIELD):MESSAGE,(CONTENT_FIELD):[]})).entry,
        None
    );
    assert!(parse(json!({(TYPE_FIELD):MESSAGE})).entry.is_none());
    assert!(parse(json!({})).entry.is_none());
}

#[test]
fn system_and_developer_messages_are_not_conversation_turns() {
    for role in [SYSTEM_ROLE, DEVELOPER_ROLE] {
        let projected = parse(json!({(TYPE_FIELD):MESSAGE,(ROLE_FIELD):role,
            (CONTENT_FIELD):[{(TYPE_FIELD):INPUT_TEXT,(TEXT_FIELD):"runtime instructions"}]}));
        assert!(projected.entry.is_none());
    }
}

#[test]
fn injected_user_context_is_not_a_turn_but_a_real_prompt_is() {
    let injected = format!("{}{}", CONTEXT_MESSAGE_PREFIXES[0], "\n  <cwd>/tmp</cwd>");
    for text in [injected.as_str(), CONTEXT_MESSAGE_PREFIXES[1]] {
        let projected = parse(json!({(TYPE_FIELD):MESSAGE,(ROLE_FIELD):USER_ROLE,
            (CONTENT_FIELD):[{(TYPE_FIELD):INPUT_TEXT,(TEXT_FIELD):text}]}));
        assert!(projected.entry.is_none());
    }
    let prompt = parse(json!({(TYPE_FIELD):MESSAGE,(ROLE_FIELD):USER_ROLE,
        (CONTENT_FIELD):[{(TYPE_FIELD):INPUT_TEXT,(TEXT_FIELD):"hello"}]}))
    .entry
    .unwrap();
    assert_eq!(prompt.role, Role::User);
}

#[test]
fn tool_calls_and_results_preserve_cross_page_call_identity() {
    for kind in [FUNCTION_CALL, CUSTOM_TOOL_CALL] {
        let call =
            parse(json!({(TYPE_FIELD):kind,(CALL_ID_FIELD):"call-a",(NAME_FIELD):"shell",(ARGUMENTS_FIELD):{"cmd":"pwd"}}))
                .entry
                .unwrap();
        assert!(
            matches!(&call.parts[0], Part::ToolCall { call_id, arguments, .. }
            if call_id == "call-a" && arguments.contains("pwd"))
        );
    }
    let call = parse(
        json!({(TYPE_FIELD):CUSTOM_TOOL_CALL,(CALL_ID_FIELD):"call-a",(NAME_FIELD):"apply_patch",(INPUT_FIELD):"patch"}),
    )
    .entry
    .unwrap();
    assert!(matches!(&call.parts[0], Part::ToolCall { arguments, .. } if arguments == "patch"));
    for kind in [FUNCTION_CALL_OUTPUT, CUSTOM_TOOL_CALL_OUTPUT] {
        let result =
            parse(json!({(TYPE_FIELD):kind,(CALL_ID_FIELD):"call-a",(OUTPUT_FIELD):"failed",(IS_ERROR_FIELD):true}))
                .entry
                .unwrap();
        assert_eq!(result.role, Role::Tool);
        assert!(
            matches!(&result.parts[0], Part::ToolResult { call_id, is_error: true, .. } if call_id == "call-a")
        );
    }
    assert!(parse(json!({(TYPE_FIELD):FUNCTION_CALL})).entry.is_none());
    assert!(
        parse(json!({(TYPE_FIELD):FUNCTION_CALL_OUTPUT}))
            .entry
            .is_none()
    );
    assert!(
        parse(json!({(TYPE_FIELD):FUNCTION_CALL_OUTPUT,(CALL_ID_FIELD):"call"}))
            .entry
            .is_some()
    );
    assert!(
        parse(json!({(TYPE_FIELD):FUNCTION_CALL,(CALL_ID_FIELD):"call",(NAME_FIELD):"tool"}))
            .entry
            .is_some()
    );
}

#[test]
fn images_never_publish_local_paths_or_unverified_urls() {
    let record = parse(json!({(TYPE_FIELD):MESSAGE,(CONTENT_FIELD):[
        {(TYPE_FIELD):INPUT_IMAGE,"image_url":"file:///private/secret"},
        {(TYPE_FIELD):"future_media"}, {(TYPE_FIELD):TEXT_FIELD}, {}
    ]}));
    assert!(record.unsupported);
    let parts = record.entry.unwrap().parts;
    assert_eq!(parts[0], Part::ImageUnavailable { media_type: None });
    assert!(!serde_json::to_string(&parts).unwrap().contains("secret"));
}

#[test]
fn reasoning_and_runtime_events_are_content_not_lifecycle_authority() {
    let reason =
        parse(json!({(TYPE_FIELD):REASONING,(SUMMARY_FIELD):[{(TEXT_FIELD):SUMMARY_FIELD}, {}]}))
            .entry
            .unwrap();
    assert_eq!(
        reason.parts,
        vec![Part::Thinking {
            text: SUMMARY_FIELD.into()
        }]
    );
    assert!(parse(json!({(TYPE_FIELD):REASONING})).entry.is_none());
    assert!(parse(json!({(TYPE_FIELD):"future_item"})).unsupported);
    for kind in [CONTEXT_CLEARED, SESSION_CLEARED] {
        assert!(
            CodexTranscriptParser
                .parse(&json!({(TYPE_FIELD):EVENT_MESSAGE,(PAYLOAD_FIELD):{(TYPE_FIELD):kind}}))
                .clear
        );
    }
    let rollback = CodexTranscriptParser.parse(
        &json!({(TYPE_FIELD):EVENT_MESSAGE,(PAYLOAD_FIELD):{(TYPE_FIELD):THREAD_ROLLED_BACK}}),
    );
    assert!(rollback.clear && rollback.branch_unresolved);
    for kind in [WARNING, ERROR] {
        assert!(
            CodexTranscriptParser
                .parse(&json!({(TYPE_FIELD):EVENT_MESSAGE,(PAYLOAD_FIELD):{(TYPE_FIELD):kind,(MESSAGE):"notice"}}))
                .entry
                .is_some()
        );
    }
    for kind in [SESSION_META, TURN_CONTEXT, WORLD_STATE, TOKEN_USAGE_RECORD] {
        let parsed = CodexTranscriptParser.parse(&json!({(TYPE_FIELD):kind}));
        assert!(parsed.entry.is_none());
        assert!(!parsed.unsupported, "{kind}");
    }
    for kind in [
        USER_MESSAGE,
        AGENT_MESSAGE,
        AGENT_REASONING,
        TOKEN_COUNT,
        TASK_COMPLETE,
        THREAD_SETTINGS_APPLIED,
    ] {
        let parsed = CodexTranscriptParser
            .parse(&json!({(TYPE_FIELD):EVENT_MESSAGE,(PAYLOAD_FIELD):{(TYPE_FIELD):kind}}));
        assert!(parsed.entry.is_none(), "{kind}");
        assert!(!parsed.unsupported, "{kind}");
    }
    assert!(
        CodexTranscriptParser
            .parse(&json!({(TYPE_FIELD):COMPACTED,(PAYLOAD_FIELD):{(MESSAGE):"sum"}}))
            .entry
            .is_some()
    );
    assert!(
        CodexTranscriptParser
            .parse(&json!({(TYPE_FIELD):COMPACTED}))
            .entry
            .is_none()
    );
    assert!(
        CodexTranscriptParser
            .parse(&json!({(TYPE_FIELD):EVENT_MESSAGE,(PAYLOAD_FIELD):{}}))
            .unsupported
    );
    assert!(CodexTranscriptParser.parse(&json!({})).unsupported);
}
