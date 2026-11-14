//! Durable at-most-once intent and receipt storage; this crate performs no effects.
//! Create only during trusted runtime initialization, before publishing Ready.
//! Persist the returned anchor with that runtime. Reopening must always provide
//! that anchor; creation is never a fallback for missing or damaged history.
//! The external anchor pins the immutable header. Mutable history is not an
//! external monotonic counter against coherent rollback by the OS account owner.

mod anchor;
mod constants;
mod error;
mod journal;
mod open;
mod operations;
mod receipt;
mod record;
mod reservation;
mod scope;

pub use anchor::JournalAnchor;
pub use constants::{HEADER_FILE, JOURNAL_FILE, MAX_RECEIPTS};
pub use error::JournalError;
pub use journal::Journal;
pub use receipt::{Completion, Receipt};
pub use reservation::{Reservation, ReservationResult};
pub use scope::JournalScope;

#[cfg(test)]
extern crate self as yolo_agent_control_journal;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
