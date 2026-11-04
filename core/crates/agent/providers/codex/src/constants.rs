pub const EXECUTABLE_NAMES: &[&str] = &["codex"];
pub const NODE_EXECUTABLE: &str = "node";
pub const NODE_PACKAGE_COMPONENTS: &[&str] = &["@openai", "codex"];
pub const NODE_ENTRYPOINTS: &[&str] = &["codex", "codex.js"];
pub const RECORD_TYPE_KEY: &str = "type";
pub const PAYLOAD_KEY: &str = "payload";
pub const SESSION_META_TYPE: &str = "session_meta";
pub const SESSION_ID_KEY: &str = "id";
pub const CWD_KEY: &str = "cwd";
pub const SOURCE_KEY: &str = "source";
pub const SUBAGENT_SOURCE_KEY: &str = "subagent";
pub const CLI_SOURCE: &str = "cli";
/// `codex exec` writes the same main-session metadata with `source: "exec"`.
pub const EXEC_SOURCE: &str = "exec";
pub const EVENT_MESSAGE_TYPE: &str = "event_msg";
pub const EVENT_KIND_KEY: &str = "type";
pub const TASK_STARTED_EVENT: &str = "task_started";
pub const TASK_COMPLETE_EVENT: &str = "task_complete";
pub const TOKEN_COUNT_EVENT: &str = "token_count";
pub const TURN_ABORTED_EVENT: &str = "turn_aborted";
pub const TURN_ID_KEY: &str = "turn_id";
pub const AGENT_ID_KEY: &str = "agent_id";
pub const TURN_TAIL_BYTES: u64 = 256 * 1024;
pub const RESUME_ARGUMENT: &str = "resume";
pub const FORK_ARGUMENT: &str = "fork";
pub const YOLO_ARGUMENT: &str = "--yolo";
pub const CONFIG_ARGUMENT: &str = "--config";
pub const SHORT_CONFIG_ARGUMENT: &str = "-c";
pub const DISABLE_ARGUMENT: &str = "--disable";
pub const BYPASS_HOOK_TRUST_ARGUMENT: &str = "--dangerously-bypass-hook-trust";
pub const LAST_ARGUMENT: &str = "--last";
pub const OPTION_TERMINATOR: &str = "--";
pub const LONG_OPTION_PREFIX: &str = "--";
pub const LONG_OPTION_ASSIGNMENT: char = '=';
pub const OPTION_PREFIX: char = '-';
pub const CWD_ARGUMENT: &str = "-C";
pub const LONG_CWD_ARGUMENT: &str = "--cd";
pub const CWD_ARGUMENTS: &[&str] = &[CWD_ARGUMENT, LONG_CWD_ARGUMENT];
pub const BOOLEAN_ARGUMENTS: &[&str] = &[
    "--all",
    "--dangerously-bypass-approvals-and-sandbox",
    BYPASS_HOOK_TRUST_ARGUMENT,
    "--include-non-interactive",
    "--no-alt-screen",
    "--oss",
    "--search",
    "--strict-config",
    YOLO_ARGUMENT,
];
pub const VALUE_ARGUMENTS: &[&str] = &[
    CWD_ARGUMENT,
    "-a",
    SHORT_CONFIG_ARGUMENT,
    "-m",
    "-p",
    "-s",
    "--add-dir",
    "--ask-for-approval",
    LONG_CWD_ARGUMENT,
    CONFIG_ARGUMENT,
    DISABLE_ARGUMENT,
    "--enable",
    "--local-provider",
    "--model",
    "--profile",
    "--sandbox",
];
pub const VARIABLE_VALUE_ARGUMENTS: &[&str] = &["-i", "--image"];
pub const REMOTE_ARGUMENTS: &[&str] = &["--remote", "--remote-auth-token-env"];
pub const EXECUTABLE: &str = "codex";
pub const PROMPT_INTERRUPT_KEY: &[u8] = b"\x1b";
pub use yolo_codex_identity::KIND;
pub const DATA_HOME_ENV: &str = "CODEX_HOME";
pub const DEFAULT_DATA_HOME_DIR: &str = ".codex";
pub const EVIDENCE_DIR: &str = "sessions";
pub const SESSION_INDEX_FILE: &str = "session_index.jsonl";
pub const SESSION_INDEX_TAIL_BYTES: u64 = 64 * 1024;
pub const SESSION_INDEX_ID_KEY: &str = "id";
pub const SESSION_INDEX_TITLE_KEY: &str = "thread_name";
pub const ROLLOUT_FILE_PREFIX: &str = "rollout-";
pub const ROLLOUT_ID_SEPARATOR: char = '-';
pub const UUID_HYPHENATED_BYTES: usize = 36;
pub const EVIDENCE_RELATIVE_COMPONENTS: usize = 5;
pub const EXPECTED_FILE_LINKS: u64 = 1;
pub const DIRECT_COMMAND_ARGUMENT_INDEX: usize = 1;
pub const NODE_COMMAND_ARGUMENT_INDEX: usize = 2;
pub const NEXT_ARGUMENT_OFFSET: usize = 1;
pub const SINGLE_ARGUMENT_COUNT: usize = 1;
pub const OPTION_AND_VALUE_COUNT: usize = 2;
pub const FILE_START_OFFSET: u64 = 0;
pub const JSON_LINE_ENDING_BYTE: u8 = b'\n';
pub const DATA_HOME_NUL_ERROR: &str = "Codex data home contains NUL";
pub const DATA_HOME_INSPECT_ERROR: &str = "inspect Codex data home";
pub const EVIDENCE_OPEN_ERROR: &str = "open anchored Codex evidence";
pub const EVIDENCE_INSPECT_ERROR: &str = "inspect anchored Codex evidence";
pub const EVIDENCE_SEEK_ERROR: &str = "seek Codex evidence";
pub const EVIDENCE_READ_ERROR: &str = "read Codex evidence";
pub const SESSION_ID_INVALID_ERROR: &str = "Codex binding session id is not a canonical UUID";
#[cfg(test)]
pub const TEST_REMOTE_ENDPOINT_ARGUMENT: &str = "--remote=unix://codex.sock";
#[cfg(test)]
pub const TEST_UNKNOWN_ARGUMENT: &str = "--future-option";
#[cfg(test)]
pub const REAL_ORIGINATOR: &str = "codex_cli_rs";
#[cfg(test)]
pub const REAL_CLI_VERSION_KEY: &str = "cli_version";
#[cfg(test)]
pub const REAL_SESSION_LINEAGE_ID_KEY: &str = "session_id";
#[cfg(test)]
pub const REAL_THREAD_SOURCE_KEY: &str = "thread_source";
#[cfg(test)]
pub const REAL_MODEL_PROVIDER_KEY: &str = "model_provider";
#[cfg(test)]
pub const REAL_BASE_INSTRUCTIONS_KEY: &str = "base_instructions";
#[cfg(test)]
pub const REAL_HISTORY_MODE_KEY: &str = "history_mode";
#[cfg(test)]
pub const REAL_PARENT_THREAD_ID_KEY: &str = "parent_thread_id";
#[cfg(test)]
pub const REAL_FORKED_FROM_ID_KEY: &str = "forked_from_id";
#[cfg(test)]
pub const REAL_THREAD_SPAWN_KEY: &str = "thread_spawn";
#[cfg(test)]
pub const REAL_AGENT_PATH_KEY: &str = "agent_path";
#[cfg(test)]
pub const REAL_AGENT_NICKNAME_KEY: &str = "agent_nickname";
#[cfg(test)]
pub const REAL_AGENT_ROLE_KEY: &str = "agent_role";
#[cfg(test)]
pub const TEST_UNKNOWN_EVENT: &str = "future_event";
