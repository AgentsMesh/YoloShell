//! Qwen Code provider policy: identity (re-exported from the identity
//! target), hook installation profile, conversation source and runtime row.
//! Shared crates consume this package; they never name Qwen themselves.
pub use yolo_qwen_identity as identity;
pub use identity::{CONVERSATION_TOKEN, DISPLAY_NAME};

pub const INSTALL_TOKEN: &str = yolo_qwen_hooks::PROVIDER;


mod conversation;
mod install;
mod package;

pub use conversation::QwenConversationSource;
pub use install::{INSTALL_SPEC, install_spec};
pub use package::runtime;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
