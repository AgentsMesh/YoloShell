//! Grok Build native session-log projection. Hook state remains the lifecycle
//! authority.
mod constants;
mod parts;
mod records;

pub use records::GrokTranscriptParser;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
