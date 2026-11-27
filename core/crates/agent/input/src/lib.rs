//! Public text input values. These values convey intent, never execution authority.
mod constants;
mod prompt;

pub use constants::MAX_PROMPT_BYTES;
pub use prompt::{InvalidPrompt, Prompt};
