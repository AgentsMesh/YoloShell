use std::time::Duration;

pub const HOOK_RETRY_DELAY: Duration = Duration::from_millis(50);
pub const HOOK_DELIVERY_BUDGET: Duration = crate::claude_hooks::CLAUDE_MANAGEMENT_HANDSHAKE_TIMEOUT;
