//! Gemini CLI hook profile: payload codec and ingress data for the
//! provider-neutral hook engine.
pub mod constants;
mod codec;

pub use codec::GeminiHookCodec;
pub use constants::*;
use std::sync::Arc;
use yolo_hook_codec::HookCodec;

pub fn codec() -> Arc<dyn HookCodec> {
    Arc::new(GeminiHookCodec)
}

#[cfg(test)]
#[path = "../unit_tests/tests.rs"]
mod tests;
