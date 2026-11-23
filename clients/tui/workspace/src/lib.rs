//! Workspace filter picker: lists every workspace visible to the selected Hub
//! and applies one as the inventory scope. The picker only changes local
//! projection state; workspace records remain Hub-owned.

mod constants;
mod message;
mod picker;
mod render;

pub use picker::{Handle, Picker};
pub use render::render;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
