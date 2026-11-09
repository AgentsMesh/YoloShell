use super::*;
use crate::claude_hooks::{CLAUDE_HOOK_OUTBOX_DIRECTORY, CLAUDE_HOOK_OUTBOX_FILE};
use crate::paths::identity::ObjectIdentity;
use crate::process::ProcessIdentity;

pub(super) type Case = (&'static str, fn(&mut OutboxState));

pub(super) struct Fixture {
    directory: tempfile::TempDir,
    pub path: PathBuf,
    pub credentials: ClaudeHookCredentials,
    pub agent: ProcessIdentity,
    pub delivery: Delivery,
    pub accepted: AuthenticatedSessionStart,
    pub identity: ObjectIdentity,
    pub session: String,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join(CLAUDE_HOOK_OUTBOX_DIRECTORY)
            .join(CLAUDE_HOOK_OUTBOX_FILE);
        let transcript = directory.path().join(TRANSCRIPT_FILE);
        std::fs::write(&transcript, TRANSCRIPT_BYTES).unwrap();
        std::fs::create_dir(directory.path().join(NESTED_DIRECTORY)).unwrap();
        let identity = ObjectIdentity::from_file(&std::fs::File::open(&transcript).unwrap())
            .unwrap()
            .unwrap();
        let credentials = events::credentials();
        let agent = ProcessIdentity::capture(std::process::id()).unwrap();
        let session = Uuid::new_v4().to_string();
        let start = events::session_start(directory.path(), &transcript, &session);
        let ClaudeLifecycleEvent::SessionStart { event_id, .. } = start else {
            unreachable!()
        };
        for (event, evidence) in [
            (start, Some(identity)),
            (
                events::cwd_changed(directory.path(), &transcript, &session),
                Some(identity),
            ),
            (events::state_changed(&session), None),
        ] {
            enqueue_for_recovery(&path, credentials, event, evidence, agent).unwrap();
        }
        let delivery = enqueue(&path, credentials, events::session_end(&session))
            .unwrap()
            .unwrap();
        assert!(next(&path, delivery).unwrap().is_some());
        let accepted = AuthenticatedSessionStart {
            event_sequence: crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            event_id,
        };
        Self {
            directory,
            path,
            credentials,
            agent,
            delivery,
            accepted,
            identity,
            session,
        }
    }

    pub fn snapshot(&self) -> OutboxState {
        persistence::read(&self.path, |state| Ok(state.clone())).unwrap()
    }

    pub fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap()
    }

    pub fn prepare(&self) -> anyhow::Result<SessionStartPreparation> {
        prepare_session_start(
            &self.path,
            self.credentials,
            self.accepted.event_sequence,
            self.accepted.event_id,
            self.agent,
            events::before(),
            events::after(),
        )
    }

    pub fn commit(&self) -> anyhow::Result<SessionStartAcceptance> {
        commit_session_start(
            &self.path,
            self.credentials,
            self.accepted.event_sequence,
            self.accepted.event_id,
            self.agent,
            events::after(),
        )
    }

    pub fn corrupt_cases(&self, original: &OutboxState, cases: &[Case]) {
        for (label, mutate) in cases {
            let mut damaged = original.clone();
            mutate(&mut damaged);
            self.reject_corruption(label, &damaged);
        }
    }

    pub fn append_start(&mut self) -> AuthenticatedSessionStart {
        let accepted_sequence = self.snapshot().invocations[FIRST_QUEUE].next_sequence;
        let start = events::session_start(
            self.directory.path(),
            &self.directory.path().join(TRANSCRIPT_FILE),
            &self.session,
        );
        let ClaudeLifecycleEvent::SessionStart { event_id, .. } = start else {
            unreachable!()
        };
        enqueue_for_recovery(
            &self.path,
            self.credentials,
            start,
            Some(self.identity),
            self.agent,
        )
        .unwrap();
        self.delivery = enqueue(
            &self.path,
            self.credentials,
            events::session_end(&self.session),
        )
        .unwrap()
        .unwrap();
        AuthenticatedSessionStart {
            event_sequence: accepted_sequence,
            event_id,
        }
    }

    pub fn add_failed_handshake(&self) {
        let stale_agent = ProcessIdentity {
            pid: self.agent.pid,
            start_time: self.agent.start_time.checked_sub(STEP).unwrap(),
        };
        enqueue_for_recovery(
            &self.path,
            events::credentials(),
            events::state_changed(&self.session),
            None,
            stale_agent,
        )
        .unwrap();
        enqueue(
            &self.path,
            self.credentials,
            events::state_changed(&self.session),
        )
        .unwrap();
        assert_eq!(self.snapshot().failed_handshakes.len(), COUNT_STEP);
    }
}
