//! Qwen Code native JSONL projection. Hook state remains the lifecycle authority.
mod constants;
mod parts;
mod records;

pub use records::QwenTranscriptParser;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
