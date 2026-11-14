pub const ROWS: u16 = 8;
pub const COLUMNS: u16 = 20;
pub const ATTRIBUTES: &[u8] = b"\x1b[1;2;3;4;7;38;5;196;48;2;1;2;3mX";
pub const BOLD_TEXT: &[u8] = b"\x1b[1mX";
pub const INDEXED_RED: u8 = 196;
pub const SCREEN_TEXT: &[u8] = b"terminal";
pub const WIDE_TEXT: &str = "\u{754c}";
pub const COMBINED_TEXT: &str = "e\u{301}";
pub const SINGLE_VIEWPORT: usize = 1;
pub const START_CELL: (u16, u16) = (1, 2);
pub const END_CELL: (u16, u16) = (2, 3);
pub const INNER_INSET: u16 = 1;
pub const SHORT_EXTENT: u16 = 4;
pub const SELECTED_BACKGROUND: ratatui::style::Color = ratatui::style::Color::Cyan;
pub const NARROW_COLUMNS: u16 = 4;
pub const SCROLLBACK_ROWS: usize = 16;
pub const SCROLLED_LINE: &[u8] = b"old\r\n";
pub const SCROLLED_TEXT: &str = "old";
pub const BLANK_CELL: &str = " ";
pub const CURSOR_BELOW_VIEW: &[u8] = b"\x1b[8;1H";
pub const CURSOR_RIGHT_OF_VIEW: &[u8] = b"\x1b[1;20H";
pub const HISTORY_STEP: usize = 1;

pub const WRITING: yolo_tui_lifecycle::TerminalProjection =
    yolo_tui_lifecycle::TerminalProjection {
        phase: yolo_tui_lifecycle::TerminalPhase::Running,
        connection: yolo_tui_lifecycle::ConnectionState::Connected,
        input: yolo_tui_lifecycle::InputAccess::Writing,
    };
