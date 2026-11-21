use crate::claude_wire::{ClaudeHookFrame, HookAuthority};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;

pub(crate) type ContractResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityState {
    Running,
    WaitingForInput,
    Exited,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HookDisposition {
    Applied,
    Replay,
    Stale,
    Invalidated,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HookFailure {
    Missing,
    Error,
    Disabled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthorityView {
    pub(crate) authority: HookAuthority,
    pub(crate) provider_session_id: String,
    pub(crate) transcript_path: PathBuf,
    pub(crate) generation: u64,
    pub(crate) activity: ActivityState,
    pub(crate) runtime_active: bool,
}

pub(crate) trait ClaudeHookEndpoint {
    fn apply(&mut self, frame: &ClaudeHookFrame) -> ContractResult<HookDisposition>;
    fn mark_unavailable(&mut self, failure: HookFailure) -> ContractResult<()>;
    fn view(&self) -> Option<AuthorityView>;
}
