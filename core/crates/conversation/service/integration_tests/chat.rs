use super::constants::{AGENT_MESSAGE_CHUNK, OUTPUT_TEXT, RESPONSE_ITEM};
use serde_json::{Value, json};
use yolo_conversation_protocol::ConversationEntry;
use yolo_conversation_source::ConversationSource;
use yolo_crypto::Sha256Digest;

#[test]
fn all_registered_native_transcript_shapes_yield_the_same_chat_message() {
    let text = "回复";
    let cases: [(&dyn ConversationSource, Value); 5] = [
        (
            &yolo_codex_provider::CodexConversationSource,
            json!({"type":RESPONSE_ITEM,"payload":{"type":"message","role":"assistant","content":[{"type":OUTPUT_TEXT,"text":text}]}}),
        ),
        (
            &yolo_claude_provider::ClaudeConversationSource,
            json!({"type":"assistant","message":{"content":[{"type":"text","text":text}]}}),
        ),
        (
            &yolo_qwen_provider::QwenConversationSource,
            json!({"type":"assistant","message":{"parts":[{"text":text}]}}),
        ),
        (
            &yolo_gemini_provider::GeminiConversationSource,
            json!({"type":"gemini","content":[{"text":text}]}),
        ),
        (
            &yolo_grok_provider::GrokConversationSource,
            json!({"params":{"update":{"sessionUpdate":AGENT_MESSAGE_CHUNK,"content":{"type":"text","text":text}}}}),
        ),
    ];
    for (source, record) in cases {
        let native = source.parser().parse(&record).entry.unwrap();
        let entry = ConversationEntry {
            id: Sha256Digest::capture(&record).unwrap(),
            native_id: native.native_id,
            parent_id: native.parent_id,
            timestamp: native.timestamp,
            role: native.role,
            parts: native.parts,
        };
        assert_eq!(
            json!(entry.chat_updates().collect::<Vec<_>>()),
            json!([{
                "sessionUpdate":AGENT_MESSAGE_CHUNK,"content":{"type":"text","text":text}
            }])
        );
    }
}
