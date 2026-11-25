use super::*;
use serde_json::{Value, json};
use yolo_conversation_protocol::*;

mod framing;
mod selection;

struct TestParser;
impl TranscriptParser for TestParser {
    fn parse(&self, value: &Value) -> ParsedRecord {
        ParsedRecord {
            clear: value["clear"] == true,
            branch_unresolved: value["branch"] == true,
            unsupported: value["unknown"] == true,
            ignored: value["ignored"] == true,
            entry: string(value, "text").map(|text| NativeEntry {
                native_id: identifier(value, "id"),
                parent_id: None,
                timestamp: None,
                role: ConversationRole::User,
                parts: vec![ConversationPart::Text { text }],
            }),
        }
    }
}

#[test]
fn text_is_utf8_bounded_and_identity_does_not_truncate() {
    let text = "中".repeat(MAX_INLINE_TEXT_BYTES);
    let bounded = bounded_text(&text);
    assert!(bounded.len() < text.len());
    assert!(bounded.ends_with("[transcript text truncated]"));
    assert!(identifier(&json!({"id": "x".repeat(MAX_NATIVE_ID_BYTES + 1)}), "id").is_none());
    assert!(identifier(&json!({"id": "\n"}), "id").is_none());
    assert!(identifier(&json!({"id": ""}), "id").is_none());
    assert_eq!(
        identifier(&json!({"id": "call-1"}), "id").as_deref(),
        Some("call-1")
    );
    assert!(string(&json!({}), "text").is_none());
    assert!(string(&json!({"text": false}), "text").is_none());
    assert_eq!(json_text(&json!("value")), "value");
    assert_eq!(json_text(&json!({"x":1})), "{\"x\":1}");
}

#[test]
fn entry_identity_is_stable_and_scoped_to_history_and_offset() {
    let parsed = TestParser
        .parse(&json!({"text":"hi", "id":"entry"}))
        .entry
        .unwrap();
    let generation = yolo_crypto::Sha256Digest::repeated(1);
    let first = parsed.clone().project(generation, 1);
    assert_eq!(first, parsed.clone().project(generation, 1));
    assert_ne!(first.id, parsed.clone().project(generation, 2).id);
    assert_ne!(
        first.id,
        parsed.project(yolo_crypto::Sha256Digest::repeated(2), 1).id
    );
}
