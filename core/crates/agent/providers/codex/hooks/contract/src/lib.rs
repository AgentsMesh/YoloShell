use yolo_hooks::ProviderVersion;
mod events;
mod native;
pub use events::{SESSION_END_HOOK_TIMEOUT_SECONDS, event_names};
pub use native::*;

pub const ACCEPTED_PROVIDER_VERSION: ProviderVersion = ProviderVersion::new(0, 150, 1);
pub const ACCEPTED_PROVIDER_VERSION_TEXT: &str = "0.150.1";
pub const TERMINAL_ENV: &str = "YOLO_TERMINAL_ID";
pub const RUNTIME_ENV: &str = "YOLO_RUNTIME_EPOCH";
pub const SOCKET_ENV: &str = "YOLO_CODEX_HOOK_SOCKET";
pub const SENSITIVE_ENVIRONMENT_NAMES: &[&str] = &[
    TERMINAL_ENV,
    RUNTIME_ENV,
    SOCKET_ENV,
    NATIVE_HOOK_EXECUTABLE_ENV,
    NATIVE_HOOK_HOME_ENV,
];
