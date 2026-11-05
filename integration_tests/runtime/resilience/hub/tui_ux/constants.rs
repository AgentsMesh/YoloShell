pub(super) use crate::hub::tui_queries::constants::{
    CONTROL, FIRST_NAME, INVENTORY, PREFIX, PREFIX_HINT, SEARCH, SEARCH_STATUS,
    TERMINAL_PICKER_READY_SCREEN,
};

pub(super) const SMALL_COLUMNS: u16 = 80;
pub(super) const SMALL_ROWS: u16 = 24;
pub(super) const ONE_TAB: usize = 1;
pub(super) const ONE_PANE: usize = 1;
pub(super) const ONE_TERMINAL: usize = 1;
pub(super) const TWO_TABS: usize = 2;
pub(super) const TWO_TERMINALS: usize = 2;
pub(super) const ENTER: &[u8] = b"\r";
pub(super) const CLEAR_LINE: &[u8] = b"\x15";
pub(super) const DIRECTORY: &[u8] = b"n";
pub(super) const MANAGER: &[u8] = b"\x02w";
pub(super) const RENAME: &[u8] = b"r";
pub(super) const REMOVE: &[u8] = b"x";
pub(super) const CLOSE_TAB: &[u8] = b"\x02X";
pub(super) const HELP: &[u8] = b"\x02?";
pub(super) const END: &[u8] = b"\x1b[F";
pub(super) const PAGE_UP: &[u8] = b"\x1b[5~";
pub(super) const CLOSE_HELP: &[u8] = b"q";
pub(super) const PASTE_START: &str = "\x1b[200~";
pub(super) const PASTE_END: &str = "\x1b[201~";
pub(super) const DIRECTORY_LABEL: &str = "Directory:";
pub(super) const INVALID_DIRECTORY: &str = "working directory is unavailable";
pub(super) const LONG_COMPONENT: &str = "long directory 中文 with spaces";
pub(super) const LONG_COMPONENT_REPETITIONS: usize = 3;
pub(super) const MISSING_DIRECTORY: &str = "missing location";
pub(super) const VALID_DIRECTORY: &str = "corrected 中文 path";
pub(super) const MISSING_FILTER: &str = "不存在的终端-filter";
pub(super) const NO_TERMINALS: &str = "No terminals";
pub(super) const ONE_TERMINAL_HEADER: &str = "Terminals: 1";
pub(super) const MANAGER_TITLE: &str = "Tabs";
pub(super) const RENAME_PROMPT: &str = "Rename tab:";
pub(super) const REMOVE_PROMPT: &str = "Close tab view";
pub(super) const RENAMED_TAB: &str = "MY-BUILD-工作";
pub(super) const EMPTY_AGENT_INVENTORY: &str = "No agent runs";
pub(super) const HELP_TITLE: &str = "Yolo · tabs and terminals";
pub(super) const HELP_TOP: &str = "Type normally in a pane";
pub(super) const HELP_LAST: &str = "Ordinary q and ? remain shell input.";
pub(super) const HELP_CLOSE: &str = "Esc/q close";
pub(super) const MARKER_PREFIX: &str = "YOLO_UX_";
