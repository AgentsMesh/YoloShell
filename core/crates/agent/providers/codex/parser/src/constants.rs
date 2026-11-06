pub(super) const TYPE_FIELD: &str = "type";
pub(super) const PAYLOAD_FIELD: &str = "payload";
pub(super) const ROLE_FIELD: &str = "role";
pub(super) const ID_FIELD: &str = "id";
pub(super) const TIMESTAMP_FIELD: &str = "timestamp";
pub(super) const SESSION_META: &str = "session_meta";
pub(super) const TURN_CONTEXT: &str = "turn_context";
pub(super) const COMPACTED: &str = "compacted";
pub(super) const RESPONSE_ITEM: &str = "response_item";
pub(super) const EVENT_MESSAGE: &str = "event_msg";
pub(super) const MESSAGE: &str = "message";
pub(super) const USER_ROLE: &str = "user";
pub(super) const ASSISTANT_ROLE: &str = "assistant";
pub(super) const SYSTEM_ROLE: &str = "system";
pub(super) const DEVELOPER_ROLE: &str = "developer";
pub(super) const FUNCTION_CALL: &str = "function_call";
pub(super) const CUSTOM_TOOL_CALL: &str = "custom_tool_call";
pub(super) const FUNCTION_CALL_OUTPUT: &str = "function_call_output";
pub(super) const CUSTOM_TOOL_CALL_OUTPUT: &str = "custom_tool_call_output";
pub(super) const CONTENT_FIELD: &str = "content";
pub(super) const REASONING: &str = "reasoning";
pub(super) const SUMMARY_FIELD: &str = "summary";
pub(super) const TEXT_FIELD: &str = "text";
pub(super) const CALL_ID_FIELD: &str = "call_id";
pub(super) const NAME_FIELD: &str = "name";
pub(super) const ARGUMENTS_FIELD: &str = "arguments";
pub(super) const INPUT_FIELD: &str = "input";
pub(super) const OUTPUT_FIELD: &str = "output";
pub(super) const IS_ERROR_FIELD: &str = "is_error";
pub(super) const INPUT_TEXT: &str = "input_text";
pub(super) const OUTPUT_TEXT: &str = "output_text";
pub(super) const INPUT_IMAGE: &str = "input_image";
pub(super) const IMAGE: &str = "image";
pub(super) const USER_MESSAGE: &str = "user_message";
pub(super) const AGENT_MESSAGE: &str = "agent_message";
pub(super) const AGENT_REASONING: &str = "agent_reasoning";
pub(super) const TOKEN_COUNT: &str = "token_count";
pub(super) const TASK_STARTED: &str = "task_started";
pub(super) const TASK_COMPLETE: &str = "task_complete";
pub(super) const TURN_ABORTED: &str = "turn_aborted";
pub(super) const ITEM_COMPLETED: &str = "item_completed";
pub(super) const CONTEXT_CLEARED: &str = "context_cleared";
pub(super) const SESSION_CLEARED: &str = "session_cleared";
pub(super) const THREAD_ROLLED_BACK: &str = "thread_rolled_back";
pub(super) const WARNING: &str = "warning";
pub(super) const ERROR: &str = "error";
pub(super) const WORLD_STATE: &str = "world_state";
pub(super) const TOKEN_USAGE_RECORD: &str = "token_usage_record";
pub(super) const THREAD_SETTINGS_APPLIED: &str = "thread_settings_applied";

/// Codex injects these tagged messages as model context before a turn. They
/// are not conversation turns even when the rollout records them as user or
/// developer messages.
pub(super) const CONTEXT_MESSAGE_PREFIXES: [&str; 5] = [
    "<environment_context>",
    "<user_instructions>",
    "<skills_instructions>",
    "<multi_agent_role>",
    "<multi_agent_mode>",
];
