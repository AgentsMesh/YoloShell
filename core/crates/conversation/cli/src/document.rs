use crate::arguments::Action;
use anyhow::{Context, ensure};
use std::io::Read;
use yolo_conversation_protocol::ConversationReadRequest;

pub(crate) const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const LIMIT_SENTINEL: u64 = 1;

pub(crate) fn request(
    action: &Action,
    input: impl Read,
) -> anyhow::Result<Option<ConversationReadRequest>> {
    if !matches!(
        action,
        Action::Read { agent: None, .. } | Action::Watch { .. }
    ) {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    input
        .take(MAX_REQUEST_BYTES + LIMIT_SENTINEL)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_REQUEST_BYTES,
        "conversation request exceeds the input limit"
    );
    let request: ConversationReadRequest = serde_json::from_slice(&bytes)
        .context("expected a ConversationReadRequest JSON document on stdin")?;
    request.validate()?;
    Ok(Some(request))
}
