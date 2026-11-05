//! Gemini CLI provider policy: identity (re-exported from the identity
//! target), hook installation profile, conversation source and runtime row.
//! Shared crates consume this package; they never name Gemini themselves.
pub use yolo_gemini_identity as identity;
pub use identity::{CONVERSATION_TOKEN, DISPLAY_NAME};

pub const INSTALL_TOKEN: &str = yolo_gemini_hooks::PROVIDER;


mod conversation;
mod install;
mod package;

pub use conversation::GeminiConversationSource;
pub use install::{INSTALL_SPEC, install_spec};
pub use package::runtime;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
