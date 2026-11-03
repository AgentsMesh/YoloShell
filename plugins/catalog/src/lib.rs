//! Bundled TUI client descriptor. External trusted plugins live in the local plugin registry.
mod tui;

pub use tui::{TUI_ID, tui_api, tui_descriptor};
use yolo_builtin_client_contract::BuiltinClientDescriptor;

pub fn builtin_client_descriptors() -> Vec<BuiltinClientDescriptor> {
    vec![tui_descriptor()]
}
