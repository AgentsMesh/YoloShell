//! Conversation access through the public Hub protocol, without provider files or runtime access.
mod client;
mod error;

pub use client::{read, resolve, resolve_agent, resolve_current, watch};
pub use error::ConversationClientError;
