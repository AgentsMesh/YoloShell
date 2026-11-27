use crate::constants::*;

pub(crate) fn truncate(message: &str) -> String {
    if message.chars().count() <= MAX_MESSAGE_CHARS {
        return message.to_owned();
    }
    let kept = message
        .chars()
        .take(MAX_MESSAGE_CHARS.saturating_sub(1))
        .collect::<String>();
    format!("{kept}{ELLIPSIS}")
}
