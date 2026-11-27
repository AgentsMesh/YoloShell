use crate::agent::claude::generation::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::namespace_lease::NamespaceLease;
use crate::{
    AgentBinding, AgentBindingStore, AgentKind, AgentRecoveryAvailability, AgentState,
    DetectionContext,
};
use std::path::PathBuf;
use uuid::Uuid;

#[path = "store/manifest.rs"]
mod manifest;
#[path = "native_fixture_provider.rs"]
mod provider;

pub(super) const AGENT_PID: u32 = 91_001;
pub(super) const AGENT_START: u64 = 91_002;
const WORKER_PID: u32 = 91_003;
const WORKER_START: u64 = 91_004;
const HOME: &str = "yolo-home";
const PROJECT: &str = "project";
const PROVIDER_HOME: &str = "provider-home";
const NAMESPACE: &str = "native-store-fixture";

pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    pub paths: crate::paths::Paths,
    pub namespace: NamespaceLease,
    pub context: DetectionContext,
    pub store: AgentBindingStore,
    pub runtime: AuthorityRuntimeIdentity,
    pub binding: AgentBinding,
    pub source_path: PathBuf,
}

impl Fixture {
    pub fn new(kind: AgentKind) -> Self {
        Self::for_process(
            kind,
            crate::process::ProcessIdentity {
                pid: AGENT_PID,
                start_time: AGENT_START,
            },
        )
    }

    pub fn for_process(kind: AgentKind, process: crate::process::ProcessIdentity) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let paths = crate::paths::Paths::new(root.join(HOME));
        let session_id = Uuid::new_v4();
        paths.ensure_session(session_id).unwrap();
        let cwd = root.join(PROJECT);
        let data_home = root.join(PROVIDER_HOME);
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&data_home).unwrap();
        manifest::write(&paths, session_id, &cwd);
        let namespace =
            NamespaceLease::capture(&[(NAMESPACE, &paths.session_dir(session_id))]).unwrap();
        let context =
            DetectionContext::new(session_id, cwd.clone(), [(kind.clone(), data_home.clone())]);
        let store = AgentBindingStore::new(paths.agent_binding(session_id));
        let runtime = AuthorityRuntimeIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START,
            },
        };
        let agent_session = Uuid::new_v4().to_string();
        let evidence = provider::evidence(&kind, &data_home, &cwd, &agent_session);
        let now = crate::model::now_ms();
        let mut binding = AgentBinding {
            schema_version: crate::constants::BINDING_SCHEMA_VERSION,
            terminal_id: session_id,
            kind: kind.clone(),
            provider_session_id: agent_session,
            executable: match kind {
                AgentKind::ClaudeCode => crate::claude::constants::EXECUTABLE,
                AgentKind::Other(kind) if kind == yolo_qwen_identity::KIND => "qwen",
                _ => crate::codex::constants::EXECUTABLE,
            }
            .to_owned(),
            cwd: cwd.to_string_lossy().into_owned(),
            data_home: data_home.to_string_lossy().into_owned(),
            evidence_path: evidence.to_string_lossy().into_owned(),
            process_pid: Some(process.pid),
            process_start_time: Some(process.start_time),
            state: AgentState::Running,
            recovery_availability: AgentRecoveryAvailability::Available,
            detected_at_ms: now,
            updated_at_ms: now,
        };
        let source_path = provider::publish(&store, &namespace, runtime, &mut binding);
        Self {
            _directory: directory,
            paths,
            namespace,
            context,
            store,
            runtime,
            binding,
            source_path,
        }
    }

    pub fn load(&self) -> anyhow::Result<Option<AgentBinding>> {
        self.store
            .load_authoritative_in_namespace(&self.namespace, &self.context, self.runtime)
    }

    pub fn repair(&self) -> anyhow::Result<Option<AgentBinding>> {
        self.store
            .repair_native_binding_in_namespace_with_commit_fence(
                &self.namespace,
                &self.context,
                self.runtime,
                || Ok(()),
            )
    }
}
