//! Optional Agent control durability, independent of PTY availability.
//! Fresh initialization requires retained pre-Ready runtime evidence. Existing
//! runtimes only reopen; missing history never becomes permission to initialize.
//! The caller still owns writer, provider-source and process-effect admission.
//! Same-user coherent rollback of all trusted artifacts is outside this contract.

mod anchor;
mod constants;
mod fresh;
mod fresh_validation;
mod initialize;
mod operations;
mod runtime;
mod storage;

pub use constants::{ANCHOR_EXTENSION, ANCHOR_PREFIX, JOURNAL_DIRECTORY};
pub use fresh::FreshInitialization;
pub use runtime::ControlRuntime;
pub use yolo_agent_control_journal::{
    Completion, JournalError, Receipt, Reservation, ReservationResult,
};

#[cfg(test)]
extern crate self as yolo_agent_control_runtime;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
