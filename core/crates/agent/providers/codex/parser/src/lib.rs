//! Codex rollout records to read-only conversation entries; no lifecycle state mutation.
mod constants;
mod parts;
mod records;

pub use records::CodexTranscriptParser;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
