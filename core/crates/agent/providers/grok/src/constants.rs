pub use yolo_grok_identity::{KIND, PROVIDER_ID};

pub const EXECUTABLE: &str = "grok";
pub const EXECUTABLE_NAMES: &[&str] = &["grok", "grok.exe"];
/// The native binary under the data home is versioned, e.g.
/// `grok-linux-aarch64`; the launcher on PATH is a `grok` symlink.
pub const EXECUTABLE_PREFIX: &str = "grok-";

pub const DATA_HOME_ENV: &str = "GROK_HOME";
pub const HOME_ENV: &str = "HOME";
pub const DATA_HOME_DIR: &str = ".grok";
pub const EVIDENCE_DIR: &str = "sessions";
/// `<data home>/sessions/<encoded-cwd>/<session-id>/updates.jsonl`
pub const EVIDENCE_RELATIVE_COMPONENTS: usize = 4;
pub const TRANSCRIPT_FILE: &str = "updates.jsonl";
pub const SESSION_ID_BYTES: usize = 36;

pub const RESUME_ARGUMENT: &str = "--resume";
pub const SHORT_RESUME_ARGUMENT: &str = "-r";
pub const SESSION_ID_ARGUMENT: &str = "--session-id";
pub const SHORT_SESSION_ID_ARGUMENT: &str = "-s";
pub const CONTINUE_ARGUMENT: &str = "--continue";
pub const SHORT_CONTINUE_ARGUMENT: &str = "-c";
pub const ARGUMENT_VALUE_SEPARATOR: char = '=';
pub const ARGUMENT_TERMINATOR: &str = "--";
pub const DIRECT_COMMAND_ARGUMENT_INDEX: usize = 1;

/// Grok's TUI binds Ctrl+C to "cancel the active turn" and installs no SIGINT
/// handler, so a native interrupt must write the Ctrl+C byte. A process SIGINT
/// terminates the CLI instead of cancelling the turn.
pub const PROMPT_INTERRUPT_KEY: &[u8] = b"\x03";

pub const INVALID_SESSION_ID: &str = "grok provider session id is invalid";
pub const INVALID_CWD: &str = "grok working directory is invalid";
pub const INVALID_DATA_HOME: &str = "grok data home is invalid";
pub const INVALID_EVIDENCE: &str = "grok transcript evidence is invalid";
