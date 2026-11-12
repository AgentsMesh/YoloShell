use yolo_hook_codec::{HookAncestry, HookIngressProfile, OUTBOX_ENV, SOCKET_ENV};

pub const PROVIDER: &str = "gemini";
pub const PROVIDER_MAJOR: u16 = 0;
pub const PROVIDER_MINOR: u16 = 66;
pub const PROVIDER_PATCH: u16 = 0;
pub const MAX_ANCESTORS: usize = 8;

/// Snake_case payload keys carried by every callback.
pub const PAYLOAD_HOOK_EVENT_NAME: &str = "hook_event_name";
pub const PAYLOAD_SESSION_ID: &str = "session_id";
pub const PAYLOAD_TRANSCRIPT_PATH: &str = "transcript_path";

pub const NOTIFICATION_PERMISSION_PROMPT: &str = "permission_prompt";
pub const NOTIFICATION_IDLE_PROMPT: &str = "idle_prompt";
pub const NOTIFICATION_AGENT_NEEDS_INPUT: &str = "agent_needs_input";

pub const EVENT_SESSION_START: &str = "SessionStart";
pub const EVENT_SESSION_END: &str = "SessionEnd";
pub const EVENT_BEFORE_AGENT: &str = "BeforeAgent";
pub const EVENT_AFTER_AGENT: &str = "AfterAgent";
pub const EVENT_BEFORE_TOOL: &str = "BeforeTool";
pub const EVENT_AFTER_TOOL: &str = "AfterTool";
pub const EVENT_NOTIFICATION: &str = "Notification";

pub const HOOK_EVENTS: &[&str] = &[
    EVENT_SESSION_START,
    EVENT_SESSION_END,
    EVENT_BEFORE_AGENT,
    EVENT_AFTER_AGENT,
    EVENT_BEFORE_TOOL,
    EVENT_AFTER_TOOL,
    EVENT_NOTIFICATION,
];

/// Gemini runs hook commands through a shell that is a child of the agent
/// process. Admission walks that ancestry to the node agent and relies on the
/// agent's environment for the terminal fence; the callback environment is
/// inherited today but is not a contract, so it is not required.
pub const PROFILE: HookIngressProfile = HookIngressProfile {
    provider: PROVIDER,
    socket_env: SOCKET_ENV,
    outbox_env: OUTBOX_ENV,
    max_ancestors: MAX_ANCESTORS,
    ancestry: HookAncestry::DetachedSession,
    require_peer_environment: false,
};

pub const INVALID_PAYLOAD: &str = "gemini hook payload is invalid";
pub const INVALID_SESSION: &str = "gemini hook session identity is invalid";
