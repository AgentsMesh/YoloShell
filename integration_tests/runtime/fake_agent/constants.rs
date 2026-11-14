pub const INVOCATION_LOG_FILE: &str = "invocations.jsonl";
pub const PROVIDER_TOOL_PATH_FIELD: &str = "file_path";
pub const PROVIDER_STOP_ACTIVE_FIELD: &str = "stop_hook_active";
pub const PROVIDER_LAST_MESSAGE_FIELD: &str = "last_assistant_message";
pub const PROVIDER_TOOL_NAME_FIELD: &str = "tool_name";
pub const PROVIDER_TOOL_INPUT_FIELD: &str = "tool_input";
pub const PROVIDER_TOOL_USE_ID_FIELD: &str = "tool_use_id";
pub const CODEX_ROLLOUT_TEST_LABEL: &str = "test-";
pub const CODEX_ROLLOUT_DATE_DIRECTORY: &str = "2026/08/05";
pub const INVOCATION_RUNTIME_EPOCH_KEY: &str = "runtime_epoch";
pub const INVOCATION_ARGUMENTS_KEY: &str = "arguments";
#[cfg(test)]
pub const INVOCATION_RECORD_FIELD_COUNT: usize = 2;
pub const INVOCATION_LINE_ENDING: u8 = b'\n';
pub const CODEX_VERSION_ARGUMENT: &str = "--version";
pub const CODEX_VERSION_OUTPUT_PREFIX: &str = "codex-cli ";
pub const CODEX_VERSION_SEPARATOR: char = '.';
pub const CODEX_MODEL: &str = "gpt-5";
pub const CODEX_HOOK_EVENT_NAME: &str = "hook_event_name";
pub const CODEX_HOOK_SESSION_ID: &str = "session_id";
pub const CODEX_HOOK_TRANSCRIPT_PATH: &str = "transcript_path";
pub const CODEX_HOOK_CWD: &str = "cwd";
pub const CODEX_HOOK_MODEL: &str = "model";
pub const CODEX_HOOK_PERMISSION_MODE: &str = "permission_mode";
pub const CODEX_HOOK_SOURCE: &str = "source";
pub const CODEX_SESSION_START_EVENT: &str = "SessionStart";
pub const CODEX_SESSION_STARTUP_SOURCE: &str = "startup";
pub const CODEX_SESSION_RESUME_SOURCE: &str = "resume";
pub const CODEX_PERMISSION_DEFAULT: &str = "default";
pub const CODEX_TEST_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub const CODEX_TEST_CWD: &str = "/tmp/yolo-codex/project";
pub const CODEX_TEST_TRANSCRIPT: &str = "/tmp/yolo-codex/sessions/rollout-test.jsonl";
pub const CODEX_EXPECTED_VERSION_OUTPUT: &str = "codex-cli 0.150.1\n";
pub const CODEX_SESSION_START_ARGUMENTS_INVALID: &str =
    "fake Codex session start arguments are invalid";
pub const CODEX_TUI_MARKER_ERROR: &str = "fake Codex launch marker is incomplete";
pub const HOLD_AFTER_EOF_ENV: &str = "YOLO_TEST_AGENT_HOLD_AFTER_EOF";
pub const NATIVE_CONTROL_SIGNAL_MARKER_ENV: &str = "YOLO_TEST_AGENT_SIGINT_MARKER";
pub const HOLD_AFTER_EOF_ENABLED: &str = "1";
pub const PACKAGED_E2E_ENV: &str = "YOLO_TEST_CLAUDE_PACKAGED_E2E";
pub const PACKAGED_E2E_ENABLED: &str = "1";
pub const PACKAGED_RECORD_DIRECTORY_ENV: &str = "YOLO_TEST_CLAUDE_RECORD_DIRECTORY";
pub const PACKAGED_PROJECT_DIRECTORY_ENV: &str = "YOLO_TEST_CLAUDE_PROJECT_DIRECTORY";
pub const PACKAGED_MARKER_LEFT_ENV: &str = "YOLO_E2E_FAKE_MARKER_LEFT";
pub const PACKAGED_MARKER_RIGHT_ENV: &str = "YOLO_E2E_FAKE_MARKER_RIGHT";
pub const PACKAGED_MARKER_NONCE_ENV: &str = "YOLO_E2E_FAKE_MARKER_NONCE";
pub const PACKAGED_FRESH_MODE: &str = "fresh";
pub const PACKAGED_RESUME_MODE: &str = "resume";
pub const PACKAGED_FRESH_RECORD: &str = "fresh.json";
pub const PACKAGED_RESUME_RECORD: &str = "resume.json";
pub const PACKAGED_PROJECT_DIRECTORY: &str = "packaged-e2e";
pub const PACKAGED_PROJECT_DIRECTORY_INVALID: &str =
    "packaged Claude project directory must be one safe path component";
pub const PACKAGED_START_SOURCE: &str = "startup";
pub const PACKAGED_RESUME_SOURCE: &str = "resume";
pub const PACKAGED_SESSION_START_EVENT: &str =
    yolo_runtime::claude_hooks::CLAUDE_SESSION_START_EVENT;
pub const PACKAGED_USER_PROMPT_EVENT: &str = yolo_runtime::claude_hooks::CLAUDE_USER_PROMPT_EVENT;
pub const PACKAGED_STOP_EVENT: &str = yolo_runtime::claude_hooks::CLAUDE_STOP_EVENT;
pub const PACKAGED_NOTIFICATION_EVENT: &str = yolo_runtime::claude_hooks::CLAUDE_NOTIFICATION_EVENT;
pub const PACKAGED_WAITING_NOTIFICATION_TYPE: &str = "idle_prompt";
pub const PACKAGED_HOOK_EVENTS: &[(&str, bool)] =
    yolo_runtime::claude_hooks::CLAUDE_MANAGED_HOOK_EVENTS;
pub const PACKAGED_SETTINGS_HOOKS_FIELD: &str = "hooks";
pub const PACKAGED_SETTINGS_MATCHER_FIELD: &str = "matcher";
pub const PACKAGED_SETTINGS_TYPE_FIELD: &str = "type";
pub const PACKAGED_SETTINGS_COMMAND_FIELD: &str = "command";
pub const PACKAGED_SETTINGS_COMMAND_TYPE: &str = "command";
pub const PACKAGED_HOOK_EVENT_FIELD: &str = "hook_event_name";
pub const PACKAGED_HOOK_SESSION_FIELD: &str = "session_id";
pub const PACKAGED_HOOK_TRANSCRIPT_FIELD: &str = "transcript_path";
pub const PACKAGED_HOOK_CWD_FIELD: &str = "cwd";
pub const PACKAGED_HOOK_SOURCE_FIELD: &str = "source";
pub const PACKAGED_HOOK_NOTIFICATION_TYPE_FIELD: &str = "notification_type";
pub const PACKAGED_RECORD_SCHEMA_VERSION: u32 = 1;
pub const PACKAGED_TRANSCRIPT_VERSION: &str = "2.1.241";
pub const PACKAGED_VERSION_FIELD: &str = "version";
pub const PACKAGED_PARENT_UUID_FIELD: &str = "parentUuid";
pub const PACKAGED_SIDECHAIN_FIELD: &str = "isSidechain";
pub const PACKAGED_USER_TYPE_FIELD: &str = "userType";
pub const PACKAGED_GIT_BRANCH_FIELD: &str = "gitBranch";
pub const PACKAGED_UUID_FIELD: &str = "uuid";
pub const PACKAGED_TIMESTAMP_FIELD: &str = "timestamp";
pub const PACKAGED_ROLE_FIELD: &str = "role";
pub const PACKAGED_MESSAGE_ID_FIELD: &str = "id";
pub const PACKAGED_MODEL_FIELD: &str = "model";
pub const PACKAGED_MESSAGE_TYPE_FIELD: &str = "type";
pub const PACKAGED_STOP_SEQUENCE_FIELD: &str = "stop_sequence";
pub const PACKAGED_DURATION_FIELD: &str = "durationMs";
pub const PACKAGED_MESSAGE_COUNT_FIELD: &str = "messageCount";
pub const PACKAGED_REDACTED_USER_TYPE: &str = "redacted-user-type";
pub const PACKAGED_REDACTED_BRANCH: &str = "redacted-branch";
pub const PACKAGED_REDACTED_ROLE: &str = "redacted-role";
pub const PACKAGED_REDACTED_MODEL: &str = "redacted-model";
pub const PACKAGED_REDACTED_MESSAGE_TYPE: &str = "redacted-message-type";
pub const PACKAGED_REDACTED_USER_CONTENT: &str = "[REDACTED_USER_CONTENT]";
pub const PACKAGED_REDACTED_ASSISTANT_CONTENT: &str = "[REDACTED_ASSISTANT_CONTENT]";
pub const PACKAGED_TIMESTAMP: &str = "1970-01-01T00:00:00.000Z";
pub const PACKAGED_SYSTEM_UUID: &str = "00000000-0000-4000-8000-000000000010";
pub const PACKAGED_USER_UUID: &str = "00000000-0000-4000-8000-000000000011";
pub const PACKAGED_ASSISTANT_UUID: &str = "00000000-0000-4000-8000-000000000012";
pub const PACKAGED_ASSISTANT_MESSAGE_ID: &str = "redacted-message-end-turn";
pub const PACKAGED_TUI_PING_COMMAND: &str = "yolo-ping";
pub const PACKAGED_TUI_CWD_COMMAND: &str = "yolo-cwd";
pub const PACKAGED_TUI_CWD_ACK: &str = "YOLO_CWD_CHANGED";
pub const PACKAGED_HOOK_OLD_CWD_FIELD: &str = "old_cwd";
pub const PACKAGED_HOOK_NEW_CWD_FIELD: &str = "new_cwd";
pub const PACKAGED_PRIVATE_MODE: u32 = 0o600;
pub const PACKAGED_SETTINGS_MATCH_ALL: &str = "";
pub const PACKAGED_RECORD_SCHEMA_FIELD: &str = "schema_version";
pub const PACKAGED_RECORD_MODE_FIELD: &str = "mode";
pub const PACKAGED_RECORD_SESSION_FIELD: &str = "session_id";
pub const PACKAGED_RECORD_RUNTIME_FIELD: &str = "runtime_epoch";
pub const PACKAGED_RECORD_ARGUMENTS_FIELD: &str = "arguments";
pub const PACKAGED_RECORD_HOOK_COUNT_FIELD: &str = "hook_count";
pub const PACKAGED_RECORD_PROCESS_FIELD: &str = "process_pid";
pub const PACKAGED_RECORD_CWD_FIELD: &str = "cwd";
pub const PACKAGED_RECORD_DATA_HOME_FIELD: &str = "data_home";
pub const PACKAGED_RECORD_TRANSCRIPT_FIELD: &str = "transcript_path";
#[cfg(test)]
pub const CLAUDE_STARTUP_SOURCE: &str = "startup";
#[cfg(test)]
pub const CLAUDE_RESUME_SOURCE: &str = "resume";
#[cfg(test)]
pub const CLAUDE_CLEAR_SOURCE: &str = "clear";
#[cfg(test)]
pub const CLAUDE_COMPACT_SOURCE: &str = "compact";
#[cfg(test)]
pub const CLAUDE_FORK_SOURCE: &str = "fork";
#[cfg(test)]
pub const CLAUDE_LOGOUT_REASON: &str = "logout";
#[cfg(test)]
pub const CLAUDE_PROMPT_EXIT_REASON: &str = "prompt_input_exit";
#[cfg(test)]
pub const CLAUDE_BYPASS_DISABLED_REASON: &str = "bypass_permissions_disabled";
#[cfg(test)]
pub const CLAUDE_OTHER_REASON: &str = "other";
#[cfg(test)]
pub const CLAUDE_MANUAL_TRIGGER: &str = "manual";
#[cfg(test)]
pub const CLAUDE_AUTO_TRIGGER: &str = "auto";
#[cfg(test)]
pub const CLAUDE_HOOKS_ENABLED: &str = "enabled";
#[cfg(test)]
pub const CLAUDE_HOOKS_MISSING: &str = "missing";
#[cfg(test)]
pub const CLAUDE_HOOKS_ERROR: &str = "error";
#[cfg(test)]
pub const CLAUDE_HOOKS_DISABLED: &str = "disableAllHooks";
#[cfg(test)]
pub const CLAUDE_EVENT_NAME_KEY: &str = "hook_event_name";
#[cfg(test)]
pub const CLAUDE_INPUT_KEY: &str = "input";
#[cfg(test)]
pub const CLAUDE_SOURCE_KEY: &str = "source";
#[cfg(test)]
pub const CLAUDE_EVENT_ID_KEY: &str = "event_id";
#[cfg(test)]
pub const CLAUDE_SESSION_START_EVENT: &str = "SessionStart";
#[cfg(test)]
pub const INVOCATION_JSON_INVALID: &str = "fake agent invocation JSON is invalid";
#[cfg(test)]
pub const INVOCATION_OBJECT_INVALID: &str = "fake agent invocation must be an exact object";
#[cfg(test)]
pub const INVOCATION_RUNTIME_EPOCH_INVALID: &str = "fake agent runtime epoch must be a string";
#[cfg(test)]
pub const INVOCATION_ARGUMENTS_INVALID: &str = "fake agent arguments must be strings";
