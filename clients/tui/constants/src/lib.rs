use std::time::Duration;
pub const RPC_TIMEOUT: Duration = Duration::from_secs(5);
pub const EVENT_POLL: Duration = Duration::from_millis(100);
pub const TAB_VERIFY_INTERVAL: Duration = Duration::from_secs(1);
pub const CATALOG_REFRESH: Duration = Duration::from_secs(3);
pub const QUEUE_CAPACITY: usize = 64;
pub const DEFERRED_INPUT_CAPACITY: usize = 32;
pub const PREGRANT_INPUT_CAPACITY: usize = DEFERRED_INPUT_CAPACITY;
pub const STATUS_ROWS: u16 = 1;
pub const MAX_FILTER_BYTES: usize = 256;
pub const DEFAULT_DIRECTORY: &str = "~";
pub const PRODUCT_NAME: &str = "Yolo";
pub const SEARCH_LABEL: &str = "Filter";
pub const DIRECTORY_LABEL: &str = "Directory";
pub const CONNECTING_LABEL: &str = "Connecting";
pub const CONNECTED_LABEL: &str = "Connected";
pub const CLIENT_STARTUP_FAILURE_LABEL: &str = "TUI cannot start against this Hub";
pub const DISCONNECTED_LABEL: &str = "Disconnected";
pub const CATALOG_REFRESHING_LABEL: &str = "Refreshing";
pub const UNSAVED_SHELL_CHANGES_LABEL: &str = "Unsaved shell changes";
pub const NO_RUNTIME_LABEL: &str = "No running terminal";
/// Set by the Worker for a shell it owns; a nested client can never display
/// the terminal it is running in without recursively rendering itself.
pub const SELF_TERMINAL_ENV: &str = "YOLO_TERMINAL_ID";
pub const SELF_TERMINAL_LABEL: &str =
    "This is the terminal running YOLO; open another terminal to view it";
pub const READ_ONLY_LABEL: &str = "Read Only";
pub const CONTROL_LOST_LABEL: &str = "Input control was taken by another client";
pub const INPUT_WAITING_LABEL: &str = "Waiting for terminal input control; input queued";
pub const QUEUE_FULL_LABEL: &str = "Connection busy; command was not sent";
pub const INPUT_OVERFLOW_LABEL: &str =
    "Input overflowed; some input was not sent; Ctrl-b a/t to resume";
pub const UNEXPECTED_REPLY_LABEL: &str = "Unexpected Hub response";
pub const SHUTDOWN_GRACE: Duration = Duration::from_secs(1);
pub const STREAM_DEADLINE: Duration = Duration::from_secs(30);
pub const INITIAL_CONTROL_TIMEOUT: Duration = Duration::from_secs(30);
pub const MIN_CONFIRM_TARGET_COLUMNS: usize = 8;
pub const MIN_INVENTORY_TITLE_WIDTH: u16 = 100;
pub const STREAM_RECONNECT_DELAY: Duration = Duration::from_secs(1);
pub const SESSION_CONTROL_ONLY_LABEL: &str = "Agent semantic control is unavailable";
pub const HISTORY_SCOPE_LABEL: &str = "History: includes ended, lost and archived records";
pub const ENDED_INVENTORY_HINT: &str =
    "Terminal ended; c starts a new terminal · h history · R recover";
pub const UNAVAILABLE_INVENTORY_HINT: &str = "No running terminal; c starts one";
pub const HISTORY_INVENTORY_HINT: &str =
    "c new tab  Enter open/details  R recover  a archive  h active  ? help  q detach";
pub const ARCHIVED_HISTORY_HINT: &str = "c new tab  Enter open/details  h active  ? help  q detach";
pub const HISTORY_KEY: char = 'h';
pub const ACTIVE_SCOPE_LABEL: &str = "Active: live and unconfirmed records";
pub const AGENT_HISTORY_DETAILS_LABEL: &str = "Agent details · Esc back · R view terminal recovery";
pub const TERMINAL_RECOVERY_HINT: &str = "Terminal history · R resume · a archive · h active";
pub const HUB_UNAVAILABLE_LABEL: &str = "No Hub selected";
pub const PANE_RATIO_STEP: u16 = 50;
pub const PANE_BORDER_CELLS: u16 = 1;
pub const SCROLL_ROWS_PER_TICK: usize = 3;
pub const MAX_COALESCED_UPDATES: usize = 64;
pub const EMPTY_INVENTORY_HINT: &str = "c new tab  n directory  ? help  q detach";
pub const INVENTORY_HINT: &str = "c new tab  Enter open/details  h history  ? help  q detach";
pub const PREFIX_HINT: &str =
    "c tab · v/- split · n/p tabs · s agents · b sidebar · d detach · ? help";
pub const OBSERVER_PREFIX_HINT: &str =
    "n/p tabs · o pane · g terminals · s agents · d detach · ? help";
pub const OBSERVER_INVENTORY_HINT: &str =
    "Enter open/details · g terminals · h history · ? help · q detach";
pub const OBSERVER_HISTORY_HINT: &str = "Terminal history · Enter view · h active";

pub fn is_self_terminal(terminal_id: uuid::Uuid) -> bool {
    parsed_self_terminal(std::env::var(SELF_TERMINAL_ENV).ok().as_deref(), terminal_id)
}

pub fn parsed_self_terminal(value: Option<&str>, terminal_id: uuid::Uuid) -> bool {
    value.and_then(|value| value.parse::<uuid::Uuid>().ok()) == Some(terminal_id)
}
