//! ACP v1 session updates projected from authenticated native history.
use crate::{ConversationEntry, ConversationPart, ConversationRole};
mod protocol;
pub use protocol::*;

impl ConversationEntry {
    /// Entries are replayed as a unit. Preserve the original entry ID and part order.
    pub fn chat_updates(&self) -> impl Iterator<Item = ChatUpdate<'_>> {
        self.parts
            .iter()
            .enumerate()
            .map(|(index, part)| match part {
                ConversationPart::Text { text } => {
                    let content = ChatContent::Text { text: text.as_str() };
                    match self.role {
                        ConversationRole::User => ChatUpdate::UserMessageChunk { content },
                        ConversationRole::Assistant => ChatUpdate::AgentMessageChunk { content },
                        ConversationRole::Runtime | ConversationRole::Tool => info(text),
                    }
                }
                ConversationPart::Thinking { text } => ChatUpdate::AgentThoughtChunk {
                    content: ChatContent::Text { text },
                },
                ConversationPart::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => ChatUpdate::ToolCall {
                    tool_call_id: call_id,
                    title: name,
                    raw_input: arguments,
                },
                ConversationPart::ToolResult {
                    call_id,
                    output,
                    is_error,
                } => ChatUpdate::ToolCallUpdate {
                    tool_call_id: call_id,
                    status: if *is_error {
                        ToolOutcome::Failed
                    } else {
                        ToolOutcome::Completed
                    },
                    content: [ToolContent::Content {
                        content: ChatContent::Text { text: output },
                    }],
                },
                ConversationPart::Compaction { summary } => ChatUpdate::CompactionUpdate {
                    compaction_id: format!("{}-{index}", self.id.to_hex()),
                    status: ToolOutcome::Completed,
                    summary: [ChatContent::Text { text: summary }],
                },
                ConversationPart::Notice { text } => info(text),
                ConversationPart::Unsupported { native_kind } => ChatUpdate::Notice {
                    severity: NoticeSeverity::Warning,
                    title: crate::constants::UNSUPPORTED_CONTENT,
                    description: Some(native_kind),
                },
                ConversationPart::ImageUnavailable { media_type } => ChatUpdate::Notice {
                    severity: NoticeSeverity::Info,
                    title: crate::constants::IMAGE_UNAVAILABLE,
                    description: media_type.as_deref(),
                },
            })
    }
}

fn info(title: &str) -> ChatUpdate<'_> {
    ChatUpdate::Notice {
        severity: NoticeSeverity::Info,
        title,
        description: None,
    }
}
