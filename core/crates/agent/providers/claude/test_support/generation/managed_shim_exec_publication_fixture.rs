use super::super::{
    ClaudeGenerationAuthority, GenerationApplyDisposition, GenerationEvent, InvocationMode,
    SessionStartSource,
};
use super::observed_unmanaged_publication_test_support::{
    ABSOLUTE_CLAUDE, event_identity, observed,
};
use super::support::Fixture;
use crate::agent::{AgentBinding, AgentBindingStore, AgentRecoveryAvailability, AgentState};
use std::path::{Path, PathBuf};

const SHIM_DIRECTORY: &str = "bin";
const DETECTED_AT_MS: i64 = 1;
const UPDATED_AT_MS: i64 = 2;
const EXPIRED_HANDSHAKE_DEADLINE_NS: u64 = 1;

pub(super) struct Case {
    pub fixture: Fixture,
    pub authority: ClaudeGenerationAuthority,
    pub successor: crate::agent::ObservedProcess,
    pub predecessor: AgentBinding,
    pub shim: PathBuf,
}

impl Case {
    pub fn native(state: AgentState) -> Self {
        Self::new(state, ABSOLUTE_CLAUDE, vec![ABSOLUTE_CLAUDE.to_owned()])
    }

    pub fn new(state: AgentState, executable: &str, argv: Vec<String>) -> Self {
        Self::build(state, executable, argv, None)
    }

    pub fn resumed(state: AgentState, source: SessionStartSource) -> Self {
        Self::build_with_mode(
            state,
            ABSOLUTE_CLAUDE,
            vec![ABSOLUTE_CLAUDE.to_owned()],
            None,
            InvocationMode::Resume,
            source,
        )
    }

    pub fn expired_handshake(state: AgentState) -> Self {
        Self::build(
            state,
            ABSOLUTE_CLAUDE,
            vec![ABSOLUTE_CLAUDE.to_owned()],
            Some(EXPIRED_HANDSHAKE_DEADLINE_NS),
        )
    }

    fn build(
        state: AgentState,
        executable: &str,
        argv: Vec<String>,
        deadline: Option<u64>,
    ) -> Self {
        Self::build_with_mode(
            state,
            executable,
            argv,
            deadline,
            InvocationMode::Fresh,
            SessionStartSource::Fresh,
        )
    }

    fn build_with_mode(
        state: AgentState,
        executable: &str,
        argv: Vec<String>,
        deadline: Option<u64>,
        mode: InvocationMode,
        source: SessionStartSource,
    ) -> Self {
        let mut fixture = Fixture::new();
        fixture.begin.mode = mode;
        if let Some(deadline) = deadline {
            fixture.begin.handshake_deadline_ns = deadline;
        }
        let mut event = fixture.start(&fixture.begin.provider_session_id, source);
        let GenerationEvent::SessionStart {
            state: event_state, ..
        } = &mut event
        else {
            unreachable!();
        };
        *event_state = state;
        let GenerationEvent::SessionStart {
            transcript_path, ..
        } = &event
        else {
            unreachable!();
        };
        let shim = fixture
            .binding_path()
            .parent()
            .unwrap()
            .join(SHIM_DIRECTORY)
            .join(crate::agent::claude::constants::EXECUTABLE);
        let binding_store = AgentBindingStore::new(fixture.binding_path());
        fixture
            .lease
            .write_json(
                &fixture.binding_path(),
                &scanned_binding(&fixture, transcript_path, &shim),
            )
            .unwrap();
        let predecessor = binding_store
            .invalidate_in_namespace(&fixture.lease, fixture.begin.terminal_id)
            .unwrap()
            .unwrap();
        let initial = fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .unwrap();
        let identity = event_identity(&fixture, &event);
        let (authority, disposition, _) = fixture
            .store
            .transition_foreground_durable_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &initial,
                crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE,
                &event,
                Some(identity),
            )
            .unwrap();
        assert_eq!(disposition, GenerationApplyDisposition::Applied);
        let transcript = authority.transcript_path.clone().unwrap();
        let successor = observed(&authority, executable, argv, transcript);
        Self {
            fixture,
            authority,
            successor,
            predecessor,
            shim,
        }
    }
}

fn scanned_binding(fixture: &Fixture, transcript: &Path, shim: &Path) -> AgentBinding {
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: fixture.begin.terminal_id,
        kind: crate::agent::AgentKind::ClaudeCode,
        provider_session_id: fixture.begin.provider_session_id.clone(),
        executable: path_text(shim),
        cwd: path_text(&fixture.begin.cwd),
        evidence_path: path_text(transcript),
        data_home: path_text(&fixture.begin.data_home),
        process_pid: Some(fixture.begin.process.pid),
        process_start_time: Some(fixture.begin.process.start_time),
        state: AgentState::Unknown,
        recovery_availability: AgentRecoveryAvailability::UnmanagedLaunch,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    }
}

pub(super) fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
