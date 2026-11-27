//! Local, frozen terminal selection. This component never owns PTY input.
mod clipboard;
mod constants;
mod error;
mod frozen;
mod selection;
mod snapshot;
mod text;

pub use clipboard::ClipboardCommand;
pub use constants::{MAX_SELECTION_BYTES, MAX_SELECTION_CELLS};
pub use error::SelectionError;
pub use selection::{CellPoint, Selection};
