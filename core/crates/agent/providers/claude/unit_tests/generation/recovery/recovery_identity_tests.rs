use super::support::*;
use crate::agent::{AgentBindingStore, AgentKind, AgentState, DetectionContext};
use crate::claude_hooks::{
    ClaudeHookCredentials, ClaudeHookState, ClaudeLifecycleEvent, ClaudeSessionStartSource,
};
use std::fs;
use uuid::Uuid;

#[test]
fn recovery_accepts_a_valid_session_transition_to_a_new_transcript_identity() {
    let base = Fixture::new();
    let authority = base
        .store
        .begin_in_namespace(&base.lease, &base.begin)
        .unwrap();
    let first_session = base.begin.provider_session_id.clone();
    let second_session = session_id();
    let first_path = base.transcript(&first_session);
    let second_path = base.transcript(&second_session);
    write_waiting_transcript(&first_path, &first_session, &base.begin.cwd);
    write_waiting_transcript(&second_path, &second_session, &base.begin.cwd);
    let first_identity = identity(&base, &first_path);
    let second_identity = identity(&base, &second_path);
    assert_ne!(first_identity, second_identity);
    let credentials = credentials(&base);
    let settings = base
        .binding_path()
        .parent()
        .unwrap()
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    let outbox = crate::claude_hooks::outbox::path(&settings).unwrap();
    let queued = [
        (
            start(
                &base,
                first_session,
                first_path,
                ClaudeSessionStartSource::Fresh,
            ),
            first_identity,
        ),
        (
            start(
                &base,
                second_session.clone(),
                second_path.clone(),
                ClaudeSessionStartSource::Clear,
            ),
            second_identity,
        ),
    ];
    let first_event_id = match &queued[0].0 {
        ClaudeLifecycleEvent::SessionStart { event_id, .. } => *event_id,
        _ => unreachable!(),
    };
    for (event, identity) in queued {
        crate::claude_hooks::outbox::enqueue_for_recovery(
            &outbox,
            credentials,
            event,
            Some(identity),
            crate::process::ProcessIdentity {
                pid: PROCESS_PID,
                start_time: PROCESS_START,
            },
        )
        .unwrap();
    }
    crate::claude_hooks::outbox::authenticate_session_start(
        &outbox,
        credentials,
        crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
        first_event_id,
        crate::process::ProcessIdentity {
            pid: PROCESS_PID,
            start_time: PROCESS_START,
        },
    )
    .unwrap();
    let _process = gone(PROCESS_PID, PROCESS_START);
    let _worker = gone(WORKER_PID, WORKER_START);
    let binding_store = AgentBindingStore::new(base.binding_path());
    let context = DetectionContext::new(
        base.begin.terminal_id,
        base.begin.cwd.clone(),
        [(AgentKind::ClaudeCode, base.begin.data_home.clone())],
    );
    let mut runtime = base.runtime();
    runtime.runtime_epoch = Uuid::new_v4();
    let recovered = binding_store
        .load_authoritative_in_namespace(&base.lease, &context, runtime)
        .unwrap()
        .unwrap();
    assert_eq!(recovered.provider_session_id, second_session);
    assert_eq!(recovered.evidence_path, second_path.to_string_lossy());
    assert_eq!(recovered.state, AgentState::WaitingForInput);
    let committed = base.store.load_in_namespace(&base.lease).unwrap().unwrap();
    assert_eq!(committed.generation, authority.generation + 1);
    assert_eq!(committed.transcript_identity, Some(second_identity));
}

fn start(
    base: &Fixture,
    session: String,
    transcript_path: std::path::PathBuf,
    source: ClaudeSessionStartSource,
) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionStart {
        event_id: Uuid::new_v4(),
        provider_session_id: session,
        source,
        cwd: base.begin.cwd.clone(),
        transcript_path,
        state: ClaudeHookState::WaitingForInput,
    }
}

fn write_waiting_transcript(path: &std::path::Path, session: &str, cwd: &std::path::Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let records = [
        serde_json::json!({"type": "user", "sessionId": session, "cwd": cwd}),
        serde_json::json!({
            "type": "system",
            "subtype": crate::agent::claude::constants::TURN_DURATION_SUBTYPE,
        }),
    ];
    let value = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, value).unwrap();
}

fn identity(base: &Fixture, path: &std::path::Path) -> crate::paths::identity::ObjectIdentity {
    crate::agent::claude::durable_transcript_identity(&base.begin.data_home, path)
        .unwrap()
        .unwrap()
}

fn credentials(base: &Fixture) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: base.begin.terminal_id,
        runtime_epoch: base.begin.runtime_epoch,
        invocation_id: base.begin.invocation_id,
        hook_token: base.begin.hook_token,
        helper_capability: base.begin.helper_capability,
    }
}

fn gone(pid: u32, start_time: u64) -> crate::process::ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(
        pid,
        start_time,
        crate::process::ProcessIdentityRelation::Gone,
    )
}
