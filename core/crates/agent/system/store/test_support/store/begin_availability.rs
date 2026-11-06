use super::*;
use crate::agent::claude::generation::{
    AuthorityProcessIdentity, AuthorityRuntimeIdentity, GenerationBegin, GenerationEvent,
    InvocationMode, SessionStartSource,
};
use crate::agent::{AgentKind, AgentRecoveryAvailability};
use crate::namespace_lease::NamespaceLease;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

#[path = "manifest.rs"]
mod manifest;

const SESSION_SCOPE: &str = "managed begin availability";
const SETTINGS_FILE: &str = "settings.json";
const PROJECT_DIRECTORY: &str = "project";
const TRANSCRIPT_SCOPE: &str = "-project";
const FILE_CONTENTS: &[u8] = b"{}\n";
const PROCESS_PID: u32 = 41;
const PROCESS_START: u64 = 42;
const WORKER_PID: u32 = 51;
const WORKER_START: u64 = 52;
const DETECTED_AT_MS: i64 = 1;
const UPDATED_AT_MS: i64 = 2;

pub(super) struct Fixture {
    _directory: TempDir,
    pub(super) lease: NamespaceLease,
    pub(super) bindings: AgentBindingStore,
    begin: GenerationBegin,
}

impl Fixture {
    pub(super) fn new(state: AgentState, availability: AgentRecoveryAvailability) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let terminal_id = Uuid::new_v4();
        let paths = crate::paths::Paths::new(root.join("home"));
        paths.ensure_session(terminal_id).unwrap();
        let session = paths.session_dir(terminal_id);
        let lease = NamespaceLease::capture(&[(SESSION_SCOPE, session.as_path())]).unwrap();
        let bindings = AgentBindingStore::new(paths.agent_binding(terminal_id));
        let settings = session.join(SETTINGS_FILE);
        let settings_identity = crate::paths::private_file::PrivateFileEntry::open(&settings)
            .unwrap()
            .create_bytes_exclusive(
                FILE_CONTENTS,
                crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES,
            )
            .unwrap();
        let cwd = root.join(PROJECT_DIRECTORY);
        let data_home = root.join(crate::agent::claude::constants::DEFAULT_DATA_HOME_DIR);
        fs::create_dir(&cwd).unwrap();
        fs::create_dir(&data_home).unwrap();
        manifest::write(&paths, terminal_id, &cwd);
        let begin = generation_begin(terminal_id, settings_identity, cwd, data_home);
        lease
            .write_json(&bindings.path, &stale_binding(&begin, state, availability))
            .unwrap();
        Self {
            _directory: directory,
            lease,
            bindings,
            begin,
        }
    }

    pub(super) fn invalidate(&self) -> AgentBinding {
        self.bindings
            .invalidate_in_namespace(&self.lease, self.begin.terminal_id)
            .unwrap()
            .unwrap()
    }

    pub(super) fn persisted(&self) -> AgentBinding {
        self.bindings
            .read_raw_in_namespace(&self.lease)
            .unwrap()
            .unwrap()
    }

    pub(super) fn generations(&self) -> ClaudeGenerationStore {
        ClaudeGenerationStore::from_binding_path(self.bindings.path()).unwrap()
    }

    pub(super) fn begin_generation(&self) -> ClaudeGenerationAuthority {
        self.bindings
            .begin_managed_claude_in_namespace(&self.lease, &self.begin, false)
            .unwrap()
    }

    pub(super) fn runtime(&self) -> AuthorityRuntimeIdentity {
        AuthorityRuntimeIdentity {
            runtime_epoch: self.begin.runtime_epoch,
            worker: self.begin.worker,
        }
    }

    pub(super) fn session_start(&self) -> GenerationEvent {
        let transcript = transcript(&self.begin);
        fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        fs::write(&transcript, FILE_CONTENTS).unwrap();
        GenerationEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: self.begin.provider_session_id.clone(),
            source: SessionStartSource::Fresh,
            cwd: self.begin.cwd.clone(),
            transcript_path: transcript,
            state: AgentState::Running,
        }
    }

    pub(super) fn transcript_identity(&self) -> crate::paths::identity::ObjectIdentity {
        crate::agent::claude::durable_transcript_identity(
            &self.begin.data_home,
            &transcript(&self.begin),
        )
        .unwrap()
        .unwrap()
    }
}

fn transcript(begin: &GenerationBegin) -> std::path::PathBuf {
    begin
        .data_home
        .join(crate::agent::claude::constants::EVIDENCE_DIR)
        .join(TRANSCRIPT_SCOPE)
        .join(&begin.provider_session_id)
        .with_extension(crate::agent::constants::JSONL_EXTENSION)
}

fn generation_begin(
    terminal_id: Uuid,
    settings_identity: crate::paths::identity::FileIdentity,
    cwd: std::path::PathBuf,
    data_home: std::path::PathBuf,
) -> GenerationBegin {
    GenerationBegin {
        terminal_id,
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
        provider_session_id: Uuid::new_v4().hyphenated().to_string(),
        discover_session: false,
        mode: InvocationMode::Fresh,
        cwd,
        data_home,
    }
}

fn stale_binding(
    begin: &GenerationBegin,
    state: AgentState,
    availability: AgentRecoveryAvailability,
) -> AgentBinding {
    let active = state != AgentState::Exited;
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: begin.terminal_id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: begin.provider_session_id.clone(),
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        cwd: begin.cwd.to_string_lossy().into_owned(),
        evidence_path: transcript(begin).to_string_lossy().into_owned(),
        data_home: begin.data_home.to_string_lossy().into_owned(),
        process_pid: active.then_some(PROCESS_PID),
        process_start_time: active.then_some(PROCESS_START),
        state,
        recovery_availability: availability,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    }
}
