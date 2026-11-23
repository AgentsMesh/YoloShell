use yolo_hook_codec::{HookAncestry, HookIngressProfile, OUTBOX_ENV, SOCKET_ENV};

pub const PROVIDER: &str = "grok";
pub const PROVIDER_MAJOR: u16 = 1;
pub const PROVIDER_MINOR: u16 = 0;
pub const PROVIDER_PATCH: u16 = 70;
pub const MAX_ANCESTORS: usize = 8;

pub const HOOKS_DIRECTORY: &str = "hooks";

/// Snake_case aliases carried next to the camelCase envelope fields.
pub const PAYLOAD_SESSION_ID: &str = "session_id";
pub const PAYLOAD_HOOK_EVENT_NAME: &str = "hook_event_name";
pub const PAYLOAD_TRANSCRIPT_PATH: &str = "transcript_path";
pub const PAYLOAD_TOOL_NAME: &str = "tool_name";
pub const PAYLOAD_TOOL_USE_ID: &str = "tool_use_id";
pub const PAYLOAD_STOP_HOOK_ACTIVE: &str = "stop_hook_active";
pub const PAYLOAD_NOTIFICATION_TYPE: &str = "notification_type";

/// Event names as `hookEventName` spells them (`hook_event_name` uses the
/// PascalCase install names).
pub const EVENT_SESSION_START_SNAKE: &str = "session_start";
pub const EVENT_SESSION_END_SNAKE: &str = "session_end";
pub const EVENT_USER_PROMPT_SUBMIT_SNAKE: &str = "user_prompt_submit";
pub const EVENT_STOP_SNAKE: &str = "stop";
pub const EVENT_STOP_CANCELLED_SNAKE: &str = "stop_cancelled";
pub const EVENT_STOP_FAILURE_SNAKE: &str = "stop_failure";
pub const EVENT_PRE_TOOL_USE_SNAKE: &str = "pre_tool_use";
pub const EVENT_POST_TOOL_USE_SNAKE: &str = "post_tool_use";
pub const EVENT_POST_TOOL_USE_FAILURE_SNAKE: &str = "post_tool_use_failure";
pub const EVENT_NOTIFICATION_SNAKE: &str = "notification";
pub const EVENT_PRE_COMPACT_SNAKE: &str = "pre_compact";
pub const EVENT_POST_COMPACT_SNAKE: &str = "post_compact";
pub const EVENT_SUBAGENT_START_SNAKE: &str = "subagent_start";
pub const EVENT_SUBAGENT_STOP_SNAKE: &str = "subagent_stop";
/// Telemetry callbacks that carry no lifecycle fact.
pub const EVENT_TASK_COMPLETE_SNAKE: &str = "task_complete";

pub const NOTIFICATION_PERMISSION_PROMPT: &str = "permission_prompt";
pub const NOTIFICATION_IDLE_PROMPT: &str = "idle_prompt";
pub const NOTIFICATION_AGENT_NEEDS_INPUT: &str = "agent_needs_input";
pub const NOTIFICATION_ELICITATION: &str = "elicitation";

pub const REASON_END_TURN: &str = "end_turn";
pub const REASON_SHUTDOWN: &str = "shutdown";

/// Room for the underscores inserted when normalizing PascalCase event names.
pub const EVENT_NAME_CAPACITY_MARGIN: usize = 4;

/// One percent sign and two hexadecimal digits per encoded byte.
pub const PERCENT_ENCODED_BYTE_WIDTH: usize = 3;

pub const EVENT_SESSION_START: &str = "SessionStart";
pub const EVENT_SESSION_END: &str = "SessionEnd";
pub const EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
pub const EVENT_STOP: &str = "Stop";
pub const EVENT_STOP_CANCELLED: &str = "StopCancelled";
pub const EVENT_STOP_FAILURE: &str = "StopFailure";
pub const EVENT_PRE_TOOL_USE: &str = "PreToolUse";
pub const EVENT_POST_TOOL_USE: &str = "PostToolUse";
pub const EVENT_POST_TOOL_USE_FAILURE: &str = "PostToolUseFailure";
pub const EVENT_PERMISSION_DENIED: &str = "PermissionDenied";
pub const EVENT_NOTIFICATION: &str = "Notification";
pub const EVENT_PRE_COMPACT: &str = "PreCompact";
pub const EVENT_POST_COMPACT: &str = "PostCompact";
pub const EVENT_SUBAGENT_START: &str = "SubagentStart";
pub const EVENT_SUBAGENT_STOP: &str = "SubagentStop";

pub const HOOK_EVENTS: &[&str] = &[
    EVENT_SESSION_START,
    EVENT_SESSION_END,
    EVENT_USER_PROMPT_SUBMIT,
    EVENT_STOP,
    EVENT_STOP_CANCELLED,
    EVENT_STOP_FAILURE,
    EVENT_PRE_TOOL_USE,
    EVENT_POST_TOOL_USE,
    EVENT_POST_TOOL_USE_FAILURE,
    EVENT_PERMISSION_DENIED,
    EVENT_NOTIFICATION,
    EVENT_PRE_COMPACT,
    EVENT_POST_COMPACT,
    EVENT_SUBAGENT_START,
    EVENT_SUBAGENT_STOP,
];

/// Grok runs hook commands through a shell that is a child of the agent
/// process. Admission walks that ancestry to the native agent; the callback
/// environment is inherited today but is not a contract, so it is not required.
pub const PROFILE: HookIngressProfile = HookIngressProfile {
    provider: PROVIDER,
    socket_env: SOCKET_ENV,
    outbox_env: OUTBOX_ENV,
    max_ancestors: MAX_ANCESTORS,
    ancestry: HookAncestry::DetachedSession,
    require_peer_environment: false,
};

pub const INVALID_PAYLOAD: &str = "grok hook payload is invalid";
pub const INVALID_SESSION: &str = "grok hook session identity is invalid";
