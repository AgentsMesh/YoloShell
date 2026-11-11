pub const PROVIDER: &str = "qwen";
pub const MAX_ANCESTORS: usize = 32;
/// The forked engine's outbox directory, removed once on upgrade.
pub const LEGACY_OUTBOX_DIRECTORY: &str = "qwen-hook";

pub const PROVIDER_MAJOR: u16 = 0;
pub const PROVIDER_MINOR: u16 = 26;
pub const PROVIDER_PATCH: u16 = 2;

pub const EVENT_SESSION_START: &str = "SessionStart";
pub const EVENT_SESSION_END: &str = "SessionEnd";
pub const EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
pub const EVENT_PRE_TOOL_USE: &str = "PreToolUse";
pub const EVENT_POST_TOOL_USE: &str = "PostToolUse";
pub const EVENT_POST_TOOL_USE_FAILURE: &str = "PostToolUseFailure";
pub const EVENT_PERMISSION_REQUEST: &str = "PermissionRequest";
pub const EVENT_PERMISSION_DENIED: &str = "PermissionDenied";
pub const EVENT_PRE_COMPACT: &str = "PreCompact";
pub const EVENT_POST_COMPACT: &str = "PostCompact";
pub const EVENT_SUBAGENT_START: &str = "SubagentStart";
pub const EVENT_SUBAGENT_STOP: &str = "SubagentStop";
pub const EVENT_NOTIFICATION: &str = "Notification";
pub const EVENT_SESSION_DELETE: &str = "SessionDelete";
pub const EVENT_STOP: &str = "Stop";
pub const EVENT_MESSAGE_DISPLAY: &str = "MessageDisplay";
pub const EVENT_INSTRUCTIONS_LOADED: &str = "InstructionsLoaded";
pub const EVENT_POST_TOOL_BATCH: &str = "PostToolBatch";

pub const NOTIFICATION_IDLE_PROMPT: &str = "idle_prompt";
pub const NOTIFICATION_PERMISSION_PROMPT: &str = "permission_prompt";
pub const NOTIFICATION_AUTH_SUCCESS: &str = "auth_success";

pub const SOURCE_STARTUP: &str = "startup";
pub const SOURCE_RESUME: &str = "resume";
pub const SOURCE_CLEAR: &str = "clear";
pub const SOURCE_COMPACT: &str = "compact";
pub const SOURCE_FORK: &str = "fork";
pub const REASON_CLEAR: &str = "clear";
pub const REASON_RESUME: &str = "resume";

pub const MAX_IDENTIFIER_BYTES: usize = 256;

pub const INVALID: &str = "Qwen hook payload, scope or outbox is invalid";
