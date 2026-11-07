use super::*;
use crate::constants::*;
use serde_json::json;
use yolo_conversation_protocol::{ConversationPart as Part, ConversationRole as Role};
use yolo_conversation_transcript::TranscriptParser;

fn parse(kind: &str, content: serde_json::Value) -> yolo_conversation_transcript::NativeEntry {
    ClaudeTranscriptParser
        .parse(
            &json!({(TYPE_FIELD):kind,(UUID_FIELD):"record",(PARENT_UUID_FIELD):"parent",
        (TIMESTAMP_FIELD):"now",(MESSAGE_FIELD):{(CONTENT_FIELD):content}}),
        )
        .entry
        .unwrap()
}

#[test]
fn internal_metadata_is_ignored_without_marking_history_unsupported() {
    for kind in ["attachment", "api-request-blob", "permission-mode", "cost-state"] {
        let record = ClaudeTranscriptParser.parse(&json!({(TYPE_FIELD):kind}));
        assert!(record.ignored && !record.unsupported && record.entry.is_none());
    }
    let sidechain = ClaudeTranscriptParser.parse(
        &json!({(TYPE_FIELD):"attachment",(SIDECHAIN_FIELD):true}),
    );
    assert!(sidechain.ignored && !sidechain.branch_unresolved);
    let unknown = ClaudeTranscriptParser.parse(&json!({(TYPE_FIELD):"future-content"}));
    assert!(unknown.unsupported && !unknown.ignored);
}

#[test]
fn parent_links_and_native_ids_survive_projection() {
    let entry = parse(USER_RECORD, json!("hello"));
    assert_eq!(entry.native_id.as_deref(), Some("record"));
    assert_eq!(entry.parent_id.as_deref(), Some("parent"));
    assert_eq!(entry.timestamp.as_deref(), Some("now"));
    assert_eq!(entry.role, Role::User);
    assert_eq!(
        entry.parts,
        vec![Part::Text {
            text: "hello".into()
        }]
    );
}

#[test]
fn tools_associate_by_call_id_without_merging_unrelated_user_turns() {
    let call = parse(
        ASSISTANT_RECORD,
        json!([
            {(TYPE_FIELD):TOOL_USE,(ID_FIELD):"call",(NAME_FIELD):"Read",(INPUT_FIELD):{"path":"src/lib.rs"}},
            {(TYPE_FIELD):THINKING,(THINKING):"reason"}, {(TYPE_FIELD):TEXT_FIELD,(TEXT_FIELD):"answer"}
        ]),
    );
    assert_eq!(call.role, Role::Assistant);
    assert!(matches!(&call.parts[0], Part::ToolCall { call_id, .. } if call_id == "call"));
    let output = parse(
        USER_RECORD,
        json!([{(TYPE_FIELD):TOOL_RESULT,(TOOL_USE_ID_FIELD):"call",(CONTENT_FIELD):"result",(IS_ERROR_FIELD):true}]),
    );
    assert_eq!(output.role, Role::Tool);
    assert!(
        matches!(&output.parts[0], Part::ToolResult { call_id, is_error:true, .. } if call_id == "call")
    );
    assert_eq!(
        parse(
            USER_RECORD,
            json!([{(TYPE_FIELD):TOOL_RESULT,(TOOL_USE_ID_FIELD):"call"}])
        )
        .role,
        Role::Tool
    );
    assert!(matches!(
        parse(
            ASSISTANT_RECORD,
            json!([{(TYPE_FIELD):TOOL_USE,(ID_FIELD):"call",(NAME_FIELD):"Read"}])
        )
        .parts[0],
        Part::ToolCall { .. }
    ));
}

#[test]
fn image_paths_remain_private_and_unknown_blocks_remain_explicit() {
    let entry = parse(
        USER_RECORD,
        json!([
            {(TYPE_FIELD):IMAGE,(SOURCE_FIELD):{(MEDIA_TYPE_FIELD):"image/png","data":"secret"}},
            {(TYPE_FIELD):IMAGE}, {(TYPE_FIELD):"future"}, {}, {(TYPE_FIELD):TEXT_FIELD},
            {(TYPE_FIELD):THINKING}, {(TYPE_FIELD):TOOL_USE}, {(TYPE_FIELD):TOOL_RESULT}
        ]),
    );
    assert_eq!(entry.parts.len(), 3);
    assert_eq!(
        entry.parts[0],
        Part::ImageUnavailable {
            media_type: Some("image/png".into())
        }
    );
    assert!(
        !serde_json::to_string(&entry.parts)
            .unwrap()
            .contains("secret")
    );
    assert!(
        ClaudeTranscriptParser
            .parse(&json!({(TYPE_FIELD):USER_RECORD,(MESSAGE_FIELD):{(CONTENT_FIELD):[{(TYPE_FIELD):"future"}]}}))
            .unsupported
    );
}

#[test]
fn resets_compaction_sidechains_and_incomplete_native_shapes_are_distinct() {
    let parser = ClaudeTranscriptParser;
    assert!(
        parser
            .parse(&json!({(TYPE_FIELD):SYSTEM_RECORD,(SUBTYPE_FIELD):CLEAR}))
            .clear
    );
    assert!(
        parser
            .parse(&json!({(TYPE_FIELD):ASSISTANT_RECORD,(SIDECHAIN_FIELD):true}))
            .branch_unresolved
    );
    let compact = parser
        .parse(&json!({(TYPE_FIELD):USER_RECORD,(COMPACT_SUMMARY_FIELD):true,(MESSAGE_FIELD):{(CONTENT_FIELD):SUMMARY}}))
        .entry
        .unwrap();
    assert_eq!(
        compact.parts,
        vec![Part::Compaction {
            summary: SUMMARY.into()
        }]
    );
    assert!(parser.parse(&json!({(TYPE_FIELD):USER_RECORD,(COMPACT_SUMMARY_FIELD):true,(MESSAGE_FIELD):{(CONTENT_FIELD):[{(TYPE_FIELD):IMAGE}]}})).entry.is_some());
    for value in [
        json!({(TYPE_FIELD):SUMMARY,(SUMMARY):"sum"}),
        json!({(TYPE_FIELD):SYSTEM_RECORD,(CONTENT_FIELD):"notice"}),
    ] {
        assert_eq!(parser.parse(&value).entry.unwrap().role, Role::Runtime);
    }
    for kind in [
        PROGRESS,
        QUEUE_OPERATION,
        FILE_HISTORY_SNAPSHOT,
        SUMMARY,
        SYSTEM_RECORD,
        USER_RECORD,
    ] {
        assert!(parser.parse(&json!({(TYPE_FIELD):kind})).entry.is_none());
    }
    assert!(parser.parse(&json!({})).unsupported);
    assert!(
        parser
            .parse(&json!({(TYPE_FIELD):ASSISTANT_RECORD,(MESSAGE_FIELD):{(CONTENT_FIELD):true}}))
            .entry
            .is_none()
    );
}
