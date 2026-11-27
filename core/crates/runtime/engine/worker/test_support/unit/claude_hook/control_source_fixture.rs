use super::{Fixture, lock_recover};
use crate::agent::AgentBinding;
use crate::agent::claude::constants as claude;
use crate::agent::claude::generation::native::{
    ClaudeNativeGeneration, ClaudeNativeGenerationStore,
};
use crate::agent::claude::{AuthoritativeClaudeEvidence, ClaudeAdapter};
use crate::claude_hooks::*;
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use anyhow::Result;
use uuid::Uuid;
use yolo_agent_control_contract::NativeControlWitness;

const DATA_HOME: &str = "control-claude-home";
const PROJECT: &str = "control-project";
const EVENT_STEP: u64 = 1;

pub(super) struct NativeSource {
    pub fixture: Fixture,
    pub record: ClaudeNativeGeneration,
    request: ClaudeNativeRequest,
    evidence: AuthoritativeClaudeEvidence,
}

impl NativeSource {
    pub fn new() -> Self {
        Self::with_fixture(Fixture::new())
    }

    pub fn with_provider_runtime(
        provider_runtime: yolo_agent_providers::ProviderRuntimeRegistry,
    ) -> Self {
        Self::with_fixture(Fixture::with_provider_runtime(provider_runtime))
    }

    fn with_fixture(fixture: Fixture) -> Self {
        let runtime = lock_recover(&fixture.state.record).clone();
        let home = fixture.state.paths.home().join(DATA_HOME);
        let cwd = fixture.state.paths.home().join(PROJECT);
        std::fs::create_dir_all(&cwd).unwrap();
        let id = Uuid::new_v4().to_string();
        let transcript = home
            .join(crate::agent::claude::constants::EVIDENCE_DIR)
            .join(PROJECT)
            .join(&id)
            .with_extension(crate::agent::constants::JSONL_EXTENSION);
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        let metadata = serde_json::json!({
            (claude::RECORD_TYPE_KEY): claude::USER_RECORD,
            (claude::SESSION_ID_KEY): id,
            (claude::CWD_KEY): cwd,
        });
        std::fs::write(&transcript, serde_json::to_vec(&metadata).unwrap()).unwrap();
        let evidence = ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(&transcript, &cwd, &home, || {})
            .unwrap()
            .unwrap();
        let request = ClaudeNativeRequest {
            schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
            scope: ClaudeNativeScope {
                terminal_id: runtime.terminal_id,
                runtime_epoch: runtime.runtime_epoch,
                agent: ClaudeProcessIdentity {
                    pid: runtime.child_pid,
                    start_time: runtime.child_start_time,
                },
            },
            event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
            event: ClaudeLifecycleEvent::SessionStart {
                event_id: Uuid::new_v4(),
                provider_session_id: id,
                source: ClaudeSessionStartSource::Fresh,
                cwd,
                transcript_path: transcript,
                state: ClaudeHookState::WaitingForInput,
            },
        };
        // Begin after native admission, then use the production generation reducer
        // and binding publisher. Executable admission is covered by its own tests.
        let record = apply(&fixture, &request, &evidence);
        Self {
            fixture,
            record,
            request,
            evidence,
        }
    }

    pub fn change_activity(&mut self) {
        self.request.event_sequence += EVENT_STEP;
        self.request.event = ClaudeLifecycleEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: self.record.binding.provider_session_id.clone(),
            state: ClaudeHookState::Running,
        };
        self.record = apply(&self.fixture, &self.request, &self.evidence);
    }

    pub fn revoke(&self) {
        store(&self.fixture)
            .revoke_in_namespace(
                self.fixture.state.namespace.lease(),
                &self.record,
                || Ok(()),
            )
            .unwrap();
    }

    pub fn with<T>(
        &self,
        expected: Option<&NativeControlWitness>,
        callback: impl FnOnce(
            &AgentBinding,
            &NativeControlWitness,
            &dyn Fn() -> Result<()>,
        ) -> Result<T>,
    ) -> Result<T> {
        let state = &self.fixture.state;
        let _generation = state.agent_generation_gate.try_lock().unwrap();
        let _mutation = state.mutation_gate.try_lock().unwrap();
        let durable = SessionDurableMutationGuard::acquire_participant(
            &state.paths,
            state.namespace.lease(),
            state.terminal_id(),
        )?;
        crate::worker::claude_hook::with_control_source(state, &durable, expected, callback)
    }
}

fn store(fixture: &Fixture) -> ClaudeNativeGenerationStore {
    ClaudeNativeGenerationStore::from_binding_path(
        &fixture.state.paths.agent_binding(fixture.terminal_id()),
    )
    .unwrap()
}

fn apply(
    fixture: &Fixture,
    request: &ClaudeNativeRequest,
    evidence: &AuthoritativeClaudeEvidence,
) -> ClaudeNativeGeneration {
    let state = &fixture.state;
    let store = store(fixture);
    let runtime = lock_recover(&state.record).clone();
    let record = store
        .apply_in_namespace(
            state.namespace.lease(),
            request,
            crate::worker::claude_hook::binding::runtime_identity(state).worker,
            &runtime.boot_id,
            evidence,
            || Ok(()),
        )
        .unwrap();
    store
        .publish_binding_in_namespace(state.namespace.lease(), &record, || Ok(()))
        .unwrap();
    record
}
