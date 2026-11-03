//! Bounded JSONL framing and provider-independent projection mechanics.
mod framing;
mod record;
mod scan;
mod selection;

pub use framing::*;
pub use record::*;
pub use scan::*;
pub use selection::*;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
