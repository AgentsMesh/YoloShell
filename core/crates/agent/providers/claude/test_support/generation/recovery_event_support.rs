use super::support::Fixture;
use crate::agent::AgentState;
use crate::claude_hooks::{ClaudeHookState, ClaudeLifecycleEvent, ClaudeSessionStartSource};
use std::fs;
use uuid::Uuid;

pub(super) const SESSION_START_COUNT: usize = 1;
pub(super) const RUNNING_COUNT: usize = 2;
pub(super) const CWD_CHANGED_COUNT: usize = 3;
pub(super) const ALL_EVENTS_COUNT: usize = 4;

pub(super) fn events(
    base: &Fixture,
    transcript: &std::path::Path,
    final_cwd: &std::path::Path,
    count: usize,
) -> Vec<ClaudeLifecycleEvent> {
    let session = &base.begin.provider_session_id;
    let mut events = vec![
        ClaudeLifecycleEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: session.clone(),
            source: ClaudeSessionStartSource::Fresh,
            cwd: base.begin.cwd.clone(),
            transcript_path: transcript.to_path_buf(),
            state: ClaudeHookState::WaitingForInput,
        },
        ClaudeLifecycleEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: session.clone(),
            state: ClaudeHookState::Running,
        },
        ClaudeLifecycleEvent::CwdChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: session.clone(),
            cwd: final_cwd.to_path_buf(),
            transcript_path: transcript.to_path_buf(),
        },
        ClaudeLifecycleEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: session.clone(),
            state: ClaudeHookState::WaitingForInput,
        },
    ];
    events.truncate(count);
    events
}

pub(super) fn write_transcript(
    base: &Fixture,
    path: &std::path::Path,
    cwd: &std::path::Path,
    count: usize,
) {
    let session = &base.begin.provider_session_id;
    let mut records = vec![serde_json::json!({
        "type": "user",
        "sessionId": session,
        "cwd": base.begin.cwd,
    })];
    if count >= CWD_CHANGED_COUNT {
        records.push(serde_json::json!({"type": "user", "sessionId": session, "cwd": cwd}));
    }
    if count == SESSION_START_COUNT || count == ALL_EVENTS_COUNT {
        records.push(serde_json::json!({
            "type": "system",
            "subtype": crate::agent::claude::constants::TURN_DURATION_SUBTYPE,
        }));
    }
    let contents = records
        .into_iter()
        .map(|record| serde_json::to_string(&record).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, contents).unwrap();
}

pub(super) fn expected_state(count: usize) -> AgentState {
    if count == RUNNING_COUNT || count == CWD_CHANGED_COUNT {
        AgentState::Running
    } else {
        AgentState::WaitingForInput
    }
}
