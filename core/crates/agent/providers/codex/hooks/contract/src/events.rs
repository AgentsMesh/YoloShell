pub mod event_names {
    pub const SESSION_START: &str = "SessionStart";
    pub const SESSION_END: &str = "SessionEnd";
    pub const USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
    pub const PRE_TOOL_USE: &str = "PreToolUse";
    pub const PERMISSION_REQUEST: &str = "PermissionRequest";
    pub const POST_TOOL_USE: &str = "PostToolUse";
    pub const PRE_COMPACT: &str = "PreCompact";
    pub const POST_COMPACT: &str = "PostCompact";
    pub const SUBAGENT_START: &str = "SubagentStart";
    pub const SUBAGENT_STOP: &str = "SubagentStop";
    pub const STOP: &str = "Stop";
    pub const ALL: &[&str] = &[
        SESSION_START,
        SESSION_END,
        USER_PROMPT_SUBMIT,
        PRE_TOOL_USE,
        PERMISSION_REQUEST,
        POST_TOOL_USE,
        PRE_COMPACT,
        POST_COMPACT,
        SUBAGENT_START,
        SUBAGENT_STOP,
        STOP,
    ];
}

pub const SESSION_END_HOOK_TIMEOUT_SECONDS: u64 = 3;
