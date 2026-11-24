pub(crate) const HOOK_INPUT_MAX_BYTES: u64 = 64 * 1024;
pub(crate) const SESSION_START_EVENT: &str = crate::claude_hooks::CLAUDE_SESSION_START_EVENT;
pub(crate) const SESSION_END_EVENT: &str = crate::claude_hooks::CLAUDE_SESSION_END_EVENT;
pub(crate) const USER_PROMPT_EVENT: &str = crate::claude_hooks::CLAUDE_USER_PROMPT_EVENT;
pub(crate) const STOP_EVENT: &str = crate::claude_hooks::CLAUDE_STOP_EVENT;
pub(crate) const CWD_CHANGED_EVENT: &str = crate::claude_hooks::CLAUDE_CWD_CHANGED_EVENT;
pub(crate) const NOTIFICATION_EVENT: &str = crate::claude_hooks::CLAUDE_NOTIFICATION_EVENT;
pub(crate) const PRE_TOOL_USE_EVENT: &str = crate::claude_hooks::CLAUDE_PRE_TOOL_USE_EVENT;
pub(crate) const WAITING_NOTIFICATION_TYPES: &[&str] =
    &["idle_prompt", "agent_needs_input", "elicitation_dialog"];
pub(crate) const PERMISSION_NOTIFICATION_TYPE: &str = "permission_prompt";
pub(crate) const SOURCE_STARTUP: &str = "startup";
pub(crate) const SOURCE_RESUME: &str = "resume";
pub(crate) const SOURCE_CLEAR: &str = "clear";
pub(crate) const SOURCE_COMPACT: &str = "compact";
pub(crate) const SOURCE_FORK: &str = "fork";
