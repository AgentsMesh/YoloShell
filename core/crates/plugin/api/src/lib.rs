//! Shared public client-API declarations used by built-in clients and trusted plugins.

mod api;
mod capability;
mod platform;

pub use api::ClientApiContract;
pub use capability::{ClientCapability, ClientFeatureRequirement};
pub use platform::Platform;

pub const MAX_IDENTIFIER_BYTES: usize = 128;

/// One vocabulary for plugin action ids across manifest validation and CLI invocation.
pub fn valid_action_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
