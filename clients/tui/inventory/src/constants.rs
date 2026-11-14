use ratatui::style::Color;

pub const SECTION_LABELS: [&str; 2] = [yolo_tui_lifecycle::AGENT_RUNS_LABEL, "Hubs"];
pub const COMPACT_SECTION_LABELS: [&str; 2] = ["A", "H"];
pub const AGENTS_SECTION: usize = 0;
pub const HUBS_SECTION: usize = 1;
pub const TERMINAL_PICKER_LABEL: &str = "Terminals";
pub const SECTION_ROWS: u16 = 1;
pub const CONTEXT_ROWS: u16 = 1;
const SECTION_PADDING_SIDES: usize = 2;
pub const FULL_SECTIONS_WIDTH: u16 = (SECTION_LABELS[AGENTS_SECTION].len()
    + SECTION_LABELS[HUBS_SECTION].len()
    + SECTION_LABELS.len() * SECTION_PADDING.len() * SECTION_PADDING_SIDES
    + (SECTION_LABELS.len() - 1) * SECTION_DIVIDER.len())
    as u16;
pub const COMPACT_SECTIONS_WIDTH: u16 = (COMPACT_SECTION_LABELS[AGENTS_SECTION].len()
    + COMPACT_SECTION_LABELS[HUBS_SECTION].len()
    + COMPACT_SECTION_LABELS.len() * SECTION_PADDING.len() * SECTION_PADDING_SIDES
    + (COMPACT_SECTION_LABELS.len() - 1) * SECTION_DIVIDER.len())
    as u16;
pub const SELECTED_COLOR: Color = Color::Cyan;
pub const MUTED_COLOR: Color = Color::DarkGray;
pub const WARNING_COLOR: Color = Color::Yellow;
pub const SELECTION_MARKER: &str = "> ";
pub const SECTION_DIVIDER: &str = "|";
pub const SECTION_PADDING: &str = " ";
pub const SELECTION_STEP: usize = 1;
pub const ALL_HUBS: &str = "All Hubs";
pub const ACTIVE_SCOPE: &str = "Active · h history";
pub const HISTORY_SCOPE: &str = "History · h active";
pub const AGENT_DETAILS_TITLE: &str = "Agent run details";
pub const DETAILS_CLOSE_HINT: &str = "Esc back · h active/history";
pub const INTERACTION_LABEL: &str = "Answer needed";
pub const INTERACTION_ANSWER_HINT: &str =
    "Press 1-9 to answer with the held writer lease";
pub const DETAILS_NO_TERMINAL: &str =
    "Terminal is unavailable; reconnect the origin Hub to inspect recovery.";
pub const DETAILS_RECOVERY_HINT: &str = "R view terminal recovery · Esc back";
pub const CONTROL_UNKNOWN_LABEL: &str = "Control status unknown";
pub const OBSERVE_ONLY_CONTROL_LABEL: &str = "Observe only · no input control";
pub const DETAILS_TERMINAL_LABEL: &str = "Terminal";
pub const HUB_LABEL: &str = "Hub";
pub const TERMINAL_LABEL: &str = "Term";
pub(crate) const PHASE_SEPARATOR: &str = " | ";
pub const HUB_REACHABLE_GLYPH: &str = "●";
pub const HUB_UNREACHABLE_GLYPH: &str = "○";
pub const HUB_UNKNOWN_GLYPH: &str = "?";
pub const CONTROL_WRITABLE_GLYPH: &str = "✎";
pub const CONTROL_READ_ONLY_GLYPH: &str = "◇";
pub const CONTROL_PAUSED_GLYPH: &str = "‖";
pub const CONTROL_SYNCING_GLYPH: &str = "…";
pub const CONTROL_UNAVAILABLE_GLYPH: &str = "?";
pub const HEALTH_WARNING_GLYPH: &str = "!";
pub const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
pub const SPINNER_FRAME_MS: u64 = 100;
pub const AGENT_LABEL: &str = "Agent";
pub const NOTIFICATION_MARKER: &str = "● ";
pub const DIRECTORY_LABEL: &str = "Directory";
pub const UNKNOWN_DIRECTORY_LABEL: &str = "Directory unknown";
pub const LAST_KNOWN_DIRECTORY_LABEL: &str = "last known";
pub const LIVE_LABEL: &str = "Current";
pub const STALE_LABEL: &str = "Stale";
pub const REACHABLE_LABEL: &str = "Reachable";
pub const UNREACHABLE_LABEL: &str = "Unreachable";
pub const AGENTS_UNSUPPORTED: &str = "Agent inventory unavailable";
pub const HUBS_UNSUPPORTED: &str = "Hub inventory unavailable";
pub const EMPTY_SESSIONS: &str = "No terminals";
pub const EMPTY_AGENTS: &str = "No agent runs";
pub const EMPTY_HUBS: &str = "No hubs";
pub const UNKNOWN_LABEL: &str = "Unknown";
pub const STARTING_LABEL: &str = "Starting";
pub const RUNNING_LABEL: &str = "Running";
pub const EXITED_LABEL: &str = "Exited";
pub const FINALIZING_LABEL: &str = yolo_tui_lifecycle::FINALIZING_LABEL;
pub const OUTPUT_BLOCKED_LABEL: &str = yolo_tui_lifecycle::OUTPUT_BLOCKED_LABEL;
pub const ARCHIVED_LABEL: &str = yolo_tui_lifecycle::ARCHIVED_LABEL;
pub const IDLE_LABEL: &str = "Idle";
pub const WORKING_LABEL: &str = "Working";
pub const WAITING_INPUT_LABEL: &str = "Waiting for input";
pub const WAITING_APPROVAL_LABEL: &str = "Waiting for approval";
pub const RESUME_AVAILABLE_LABEL: &str = "Resume available";
pub const RESUME_UNSUPPORTED_LABEL: &str = "Resume unsupported";
pub const RESUME_UNAVAILABLE_LABEL: &str = "Resume unavailable";
pub const RESUME_UNKNOWN_LABEL: &str = "Resume unknown";
pub const HOOK_SOURCE_LABEL: &str = "Hooks";
pub const PROCESS_SOURCE_LABEL: &str = "Process";
pub const CONNECTOR_SOURCE_LABEL: &str = "Connector";

pub(crate) const NO_WORKSPACE_FILTER_LABEL: &str = "Workspace filter cleared";
