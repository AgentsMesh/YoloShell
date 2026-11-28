use super::binding::ProjectedBinding;
use crate::agent::authority::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use crate::agent::codex::authority::CodexBindingAuthority;
use crate::agent::launch::{ResumeIntent, apply_resume_with_registry};
use crate::agent::{AgentBinding, AgentBindingStore, AgentKind};
use crate::model::WorkerLaunchSpec;
use std::path::PathBuf;
use uuid::Uuid;

const SESSION_DIRECTORY: &str = "projection-authority-session";
const NAMESPACE_LABEL: &str = "Codex projection authority test";
const AUTH_TOKEN: &str = "projection-authority-token";
const SHELL_COMMAND: &str = "/bin/sh";
const CONTROLLER_GENERATION: u64 = 1;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const RESUME_LINE_ENDING: &str = "\n";

pub(super) struct PersistedProjection {
    _directory: tempfile::TempDir,
    namespace: crate::namespace_lease::NamespaceLease,
    store: AgentBindingStore,
    context: crate::agent::DetectionContext,
    original_runtime: AuthorityRuntimeIdentity,
    successor_runtime: AuthorityRuntimeIdentity,
}

impl PersistedProjection {
    pub(super) fn capture(projected: &ProjectedBinding) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let session = directory.path().join(SESSION_DIRECTORY);
        crate::paths::directory::PrivateDirectory::ensure(&session).unwrap();
        let namespace =
            crate::namespace_lease::NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)])
                .unwrap();
        let store =
            AgentBindingStore::new(session.join(crate::agent::constants::BINDING_FILE_NAME));
        let worker = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let runtime = AuthorityRuntimeIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: worker.pid,
                start_time: worker.start_time,
            },
        };
        let authority = CodexBindingAuthority::new_with_verified_resume_identity(
            runtime,
            projected.provider,
            projected.provider_version,
            projected.version,
            projected.status,
            projected.binding.clone(),
            &crate::process::boot_id(),
        )
        .unwrap();
        store
            .codex_authority_store()
            .unwrap()
            .publish_in_namespace_with_commit_fence(&namespace, store.path(), &authority, || Ok(()))
            .unwrap();
        let context = crate::agent::DetectionContext::new(
            projected.binding.terminal_id,
            PathBuf::from(&projected.binding.cwd),
            [(
                AgentKind::Codex,
                PathBuf::from(&projected.binding.data_home),
            )],
        );
        Self {
            _directory: directory,
            namespace,
            store,
            context,
            original_runtime: runtime,
            successor_runtime: AuthorityRuntimeIdentity {
                runtime_epoch: Uuid::new_v4(),
                worker: runtime.worker,
            },
        }
    }

    pub(super) fn load(&self) -> AgentBinding {
        assert!(
            self.store
                .load_authoritative_in_namespace(
                    &self.namespace,
                    &self.context,
                    self.successor_runtime,
                )
                .is_err()
        );
        self.store
            .load_authoritative_in_namespace(&self.namespace, &self.context, self.original_runtime)
            .unwrap()
            .unwrap()
    }
}

pub(in crate::worker::codex_hook) fn assert_explicit_resume(binding: &AgentBinding) {
    let mut launch = WorkerLaunchSpec {
        schema_version: crate::constants::WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: std::env::temp_dir().join(SESSION_DIRECTORY),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: CONTROLLER_GENERATION,
        command: SHELL_COMMAND.to_owned(),
        args: Vec::new(),
        cwd: PathBuf::from(&binding.cwd),
        environment: Default::default(),
        initial_input: None,
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        context: None,
    };
    let provider_runtime = yolo_agent_providers::builtin_provider_runtime();
    let registry = crate::agent::agent_registry_from_runtime(&provider_runtime);
    apply_resume_with_registry(&mut launch, binding, ResumeIntent::Explicit, &registry).unwrap();
    assert_eq!(
        launch
            .environment
            .get(crate::agent::codex::constants::DATA_HOME_ENV),
        Some(&binding.data_home)
    );
    let expected = format!("{}{RESUME_LINE_ENDING}", binding.provider_session_id);
    assert!(launch.initial_input.unwrap().ends_with(expected.as_bytes()));
}
