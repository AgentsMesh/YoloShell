use crate::constants::CURRENT_TERMINAL_MISSING;
use crate::reject::rejected;
use anyhow::Result;
use yolo_agent_adapter_contract::constants::YOLO_TERMINAL_ID_ENV;

/// Resolve `--current` from the terminal environment Yolo injects into every
/// managed shell. The terminal id is only a selector; the Hub still resolves
/// the current agent and revalidates every control precondition.
pub fn terminal_selector() -> Result<String> {
    selector_from(std::env::var(YOLO_TERMINAL_ID_ENV).ok())
}

pub fn selector_from(value: Option<String>) -> Result<String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(rejected(CURRENT_TERMINAL_MISSING)),
    }
}
