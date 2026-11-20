//! Codex provider policy: identity (re-exported from the identity target),
//! hook installation profile, conversation source and runtime row. Shared
//! crates consume this package; they never name Codex themselves.
pub use yolo_codex_identity as identity;
pub use identity::{CONVERSATION_TOKEN, DISPLAY_NAME, INSTALL_TOKEN};


mod conversation;
mod install;
mod package;
mod turn_end;

pub use conversation::CodexConversationSource;
pub use install::{INSTALL_SPEC, install_spec};
pub use package::runtime;
pub use turn_end::CodexTurnEndProbe;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
