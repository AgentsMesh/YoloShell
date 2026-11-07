use crate::conversation_text::entry_line;
use crate::render::assistant_text;
use yolo_conversation_protocol::{ConversationEntry, ConversationPart, ConversationRole};
use yolo_crypto::Sha256Digest;

fn entry(id: u8, role: ConversationRole, parts: Vec<ConversationPart>) -> ConversationEntry {
    ConversationEntry {
        id: Sha256Digest::repeated(id),
        parent_id: None,
        native_id: None,
        timestamp: None,
        role,
        parts,
    }
}

#[test]
fn assistant_text_uses_the_last_text_message() {
    let entries = vec![
        entry(
            1,
            ConversationRole::Assistant,
            vec![ConversationPart::Text { text: "old".into() }],
        ),
        entry(
            2,
            ConversationRole::Assistant,
            vec![
                ConversationPart::Thinking {
                    text: "hidden".into(),
                },
                ConversationPart::Text { text: "new".into() },
            ],
        ),
        entry(
            3,
            ConversationRole::User,
            vec![ConversationPart::Text {
                text: "question".into(),
            }],
        ),
    ];
    assert_eq!(assistant_text(&entries).as_deref(), Some("new"));
}

#[test]
fn entry_text_falls_back_to_the_role() {
    let thinking = entry(
        1,
        ConversationRole::Assistant,
        vec![ConversationPart::Thinking {
            text: "hidden".into(),
        }],
    );
    assert_eq!(entry_line(&thinking), "agent");
    let tool = entry(
        2,
        ConversationRole::Tool,
        vec![ConversationPart::ToolResult {
            call_id: "call".into(),
            output: "out".into(),
            is_error: true,
        }],
    );
    assert_eq!(entry_line(&tool), "tool: tool failed");
}
