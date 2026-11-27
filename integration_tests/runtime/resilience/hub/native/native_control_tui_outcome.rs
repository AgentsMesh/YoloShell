use super::constants::*;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Outcome {
    Sent,
    PreconditionsChanged,
    NotSent,
    Pending,
    Unknown,
    Blocked,
}

pub(super) fn outcome(text: &str) -> Outcome {
    let raw = footer(text);
    let labels = [SENT, STALE, NOT_SENT, SENDING, ACCEPTED, UNKNOWN];
    let status = if labels.iter().any(|label| raw.starts_with(label)) {
        raw
    } else {
        raw.split_once(STATUS_SEPARATOR)
            .map_or(raw, |(_, status)| status)
    };
    match status {
        SENT => Outcome::Sent,
        STALE => Outcome::PreconditionsChanged,
        NOT_SENT => Outcome::NotSent,
        SENDING | ACCEPTED => Outcome::Pending,
        UNKNOWN => Outcome::Unknown,
        _ if status.strip_suffix(INTERRUPT_HINT) == Some(STALE) => Outcome::NotSent,
        _ => Outcome::Blocked,
    }
}

pub(super) fn searching(text: &str) -> bool {
    footer(text).starts_with(SEARCH_PROMPT)
}

pub(super) fn ready(text: &str, identity: &str, writing: &str) -> bool {
    !searching(text)
        && !text
            .lines()
            .flat_map(|line| line.split('|'))
            .any(|section| section.trim() == REFRESHING_LABEL)
        && text.contains(SINGLE_AGENT_HEADER)
        && text.contains(identity)
        && text.contains(writing)
        && text.contains(CURRENT_AGENT)
}

fn footer(text: &str) -> &str {
    text.lines().last().unwrap_or_default().trim()
}
