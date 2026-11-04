//! Qwen Code's hook payload contract and provider-neutral ingress codec. The
//! shared ingress engine owns delivery, admission, durability and publication.
mod codec;
mod constants;
mod contract;

pub use codec::{PROFILE, PROVIDER, QwenHookCodec, codec, event_kind};
pub use constants::{
    EVENT_INSTRUCTIONS_LOADED, EVENT_MESSAGE_DISPLAY, EVENT_NOTIFICATION, EVENT_PERMISSION_DENIED,
    EVENT_PERMISSION_REQUEST, EVENT_POST_COMPACT, EVENT_POST_TOOL_BATCH, EVENT_POST_TOOL_USE,
    EVENT_POST_TOOL_USE_FAILURE, EVENT_PRE_COMPACT, EVENT_PRE_TOOL_USE, EVENT_SESSION_DELETE,
    EVENT_SESSION_END, EVENT_SESSION_START, EVENT_STOP, EVENT_SUBAGENT_START, EVENT_SUBAGENT_STOP,
    EVENT_USER_PROMPT_SUBMIT, LEGACY_OUTBOX_DIRECTORY, MAX_ANCESTORS, PROVIDER_MAJOR,
    PROVIDER_MINOR, PROVIDER_PATCH,
};
pub use contract::{
    QwenHookEvent, QwenHookPayload, QwenSessionEndReason, QwenSessionStartSource,
};

/// Remove the forked engine's outbox directory. Events queued there belong to
/// a previous runtime epoch and cannot be applied safely after an upgrade.
pub fn remove_legacy_outbox(session_dir: &std::path::Path) -> std::io::Result<bool> {
    let path = session_dir.join(LEGACY_OUTBOX_DIRECTORY);
    match std::fs::remove_dir_all(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "../unit_tests/constants.rs"]
pub(crate) mod test_constants;
#[cfg(test)]
#[path = "../unit_tests/mapping_tests.rs"]
mod mapping_tests;
