#[path = "constants.rs"]
mod constants;
mod native;
#[path = "wire.rs"]
mod wire;

pub use constants::*;
pub use native::*;
pub use wire::*;

pub fn is_internal_argument(value: &str) -> bool {
    matches!(
        value,
        CLAUDE_HOOK_HELPER_ARGUMENT | CLAUDE_SETTINGS_WATCHDOG_ARGUMENT
    )
}
