use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[path = "claude_wire_validation.rs"]
mod validation;

pub(crate) const WIRE_SCHEMA_VERSION: u32 = 1;
pub(crate) const WIRE_SCHEMA_UNSUPPORTED: &str = "unsupported fake Claude wire schema";
pub(crate) const WIRE_AUTHORITY_INVALID: &str = "invalid fake Claude authority";
pub(crate) const WIRE_EVENT_INVALID: &str = "invalid fake Claude hook event";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessIdentity {
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
    pub(crate) parent_pid: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HookAuthority {
    pub(crate) terminal_id: String,
    pub(crate) runtime_epoch: String,
    pub(crate) invocation_id: String,
    pub(crate) hook_token: String,
    pub(crate) worker: ProcessIdentity,
    pub(crate) shell: ProcessIdentity,
    pub(crate) agent: ProcessIdentity,
    pub(crate) hook: ProcessIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommonHookInput {
    pub(crate) session_id: String,
    pub(crate) transcript_path: PathBuf,
    pub(crate) cwd: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
    Fork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionEndReason {
    Clear,
    Resume,
    Logout,
    PromptInputExit,
    BypassPermissionsDisabled,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompactTrigger {
    Manual,
    Auto,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub(crate) enum ClaudeHookInput {
    SessionStart {
        #[serde(flatten)]
        common: CommonHookInput,
        source: SessionStartSource,
    },
    SessionEnd {
        #[serde(flatten)]
        common: CommonHookInput,
        reason: SessionEndReason,
    },
    CwdChanged {
        #[serde(flatten)]
        common: CommonHookInput,
        old_cwd: PathBuf,
        new_cwd: PathBuf,
    },
    PreCompact {
        #[serde(flatten)]
        common: CommonHookInput,
        trigger: CompactTrigger,
        custom_instructions: String,
    },
    PostCompact {
        #[serde(flatten)]
        common: CommonHookInput,
        trigger: CompactTrigger,
        compact_summary: String,
    },
    UserPromptSubmit {
        #[serde(flatten)]
        common: CommonHookInput,
        prompt: String,
    },
    Stop {
        #[serde(flatten)]
        common: CommonHookInput,
        stop_hook_active: bool,
        last_assistant_message: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaudeHookFrame {
    pub(crate) schema_version: u32,
    pub(crate) event_id: String,
    pub(crate) authority: HookAuthority,
    pub(crate) input: ClaudeHookInput,
}
