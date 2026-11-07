use crate::claude_wire::{ClaudeHookFrame, ClaudeHookInput, HookAuthority, SessionStartSource};
use crate::fake_claude_authority_contract::{ActivityState, ContractResult, HookDisposition};
use crate::fake_claude_reference_types::{
    DurableAuthority, ENDPOINT_SCHEMA_VERSION, GENERATION_INCREMENT, INITIAL_GENERATION,
};
use crate::fake_claude_secure_transcript::TranscriptIdentity;
use std::collections::VecDeque;
use std::path::Path;

const GENERATION_OVERFLOW: &str = "fake Claude generation overflow";

pub(crate) fn apply(
    current: &mut Option<DurableAuthority>,
    data_home: &Path,
    frame: &ClaudeHookFrame,
) -> ContractResult<HookDisposition> {
    if frame.validate().is_err() {
        return Ok(HookDisposition::Rejected);
    }
    let Some(authority) = current.as_mut() else {
        return begin(current, data_home, frame);
    };
    if !authority.runtime_active || !same_invocation(&authority.authority, &frame.authority) {
        return Ok(HookDisposition::Rejected);
    }
    if authority.replay_event_ids.contains(&frame.event_id) {
        return Ok(HookDisposition::Replay);
    }
    if let ClaudeHookInput::SessionStart { common, source, .. } = &frame.input {
        return apply_start(authority, data_home, frame, common, *source);
    }
    let common = frame.input.common();
    if common.session_id != authority.provider_session_id {
        authority.remember(&frame.event_id);
        return Ok(HookDisposition::Stale);
    }
    if common.transcript_path != authority.transcript.path
        || !authority.transcript.still_matches(data_home)
    {
        return Ok(HookDisposition::Rejected);
    }
    match &frame.input {
        ClaudeHookInput::SessionEnd { .. } => authority.activity = ActivityState::Exited,
        ClaudeHookInput::UserPromptSubmit { .. } => authority.activity = ActivityState::Running,
        ClaudeHookInput::Stop { .. } => authority.activity = ActivityState::WaitingForInput,
        ClaudeHookInput::CwdChanged { common, .. } => authority.cwd = common.cwd.clone(),
        ClaudeHookInput::PreCompact { .. } | ClaudeHookInput::PostCompact { .. } => {}
        ClaudeHookInput::SessionStart { .. } => unreachable!(),
    }
    authority.remember(&frame.event_id);
    Ok(HookDisposition::Applied)
}

fn begin(
    current: &mut Option<DurableAuthority>,
    data_home: &Path,
    frame: &ClaudeHookFrame,
) -> ContractResult<HookDisposition> {
    let ClaudeHookInput::SessionStart { common, source, .. } = &frame.input else {
        return Ok(HookDisposition::Rejected);
    };
    if !matches!(
        source,
        SessionStartSource::Startup | SessionStartSource::Resume
    ) {
        return Ok(HookDisposition::Rejected);
    }
    let transcript = match TranscriptIdentity::capture(data_home, &common.transcript_path) {
        Ok(value) => value,
        Err(_) => return Ok(HookDisposition::Rejected),
    };
    *current = Some(DurableAuthority {
        schema_version: ENDPOINT_SCHEMA_VERSION,
        authority: frame.authority.clone(),
        provider_session_id: common.session_id.clone(),
        cwd: common.cwd.clone(),
        transcript,
        generation: INITIAL_GENERATION,
        activity: ActivityState::Running,
        runtime_active: true,
        replay_event_ids: VecDeque::from([frame.event_id.clone()]),
    });
    Ok(HookDisposition::Applied)
}

fn apply_start(
    authority: &mut DurableAuthority,
    data_home: &Path,
    frame: &ClaudeHookFrame,
    common: &crate::claude_wire::CommonHookInput,
    source: SessionStartSource,
) -> ContractResult<HookDisposition> {
    let same_session = common.session_id == authority.provider_session_id;
    let allowed = if same_session {
        source == SessionStartSource::Compact
            && matches!(
                authority.activity,
                ActivityState::Running | ActivityState::WaitingForInput
            )
    } else {
        matches!(source, SessionStartSource::Clear | SessionStartSource::Fork)
            && authority.activity == ActivityState::Exited
    };
    if !allowed {
        authority.activity = ActivityState::Unknown;
        authority.runtime_active = false;
        authority.remember(&frame.event_id);
        return Ok(HookDisposition::Invalidated);
    }
    let transcript = match TranscriptIdentity::capture(data_home, &common.transcript_path) {
        Ok(value) => value,
        Err(_) => return Ok(HookDisposition::Rejected),
    };
    if !same_session {
        authority.generation = authority
            .generation
            .checked_add(GENERATION_INCREMENT)
            .ok_or(GENERATION_OVERFLOW)?;
        authority.provider_session_id = common.session_id.clone();
    }
    authority.transcript = transcript;
    authority.cwd = common.cwd.clone();
    authority.activity = ActivityState::Running;
    authority.remember(&frame.event_id);
    Ok(HookDisposition::Applied)
}

pub(crate) fn same_invocation(expected: &HookAuthority, actual: &HookAuthority) -> bool {
    expected.terminal_id == actual.terminal_id
        && expected.runtime_epoch == actual.runtime_epoch
        && expected.invocation_id == actual.invocation_id
        && expected.hook_token == actual.hook_token
        && expected.worker == actual.worker
        && expected.shell == actual.shell
        && expected.agent == actual.agent
}

pub(crate) fn replacement_invocation(expected: &HookAuthority, actual: &HookAuthority) -> bool {
    expected.terminal_id == actual.terminal_id
        && expected.runtime_epoch != actual.runtime_epoch
        && expected.invocation_id != actual.invocation_id
        && expected.hook_token != actual.hook_token
        && expected.worker != actual.worker
        && expected.shell != actual.shell
        && expected.agent != actual.agent
}
