//! Provider-neutral anchored evidence primitives.
//!
//! This package owns descriptor-relative path walking, identity checks, no-follow
//! opens and bounded transcript reads. It performs no provider metadata parsing
//! and no lifecycle inference.
mod anchored;
mod constants;
mod identity;
mod layout;
mod open;
mod reader;

pub use anchored::AnchoredEvidence;
pub use constants::{EVIDENCE_HEAD_BYTES, EVIDENCE_TAIL_BYTES};
pub use identity::{EvidenceIdentity, EvidenceKind};
pub use layout::EvidenceLayout;
pub use reader::{
    EvidenceWindow, parse_bytes, read_anchored, read_at, read_with_hook,
};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
