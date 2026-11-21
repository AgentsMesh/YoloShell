use crate::claude_scenario::{ClaudeScenario, HookAvailability, SCENARIO_SCHEMA_VERSION};
use crate::claude_wire::*;
use crate::claude_wire_path::transcript;
use crate::fake_claude_constants::PROCESS_START_BASE;
use std::path::{Path, PathBuf};

pub(crate) const FRESH_SESSION_ID: &str = "11111111-1111-4111-8111-111111111111";
pub(crate) const CLEARED_SESSION_ID: &str = "22222222-2222-4222-8222-222222222222";
pub(crate) const FORKED_SESSION_ID: &str = "33333333-3333-4333-8333-333333333333";
pub(crate) const RESUMED_SESSION_ID: &str = "44444444-4444-4444-8444-444444444444";
const YOLO_SESSION_ID: &str = "5aaaaaaa-5555-4555-8555-555555555555";
const RUNTIME_EPOCH: &str = "6aaaaaaa-6666-4666-8666-666666666666";
const INVOCATION_ID: &str = "7aaaaaaa-7777-4777-8777-777777777777";
const HOOK_TOKEN: &str = "8aaaaaaa-8888-4888-8888-888888888888";
const EVENT_PREFIX: u128 = 0x90000000000040008000000000000000;
const INITIAL_EVENT_OFFSET: u128 = 0;
const EVENT_OFFSET_INCREMENT: u128 = 1;
const PROJECT_COMPONENT: &str = "-project";
const MOVED_CWD_COMPONENT: &str = "nested";
const SUMMARY: &str = "compacted summary";
const PROMPT: &str = "inspect the repository";
const RESPONSE: &str = "done";
const WORKER_PID: u32 = 101;
const SHELL_PID: u32 = 102;
const AGENT_PID: u32 = 103;
const HOOK_PID: u32 = 104;

pub(crate) fn lifecycle(cwd: &Path, data_home: &Path) -> ClaudeScenario {
    let moved = cwd.join(MOVED_CWD_COMPONENT);
    let first = common(FRESH_SESSION_ID, cwd, data_home);
    let cleared = common(CLEARED_SESSION_ID, cwd, data_home);
    let moved_common = common(CLEARED_SESSION_ID, &moved, data_home);
    let forked = common(FORKED_SESSION_ID, &moved, data_home);
    let mut sequence = FrameSequence::new();
    let mut frames = vec![
        sequence.frame(start(first.clone(), SessionStartSource::Startup)),
        sequence.frame(prompt(first.clone())),
        sequence.frame(stop(first.clone())),
        sequence.frame(end(first.clone(), SessionEndReason::Clear)),
        sequence.frame(start(cleared.clone(), SessionStartSource::Clear)),
        sequence.frame(pre_compact(cleared.clone())),
        sequence.frame(start(cleared.clone(), SessionStartSource::Compact)),
        sequence.frame(post_compact(cleared.clone())),
        sequence.frame(cwd_changed(moved_common.clone(), cwd)),
        sequence.frame(end(moved_common.clone(), SessionEndReason::Other)),
        sequence.frame(start(forked, SessionStartSource::Fork)),
    ];
    frames.push(frames.last().unwrap().clone());
    frames.push(sequence.frame(end(first, SessionEndReason::Other)));
    frames.push(sequence.frame(stop(moved_common)));
    scenario(HookAvailability::Enabled, frames)
}

pub(crate) fn resume(cwd: &Path, data_home: &Path) -> ClaudeScenario {
    let mut sequence = FrameSequence::new();
    scenario(
        HookAvailability::Enabled,
        vec![sequence.frame(start(
            common(RESUMED_SESSION_ID, cwd, data_home),
            SessionStartSource::Resume,
        ))],
    )
}

pub(crate) fn suppressed(availability: HookAvailability) -> ClaudeScenario {
    scenario(availability, Vec::new())
}

pub(crate) fn error(cwd: &Path, data_home: &Path) -> ClaudeScenario {
    let mut value = resume(cwd, data_home);
    value.hook_availability = HookAvailability::Error;
    value
}

fn scenario(availability: HookAvailability, frames: Vec<ClaudeHookFrame>) -> ClaudeScenario {
    ClaudeScenario {
        schema_version: SCENARIO_SCHEMA_VERSION,
        hook_availability: availability,
        frames,
    }
}

fn start(common: CommonHookInput, source: SessionStartSource) -> ClaudeHookInput {
    ClaudeHookInput::SessionStart { common, source }
}

struct FrameSequence {
    next: u128,
}

impl FrameSequence {
    fn new() -> Self {
        Self {
            next: INITIAL_EVENT_OFFSET,
        }
    }

    fn frame(&mut self, input: ClaudeHookInput) -> ClaudeHookFrame {
        let event_id = uuid::Uuid::from_u128(EVENT_PREFIX + self.next)
            .hyphenated()
            .to_string();
        self.next += EVENT_OFFSET_INCREMENT;
        ClaudeHookFrame {
            schema_version: WIRE_SCHEMA_VERSION,
            event_id,
            authority: authority(),
            input,
        }
    }
}

fn authority() -> HookAuthority {
    HookAuthority {
        terminal_id: YOLO_SESSION_ID.into(),
        runtime_epoch: RUNTIME_EPOCH.into(),
        invocation_id: INVOCATION_ID.into(),
        hook_token: HOOK_TOKEN.into(),
        worker: identity(WORKER_PID, None),
        shell: identity(SHELL_PID, Some(WORKER_PID)),
        agent: identity(AGENT_PID, Some(SHELL_PID)),
        hook: identity(HOOK_PID, Some(AGENT_PID)),
    }
}

fn identity(pid: u32, parent_pid: Option<u32>) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time: PROCESS_START_BASE + u64::from(pid),
        parent_pid,
    }
}

fn common(session_id: &str, cwd: &Path, data_home: &Path) -> CommonHookInput {
    CommonHookInput {
        session_id: session_id.into(),
        transcript_path: transcript(data_home, PROJECT_COMPONENT, session_id),
        cwd: cwd.into(),
    }
}

fn prompt(common: CommonHookInput) -> ClaudeHookInput {
    ClaudeHookInput::UserPromptSubmit {
        common,
        prompt: PROMPT.into(),
    }
}

fn stop(common: CommonHookInput) -> ClaudeHookInput {
    ClaudeHookInput::Stop {
        common,
        stop_hook_active: false,
        last_assistant_message: RESPONSE.into(),
    }
}

fn end(common: CommonHookInput, reason: SessionEndReason) -> ClaudeHookInput {
    ClaudeHookInput::SessionEnd { common, reason }
}

fn pre_compact(common: CommonHookInput) -> ClaudeHookInput {
    ClaudeHookInput::PreCompact {
        common,
        trigger: CompactTrigger::Manual,
        custom_instructions: String::new(),
    }
}

fn post_compact(common: CommonHookInput) -> ClaudeHookInput {
    ClaudeHookInput::PostCompact {
        common,
        trigger: CompactTrigger::Manual,
        compact_summary: SUMMARY.into(),
    }
}

fn cwd_changed(common: CommonHookInput, old_cwd: &Path) -> ClaudeHookInput {
    ClaudeHookInput::CwdChanged {
        new_cwd: common.cwd.clone(),
        common,
        old_cwd: PathBuf::from(old_cwd),
    }
}
