//! Bounded query replies from maintained VT parsers, with opaque input epochs.
//! Callers establish writer authority; this crate neither owns nor acquires it.
//! Screen parsing continues across epoch changes while query eligibility resets.

mod callbacks;
mod constants;
mod detector;
mod kind;
mod processor;
mod scope;

pub use callbacks::QueryCallbacks;
pub use constants::{MAX_REPLY_BYTES, MAX_REPLY_COUNT};
pub use processor::QueryProcessor;
pub use scope::QueryScope;
