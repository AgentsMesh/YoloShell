use crate::agent::hooks::{
    EventKind, EventSequence, Generation, HookEnvelope, HookIdentity, HookMetadata,
    ProcessIdentity, HookProviderKind,
};
use crate::codex_hooks::{CodexHookArtifacts, CodexHookIntake};
use tempfile::TempDir;
use uuid::Uuid;

pub(super) const CONCURRENT_WRITERS: usize = 8;
pub(super) const EXCLUSIVE_FILE_NAME: &str = "exclusive-publication.json";
pub(super) const HOME_DIRECTORY: &str = "home";
pub(super) const PROVIDER_SESSION: &str = "codex-session";
pub(super) const TEST_PROCESS_PID: u32 = 71;
pub(super) const TEST_PROCESS_START: u64 = 81;
pub(super) const TEST_TURN: &str = "turn";
pub(super) const UNKNOWN_ENTRY_PREFIX: &str = "unknown-entry-";

pub(super) struct IntakeFixture {
    _root: TempDir,
    pub(super) identity: HookIdentity,
    pub(super) artifacts: CodexHookArtifacts,
}

impl IntakeFixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::Paths::new(root.path().join(HOME_DIRECTORY));
        let identity = identity(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let artifacts = artifacts(&paths, &identity);
        let intake =
            CodexHookIntake::for_identity(artifacts.intake_directory(), &identity).unwrap();
        drop(intake.acquire_writer_fence().unwrap());
        Self {
            _root: root,
            identity,
            artifacts,
        }
    }

    pub(super) fn intake(&self) -> CodexHookIntake {
        CodexHookIntake::for_identity(self.artifacts.intake_directory(), &self.identity).unwrap()
    }

    pub(super) fn envelope(&self) -> HookEnvelope {
        envelope(&self.identity)
    }
}

pub(super) fn artifacts(
    paths: &crate::paths::Paths,
    identity: &HookIdentity,
) -> CodexHookArtifacts {
    CodexHookArtifacts::new(
        paths,
        identity.terminal_id,
        identity.runtime_epoch,
        identity.invocation_id,
    )
    .unwrap()
}

pub(super) fn identity(session_id: Uuid, runtime_epoch: Uuid, invocation_id: Uuid) -> HookIdentity {
    HookIdentity {
        terminal_id: session_id,
        runtime_epoch,
        invocation_id,
        hook_token: Uuid::new_v4(),
        process: ProcessIdentity::new(TEST_PROCESS_PID, TEST_PROCESS_START),
        generation: Generation::INITIAL,
    }
}

pub(super) fn envelope(identity: &HookIdentity) -> HookEnvelope {
    HookEnvelope {
        schema_version: crate::agent::hooks::CURRENT_SCHEMA_VERSION,
        event_id: Uuid::nil(),
        sequence: EventSequence::ZERO,
        provider: HookProviderKind::Codex,
        provider_version: crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        identity: identity.clone(),
        provider_session_id: PROVIDER_SESSION.to_owned(),
        turn_id: Some(TEST_TURN.to_owned()),
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: HookMetadata::default(),
        event: EventKind::UserPromptSubmit,
    }
}
