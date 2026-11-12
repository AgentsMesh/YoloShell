pub use yolo_qwen_identity::KIND;
pub const EXECUTABLE_NAMES: &[&str] = &["qwen", "qwen.exe"];
pub const NODE_PACKAGE_COMPONENTS: &[&str] = &[".bin"];
pub const NODE_ENTRYPOINTS: &[&str] = &["qwen"];
pub const NODE_PACKAGE_ENTRYPOINTS: &[&str] = &["cli-entry.js", "cli.js"];
pub const NODE_PACKAGE_COMPONENT_PAIRS: &[&[&str]] = &[
    &["@qwen-code", "qwen-code"],
    &["node_modules", ".bin"],
];
pub const PROMPT_INTERRUPT_KEY: &[u8] = b"\x1b";
pub const DATA_HOME_ENV: &str = "QWEN_HOME";
pub const RUNTIME_HOME_ENV: &str = "QWEN_RUNTIME_DIR";
pub const DEFAULT_DATA_HOME_DIR: &str = ".qwen";
pub const EXECUTABLE: &str = "qwen";
pub const RESUME_ARGUMENT: &str = "--resume";
pub const SHORT_RESUME_ARGUMENT: &str = "-r";
pub const SESSION_ID_ARGUMENT: &str = "--session-id";
pub const CONTINUE_ARGUMENT: &str = "--continue";
pub const SHORT_CONTINUE_ARGUMENT: &str = "-c";
pub const FORK_SESSION_ARGUMENT: &str = "--fork-session";
pub const AUTH_TYPE_ARGUMENT: &str = "--auth-type";
pub const ARGUMENT_VALUE_SEPARATOR: char = '=';
pub const ARGUMENT_TERMINATOR: &str = "--";
pub const DIRECT_COMMAND_ARGUMENT_INDEX: usize = 1;
pub const NODE_COMMAND_ARGUMENT_INDEX: usize = 2;
pub const EVIDENCE_DIR: &str = "projects";
pub const TRANSCRIPT_DIR: &str = "chats";
pub const TRANSCRIPT_EXTENSION: &str = "jsonl";
pub const EVIDENCE_RELATIVE_COMPONENTS: usize = 4;
pub const SESSION_ID_BYTES: usize = 36;
pub const INVALID_SESSION_ID: &str = "qwen provider session id is invalid";
pub const INVALID_CWD: &str = "qwen working directory is invalid";
pub const INVALID_DATA_HOME: &str = "qwen data home is invalid";
pub const INVALID_EVIDENCE: &str = "qwen transcript evidence is invalid";
pub const DATA_HOME_NUL: &str = "qwen data home contains a NUL byte";
