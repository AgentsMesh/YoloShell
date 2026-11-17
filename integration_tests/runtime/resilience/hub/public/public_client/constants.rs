use std::time::Duration;
use yolo_hub_routing::Right;

pub const PRIVATE_FILE_MODE: u32 = 0o600;
pub const CLIENT_KEY_FILE: &str = "public-client-key";
pub const PROFILE_FILE: &str = "remote-entry.json";
pub const STATE_DIRECTORY: &str = "remote-hub";
pub const SESSION_NAME: &str = "public-client-shell";
pub const CREATED_DIRECTORY: &str = "remote TUI directory";
pub const DRIVER_NAME: &str = "public-client TUI driver";
pub const GRANT_LIFETIME: Duration = Duration::from_secs(300);
pub const FULL_RIGHTS: [Right; 6] = [
    Right::Observe,
    Right::TerminalRead,
    Right::TerminalWrite,
    Right::Lifecycle,
    Right::PresentationRead,
    Right::PresentationWrite,
];
pub const OBSERVER_RIGHTS: [Right; 4] = [
    Right::Observe,
    Right::TerminalRead,
    Right::PresentationRead,
    Right::PresentationWrite,
];
pub const PROFILE_ARGUMENT: &str = "--remote-profile";
pub const ACCOUNT_KEY_ARGUMENT: &str = "--account-key";
pub const CLIENT_KEY_ARGUMENT: &str = "--client-key";
pub const QUOTE_ESCAPE: &str = "'\"'\"'";
pub const COMPLETE_PREFIX: &str = "YOLO_REMOTE_TUI_RETURNED_";
pub const MULTIHOP_TUI_COMPLETE_PREFIX: &str = "YOLO_PUBLIC_TUI_DONE_";
pub const MULTIHOP_TUI_DRIVER_NAME: &str = "public multihop TUI";
pub const MULTIHOP_PROFILE_FILE: &str = "entry.json";
pub const MULTIHOP_KEY_FILE: &str = "multihop-client-key";
pub const MULTIHOP_SHELL_NAME: &str = "multihop-plain-shell";
pub const MULTIHOP_AGENT_A: &str = "public-a-agent";
pub const MULTIHOP_AGENT_B: &str = "public-b-agent";
pub const MULTIHOP_AGENT_C: &str = "public-c-agent";
pub const MULTIHOP_MARKER_PREFIX: &str = "YOLO_MULTIHOP_";
pub const MARKER_PREFIX: &str = "YOLO_PUBLIC_";
pub const MARKER_SUFFIX: &str = "ENTRY_";
pub const NEW_SHELL: &[u8] = b"n";
pub const DIRECTORY_PROMPT: &str = "Directory: ~";
pub const CLEAR_DEFAULT_DIRECTORY: &[u8] = b"\x7f";
pub const FINISH_INPUT: &[u8] = b"\r";
pub const DISCONNECTED_HEADER: &str = "Hub reconnecting";
pub const SHELL_COUNT: usize = 1;
pub use crate::hub::tui_queries::constants::{
    COLUMNS, CONTROL, CONTROL_READY_HEADER, DETACH, ESCAPE, INVENTORY, QUIT, READ_ONLY, ROWS,
    SCROLLBACK, SEARCH, SEARCH_STATUS, SELECT_SEARCH_RESULT, TERMINAL_PICKER_READY_SCREEN,
    TUI_BINARY_ENV,
};
