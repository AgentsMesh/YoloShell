use serde_json::Value;

pub const FIXTURE_BYTES: &[u8] =
    include_bytes!("../src/fixtures/claude-code-2.1.222-transcript.redacted.jsonl");
pub const SESSION_ID: &str = "00000000-0000-4000-8000-000000000222";
pub const CWD: &str = "/redacted/claude-code/project";
pub const CLAUDE_VERSION: &str = "2.1.222";
pub const RECORD_COUNT: usize = 8;
pub const USER_PROMPT_INDEX: usize = 0;
pub const ASSISTANT_TOOL_USE_INDEX: usize = 1;
pub const USER_TOOL_RESULT_INDEX: usize = 2;
pub const ASSISTANT_END_TURN_INDEX: usize = 3;
pub const MODE_INDEX: usize = 4;
pub const PERMISSION_MODE_INDEX: usize = 5;
pub const AI_TITLE_INDEX: usize = 6;
pub const TURN_DURATION_INDEX: usize = 7;
pub const VERSION_KEY: &str = "version";
pub const SIGNATURE_KEY: &str = "signature";
pub const CALLER_KEY: &str = "caller";
pub const THINKING_METADATA_KEY: &str = "thinkingMetadata";
pub const TODOS_KEY: &str = "todos";
pub const PROMPT_ID_KEY: &str = "promptId";
pub const EFFORT_KEY: &str = "effort";
pub const REQUEST_ID_KEY: &str = "requestId";
pub const TOOL_USE_RESULT_KEY: &str = "toolUseResult";
pub const SOURCE_TOOL_UUID_KEY: &str = "sourceToolAssistantUUID";
pub const PENDING_AGENT_COUNT_KEY: &str = "pendingBackgroundAgentCount";
pub const MODE_KEY: &str = crate::claude::constants::MODE_RECORD;
pub const PERMISSION_MODE_KEY: &str = "permissionMode";
pub const AI_TITLE_KEY: &str = "aiTitle";
pub const REDACTED_MARKER: &str = "[REDACTED_";
pub const FORBIDDEN_HOME_PREFIX: &str = "/Users/";
pub const DATA_HOME_DIRECTORY: &str = "claude-home";
pub const THINKING_BLOCK_INDEX: usize = 0;
pub const TOOL_USE_BLOCK_INDEX: usize = 2;
pub const JSONL_RECORD_SEPARATOR: u8 = b'\n';
pub const INVALID_FIXTURE_ERROR: &str = "redacted Claude fixture must be JSON";

pub fn records() -> Vec<Value> {
    FIXTURE_BYTES
        .split(|byte| *byte == JSONL_RECORD_SEPARATOR)
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect(INVALID_FIXTURE_ERROR))
        .collect()
}

pub fn content_blocks(record: &Value) -> &[Value] {
    record
        .get(super::super::constants::MESSAGE_KEY)
        .and_then(|message| message.get(super::super::constants::CONTENT_KEY))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
