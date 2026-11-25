use super::{Fixture, lock_recover};
use crate::agent::{AgentBinding, AgentKind};
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use anyhow::Result;
use uuid::Uuid;
use yolo_agent_control_contract::NativeControlWitness;
use yolo_agent_hook_generation::{
    EventKind, GenerationEvidence, GenerationRequest, GenerationScope, HookGenerationRecord,
    HookGenerationStore, HookMetadata, HookProviderKind, ProviderVersion, SessionStartSource,
};
use yolo_qwen::qwen::QwenAdapter;

const DATA_HOME: &str = "control-qwen-home";
const PROJECT: &str = "control-project";
const STEP: u64 = 1;

pub(super) struct NativeSource {
    pub fixture: Fixture,
    pub record: HookGenerationRecord,
    request: GenerationRequest,
    evidence: GenerationEvidence,
}

impl NativeSource {
    pub fn new() -> Self {
        let fixture = Fixture::new();
        let runtime = lock_recover(&fixture.state.record).clone();
        let home = fixture.state.paths.home().join(DATA_HOME);
        let cwd = fixture.state.paths.home().join(PROJECT);
        std::fs::create_dir_all(&cwd).unwrap();
        let session = Uuid::new_v4().hyphenated().to_string();
        let transcript = home
            .join(yolo_qwen::qwen::constants::EVIDENCE_DIR)
            .join(PROJECT)
            .join(yolo_qwen::qwen::constants::TRANSCRIPT_DIR)
            .join(format!("{session}.jsonl"));
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        let metadata = serde_json::json!({
            "sessionId": session.clone(),
            "cwd": cwd,
        });
        std::fs::write(&transcript, serde_json::to_vec(&metadata).unwrap()).unwrap();
        let authoritative = QwenAdapter
            .inspect_authoritative_evidence_with_hook(&transcript, &cwd, &home, || {})
            .unwrap()
            .unwrap();
        let evidence = GenerationEvidence::new(
            AgentKind::Other(yolo_qwen::qwen::constants::KIND.into()),
            yolo_qwen::qwen::constants::EXECUTABLE.to_owned(),
            session.clone(),
            cwd.clone(),
            home,
            transcript.clone(),
            Some(authoritative.identity()),
        )
        .unwrap();
        let request = GenerationRequest {
            scope: GenerationScope {
                terminal_id: runtime.terminal_id,
                runtime_epoch: runtime.runtime_epoch,
                agent: crate::agent::authority::AuthorityProcessIdentity {
                    pid: runtime.child_pid,
                    start_time: runtime.child_start_time,
                },
            },
            event_sequence: yolo_agent_hook_generation::FIRST_SEQUENCE,
            event_id: Uuid::new_v4(),
            provider: HookProviderKind::Other(yolo_qwen::qwen::constants::KIND.into()),
            provider_version: ProviderVersion::new(
                yolo_qwen_hooks::PROVIDER_MAJOR,
                yolo_qwen_hooks::PROVIDER_MINOR,
                yolo_qwen_hooks::PROVIDER_PATCH,
            ),
            provider_session_id: session,
            turn_id: None,
            agent_id: None,
            tool_name: None,
            tool_use_id: None,
            stop_hook_active: None,
            metadata: HookMetadata {
                cwd: Some(cwd),
                transcript_path: Some(transcript),
            },
            event: EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        };
        let record = apply(&fixture, &request, &evidence);
        Self {
            fixture,
            record,
            request,
            evidence,
        }
    }

    pub fn change_activity(&mut self) {
        self.request.event_sequence += STEP;
        self.request.event_id = Uuid::new_v4();
        self.request.event = EventKind::UserPromptSubmit;
        self.record = apply(&self.fixture, &self.request, &self.evidence);
    }

    pub fn revoke(&self) {
        store(&self.fixture)
            .revoke_in_namespace(
                self.fixture.state.namespace.lease(),
                self.fixture.terminal_id(),
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
        crate::worker::native_control::with_generic_hook_control_source(
            state, &durable, expected, callback,
        )
    }
}

fn store(fixture: &Fixture) -> HookGenerationStore {
    HookGenerationStore::from_binding_path(
        &fixture.state.paths.agent_binding(fixture.terminal_id()),
    )
    .unwrap()
}

fn apply(
    fixture: &Fixture,
    request: &GenerationRequest,
    evidence: &GenerationEvidence,
) -> HookGenerationRecord {
    let state = &fixture.state;
    let runtime = lock_recover(&state.record).clone();
    let store = store(fixture);
    let worker = crate::agent::authority::AuthorityProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let record = store
        .apply_in_namespace(
            state.namespace.lease(),
            request,
            worker,
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
