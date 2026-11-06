use yolo_conversation_protocol::{
    ConversationEntry, ConversationPart, ConversationReadResult, ConversationRole,
};

pub fn conversation_text(reading: &ConversationReadResult) -> String {
    match reading {
        ConversationReadResult::Page { page } => page
            .entries
            .iter()
            .map(entry_line)
            .collect::<Vec<_>>()
            .join("\n"),
        ConversationReadResult::Unchanged { .. } => "conversation unchanged".into(),
    }
}

pub fn entry_line(entry: &ConversationEntry) -> String {
    let role = match entry.role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "agent",
        ConversationRole::Tool => "tool",
        ConversationRole::Runtime => "runtime",
    };
    match entry.parts.iter().find_map(part_text) {
        Some(content) => format!("{role}: {content}"),
        None => role.to_owned(),
    }
}

fn part_text(part: &ConversationPart) -> Option<String> {
    match part {
        ConversationPart::Text { text } => Some(text.clone()),
        ConversationPart::Notice { text } => Some(text.clone()),
        ConversationPart::ToolCall { name, .. } => Some(format!("tool call {name}")),
        ConversationPart::ToolResult { is_error, .. } => Some(if *is_error {
            "tool failed".into()
        } else {
            "tool result".into()
        }),
        ConversationPart::Compaction { summary } => Some(format!("compaction: {summary}")),
        ConversationPart::Thinking { .. }
        | ConversationPart::Unsupported { .. }
        | ConversationPart::ImageUnavailable { .. } => None,
    }
}

pub fn assistant_text(entries: &[ConversationEntry]) -> Option<String> {
    entries.iter().rev().find_map(|entry| {
        if entry.role != ConversationRole::Assistant {
            return None;
        }
        let text = entry
            .parts
            .iter()
            .filter_map(|part| match part {
                ConversationPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        (!text.trim().is_empty()).then_some(text)
    })
}
