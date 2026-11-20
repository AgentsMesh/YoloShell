use crate::constants::*;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;

pub(crate) fn target(command: &Command) -> Option<String> {
    let value = serde_json::to_value(command).ok()?;
    for key in [
        WORKSPACE_ID_FIELD,
        PATH_FIELD,
        TERMINAL_ID_FIELD,
        RUN_ID_FIELD,
        HOST_ID_FIELD,
        PROFILE_ID_FIELD,
        ADDRESS_FIELD,
    ] {
        if let Some(value) = find(&value, key) {
            return Some(match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            });
        }
    }
    None
}

pub(crate) fn request_id(command: &Command) -> Option<Uuid> {
    let value = serde_json::to_value(command).ok()?;
    find(&value, REQUEST_ID_FIELD)
        .and_then(serde_json::Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
}

/// Search only command identity envelopes, never caller-controlled payloads
/// such as prompts or generic input values.
fn find<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let object = value.as_object()?;
    if let Some(found) = object.get(key) {
        return Some(found);
    }
    IDENTITY_ENVELOPES
        .iter()
        .find_map(|envelope| object.get(*envelope).and_then(|value| find(value, key)))
}
