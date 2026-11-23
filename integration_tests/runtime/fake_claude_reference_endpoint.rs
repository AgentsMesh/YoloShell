use crate::claude_wire::{ClaudeHookFrame, ClaudeHookInput, SessionStartSource};
use crate::fake_claude_authority_contract::{
    ActivityState, AuthorityView, ClaudeHookEndpoint, ContractResult, HookDisposition, HookFailure,
};
use crate::fake_claude_reference_store;
use crate::fake_claude_reference_transition::{self, replacement_invocation};
use crate::fake_claude_reference_types::{DurableAuthority, GENERATION_INCREMENT};
use crate::fake_claude_secure_transcript::TranscriptIdentity;
use std::path::{Path, PathBuf};

const GENERATION_OVERFLOW: &str = "fake Claude runtime generation overflow";

pub(crate) struct ReferenceEndpoint {
    state_path: PathBuf,
    data_home: PathBuf,
    state: Option<DurableAuthority>,
}

impl ReferenceEndpoint {
    pub(crate) fn open(state_path: &Path, data_home: &Path) -> ContractResult<Self> {
        Ok(Self {
            state_path: state_path.to_path_buf(),
            data_home: data_home.to_path_buf(),
            state: fake_claude_reference_store::load(state_path)?,
        })
    }

    pub(crate) fn retire_runtime(&mut self) -> ContractResult<()> {
        if let Some(state) = &mut self.state {
            state.runtime_active = false;
            state.activity = ActivityState::Unknown;
            fake_claude_reference_store::save(&self.state_path, state)?;
        }
        Ok(())
    }

    pub(crate) fn resume_runtime(
        &mut self,
        frame: &ClaudeHookFrame,
    ) -> ContractResult<HookDisposition> {
        if frame.validate().is_err() {
            return Ok(HookDisposition::Rejected);
        }
        let Some(state) = &mut self.state else {
            return Ok(HookDisposition::Rejected);
        };
        let ClaudeHookInput::SessionStart {
            common,
            source: SessionStartSource::Resume,
        } = &frame.input
        else {
            return Ok(HookDisposition::Rejected);
        };
        let valid = !state.runtime_active
            && state.provider_session_id == common.session_id
            && replacement_invocation(&state.authority, &frame.authority);
        let transcript = TranscriptIdentity::capture(&self.data_home, &common.transcript_path);
        if !valid || transcript.as_ref().ok() != Some(&state.transcript) {
            return Ok(HookDisposition::Rejected);
        }
        state.authority = frame.authority.clone();
        state.runtime_active = true;
        state.activity = ActivityState::Running;
        state.cwd = common.cwd.clone();
        state.generation = state
            .generation
            .checked_add(GENERATION_INCREMENT)
            .ok_or(GENERATION_OVERFLOW)?;
        state.replay_event_ids.clear();
        state.remember(&frame.event_id);
        fake_claude_reference_store::save(&self.state_path, state)?;
        Ok(HookDisposition::Applied)
    }

    fn persist_if_changed(&self, previous: &Option<DurableAuthority>) -> ContractResult<()> {
        if self.state != *previous
            && let Some(state) = &self.state
        {
            fake_claude_reference_store::save(&self.state_path, state)?;
        }
        Ok(())
    }
}

impl ClaudeHookEndpoint for ReferenceEndpoint {
    fn apply(&mut self, frame: &ClaudeHookFrame) -> ContractResult<HookDisposition> {
        let previous = self.state.clone();
        let disposition =
            fake_claude_reference_transition::apply(&mut self.state, &self.data_home, frame)?;
        self.persist_if_changed(&previous)?;
        Ok(disposition)
    }

    fn mark_unavailable(&mut self, _failure: HookFailure) -> ContractResult<()> {
        self.retire_runtime()
    }

    fn view(&self) -> Option<AuthorityView> {
        self.state.as_ref().map(DurableAuthority::view)
    }
}
