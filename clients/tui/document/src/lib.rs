mod constants;
mod view;

pub use constants::{DOCUMENT_PANE_FAILED, HINT, INVALID_DOCUMENT};
pub use view::{View, open, text};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
