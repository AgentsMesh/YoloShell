use super::invocation_set_tests::{ParkedPair, parked_pair};
use super::support::{Fixture, binding};
use crate::agent::{AgentBinding, AgentBindingStore, AgentKind, DetectionContext};
use crate::claude_hooks::{ClaudeHookCredentials, ClaudeHookState, ClaudeLifecycleEvent};
use std::path::PathBuf;

const EXISTING_OUTBOX_QUEUES: usize = 1;

#[path = "recovery_orphan_event_support.rs"]
mod event_support;
use event_support::hook_event;

pub(super) struct ParkedRecoveryFixture {
    pub base: Fixture,
    pub pair: ParkedPair,
    pub binding_store: AgentBindingStore,
    pub context: DetectionContext,
    pub outbox: PathBuf,
    pub credentials: ClaudeHookCredentials,
}

impl ParkedRecoveryFixture {
    pub fn new() -> Self {
        let base = Fixture::new();
        let pair = parked_pair(&base);
        let binding_store = AgentBindingStore::new(base.binding_path());
        let context = DetectionContext::new(
            base.begin.terminal_id,
            base.begin.cwd.clone(),
            [(AgentKind::ClaudeCode, base.begin.data_home.clone())],
        );
        let binding = binding(&pair.second);
        binding_store
            .save_authoritative_in_namespace(&base.lease, &binding, &pair.second, base.runtime())
            .unwrap();
        let settings = base
            .binding_path()
            .parent()
            .unwrap()
            .join(crate::claude_settings_artifact::DIRECTORY_NAME);
        let outbox = crate::claude_hooks::outbox::path(&settings).unwrap();
        let credentials = credentials(&pair.first);
        crate::claude_hooks::outbox::enqueue_for_recovery(
            &outbox,
            credentials,
            hook_event(&pair.first_event),
            pair.first.transcript_identity,
            process(&pair.first),
        )
        .unwrap();
        authenticate(&outbox, credentials, &pair);
        Self {
            base,
            pair,
            binding_store,
            context,
            outbox,
            credentials,
        }
    }

    pub fn gone_guards(&self) -> Vec<crate::process::ProcessIdentityRelationGuard> {
        [
            self.pair.first.worker,
            self.pair.first.process,
            self.pair.second.process,
        ]
        .into_iter()
        .map(|identity| {
            crate::process::install_test_process_identity_relation(
                identity.pid,
                identity.start_time,
                crate::process::ProcessIdentityRelation::Gone,
            )
        })
        .collect()
    }

    pub fn load(&self) -> anyhow::Result<Option<AgentBinding>> {
        self.binding_store.load_authoritative_in_namespace(
            &self.base.lease,
            &self.context,
            super::super::AuthorityRuntimeIdentity {
                runtime_epoch: uuid::Uuid::new_v4(),
                worker: self.pair.first.worker,
            },
        )
    }

    pub fn begin(
        &self,
        begin: &super::super::GenerationBegin,
    ) -> anyhow::Result<super::super::ClaudeGenerationAuthority> {
        self.binding_store
            .begin_managed_claude_in_namespace(&self.base.lease, begin, false)
    }

    pub fn acknowledge_without_retire(&self) {
        let pending = crate::claude_hooks::outbox::pending_recovery(&self.outbox, self.credentials)
            .unwrap()
            .unwrap();
        crate::claude_hooks::outbox::acknowledge_recovery(&self.outbox, self.credentials, &pending)
            .unwrap();
    }

    pub fn fill_outbox_capacity(&self) {
        for _ in EXISTING_OUTBOX_QUEUES..crate::claude_hooks::MAX_CLAUDE_OUTBOX_INVOCATIONS {
            crate::claude_hooks::outbox::enqueue_for_recovery(
                &self.outbox,
                credentials_with_invocation(&self.pair.first, uuid::Uuid::new_v4()),
                state_event(),
                None,
                process(&self.pair.first),
            )
            .unwrap();
        }
    }

    pub fn reuse_outbox_capacity(&self) {
        crate::claude_hooks::outbox::enqueue(
            &self.outbox,
            credentials_with_invocation(&self.pair.first, uuid::Uuid::new_v4()),
            state_event(),
        )
        .unwrap();
    }

    pub fn active_binding(&self) -> AgentBinding {
        binding(&self.pair.second)
    }
}

fn credentials(authority: &super::super::ClaudeGenerationAuthority) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: authority.terminal_id,
        runtime_epoch: authority.runtime_epoch,
        invocation_id: authority.invocation_id,
        hook_token: authority.hook_token,
        helper_capability: authority.helper_capability,
    }
}

fn credentials_with_invocation(
    authority: &super::super::ClaudeGenerationAuthority,
    invocation_id: uuid::Uuid,
) -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: authority.terminal_id,
        runtime_epoch: authority.runtime_epoch,
        invocation_id,
        hook_token: uuid::Uuid::new_v4(),
        helper_capability: uuid::Uuid::new_v4(),
    }
}

fn process(authority: &super::super::ClaudeGenerationAuthority) -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: authority.process.pid,
        start_time: authority.process.start_time,
    }
}

fn authenticate(path: &std::path::Path, credentials: ClaudeHookCredentials, pair: &ParkedPair) {
    let super::super::GenerationEvent::SessionStart { event_id, .. } = &pair.first_event else {
        unreachable!();
    };
    crate::claude_hooks::outbox::authenticate_session_start(
        path,
        credentials,
        crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
        *event_id,
        process(&pair.first),
    )
    .unwrap();
}

fn state_event() -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id: uuid::Uuid::new_v4(),
        provider_session_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        state: ClaudeHookState::Running,
    }
}
