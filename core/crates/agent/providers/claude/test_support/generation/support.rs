use super::super::termination::SessionEndBoundary;
use super::super::*;
use crate::agent::constants::BINDING_SCHEMA_VERSION;
use crate::agent::{AgentBinding, AgentKind, AgentState};
use crate::namespace_lease::NamespaceLease;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

#[path = "session_fixture.rs"]
mod session_fixture;

pub(super) const PROCESS_PID: u32 = 41;
pub(super) const PROCESS_START: u64 = 42;
pub(super) const WORKER_PID: u32 = 51;
pub(super) const WORKER_START: u64 = 52;

pub(super) struct Fixture {
    _directory: TempDir,
    pub lease: NamespaceLease,
    pub store: ClaudeGenerationStore,
    pub begin: GenerationBegin,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let managed = session_fixture::prepare(directory.path());
        let session = managed.paths.session_dir(managed.session_id);
        let data_home = managed.data_home;
        let cwd = managed.cwd;
        let lease = NamespaceLease::capture(&[("generation test", session.as_path())]).unwrap();
        let binding = managed.paths.agent_binding(managed.session_id);
        let store = ClaudeGenerationStore::from_binding_path(&binding).unwrap();
        let settings = session.join("generation-test-settings");
        let settings_identity = crate::paths::private_file::PrivateFileEntry::open(&settings)
            .unwrap()
            .create_bytes_exclusive(
                b"generation test settings",
                crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES,
            )
            .unwrap();
        let begin = GenerationBegin {
            terminal_id: managed.session_id,
            runtime_epoch: Uuid::new_v4(),
            invocation_id: Uuid::new_v4(),
            hook_token: Uuid::new_v4(),
            helper_capability: Uuid::new_v4(),
            worker: AuthorityProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START,
            },
            process: AuthorityProcessIdentity {
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
            mode: InvocationMode::Fresh,
            cwd,
            data_home,
        };
        Self {
            _directory: directory,
            lease,
            store,
            begin,
        }
    }

    pub fn authentication(&self) -> AuthorityAuthentication {
        authentication(&self.begin)
    }

    pub fn authentication_for(&self, begin: &GenerationBegin) -> AuthorityAuthentication {
        authentication(begin)
    }

    pub fn runtime_for(&self, begin: &GenerationBegin) -> AuthorityRuntimeIdentity {
        AuthorityRuntimeIdentity {
            runtime_epoch: begin.runtime_epoch,
            worker: begin.worker,
        }
    }

    pub fn runtime(&self) -> AuthorityRuntimeIdentity {
        self.runtime_for(&self.begin)
    }

    pub fn binding_path(&self) -> PathBuf {
        self.store
            .path()
            .parent()
            .unwrap()
            .join(crate::agent::constants::BINDING_FILE_NAME)
    }

    pub fn transcript(&self, session_id: &str) -> PathBuf {
        self.begin
            .data_home
            .join(crate::agent::claude::constants::EVIDENCE_DIR)
            .join("-project")
            .join(session_id)
            .with_extension(crate::agent::constants::JSONL_EXTENSION)
    }

    pub fn start(&self, session_id: &str, source: SessionStartSource) -> GenerationEvent {
        let transcript_path = self.transcript(session_id);
        fs::create_dir_all(transcript_path.parent().unwrap()).unwrap();
        fs::write(
            &transcript_path,
            format!(
                "{{\"type\":\"user\",\"sessionId\":{},\"cwd\":{}}}\n",
                serde_json::to_string(session_id).unwrap(),
                serde_json::to_string(&self.begin.cwd).unwrap(),
            ),
        )
        .unwrap();
        GenerationEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: session_id.to_owned(),
            source,
            cwd: self.begin.cwd.clone(),
            transcript_path,
            state: AgentState::Running,
        }
    }
}

fn authentication(begin: &GenerationBegin) -> AuthorityAuthentication {
    AuthorityAuthentication {
        terminal_id: begin.terminal_id,
        runtime_epoch: begin.runtime_epoch,
        invocation_id: begin.invocation_id,
        hook_token: begin.hook_token,
        helper_capability: begin.helper_capability,
        process: begin.process,
    }
}

pub(super) fn binding(authority: &ClaudeGenerationAuthority) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: authority.terminal_id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: authority.provider_session_id.clone(),
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        cwd: authority.cwd.to_string_lossy().into_owned(),
        evidence_path: authority
            .transcript_path
            .as_ref()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        data_home: authority.data_home.to_string_lossy().into_owned(),
        process_pid: Some(authority.process.pid),
        process_start_time: Some(authority.process.start_time),
        state: authority.state,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 1,
    }
}

pub(super) fn session_id() -> String {
    Uuid::new_v4().hyphenated().to_string()
}

pub(super) fn end(session_id: &str) -> GenerationEvent {
    end_with_reason(session_id, SessionEndReason::Other)
}

pub(super) fn end_with_reason(session_id: &str, reason: SessionEndReason) -> GenerationEvent {
    GenerationEvent::SessionEnd {
        event_id: Uuid::new_v4(),
        provider_session_id: session_id.to_owned(),
        reason,
        boundary: SessionEndBoundary::from_reason(reason),
    }
}

pub(super) fn assert_path_exists(path: &Path) {
    assert!(path.exists());
}
