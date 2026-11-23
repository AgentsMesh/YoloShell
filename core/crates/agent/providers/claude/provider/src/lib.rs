//! Claude Code provider policy: identity (re-exported from the identity
//! target), hook installation profile, conversation source and runtime row.
//! Shared crates consume this package; they never name Claude themselves.
pub use yolo_claude_identity as identity;
pub use identity::{CONVERSATION_TOKEN, DISPLAY_NAME, INSTALL_TOKEN};


mod conversation;
mod install;
mod package;

pub use conversation::ClaudeConversationSource;
pub use install::{INSTALL_SPEC, install_spec};
pub use package::runtime;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
