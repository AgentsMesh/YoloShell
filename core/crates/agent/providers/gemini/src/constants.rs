pub use yolo_gemini_identity::{KIND, PROVIDER_ID};

pub const EXECUTABLE: &str = "gemini";
pub const EXECUTABLE_NAMES: &[&str] = &["gemini", "gemini.exe"];
pub const NODE_PACKAGE_COMPONENTS: &[&str] = &[".bin"];
pub const NODE_ENTRYPOINTS: &[&str] = &["gemini"];
pub const NODE_PACKAGE_COMPONENT_PAIRS: &[&[&str]] = &[
    &["@google", "gemini-cli"],
    &["node_modules", ".bin"],
];

pub const DATA_HOME_ENV: &str = "GEMINI_CLI_HOME";
pub const HOME_ENV: &str = "HOME";
pub const DATA_HOME_DIR: &str = ".gemini";
pub const EVIDENCE_DIR: &str = "tmp";
pub const EVIDENCE_RELATIVE_COMPONENTS: usize = 4;
pub const TRANSCRIPT_DIR: &str = "chats";
pub const TRANSCRIPT_PREFIX: &str = "session-";
pub const TRANSCRIPT_EXTENSION: &str = "jsonl";

pub const RESUME_ARGUMENT: &str = "--resume";
pub const SHORT_RESUME_ARGUMENT: &str = "-r";
pub const SESSION_ID_ARGUMENT: &str = "--session-id";
pub const ARGUMENT_VALUE_SEPARATOR: char = '=';
pub const ARGUMENT_TERMINATOR: &str = "--";
pub const DIRECT_COMMAND_ARGUMENT_INDEX: usize = 1;
pub const SESSION_ID_BYTES: usize = 36;

/// Gemini's keypress reader converts an Enter arriving within 30 ms of an
/// insertable key into a newline (`bufferFastReturn`). A native submit must
/// therefore land the submit key after the pasted prompt settles.
pub const PROMPT_SUBMIT_SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// Gemini's TUI reads its cancel key from the terminal and installs no SIGINT
/// handler, so a native interrupt must write Esc. A process SIGINT terminates
/// the CLI instead of cancelling the turn.
pub const PROMPT_INTERRUPT_KEY: &[u8] = b"\x1b";

pub const INVALID_SESSION_ID: &str = "gemini provider session id is invalid";
pub const INVALID_CWD: &str = "gemini working directory is invalid";
pub const INVALID_DATA_HOME: &str = "gemini data home is invalid";
pub const INVALID_EVIDENCE: &str = "gemini transcript evidence is invalid";
