use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeSessionStartSource};

const AGENT_PID: u32 = 941;
const AGENT_START: u64 = 942;
const BEFORE_DIGEST_BYTE: u8 = 1;
const AFTER_DIGEST_BYTE: u8 = 2;
const CHANGED_DIGEST_BYTE: u8 = 3;

#[path = "outbox_session_start_replay_tests.rs"]
mod replay_tests;

#[test]
fn crash_after_prepared_rolls_back_against_the_exact_before_authority() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.prepare(before(), after()).unwrap(),
        SessionStartPreparation::Apply
    );
    assert!(!fixture.fenced());

    reconcile_prepared_session_starts(&fixture.path, |_| Ok(Some(before()))).unwrap();

    assert_eq!(
        fixture.prepare(before(), changed()).unwrap(),
        SessionStartPreparation::Apply
    );
    assert!(!fixture.fenced());
}

#[test]
fn crash_after_authority_cas_commits_only_the_exact_after_authority() {
    let fixture = Fixture::new();
    fixture.prepare(before(), after()).unwrap();

    reconcile_prepared_session_starts(&fixture.path, |_| Ok(Some(after()))).unwrap();

    assert!(fixture.fenced());
    assert_eq!(
        fixture.prepare(after(), after()).unwrap(),
        SessionStartPreparation::Committed
    );
}

#[test]
fn indeterminate_authority_keeps_prepared_state_blocked() {
    let fixture = Fixture::new();
    fixture.prepare(before(), after()).unwrap();

    reconcile_prepared_session_starts(&fixture.path, |_| Ok(Some(changed()))).unwrap();

    assert!(!fixture.fenced());
    assert!(fixture.prepare(changed(), changed()).is_err());
}

#[test]
fn committed_persistence_is_replayable_and_watchdog_visible() {
    let fixture = Fixture::new();
    fixture.prepare(before(), after()).unwrap();
    assert_eq!(
        fixture.commit(after()).unwrap(),
        SessionStartAcceptance::Recorded
    );

    assert_eq!(
        fixture.commit(after()).unwrap(),
        SessionStartAcceptance::Replay
    );
    assert!(fixture.fenced());
}

struct Fixture {
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
    credentials: ClaudeHookCredentials,
    event_sequence: u64,
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
            event_sequence: crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            event_id,
            agent,
        }
    }

    fn prepare(
        &self,
        observed: crate::sha256_digest::Sha256Digest,
        planned: crate::sha256_digest::Sha256Digest,
    ) -> anyhow::Result<SessionStartPreparation> {
        prepare_session_start(
            &self.path,
            self.credentials,
            self.event_sequence,
            self.event_id,
            self.agent,
            observed,
            planned,
        )
    }

    fn commit(
        &self,
        observed: crate::sha256_digest::Sha256Digest,
    ) -> anyhow::Result<SessionStartAcceptance> {
        commit_session_start(
            &self.path,
            self.credentials,
            self.event_sequence,
            self.event_id,
            self.agent,
            observed,
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

fn changed() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(CHANGED_DIGEST_BYTE)
}
