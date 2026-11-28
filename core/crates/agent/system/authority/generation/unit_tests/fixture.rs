use std::path::Path;
use yolo_agent_authority_contract::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use yolo_agent_contract::{AgentKind, AgentState};
use crate::{
    EventKind, GenerationEvidence, GenerationRequest, GenerationScope, HookGenerationRecord,
    HookGenerationStore, HookMetadata, HookProviderKind, ProviderVersion, SessionEndReason,
    SessionStartSource,
};
use yolo_hooks_runtime::ProcessIdentity;
use yolo_paths::identity::ObjectIdentity;
use yolo_paths::namespace_lease::NamespaceLease;

pub(super) const SESSION: &str = "e524f9d8-8829-4858-b0a2-fb73a14cd53c";
pub(super) const NEXT_SESSION: &str = "fe2897a5-234e-4ccd-bbba-15fc47e18a4b";
pub(super) const TRANSCRIPT: &str = "transcript.jsonl";
pub(super) const NEXT_TRANSCRIPT: &str = "next.jsonl";
pub(super) const VERSION: ProviderVersion = ProviderVersion::new(0, 26, 2);

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) namespace: NamespaceLease,
    pub(super) store: HookGenerationStore,
    pub(super) scope: GenerationScope,
    pub(super) worker: AuthorityProcessIdentity,
    pub(super) cwd: std::path::PathBuf,
    pub(super) data_home: std::path::PathBuf,
    pub(super) transcript: std::path::PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let cwd = std::fs::canonicalize(root.path()).unwrap();
        let data_home = cwd.join("qwen-home");
        std::fs::create_dir(&data_home).unwrap();
        let transcript = cwd.join(TRANSCRIPT);
        std::fs::write(&transcript, b"{\"sessionId\":\"evidence\"}\n").unwrap();
        let binding = cwd.join("agent-binding.json");
        let namespace = NamespaceLease::capture(&[("test", root.path())]).unwrap();
        Self {
            store: HookGenerationStore::from_binding_path(&binding).unwrap(),
            scope: GenerationScope {
                terminal_id: uuid::Uuid::new_v4(),
                runtime_epoch: uuid::Uuid::new_v4(),
                agent: AuthorityProcessIdentity {
                    pid: 42,
                    start_time: 7,
                },
            },
            worker: AuthorityProcessIdentity {
                pid: 84,
                start_time: 9,
            },
            cwd,
            data_home,
            transcript,
            namespace,
            root,
        }
    }

    pub(super) fn evidence(&self, path: &Path) -> GenerationEvidence {
        let identity = std::fs::File::open(path)
            .ok()
            .and_then(|file| ObjectIdentity::from_file(&file).unwrap());
        let session = if path == self.transcript {
            SESSION
        } else {
            NEXT_SESSION
        };
        GenerationEvidence::new(
            AgentKind::Other(yolo_qwen_provider::identity::KIND.into()),
            "qwen".into(),
            session.into(),
            self.cwd.clone(),
            self.data_home.clone(),
            path.to_path_buf(),
            identity,
        )
        .unwrap()
    }

    pub(super) fn request(&self, sequence: u64, event: EventKind, session: &str, path: &Path) -> GenerationRequest {
        GenerationRequest {
            scope: self.scope,
            event_sequence: sequence,
            event_id: uuid::Uuid::new_v4(),
            provider: HookProviderKind::Other(yolo_qwen_provider::identity::KIND.into()),
            provider_version: VERSION,
            provider_session_id: session.into(),
            turn_id: Some("turn".into()),
            agent_id: None,
            tool_name: None,
            tool_use_id: None,
            stop_hook_active: None,
            metadata: HookMetadata {
                cwd: Some(self.cwd.clone()),
                transcript_path: Some(path.to_path_buf()),
            },
            event,
        }
    }

    pub(super) fn apply(&self, request: &GenerationRequest) -> anyhow::Result<HookGenerationRecord> {
        self.store.apply_in_namespace(
            &self.namespace,
            request,
            self.worker,
            "boot",
            &self.evidence(Path::new(&request.metadata.transcript_path.clone().unwrap())),
            || Ok(()),
        )
    }

    pub(super) fn runtime(&self) -> AuthorityRuntimeIdentity {
        AuthorityRuntimeIdentity {
            runtime_epoch: self.scope.runtime_epoch,
            worker: self.worker,
        }
    }

    pub(super) fn start(&self) -> GenerationRequest {
        self.request(
            1,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
            SESSION,
            &self.transcript,
        )
    }
}

