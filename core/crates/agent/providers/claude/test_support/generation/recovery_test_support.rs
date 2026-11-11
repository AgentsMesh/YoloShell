use super::super::*;
use super::recovery_event_support::*;
use super::support::*;
use crate::agent::{AgentBinding, AgentBindingStore, AgentKind, DetectionContext};
use crate::claude_hooks::{ClaudeHookCredentials, ClaudeLifecycleEvent};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub(super) struct RecoveryFixture {
    pub base: Fixture,
    pub authority: ClaudeGenerationAuthority,
    pub binding_store: AgentBindingStore,
    pub context: DetectionContext,
    pub credentials: ClaudeHookCredentials,
    pub outbox: PathBuf,
    pub final_cwd: PathBuf,
}

impl RecoveryFixture {
    pub fn new(event_count: usize) -> Self {
        assert!((SESSION_START_COUNT..=ALL_EVENTS_COUNT).contains(&event_count));
        let base = Fixture::new();
        let authority = base
            .store
            .begin_in_namespace(&base.lease, &base.begin)
            .unwrap();
        let final_cwd = base.begin.cwd.join("nested");
        fs::create_dir_all(&final_cwd).unwrap();
        let transcript = base.transcript(&base.begin.provider_session_id);
        fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        write_transcript(&base, &transcript, &final_cwd, event_count);
        let identity =
            crate::agent::claude::durable_transcript_identity(&base.begin.data_home, &transcript)
                .unwrap()
                .unwrap();
        let events = events(&base, &transcript, &final_cwd, event_count);
        let credentials = credentials(&base.begin);
        let settings = base
            .binding_path()
            .parent()
            .unwrap()
            .join(crate::claude_settings_artifact::DIRECTORY_NAME);
        let outbox = crate::claude_hooks::outbox::path(&settings).unwrap();
        for event in &events {
            let evidence = matches!(
                event,
                ClaudeLifecycleEvent::SessionStart { .. } | ClaudeLifecycleEvent::CwdChanged { .. }
            )
            .then_some(identity);
            crate::claude_hooks::outbox::enqueue_for_recovery(
                &outbox,
                credentials,
                event.clone(),
                evidence,
                crate::process::ProcessIdentity {
                    pid: PROCESS_PID,
                    start_time: PROCESS_START,
                },
            )
            .unwrap();
        }
        let ClaudeLifecycleEvent::SessionStart { event_id, .. } = &events[0] else {
            unreachable!();
        };
        crate::claude_hooks::outbox::authenticate_session_start(
            &outbox,
            credentials,
            crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
            *event_id,
            crate::process::ProcessIdentity {
                pid: PROCESS_PID,
                start_time: PROCESS_START,
            },
        )
        .unwrap();
        let binding_store = AgentBindingStore::new(base.binding_path());
        let context = DetectionContext::new(
            base.begin.terminal_id,
            base.begin.cwd.clone(),
            [(AgentKind::ClaudeCode, base.begin.data_home.clone())],
        );
        Self {
            base,
            authority,
            binding_store,
            context,
            credentials,
            outbox,
            final_cwd,
        }
    }

    pub fn gone_guards(
        &self,
    ) -> (
        crate::process::ProcessIdentityRelationGuard,
        crate::process::ProcessIdentityRelationGuard,
    ) {
        let gone = crate::process::ProcessIdentityRelation::Gone;
        (
            crate::process::install_test_process_identity_relation(
                PROCESS_PID,
                PROCESS_START,
                gone,
            ),
            crate::process::install_test_process_identity_relation(
                WORKER_PID,
                WORKER_START,
                gone,
            ),
        )
    }

    pub fn runtime(&self) -> AuthorityRuntimeIdentity {
        AuthorityRuntimeIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: WORKER_PID + 1,
                start_time: WORKER_START + 1,
            },
        }
    }

    pub fn pending(&self) -> crate::claude_hooks::outbox::PendingRecovery {
        crate::claude_hooks::outbox::pending_recovery(&self.outbox, self.credentials)
            .unwrap()
            .unwrap()
    }

    pub fn recovery_events(&self) -> Vec<GenerationRecoveryEvent> {
        self.pending()
            .events
            .iter()
            .map(|event| GenerationRecoveryEvent {
                sequence: event.event_sequence,
                event: from_hook_event(&event.event),
                transcript_identity: event.transcript_identity,
            })
            .collect()
    }

    pub fn load(&self) -> anyhow::Result<Option<AgentBinding>> {
        self.binding_store.load_authoritative_in_namespace(
            &self.base.lease,
            &self.context,
            self.runtime(),
        )
    }
}

fn credentials(begin: &GenerationBegin) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: begin.terminal_id,
        runtime_epoch: begin.runtime_epoch,
        invocation_id: begin.invocation_id,
        hook_token: begin.hook_token,
        helper_capability: begin.helper_capability,
    }
}
