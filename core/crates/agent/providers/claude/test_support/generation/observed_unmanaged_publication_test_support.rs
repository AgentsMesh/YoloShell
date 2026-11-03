use super::super::ClaudeGenerationAuthority;
use super::support::{Fixture, binding, session_id};
use crate::agent::{
    AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState, ObservedProcess,
};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) const ABSOLUTE_CLAUDE: &str = "/managed/agent-bin/claude";
pub(super) const ABSOLUTE_NODE: &str = "/usr/local/bin/node";
pub(super) const CLAUDE_ENTRYPOINT: &str = "/pkg/@anthropic-ai/claude-code/cli.js";
pub(super) const FOREIGN_ENTRYPOINT: &str = "/pkg/@anthropic-ai/other/cli.js";
const DETECTED_AT_MS: i64 = 7;
const FOREIGN_CWD: &str = "/foreign";
const FOREIGN_EXECUTABLE: &str = "/usr/bin/other";
const FOREIGN_SUFFIX: &str = "-foreign";
const IDENTITY_DELTA: u64 = 1;
const PARENT_PID: u32 = 1;
const PROCESS_GROUP: i32 = 41;
const UPDATED_AT_MS: i64 = 8;

pub(super) fn observed(
    authority: &ClaudeGenerationAuthority,
    executable: &str,
    argv: Vec<String>,
    transcript: std::path::PathBuf,
) -> ObservedProcess {
    ObservedProcess {
        pid: authority.process.pid,
        start_time: authority.process.start_time,
        parent_pid: PARENT_PID,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: executable.to_owned(),
        argv,
        environment: Some(BTreeMap::new()),
        cwd: Some(authority.cwd.clone()),
        evidence_paths: Some(vec![transcript]),
    }
}

pub(super) fn scanned(
    authority: &ClaudeGenerationAuthority,
    process: &ObservedProcess,
) -> AgentBinding {
    let mut scanned = binding(authority);
    scanned.executable = process.executable.clone();
    scanned.state = AgentState::Unknown;
    scanned.recovery_availability = AgentRecoveryAvailability::UnmanagedLaunch;
    scanned.detected_at_ms = DETECTED_AT_MS;
    scanned.updated_at_ms = UPDATED_AT_MS;
    scanned
}

pub(super) type BindingMutation = fn(&mut ObservedProcess, &mut AgentBinding);

pub(super) fn binding_mutations() -> Vec<BindingMutation> {
    vec![
        |_, value| value.terminal_id = uuid::Uuid::new_v4(),
        |_, value| value.kind = AgentKind::Codex,
        |_, value| value.provider_session_id = session_id(),
        |_, value| value.cwd.push_str(FOREIGN_SUFFIX),
        |_, value| value.data_home.push_str(FOREIGN_SUFFIX),
        |_, value| value.evidence_path.push_str(FOREIGN_SUFFIX),
        |_, value| value.process_pid = Some(value.process_pid.unwrap() + IDENTITY_DELTA as u32),
        |_, value| {
            value.process_start_time = Some(value.process_start_time.unwrap() + IDENTITY_DELTA)
        },
        |_, value| value.executable.push_str(FOREIGN_SUFFIX),
        |_, value| value.state = AgentState::Running,
        |_, value| value.state = AgentState::WaitingForInput,
        |_, value| value.state = AgentState::Exited,
        |_, value| value.recovery_availability = AgentRecoveryAvailability::Available,
        |_, value| value.recovery_availability = AgentRecoveryAvailability::Unavailable,
    ]
}

pub(super) type ProcessMutation = fn(&ClaudeGenerationAuthority, &mut ObservedProcess);

pub(super) fn process_mutations() -> Vec<ProcessMutation> {
    vec![
        |_, value| value.pid += IDENTITY_DELTA as u32,
        |_, value| value.start_time += IDENTITY_DELTA,
        |_, value| value.cwd = Some(Path::new(FOREIGN_CWD).to_path_buf()),
        |_, value| value.foreground_group += IDENTITY_DELTA as i32,
        |_, value| value.process_group += IDENTITY_DELTA as i32,
        |_, value| value.executable = FOREIGN_EXECUTABLE.to_owned(),
        |authority, value| {
            value.executable = ABSOLUTE_NODE.to_owned();
            value.argv = vec![ABSOLUTE_NODE.to_owned(), FOREIGN_ENTRYPOINT.to_owned()];
            value.evidence_paths = authority.transcript_path.clone().map(|path| vec![path]);
        },
    ]
}

pub(super) fn event_identity(
    fixture: &Fixture,
    event: &super::super::GenerationEvent,
) -> crate::paths::identity::ObjectIdentity {
    let super::super::GenerationEvent::SessionStart {
        transcript_path, ..
    } = event
    else {
        unreachable!();
    };
    crate::agent::claude::durable_transcript_identity(&fixture.begin.data_home, transcript_path)
        .unwrap()
        .unwrap()
}
