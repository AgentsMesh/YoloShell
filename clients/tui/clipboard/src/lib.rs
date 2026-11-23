//! Desktop clipboard and file attachments selected by the person running the TUI.
mod dialog;
mod files;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;
mod transfer;
mod writer;

pub use dialog::{Dialog, InitialSource};
pub use files::ReferenceStyle;
pub use transfer::Transfers;
pub use writer::Writer;

#[cfg(test)]
#[path = "../unit_tests/files.rs"]
mod tests;
