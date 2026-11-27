use super::{Row, wrap};
use crate::{Chat, constants::*};
use ratatui::style::Color;
use std::collections::VecDeque;
use yolo_conversation_protocol::{
    ChatContent, ChatUpdate, ConversationRole, ToolContent, ToolOutcome,
};

pub(super) fn rows(chat: &Chat, width: u16) -> Vec<Row> {
    let mut rows = VecDeque::new();
    for entry in chat.entries() {
        let (role, color) = match entry.role {
            ConversationRole::User => (USER, Color::Cyan),
            ConversationRole::Assistant => (ASSISTANT, Color::Green),
            ConversationRole::Tool => (TOOL, Color::Yellow),
            ConversationRole::Runtime => (RUNTIME, Color::DarkGray),
        };
        add(&mut rows, role, width, color);
        for update in entry.chat_updates() {
            update_rows(&mut rows, update, width);
        }
    }
    if rows.is_empty() {
        add(&mut rows, EMPTY, width, Color::DarkGray);
    }
    rows.into_iter().collect()
}

fn update_rows(rows: &mut VecDeque<Row>, update: ChatUpdate<'_>, width: u16) {
    let plain = Color::Reset;
    match update {
        ChatUpdate::UserMessageChunk { content } | ChatUpdate::AgentMessageChunk { content } => {
            content_rows(rows, content, width, plain);
        }
        ChatUpdate::AgentThoughtChunk { content } => {
            add(rows, THINKING, width, Color::DarkGray);
            content_rows(rows, content, width, Color::DarkGray);
        }
        ChatUpdate::ToolCall {
            tool_call_id,
            title,
            raw_input,
        } => {
            add(
                rows,
                &format!(
                    "{TOOL}: {} ({})",
                    wrap::safe(title),
                    wrap::safe(tool_call_id)
                ),
                width,
                Color::Yellow,
            );
            add(rows, raw_input, width, plain);
        }
        ChatUpdate::ToolCallUpdate {
            tool_call_id,
            status,
            content: [ToolContent::Content { content }],
        } => {
            let (label, color) = match status {
                ToolOutcome::Failed => (TOOL_ERROR, Color::Red),
                ToolOutcome::Completed => (TOOL_RESULT, Color::Yellow),
            };
            add(
                rows,
                &format!("{label}: {}", wrap::safe(tool_call_id)),
                width,
                color,
            );
            content_rows(rows, content, width, plain);
        }
        ChatUpdate::CompactionUpdate {
            summary: [content], ..
        } => {
            add(rows, COMPACTION, width, Color::DarkGray);
            content_rows(rows, content, width, plain);
        }
        ChatUpdate::Notice {
            title, description, ..
        } => {
            add(rows, title, width, Color::Yellow);
            if let Some(detail) = description {
                add(rows, detail, width, Color::DarkGray);
            }
        }
    }
}

fn content_rows(rows: &mut VecDeque<Row>, content: ChatContent<&str>, width: u16, color: Color) {
    match content {
        ChatContent::Text { text } => add(rows, text, width, color),
        ChatContent::ResourceLink { name, uri, .. } => {
            add(rows, name, width, color);
            add(rows, uri, width, Color::DarkGray);
        }
    }
}

fn add(rows: &mut VecDeque<Row>, text: &str, width: u16, color: Color) {
    wrap::for_each_row(text, width, |text| {
        if rows.len() == MAX_RENDER_ROWS {
            rows.pop_front();
        }
        rows.push_back(Row { text, color });
        true
    });
}
