use crate::claude_wire::HookAuthority;
use crate::fake_claude_authority_contract::{ActivityState, AuthorityView};
use crate::fake_claude_secure_transcript::TranscriptIdentity;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

pub(crate) const ENDPOINT_SCHEMA_VERSION: u32 = 1;
pub(crate) const INITIAL_GENERATION: u64 = 1;
pub(crate) const GENERATION_INCREMENT: u64 = 1;
pub(crate) const MAX_REPLAY_EVENTS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurableAuthority {
    pub(crate) schema_version: u32,
    pub(crate) authority: HookAuthority,
    pub(crate) provider_session_id: String,
    pub(crate) cwd: PathBuf,
    pub(crate) transcript: TranscriptIdentity,
    pub(crate) generation: u64,
    pub(crate) activity: ActivityState,
    pub(crate) runtime_active: bool,
    pub(crate) replay_event_ids: VecDeque<String>,
}

impl DurableAuthority {
    pub(crate) fn view(&self) -> AuthorityView {
        AuthorityView {
            authority: self.authority.clone(),
            provider_session_id: self.provider_session_id.clone(),
            transcript_path: self.transcript.path.clone(),
            generation: self.generation,
            activity: self.activity,
            runtime_active: self.runtime_active,
        }
    }

    pub(crate) fn remember(&mut self, event_id: &str) {
        if self.replay_event_ids.len() == MAX_REPLAY_EVENTS {
            self.replay_event_ids.pop_front();
        }
        self.replay_event_ids.push_back(event_id.to_owned());
    }
}
