use super::{Row, wrap};
use crate::{Chat, constants::*};
use ratatui::style::Color;
use std::collections::VecDeque;
use yolo_conversation_protocol::{ConversationPart, ConversationRole};

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
        for part in &entry.parts {
            part_rows(&mut rows, part, width);
        }
    }
    if rows.is_empty() {
        add(&mut rows, EMPTY, width, Color::DarkGray);
    }
    rows.into_iter().collect()
}

fn part_rows(rows: &mut VecDeque<Row>, part: &ConversationPart, width: u16) {
    let plain = Color::Reset;
    match part {
        ConversationPart::Text { text } => add(rows, text, width, plain),
        ConversationPart::Thinking { text } => {
            add(rows, THINKING, width, Color::DarkGray);
            add(rows, text, width, Color::DarkGray);
        }
        ConversationPart::ToolCall {
            call_id,
            name,
            arguments,
        } => {
            add(
                rows,
                &format!("{TOOL}: {} ({})", wrap::safe(name), wrap::safe(call_id)),
                width,
                Color::Yellow,
            );
            add(rows, arguments, width, plain);
        }
        ConversationPart::ToolResult {
            call_id,
            output,
            is_error,
        } => {
            let label = if *is_error { TOOL_ERROR } else { TOOL_RESULT };
            let color = if *is_error { Color::Red } else { Color::Yellow };
            add(
                rows,
                &format!("{label}: {}", wrap::safe(call_id)),
                width,
                color,
            );
            add(rows, output, width, plain);
        }
        ConversationPart::Compaction { summary } => {
            add(rows, COMPACTION, width, Color::DarkGray);
            add(rows, summary, width, plain);
        }
        ConversationPart::Notice { text } => add(rows, text, width, Color::Yellow),
        ConversationPart::Unsupported { native_kind } => {
            add(
                rows,
                &format!("{UNSUPPORTED}: {}", wrap::safe(native_kind)),
                width,
                Color::DarkGray,
            );
        }
        ConversationPart::ImageUnavailable { media_type } => {
            let label = media_type.as_ref().map_or_else(
                || IMAGE_UNAVAILABLE.into(),
                |kind| format!("{IMAGE_UNAVAILABLE}: {}", wrap::safe(kind)),
            );
            add(rows, &label, width, Color::DarkGray);
        }
    }
}

fn add(rows: &mut VecDeque<Row>, text: &str, width: u16, color: Color) {
    let (lines, truncated) = wrap::wrap(text, width, MAX_PART_ROWS);
    for text in lines
        .into_iter()
        .chain(truncated.then(|| PART_TRUNCATED.to_owned()))
    {
        rows.push_back(Row { text, color });
        if rows.len() > MAX_RENDER_ROWS {
            rows.pop_front();
        }
    }
}
