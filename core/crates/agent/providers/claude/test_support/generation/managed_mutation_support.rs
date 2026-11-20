use super::support::*;
use crate::agent::claude::generation::{
    AuthorityAuthentication, AuthorityRuntimeIdentity, ClaudeGenerationAuthority,
    ClaudeGenerationStore, SessionStartSource,
};
use crate::agent::{AgentBindingStore, AgentState};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

const COMMAND: &str = "/bin/sh";
const COLUMNS: u16 = 120;
const NAME: &str = "managed-generation";
const NAMESPACE_LABEL: &str = "managed Claude generation test";
const ROWS: u16 = 32;

pub(super) struct ManagedFixture {
    pub(super) _directory: TempDir,
    pub(super) paths: Paths,
    pub(super) session_id: Uuid,
    pub(super) lease: NamespaceLease,
    pub(super) store: ClaudeGenerationStore,
    pub(super) begin: super::super::GenerationBegin,
}

impl ManagedFixture {
    pub(super) fn new(archived: bool, manifest_session: Option<Uuid>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::new(directory.path().join("home"));
        let session_id = Uuid::new_v4();
        paths.ensure_session(session_id).unwrap();
        std::fs::create_dir_all(paths.home().join("project")).unwrap();
        std::fs::create_dir_all(paths.home().join("claude")).unwrap();
        write_manifest(
            &paths,
            session_id,
            manifest_session.unwrap_or(session_id),
            archived,
        );
        let session = paths.session_dir(session_id);
        let lease = NamespaceLease::capture(&[(NAMESPACE_LABEL, session.as_path())]).unwrap();
        let store =
            ClaudeGenerationStore::from_binding_path(&paths.agent_binding(session_id)).unwrap();
        let begin = begin(&paths, session_id, &session);
        Self {
            _directory: directory,
            paths,
            session_id,
            lease,
            store,
            begin,
        }
    }

    pub(super) fn begin(&self) -> anyhow::Result<super::super::ClaudeGenerationAuthority> {
        self.store.begin_in_namespace(&self.lease, &self.begin)
    }

    pub(super) fn publish(&self) -> ClaudeGenerationAuthority {
        self.begin().unwrap();
        let event = self.start();
        let applied = self
            .store
            .apply_in_namespace(&self.lease, self.authentication(), &event)
            .unwrap()
            .0;
        AgentBindingStore::new(self.paths.agent_binding(self.session_id))
            .save_authoritative_in_namespace(
                &self.lease,
                &binding(&applied),
                &applied,
                self.runtime(),
            )
            .unwrap();
        applied
    }

    fn authentication(&self) -> AuthorityAuthentication {
        AuthorityAuthentication {
            terminal_id: self.begin.terminal_id,
            runtime_epoch: self.begin.runtime_epoch,
            invocation_id: self.begin.invocation_id,
            hook_token: self.begin.hook_token,
            helper_capability: self.begin.helper_capability,
            process: self.begin.process,
        }
    }

    fn runtime(&self) -> AuthorityRuntimeIdentity {
        AuthorityRuntimeIdentity {
            runtime_epoch: self.begin.runtime_epoch,
            worker: self.begin.worker,
        }
    }

    fn start(&self) -> super::super::GenerationEvent {
        let transcript = self
            .begin
            .data_home
            .join(crate::agent::claude::constants::EVIDENCE_DIR)
            .join("-project")
            .join(&self.begin.provider_session_id)
            .with_extension(crate::agent::constants::JSONL_EXTENSION);
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        std::fs::write(
            &transcript,
            format!(
                "{{\"type\":\"user\",\"sessionId\":{},\"cwd\":{}}}\n",
                serde_json::to_string(&self.begin.provider_session_id).unwrap(),
                serde_json::to_string(&self.begin.cwd).unwrap(),
            ),
        )
        .unwrap();
        super::super::GenerationEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: self.begin.provider_session_id.clone(),
            source: SessionStartSource::Fresh,
            cwd: self.begin.cwd.clone(),
            transcript_path: transcript,
            state: AgentState::Running,
        }
    }

    pub(super) fn intent(&self) -> PathBuf {
        self.paths.reboot_migration_intent(self.session_id)
    }

    pub(super) fn manifest(&self) -> PathBuf {
        self.paths.manifest(self.session_id)
    }
}

fn begin(
    paths: &Paths,
    terminal_id: Uuid,
    session: &std::path::Path,
) -> super::super::GenerationBegin {
    let settings = session.join("managed-generation-settings");
    let settings_identity = crate::paths::private_file::PrivateFileEntry::open(&settings)
        .unwrap()
        .create_bytes_exclusive(
            b"managed generation settings",
            crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES,
        )
        .unwrap();
    super::super::GenerationBegin {
        terminal_id,
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
        worker: super::super::AuthorityProcessIdentity {
            pid: WORKER_PID,
            start_time: WORKER_START,
        },
        process: super::super::AuthorityProcessIdentity {
            pid: PROCESS_PID,
            start_time: PROCESS_START,
        },
        settings_identity,
        handshake_boot_id: crate::process::boot_id(),
        handshake_deadline_ns: crate::process::monotonic_deadline_ns(
            crate::claude_hooks::CLAUDE_MANAGEMENT_HANDSHAKE_TIMEOUT,
        )
        .unwrap(),
        provider_session_id: session_id(),
        discover_session: false,
        mode: super::super::InvocationMode::Fresh,
        cwd: paths.home().join("project"),
        data_home: paths.home().join("claude"),
    }
}

fn write_manifest(paths: &Paths, directory_session: Uuid, manifest_session: Uuid, archived: bool) {
    let timestamp = crate::model::now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::process::runtime_constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: manifest_session,
        name: NAME.into(),
        command: COMMAND.into(),
        args: Vec::new(),
        cwd: paths.home().to_owned(),
        cols: COLUMNS,
        rows: ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(directory_session), &manifest).unwrap();
}
