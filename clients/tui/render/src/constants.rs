use ratatui::style::Color;

pub const FOCUSED_BORDER: Color = Color::Cyan;
pub const UNFOCUSED_BORDER: Color = Color::DarkGray;
pub use yolo_tui_lifecycle::{SYNCING_LABEL, WRITING_LABEL};
pub const EMPTY_PANE_LABEL: &str = "Starting shell…";
pub const CELL_WIDTH: u16 = 1;
pub const EMPTY_CELL: &str = " ";
pub const SELECTION_FOREGROUND: Color = Color::Black;
pub const SELECTION_BACKGROUND: Color = Color::Cyan;
