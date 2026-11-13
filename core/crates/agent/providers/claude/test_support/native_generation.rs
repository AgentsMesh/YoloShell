use crate::claude::generation::native::ClaudeNativeGenerationStore;
use crate::claude::{AuthoritativeClaudeEvidence, ClaudeAdapter};
use crate::claude_hooks::*;
use crate::namespace_lease::NamespaceLease;
use uuid::Uuid;

#[path = "generation/session_fixture.rs"]
mod session_fixture;

const PROJECT_COMPONENT: &str = "native-project";
const AGENT_PID: u32 = 51_001;
const AGENT_START: u64 = 51_002;
const WORKER_PID: u32 = 51_003;
const WORKER_START: u64 = 51_004;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub paths: crate::paths::Paths,
    pub namespace: NamespaceLease,
    pub store: ClaudeNativeGenerationStore,
    pub start: ClaudeNativeRequest,
    pub evidence: AuthoritativeClaudeEvidence,
    pub worker: crate::claude::generation::AuthorityProcessIdentity,
    pub boot_id: String,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let session = session_fixture::prepare(root.path());
        let id = Uuid::new_v4().to_string();
        let transcript = session
            .data_home
            .join(crate::claude::constants::EVIDENCE_DIR)
            .join(PROJECT_COMPONENT)
            .join(&id)
            .with_extension(crate::constants::JSONL_EXTENSION);
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        std::fs::write(
            &transcript,
            serde_json::to_vec(&serde_json::json!({
                "type": "user", "sessionId": id, "cwd": session.cwd,
            }))
            .unwrap(),
        )
        .unwrap();
        let evidence = ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                &transcript,
                &session.cwd,
                &session.data_home,
                || {},
            )
            .unwrap()
            .unwrap();
        let start = ClaudeNativeRequest {
            schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
            scope: ClaudeNativeScope {
                terminal_id: session.session_id,
                runtime_epoch: Uuid::new_v4(),
                agent: ClaudeProcessIdentity {
                    pid: AGENT_PID,
                    start_time: AGENT_START,
                },
            },
            event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
            event: ClaudeLifecycleEvent::SessionStart {
                event_id: Uuid::new_v4(),
                provider_session_id: id,
                source: ClaudeSessionStartSource::Fresh,
                cwd: session.cwd,
                transcript_path: transcript,
                state: ClaudeHookState::WaitingForInput,
            },
        };
        let namespace = NamespaceLease::capture(&[(
            PROJECT_COMPONENT,
            &session.paths.session_dir(session.session_id),
        )])
        .unwrap();
        let store = ClaudeNativeGenerationStore::from_binding_path(
            &session.paths.agent_binding(session.session_id),
        )
        .unwrap();
        Self {
            _root: root,
            paths: session.paths,
            namespace,
            store,
            start,
            evidence,
            worker: crate::claude::generation::AuthorityProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START,
            },
            boot_id: crate::process::boot_id(),
        }
    }

    pub(super) fn apply(
        &self,
        request: &ClaudeNativeRequest,
    ) -> anyhow::Result<crate::claude::generation::native::ClaudeNativeGeneration> {
        self.store.apply_in_namespace(
            &self.namespace,
            request,
            self.worker,
            &self.boot_id,
            &self.evidence,
            || Ok(()),
        )
    }

    pub(super) fn other_session(&self) -> (ClaudeNativeRequest, AuthoritativeClaudeEvidence) {
        let mut request = self.start.clone();
        let ClaudeLifecycleEvent::SessionStart {
            event_id,
            provider_session_id,
            cwd,
            transcript_path,
            ..
        } = &mut request.event
        else {
            unreachable!();
        };
        *event_id = Uuid::new_v4();
        *provider_session_id = Uuid::new_v4().to_string();
        transcript_path.set_file_name(provider_session_id.as_str());
        transcript_path.set_extension(crate::constants::JSONL_EXTENSION);
        std::fs::write(
            &transcript_path,
            serde_json::to_vec(&serde_json::json!({
                "type": "user", "sessionId": provider_session_id, "cwd": cwd,
            }))
            .unwrap(),
        )
        .unwrap();
        let evidence = ClaudeAdapter
            .inspect_authoritative_evidence_with_hook(
                transcript_path,
                cwd,
                &self.evidence.evidence().data_home,
                || {},
            )
            .unwrap()
            .unwrap();
        (request, evidence)
    }

    pub(super) fn state_event(&self, sequence: u64) -> ClaudeNativeRequest {
        ClaudeNativeRequest {
            schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
            scope: self.start.scope,
            event_sequence: sequence,
            event: ClaudeLifecycleEvent::StateChanged {
                event_id: Uuid::new_v4(),
                provider_session_id: self.start.event.provider_session_id().to_owned(),
                state: ClaudeHookState::Running,
            },
        }
    }
}
