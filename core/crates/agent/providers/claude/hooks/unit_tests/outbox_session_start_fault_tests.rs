use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeSessionStartSource};

const AGENT_PID: u32 = 951;
const AGENT_START: u64 = 952;
const BEFORE_DIGEST_BYTE: u8 = 4;
const AFTER_DIGEST_BYTE: u8 = 5;
const WRITE_FAULT: &str = "injected SessionStart persistence failure";

#[test]
fn prepared_write_failure_cannot_expose_an_unrecorded_intent() {
    let fixture = Fixture::new();
    super::persistence_test_hook::install(|| anyhow::bail!(WRITE_FAULT));

    assert!(fixture.prepare().is_err());
    assert!(!fixture.fenced());
    assert_eq!(fixture.prepare().unwrap(), SessionStartPreparation::Apply);
}

#[test]
fn committed_write_failure_recovers_from_the_exact_applied_authority() {
    let fixture = Fixture::new();
    fixture.prepare().unwrap();
    super::persistence_test_hook::install(|| anyhow::bail!(WRITE_FAULT));

    assert!(fixture.commit().is_err());
    assert!(!fixture.fenced());
    reconcile_prepared_session_starts(&fixture.path, |_| Ok(Some(after()))).unwrap();
    assert!(fixture.fenced());
}

struct Fixture {
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
    credentials: ClaudeHookCredentials,
    event_id: uuid::Uuid,
    agent: crate::process::ProcessIdentity,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("outbox/state.json");
        let transcript = directory.path().join("session.jsonl");
        std::fs::write(&transcript, b"durable transcript").unwrap();
        let identity = crate::paths::identity::ObjectIdentity::from_file(
            &std::fs::File::open(&transcript).unwrap(),
        )
        .unwrap()
        .unwrap();
        let credentials = credentials();
        let agent = agent();
        let event = session_start(transcript);
        let ClaudeLifecycleEvent::SessionStart { event_id, .. } = event else {
            unreachable!();
        };
        enqueue_for_recovery(&path, credentials, event, Some(identity), agent).unwrap();
        Self {
            _directory: directory,
            path,
            credentials,
            event_id,
            agent,
        }
    }

    fn prepare(&self) -> anyhow::Result<SessionStartPreparation> {
        prepare_session_start(
            &self.path,
            self.credentials,
            crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            self.event_id,
            self.agent,
            before(),
            after(),
        )
    }

    fn commit(&self) -> anyhow::Result<SessionStartAcceptance> {
        commit_session_start(
            &self.path,
            self.credentials,
            crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            self.event_id,
            self.agent,
            after(),
        )
    }

    fn fenced(&self) -> bool {
        fence_authenticated_session_start(
            &self.path,
            self.credentials.invocation_id,
            self.agent,
            Ok,
        )
        .unwrap()
    }
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

fn agent() -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: AGENT_PID,
        start_time: AGENT_START,
    }
}

fn session_start(transcript_path: std::path::PathBuf) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionStart {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        source: ClaudeSessionStartSource::Fresh,
        cwd: std::path::PathBuf::from("/tmp/project"),
        transcript_path,
        state: ClaudeHookState::WaitingForInput,
    }
}

fn before() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(BEFORE_DIGEST_BYTE)
}

fn after() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(AFTER_DIGEST_BYTE)
}
