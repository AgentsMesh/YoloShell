use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeSessionStartSource};
use std::sync::atomic::{AtomicBool, Ordering};

const AGENT_PID: u32 = 811;
const AGENT_START: u64 = 812;

#[test]
fn worker_authenticated_session_start_wins_and_remains_replayable() {
    let directory = tempfile::tempdir().unwrap();
    let outbox = directory.path().join("outbox.json");
    let transcript = directory.path().join("session.jsonl");
    std::fs::write(&transcript, b"durable transcript").unwrap();
    let identity = crate::paths::identity::ObjectIdentity::from_file(
        &std::fs::File::open(&transcript).unwrap(),
    )
    .unwrap()
    .unwrap();
    let credentials = credentials();
    let agent = crate::process::ProcessIdentity {
        pid: AGENT_PID,
        start_time: AGENT_START,
    };
    let event = session_start(transcript);
    let ClaudeLifecycleEvent::SessionStart { event_id, .. } = event else {
        unreachable!();
    };
    enqueue_for_recovery(&outbox, credentials, event, Some(identity), agent).unwrap();
    assert!(!fenced(&outbox, credentials, agent));
    let gone = crate::process::install_test_process_identity_relation(
        agent.pid,
        agent.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    );
    assert!(matches!(
        probe_recovery(&outbox, credentials).unwrap(),
        RecoveryProbe::Blocked
    ));
    drop(gone);
    assert_eq!(
        authenticate_session_start(
            &outbox,
            credentials,
            crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            event_id,
            agent,
        )
        .unwrap(),
        SessionStartAcceptance::Recorded
    );

    let artifact_pending = AtomicBool::new(true);
    fence_authenticated_session_start(&outbox, credentials.invocation_id, agent, |accepted| {
        assert!(accepted);
        artifact_pending.store(false, Ordering::SeqCst);
        Ok(())
    })
    .unwrap();
    assert!(!artifact_pending.load(Ordering::SeqCst));

    let _guard = crate::process::install_test_process_identity_relation(
        agent.pid,
        agent.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    );
    assert!(matches!(
        probe_recovery(&outbox, credentials).unwrap(),
        RecoveryProbe::Pending(pending)
            if pending.agent == agent && pending.events.len() == 1
    ));
}

fn fenced(
    outbox: &std::path::Path,
    credentials: ClaudeHookCredentials,
    agent: crate::process::ProcessIdentity,
) -> bool {
    fence_authenticated_session_start(outbox, credentials.invocation_id, agent, Ok).unwrap()
}

fn credentials() -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        invocation_id: uuid::Uuid::new_v4(),
        hook_token: uuid::Uuid::new_v4(),
        helper_capability: uuid::Uuid::new_v4(),
    }
}

fn session_start(transcript_path: PathBuf) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionStart {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        source: ClaudeSessionStartSource::Fresh,
        cwd: std::path::PathBuf::from("/tmp/project"),
        transcript_path,
        state: ClaudeHookState::WaitingForInput,
    }
}
