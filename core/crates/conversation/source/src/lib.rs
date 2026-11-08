//! Provider-neutral conversation evidence ports. Provider crates own evidence
//! inspection and transcript parsing; the service owns caching and projection.
mod evidence;
mod outcome;
mod sources;

pub use evidence::{EvidenceFile, conversation_error, require_binding};
pub use outcome::ConversationReadOutcome;
pub use sources::{ConversationSource, ConversationSources};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
